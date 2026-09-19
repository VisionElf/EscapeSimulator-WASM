use crate::logic::Sample;
use asr::{
    game_engine::unity::{
        mono::{Class, Image, Module},
        DictionaryOffsets,
    },
    Address, Address64, PointerSize, Process,
};

type Result<T> = std::result::Result<T, &'static str>;

pub struct Reader {
    pub mono: Module,
    game_class: Class,
    loading_class: Class,
    shared_class: Class,
    delegate_target: u32,
    cached_ptr: u32,
    game_delegate: u32,
    game_scene: u32,
    game_complete: u32,
    game_exiting: u32,
    canvas_instance: u32,
    canvas_alpha: u32,
    shared_objects: u32,
    net_menu: u32,
    menu_target: u32,
    menu_loading: u32,
    telemetry: Option<Telemetry>,
    telemetry_retry: u32,
    dictionary_layout: Option<DictionaryOffsets>,
}

struct Telemetry {
    class: Class,
    version: u32,
    ready: u32,
    sequence: u32,
    game: u32,
    count: u32,
}

fn class(p: &Process, m: &Module, image: &Image, name: &'static str) -> Result<Class> {
    image.get_class(p, m, name).ok_or(name)
}
fn field(p: &Process, m: &Module, c: &Class, name: &'static str) -> Result<u32> {
    c.get_field_offset(p, m, name).ok_or(name)
}
fn ptr(p: &Process, at: Address) -> Result<Address> {
    p.read_pointer(at, PointerSize::Bit64)
        .map_err(|_| "pointer read failed")
}
fn boolean(p: &Process, at: Address) -> Result<bool> {
    match p.read::<u8>(at).map_err(|_| "boolean read failed")? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err("invalid boolean"),
    }
}

impl Reader {
    pub fn attach(p: &Process) -> Result<Self> {
        let mono = Module::attach_auto_detect(p).ok_or("Mono runtime not ready or unsupported")?;
        if mono.get_pointer_size() != PointerSize::Bit64 {
            return Err("only Windows x64 is supported");
        }
        let core = mono
            .get_image(p, "EscapeSimulator.Core")
            .ok_or("EscapeSimulator.Core not ready")?;
        let system = mono.get_image(p, "mscorlib").ok_or("mscorlib not ready")?;
        let unity = mono
            .get_image(p, "UnityEngine.CoreModule")
            .ok_or("UnityEngine.CoreModule not ready")?;
        let game_class = class(p, &mono, &core, "Game")?;
        let loading_class = class(p, &mono, &core, "LoadingCanvas")?;
        let shared_class = class(p, &mono, &core, "Shared")?;
        let delegate = class(p, &mono, &system, "Delegate")?;
        let object = class(p, &mono, &unity, "Object")?;
        let net = class(p, &mono, &core, "Net")?;
        let menu = class(p, &mono, &core, "Menu")?;
        let callbacks = class(p, &mono, &core, "Menu+MenuCallbacks")?;
        Ok(Self {
            delegate_target: field(p, &mono, &delegate, "m_target")?,
            cached_ptr: field(p, &mono, &object, "m_CachedPtr")?,
            game_delegate: field(p, &mono, &game_class, "remoteSwitchActivation")?,
            game_scene: field(p, &mono, &game_class, "currentSceneName")?,
            game_complete: field(p, &mono, &game_class, "isLevelCompleted")?,
            game_exiting: field(p, &mono, &game_class, "exiting")?,
            canvas_instance: field(p, &mono, &loading_class, "instance")?,
            canvas_alpha: field(p, &mono, &loading_class, "alphaCurrent")?,
            shared_objects: field(p, &mono, &shared_class, "objects")?,
            net_menu: field(p, &mono, &net, "menuCallbacks")?,
            menu_target: field(p, &mono, &callbacks, "menu")?,
            menu_loading: field(p, &mono, &menu, "loadingLevel")?,
            mono,
            game_class,
            loading_class,
            shared_class,
            telemetry: None,
            telemetry_retry: 0,
            dictionary_layout: None,
        })
    }

    fn static_ref(&self, p: &Process, class: Class, offset: u32) -> Result<Address> {
        let table = class
            .get_static_table(p, &self.mono)
            .ok_or("static table unavailable")?;
        ptr(p, table + offset)
    }

    fn alive(&self, p: &Process, object: Address) -> Result<bool> {
        Ok(!object.is_null() && !ptr(p, object + self.cached_ptr)?.is_null())
    }

    pub fn sample(&mut self, p: &Process) -> Result<Sample> {
        let canvas = self.static_ref(p, self.loading_class, self.canvas_instance)?;
        // Created lazily on the first load. An absent canvas is valid only in a confirmed menu.
        let canvas_alive = self.alive(p, canvas)?;
        let alpha = if canvas_alive {
            p.read::<f32>(canvas + self.canvas_alpha)
                .map_err(|_| "alpha read failed")?
        } else {
            0.0
        };
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err("invalid loading alpha");
        }

