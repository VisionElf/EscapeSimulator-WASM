use crate::{
    logic::{Decision, Options, Timer, Tracker},
    reader::Reader,
};
use asr::{
    future::next_tick,
    settings::{gui::Title, Gui},
    timer, Process,
};

asr::async_main!(stable);

#[derive(Gui)]
struct Settings {
    /// General — all run types
    _general: Title,
    /// Automatic start
    #[default = true]
    auto_start: bool,
    /// Automatic splits
    #[default = true]
    auto_split: bool,
    /// Automatic reset
    #[default = true]
    auto_reset: bool,
    /// Reset when restarting the current room
    ///
    /// Applies to both Full Game and IL runs. Requires Automatic reset.
    /// Automatic start restarts the timer at the room's opening fade.
    #[default = true]
    restart_reset: bool,
    /// Level splits — Full Game / IL packs
    _level_splits: Title,
    /// Split after every completed room
    ///
    /// Applies to all run types. The first tutorial area is excluded.
    #[default = true]
    level_completion_split: bool,
    /// Split only at the end of a pack
    ///
    /// Disable "Split after every completed room" to split only on pack endings.
    last_pack_split: bool,
    /// Full Game — automatic start
    _full_game: Title,
    /// Start with the Tutorial (Toy1)
    ///
    /// For Full Game routes that begin with the Tutorial. Requires Automatic start.
    tutorial_start: bool,
    /// Start with First Chamber (Adventure1)
    ///
    /// For Full Game / First # routes that skip the Tutorial. Requires Automatic start.
    first_chamber_start: bool,
    /// IL / individual packs
    _individual: Title,
    /// Start on any room
    ///
    /// Starts at the opening fade of any playable room, including extras and new rooms.
    /// Requires Automatic start. Direct room restarts use the General reset setting.
    il_mode: bool,
    /// Reset when returning to the menu (IL mode)
    ///
    /// Requires IL mode and Automatic reset. With this disabled, Game Time stays
    /// paused in the menu without resetting the run.
    il_mode_reset: bool,
    /// Tokens — optional companion plugin
    ///
    /// Requires EscapeSimulator.Telemetry and BepInEx. Available for all run types.
    _tokens: Title,
    /// Split on every token pickup
    ///
    /// Includes previously collected tokens. Requires Automatic splits.
    /// Disable level splits if you only want token splits.
    token_split: bool,
    /// Split on the eighth token pickup in a room
    ///
    /// Requires Automatic splits. If both token options are enabled, every-pickup
    /// splitting takes priority. Pickups before attaching are not counted.
    all_tokens_split: bool,
}

impl Settings {
    fn options(&self) -> Options {
        Options {
            auto_start: self.auto_start,
            auto_split: self.auto_split,
            auto_reset: self.auto_reset,
            restart_reset: self.restart_reset,
            any_level: self.level_completion_split,
            last_pack: self.last_pack_split,
            tutorial: self.tutorial_start,
            first_chamber: self.first_chamber_start,
            il: self.il_mode,
            il_reset: self.il_mode_reset,
            token: self.token_split,
            all_tokens: self.all_tokens_split,
        }
    }
}

fn timer_state() -> Timer {
    match timer::state() {
        timer::TimerState::Running => Timer::Running,
        timer::TimerState::Paused => Timer::Paused,
        timer::TimerState::Ended => Timer::Ended,
        _ => Timer::Idle,
    }
}

fn apply(d: Decision, last_timing: &mut Option<(bool, bool)>) {
    let timing = (d.loading, d.invalidated);
    if *last_timing != Some(timing) {
        asr::print_message(&format!(
            "[Escape Simulator] Game Time: {} | invalidated: {}",
            if d.loading { "paused" } else { "running" },
            d.invalidated
        ));
        *last_timing = Some(timing);
    }
    if d.reset {
        asr::print_message("[Escape Simulator] Reset: room restart or configured menu reset");
        timer::reset();
    }
    if d.start {
        asr::print_message("[Escape Simulator] Start: room fade began");
        timer::start();
    }
    if d.splits > 0 {
        asr::print_message(&format!("[Escape Simulator] Split events: {}", d.splits));
    }
    for _ in 0..d.splits {
        timer::split();
    }
    if d.loading {
        timer::pause_game_time();
    } else {
        timer::resume_game_time();
    }
    timer::set_variable(
        "ES timing",
        if d.invalidated {
            "INVALID - data lost; reset timer"
        } else {
            "OK"
        },
    );
}

fn status(last: &mut String, message: String) {
    if *last != message {
        asr::print_message(&format!("[Escape Simulator] {message}"));
        timer::set_variable("ES status", &message);
        *last = message;
    }
}

async fn main() {
    asr::set_tick_rate(60.0);
    asr::print_message(concat!(
        "Escape Simulator ASR ",
        env!("CARGO_PKG_VERSION"),
        " / direct Mono / token protocol 1"
    ));
    let mut settings = Settings::register();
    let mut tracker = Tracker::default();
    let mut last = String::new();
    let mut last_timing = None;
    loop {
        let Some(process) = Process::attach("Escape Simulator.exe") else {
            status(&mut last, "Waiting for game".into());
            apply(tracker.unavailable(timer_state()), &mut last_timing);
            // Avoid scanning the process list 60 times a second while the game is absent.
            for _ in 0..60 {
                next_tick().await;
            }
            continue;
        };
        let mut reader = None;
        let mut errors = 0u32;
        while process.is_open() {
            settings.update();
            if reader.is_none() {
                match Reader::attach(&process) {
                    Ok(bound) => {
                        reader = Some(bound);
                        errors = 0;
                    }
                    Err(error) => {
                        status(&mut last, format!("Waiting for Mono/fields: {error}"));
                        apply(tracker.unavailable(timer_state()), &mut last_timing);
                        for _ in 0..60 {
                            next_tick().await;
                        }
                        continue;
                    }
                }
            }
            match reader.as_mut().unwrap().sample(&process) {
                Ok(sample) => {
                    errors = 0;
                    let tokens = if sample.tokens.is_some() {
                        "ready"
                    } else {
                        "unavailable"
                    };
                    timer::set_variable("ES tokens", tokens);
                    let description = if sample.gameplay() {
                        format!(
                            "Room: {} | tokens: {}",
                            sample.scene,
                            sample
                                .tokens
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "unavailable".into())
                        )
                    } else if sample.in_menu() {
                        "Menu".into()
                    } else {
                        "Loading".into()
                    };
                    status(&mut last, description);
                    apply(
                        tracker.step(sample, timer_state(), &settings.options()),
                        &mut last_timing,
                    );
                }
                Err(error) => {
                    status(&mut last, format!("Data unavailable: {error}"));
                    apply(tracker.unavailable(timer_state()), &mut last_timing);
                    errors += 1;
                    if errors >= 120 {
                        reader = None;
                    }
                }
            }
            next_tick().await;
        }
    }
}
