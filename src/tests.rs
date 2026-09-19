use crate::logic::*;

fn room(id: u64, alpha: f32, complete: bool, tokens: Option<u64>) -> Sample {
    Sample {
        game: id,
        menu: 0,
        scene: "Adventure1".into(),
        complete,
        exiting: false,
        menu_loading: false,
        alpha,
        tokens,
    }
}
fn menu() -> Sample {
    Sample {
        game: 0,
        menu: 99,
        scene: "MenuPC".into(),
        complete: false,
        exiting: false,
        menu_loading: false,
        alpha: 0.0,
        tokens: None,
    }
}
fn loading(alpha: f32) -> Sample {
    Sample {
        menu: 0,
        scene: String::new(),
        alpha,
        ..menu()
    }
}

#[test]
fn il_starts_unknown_rooms_only_when_idle_and_enabled() {
    for (il, timer, expected) in [
        (true, Timer::Idle, true),
        (false, Timer::Idle, false),
        (true, Timer::Running, false),
        (true, Timer::Paused, false),
        (true, Timer::Ended, false),
    ] {
        let mut t = Tracker::default();
        let o = Options {
            il,
            ..Options::default()
        };
        t.step(menu(), timer, &o);
        assert!(!t.step(loading(1.0), timer, &o).start);
        assert!(!t.step(loading(0.8), timer, &o).start);
        let mut r = room(1, 1.0, false, None);
        r.scene = "FutureExtraRoom".into();
        assert!(!t.step(r.clone(), timer, &o).start);
        r.alpha = 0.8;
        assert_eq!(t.step(r.clone(), timer, &o).start, expected);
        r.alpha = 0.5;
        assert!(!t.step(r, timer, &o).start);
    }
}

#[test]
fn observed_first_chamber_transition_starts_at_fade() {
    let mut t = Tracker::default();
    let o = Options {
        il: true,
        ..Options::default()
    };
    assert!(!t.step(menu(), Timer::Idle, &o).start);
    for a in [0.0, 0.1249, 0.3332, 0.9444, 1.0] {
        let d = t.step(loading(a), Timer::Idle, &o);
        assert!(d.loading);
        assert!(!d.start);
    }
    assert!(!t.step(room(1, 1.0, false, None), Timer::Idle, &o).start);
    let d = t.step(room(1, 0.9029, false, None), Timer::Idle, &o);
    assert!(d.start);
    assert!(!d.loading);
    assert!(!t.step(room(1, 0.8, false, None), Timer::Idle, &o).start);
}

#[test]
fn attach_to_completed_room_never_splits_historical_event() {
    let mut t = Tracker::default();
    let o = Options::default();
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
}

#[test]
fn completion_once_and_same_scene_retry_is_new_attempt() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        1
    );
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
    t.step(room(2, 1.0, false, None), Timer::Running, &o);
    t.step(room(2, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(2, 0.0, true, None), Timer::Running, &o).splits,
        1
    );
}

#[test]
fn completed_new_instance_does_not_create_edge() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(2, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
}

#[test]
fn batched_pickups_are_not_lost() {
    let mut t = Tracker::default();
    let o = Options {
        token: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, Some(5)), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, false, Some(8)), Timer::Running, &o)
            .splits,
        3
    );
    assert_eq!(
        t.step(room(1, 0.0, false, Some(8)), Timer::Running, &o)
            .splits,
        0
    );
}

#[test]
fn eighth_token_crossing_split_once() {
    let mut t = Tracker::default();
    let o = Options {
        all_tokens: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, Some(0)), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, false, Some(7)), Timer::Running, &o)
            .splits,
        0
    );
    assert_eq!(
        t.step(room(1, 0.0, false, Some(9)), Timer::Running, &o)
            .splits,
        1
    );
    assert_eq!(
        t.step(room(1, 0.0, false, Some(10)), Timer::Running, &o)
            .splits,
        0
    );
}

#[test]
fn counter_reset_and_initially_unavailable_telemetry_never_backfill() {
    let mut t = Tracker::default();
    let o = Options {
        token: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, Some(7)), Timer::Running, &o);
    for count in [Some(0), Some(900)] {
        assert_eq!(
            t.step(room(1, 0.0, false, count), Timer::Running, &o)
                .splits,
            0
        );
    }
    t.step(room(2, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(2, 0.0, false, Some(8)), Timer::Running, &o)
            .splits,
        0
    );
}

#[test]
fn temporary_token_read_collision_preserves_pickups() {
    let mut t = Tracker::default();
    let o = Options {
        token: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, Some(0)), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, false, None), Timer::Running, &o).splits,
        0
    );
    assert_eq!(
        t.step(room(1, 0.0, false, Some(1)), Timer::Running, &o)
            .splits,
        1
    );
}

#[test]
fn token_plugin_absence_does_not_disable_completion() {
    let mut t = Tracker::default();
    let o = Options {
        token: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        1
    );
}