        let callback = self.static_ref(p, self.game_class, self.game_delegate)?;
        let game = if callback.is_null() {
            Address::NULL
        } else {
            ptr(p, callback + self.delegate_target)?
        };
        if self.alive(p, game)? {
            if !canvas_alive {
                return Err("LoadingCanvas not ready in gameplay");
            }
            let scene = self
                .mono
                .read_string::<128>(p, game + self.game_scene)
                .map_err(|_| "scene field unreadable")?;
            let scene = String::from_utf16(scene.as_slice()).map_err(|_| "invalid scene UTF-16")?;
            if scene.is_empty() {
                return Err("Game initialization incomplete");
            }
            let complete = boolean(p, game + self.game_complete)?;
            let exiting = boolean(p, game + self.game_exiting)?;
            // Read the root again to reject a mixed snapshot during a scene replacement.
            if self.static_ref(p, self.game_class, self.game_delegate)? != callback {
                return Err("Game changed during read");
            }
            Ok(Sample {
                game: game.value(),
                menu: 0,
                scene,
                complete,
                exiting,
                menu_loading: false,
                alpha,
                tokens: self.tokens(p, game).ok(),
            })
        } else {
            // An opaque, live loading canvas is positive evidence of loading even while
            // Mono/menu objects are being replaced. No menu or gameplay event is inferred.
            if canvas_alive && alpha >= 0.99 {
                return Ok(Sample {
                    game: 0,
                    menu: 0,
                    scene: String::new(),
                    complete: false,
                    exiting: false,
                    menu_loading: false,
                    alpha,
                    tokens: None,
                });
            }
            let table = self
                .shared_class
                .get_static_table(p, &self.mono)
                .ok_or("Shared static table unavailable")?;
            let dict_at = table + self.shared_objects;
            if ptr(p, dict_at)?.is_null() {
                if !canvas_alive {
                    return Err("menu and loading canvas unavailable");
                }
                return Ok(Sample {
                    game: 0,
                    menu: 0,
                    scene: String::new(),
                    complete: false,
                    exiting: false,
                    menu_loading: false,
                    alpha,
                    tokens: None,
                });
            }
            // Shared.objects always has the same concrete Dictionary<SharedType, object>
            // type in this process. Resolve metadata once, not during every scene teardown.
            if self.dictionary_layout.is_none() {
                self.dictionary_layout = self.mono.get_dictionary_offsets(p, dict_at);
            }
            let layout = self
                .dictionary_layout
                .ok_or("Shared dictionary layout unavailable")?;
            let entries = self
                .mono
                .read_dictionary::<i32, Address64, 16>(p, layout, dict_at)
                .map_err(|_| "Shared dictionary unreadable")?;
            let net = entries
                .iter()
                .find(|(key, _)| *key == 0)
                .map(|(_, value)| Address::from(*value));
            let mut menu = Address::NULL;
            if let Some(net) = net.filter(|net| !net.is_null()) {
                let callbacks = ptr(p, net + self.net_menu)?;
                if !callbacks.is_null() {
                    let candidate = ptr(p, callbacks + self.menu_target)?;
                    if self.alive(p, candidate)? {
                        menu = candidate;
                    }
                }
            }
            let in_menu = !menu.is_null();
            if !canvas_alive && (!in_menu || boolean(p, menu + self.menu_loading)?) {
                return Err("LoadingCanvas not ready during transition");
            }
            Ok(Sample {
                game: 0,
                menu: menu.value(),
                scene: if in_menu {
                    "MenuPC".into()
                } else {
                    String::new()
                },
                complete: false,
                exiting: false,
                menu_loading: in_menu && boolean(p, menu + self.menu_loading)?,
                alpha,
                tokens: None,
            })
        }
    }

    fn tokens(&mut self, p: &Process, game: Address) -> Result<u64> {
        if self.telemetry.is_none() {
            if self.telemetry_retry > 0 {
                self.telemetry_retry -= 1;
                return Err("waiting for token telemetry");
            }
            self.telemetry_retry = 120;
            let image = self
                .mono
                .get_image(p, "EscapeSimulator.Telemetry")
                .ok_or("token telemetry not installed")?;
            let t = class(
                p,
                &self.mono,
                &image,
                "EscapeSimulator.Telemetry.TokenTelemetry",
            )?;
            self.telemetry = Some(Telemetry {
                class: t,
                version: field(p, &self.mono, &t, "ProtocolVersion")?,
                ready: field(p, &self.mono, &t, "Ready")?,
                sequence: field(p, &self.mono, &t, "Sequence")?,
                game: field(p, &self.mono, &t, "GameInstance")?,
                count: field(p, &self.mono, &t, "PickupCount")?,
            });
        }
        let t = self.telemetry.as_ref().unwrap();
        let table = t
            .class
            .get_static_table(p, &self.mono)
            .ok_or("token telemetry not ready")?;
        if p.read::<i32>(table + t.version)
            .map_err(|_| "protocol read failed")?
            != 1
        {
            return Err("unsupported token protocol");
        }
        if !boolean(p, table + t.ready)? {
            return Err("token hook not ready");
        }
        let sequence = p
            .read::<u32>(table + t.sequence)
            .map_err(|_| "token sequence unreadable")?;
        if sequence & 1 != 0 {
            return Err("token update in progress");
        }
        if ptr(p, table + t.game)? != game {
            return Err("token telemetry belongs to another room");
        }
        let count = p
            .read::<u64>(table + t.count)
            .map_err(|_| "token counter unreadable")?;
        if p.read::<u32>(table + t.sequence)
            .map_err(|_| "token sequence unreadable")?
            != sequence
            || !boolean(p, table + t.ready)?
        {
            return Err("token state changed during read");
        }
        Ok(count)
    }
}
