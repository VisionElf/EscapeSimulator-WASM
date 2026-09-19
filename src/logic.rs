// Decisions contain no process access or timer calls: trace tests use the same code.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub game: u64,
    pub menu: u64,
    pub scene: String,
    pub complete: bool,
    pub exiting: bool,
    pub menu_loading: bool,
    pub alpha: f32,
    pub tokens: Option<u64>,
}

impl Sample {
    pub fn gameplay(&self) -> bool {
        self.game != 0 && !self.scene.is_empty()
    }
    pub fn in_menu(&self) -> bool {
        self.menu != 0 && self.game == 0
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub auto_start: bool,
    pub auto_split: bool,
    pub auto_reset: bool,
    pub restart_reset: bool,
    pub any_level: bool,
    pub last_pack: bool,
    pub tutorial: bool,
    pub first_chamber: bool,
    pub il: bool,
    pub il_reset: bool,
    pub token: bool,
    pub all_tokens: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_start: true,
            auto_split: true,
            auto_reset: true,
            restart_reset: true,
            any_level: true,
            last_pack: false,
            tutorial: false,
            first_chamber: false,
            il: false,
            il_reset: false,
            token: false,
            all_tokens: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timer {
    Idle,
    Running,
    Paused,
    Ended,
}

#[derive(Default, Debug, PartialEq)]
pub struct Decision {
    pub start: bool,
    pub splits: u32,
    pub reset: bool,
    pub loading: bool,
    pub invalidated: bool,
}

#[derive(Default)]
pub struct Tracker {
    previous: Option<Sample>,
    timer: Option<Timer>,
    loading: bool,
    fade_seen: bool,
    completed: bool,
    started: bool,
    token_count: u64,
    token_baseline: Option<u64>,
    invalidated: bool,
    last_room: Option<(u64, String)>,
    menu_since_room: bool,
    restart_pending: bool,
}

impl Tracker {
    pub fn unavailable(&mut self, timer: Timer) -> Decision {
        // A data gap cannot be silently presented as correctly load-removed time.
        self.previous = None;
        self.last_room = None;
        self.restart_pending = false;
        self.token_baseline = None;
        self.timer = Some(timer);
        if timer == Timer::Idle {
            self.invalidated = false;
        }
        if matches!(timer, Timer::Running | Timer::Paused) {
            self.invalidated = true;
        }
        Decision {
            loading: true,
            invalidated: self.invalidated,
            ..Decision::default()
        }
    }

    pub fn step(&mut self, sample: Sample, timer: Timer, opt: &Options) -> Decision {
        let reset = self.timer.is_some_and(|old| old != Timer::Idle) && timer == Timer::Idle;
        let starting =
            timer == Timer::Running && !matches!(self.timer, Some(Timer::Running | Timer::Paused));
        self.timer = Some(timer);
        if starting {
            self.token_count = 0;
        }
        if reset {
            self.completed = sample.complete;
            self.started = false;
            self.token_count = 0;
            self.token_baseline = sample.tokens;
            self.invalidated = false;
        }
        let mut out = Decision::default();
        let Some(old) = self.previous.replace(sample.clone()) else {
            self.last_room = sample
                .gameplay()
                .then(|| (sample.game, sample.scene.clone()));
            self.menu_since_room = sample.in_menu();
            self.completed = sample.complete;
            self.started = false;
            self.loading =
                sample.exiting || sample.menu_loading || !sample.gameplay() || sample.alpha >= 0.99;
            self.fade_seen = false;
            // Attaching mid-room must not backfill historical tokens.
            self.token_count = 0;
            self.token_baseline = sample.tokens;
            out.loading = self.loading || self.invalidated;
            out.invalidated = self.invalidated;
            return out;
        };
        let new_room = sample.gameplay() && sample.game != old.game;
        let restarted = sample.gameplay()
            && !self.menu_since_room
            && self
                .last_room
                .as_ref()
                .is_some_and(|(id, scene)| *id != sample.game && *scene == sample.scene);
        if sample.in_menu() {
            self.menu_since_room = true;
            self.restart_pending = false;
        }
        if sample.gameplay() {
            self.last_room = Some((sample.game, sample.scene.clone()));
            self.menu_since_room = false;
        }
        let restart_reset =
            restarted && opt.auto_reset && opt.restart_reset && timer != Timer::Idle;
        if restart_reset {
            self.restart_pending = true;
        }
        let entering =
            (!old.exiting && sample.exiting) || (!old.menu_loading && sample.menu_loading);
        if entering || !sample.gameplay() {
            self.loading = true;
            if entering {
                self.fade_seen = false;
            }
        }
        if new_room {
            self.completed = sample.complete; // no completion edge on the first reading of an instance
            self.started = false;
            self.token_count = 0;
            self.token_baseline = sample.tokens;
        }
        let falling = sample.alpha < old.alpha && sample.alpha < 1.0;
        if self.loading && falling {
            self.fade_seen = true;
        }
        // Preserve the old script's menu pause. A fully transparent canvas also
        // recovers when the entire fade happened between two valid samples.
        if self.loading
            && sample.gameplay()
            && !sample.exiting
            && (self.fade_seen || sample.alpha <= 0.01)
        {
            self.loading = false;
            self.fade_seen = false;
        }
        if entering {
            self.token_count = 0;
            self.token_baseline = sample.tokens;
        }

        let eligible_start = opt.il
            || (opt.tutorial && sample.scene == "Toy1") // Tutorial part 1 (controls introduction)
            || (opt.first_chamber && sample.scene == "Adventure1"); // First Chamber
        out.reset = restart_reset
            || (matches!(timer, Timer::Running | Timer::Paused)
                && opt.auto_reset
                && opt.il
                && opt.il_reset
                && sample.in_menu()
                && !old.in_menu());
        out.start = (timer == Timer::Idle || restart_reset)
            && opt.auto_start
            && (eligible_start || self.restart_pending)
            && sample.gameplay()
            && (falling || (self.restart_pending && sample.alpha <= 0.01))
            && !self.started
            && !sample.exiting;
        if out.start {
            self.started = true;
            self.restart_pending = false;
        }

        let completion = !new_room
            && sample.gameplay()
            && sample.game == old.game
            && !self.completed
            && !old.complete
            && sample.complete
            && sample.scene != "Toy1"; // Tutorial part 1 (controls introduction)
        if completion {
            self.completed = true;
        }
        let level_split =
            completion && (opt.any_level || (opt.last_pack && last_room(&sample.scene)));
        let mut token_splits = 0;
        if !new_room && !reset && !entering && !self.loading && sample.gameplay() && !sample.exiting
        {
            if let (Some(before), Some(after)) = (self.token_baseline, sample.tokens) {
                if let Some(delta) = after.checked_sub(before).filter(|delta| *delta <= 64) {
                    let before_count = self.token_count;
                    self.token_count = self.token_count.saturating_add(delta);
                    token_splits = if opt.token {
                        delta as u32
                    } else if opt.all_tokens && before_count < 8 && self.token_count >= 8 {
                        1
                    } else {
                        0
                    };
                }
            }
        }
        // A seqlock collision is temporary; preserve the last valid counter so no pickup is lost.
        if sample.tokens.is_some() {
            self.token_baseline = sample.tokens;
        }
        if timer == Timer::Running && opt.auto_split && !out.reset && !self.invalidated {
            // Like ASL, a completion and pickup in the same sample share one split.
            out.splits = token_splits.max(u32::from(level_split));
        }
        if self.invalidated {
            out.start = false;
            out.reset = false;
        }
        out.loading = self.loading || self.invalidated;
        out.invalidated = self.invalidated;
        out
    }
}

pub fn last_room(scene: &str) -> bool {
    // Standalone rooms excluded from pack-ending splits:
    // Holiday* = Extra; Portal1 = Portal DLC; AmongUs1 = Among Us DLC;
    // PowerWash1 = PowerWash DLC; Talos1 = The Talos Principle DLC.
    matches!(
        scene,
        "Toy2" // Tutorial — Tutorial (the actual escape room)
            | "Adventure5" // Labyrinth of Egypt — The Top
            | "Space5" // Adrift in Space — Space Walk
            | "Victorian5" // Edgewood Mansion — The Underground Lab
            | "Corporation5" // Omega Corporation — Metaverse
            | "Race2" // Versus — Versus Expert (to be confirmed; Race1: Versus Apprentice)
            | "Spy4" // Spy DLC — Underwater Base
            | "Mayan4" // Mayan DLC — Monkey Temple
            | "Magic4" // Magic DLC — Divination Towers
            | "Western4" // Wild West DLC — The Train
            | "Dieselpunk4" // Steampunk DLC — The Helm Room
    )
}