#[test]
fn data_loss_invalidates_run_until_manual_reset() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert!(t.unavailable(Timer::Running).invalidated);
    assert!(
        t.step(room(1, 0.0, false, None), Timer::Running, &o)
            .invalidated
    );
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
    assert!(!t.step(menu(), Timer::Idle, &o).invalidated);
}

#[test]
fn reset_while_detached_recovers() {
    let mut t = Tracker::default();
    assert!(t.unavailable(Timer::Running).invalidated);
    assert!(!t.unavailable(Timer::Idle).invalidated);
    assert!(!t.step(menu(), Timer::Idle, &Options::default()).invalidated);
}

#[test]
fn reset_only_on_confirmed_menu_when_enabled() {
    let mut t = Tracker::default();
    let o = Options {
        il: true,
        il_reset: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert!(!t.step(loading(1.0), Timer::Running, &o).reset);
    assert!(t.step(menu(), Timer::Running, &o).reset);
    assert!(!t.step(menu(), Timer::Running, &o).reset);
}

#[test]
fn disabled_actions_and_paused_timer_do_not_split() {
    let mut t = Tracker::default();
    let o = Options {
        auto_split: false,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, true, None), Timer::Running, &o).splits,
        0
    );
    t.step(room(2, 0.0, false, None), Timer::Paused, &o);
    assert_eq!(
        t.step(room(2, 0.0, true, None), Timer::Paused, &Options::default())
            .splits,
        0
    );
}

#[test]
fn simultaneous_token_and_completion_share_one_split() {
    let mut t = Tracker::default();
    let o = Options {
        token: true,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, Some(0)), Timer::Running, &o);
    assert_eq!(
        t.step(room(1, 0.0, true, Some(1)), Timer::Running, &o)
            .splits,
        1
    );
}

#[test]
fn next_level_resumes_after_loading_without_reset() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, true, None), Timer::Running, &o);
    assert!(t.step(loading(1.0), Timer::Running, &o).loading);
    let mut next = room(2, 1.0, false, None);
    next.scene = "Adventure2".into();
    assert!(!t.step(next.clone(), Timer::Running, &o).reset);
    next.alpha = 0.9;
    let d = t.step(next, Timer::Running, &o);
    assert!(!d.loading);
    assert!(!d.invalidated);
    assert!(!d.reset);
}

#[test]
fn fully_faded_room_recovers_even_without_observing_fade_edge() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    t.step(loading(0.0), Timer::Running, &o);
    let mut next = room(2, 0.0, false, None);
    next.scene = "Adventure2".into();
    assert!(!t.step(next, Timer::Running, &o).loading);
}

#[test]
fn menu_remains_paused_until_gameplay_fade() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    t.step(loading(1.0), Timer::Running, &o);
    for a in [0.9, 0.4, 0.0] {
        let mut m = menu();
        m.alpha = a;
        assert!(t.step(m, Timer::Running, &o).loading);
    }
    t.step(loading(1.0), Timer::Running, &o);
    t.step(room(2, 1.0, false, None), Timer::Running, &o);
    assert!(
        !t.step(room(2, 0.8, false, None), Timer::Running, &o)
            .loading
    );
}

#[test]
fn restart_resets_then_starts_at_fade_even_on_a_second_level() {
    let mut t = Tracker::default();
    let o = Options::default();
    let mut r = room(1, 0.0, false, None);
    r.scene = "Victorian2".into();
    t.step(r.clone(), Timer::Running, &o);
    t.step(loading(1.0), Timer::Running, &o);
    r.game = 2;
    r.alpha = 1.0;
    let d = t.step(r.clone(), Timer::Running, &o);
    assert!(d.reset);
    assert!(!d.start);
    assert!(d.loading);
    r.alpha = 0.8;
    let d = t.step(r.clone(), Timer::Idle, &o);
    assert!(d.start);
    assert!(!d.reset);
    assert!(!d.loading);
    r.alpha = 0.5;
    assert!(!t.step(r, Timer::Running, &o).start);
}

#[test]
fn restart_can_reset_and_start_in_one_sample_if_fade_already_began() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    t.step(loading(1.0), Timer::Running, &o);
    let d = t.step(room(2, 0.7, false, None), Timer::Running, &o);
    assert!(d.reset);
    assert!(d.start);
    assert_eq!(d.splits, 0);
}

#[test]
fn same_room_after_menu_is_not_a_direct_restart() {
    let mut t = Tracker::default();
    let o = Options::default();
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    t.step(menu(), Timer::Running, &o);
    t.step(loading(1.0), Timer::Running, &o);
    assert!(!t.step(room(2, 0.7, false, None), Timer::Running, &o).reset);
}

#[test]
fn restart_setting_can_be_disabled() {
    let mut t = Tracker::default();
    let o = Options {
        restart_reset: false,
        ..Options::default()
    };
    t.step(room(1, 0.0, false, None), Timer::Running, &o);
    t.step(loading(1.0), Timer::Running, &o);
    assert!(!t.step(room(2, 0.7, false, None), Timer::Running, &o).reset);
}
