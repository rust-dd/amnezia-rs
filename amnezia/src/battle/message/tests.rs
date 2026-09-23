use super::*;
use crate::battle::{
    Battle, Phase,
    model::testkit::{build_1v2, build_party2},
};

fn advance(battle: &mut Battle, frames: u32, controls: Controls) {
    for _ in 0..frames {
        if battle.phase != Phase::Encounter {
            break;
        }
        encounter::advance(battle, controls);
    }
}

#[test]
fn waits_use_original_minimum_maximum_and_cancel_priority() {
    for (min, max) in [(4, 4), (8, 8), (20, 40), (30, 70), (36, 60), (0, 10)] {
        for fast in [false, true] {
            let mut wait = Wait::default();
            wait.set(min, max);
            for _ in 0..300 {
                assert!(!wait.ready(Controls { fast, hold: true }));
            }
            let expected = if fast { min.max(1) } else { max };
            for _ in 1..expected {
                assert!(!wait.ready(Controls { fast, hold: false }));
            }
            assert!(wait.ready(Controls { fast, hold: false }));
            assert!(wait.ready(Controls {
                fast: false,
                hold: true
            }));
        }
    }
}

#[test]
fn encounter_lists_each_enemy_with_exact_gaps_then_clears_before_commands() {
    let mut battle = build_1v2();
    battle.text.encounter = " appears!".into();
    battle.begin_encounter();
    advance(&mut battle, 3, Controls::default());
    assert!(battle.messages.console.visible().is_empty());
    advance(&mut battle, 1, Controls::default());
    assert_eq!(battle.messages.console.contents(), ["M1 appears!"]);
    assert!(battle.messages.console.visible().is_empty());
    advance(&mut battle, 6, Controls::default());
    assert_eq!(battle.messages.console.visible().len(), 1);
    advance(&mut battle, 1, Controls::default());
    assert_eq!(
        battle.messages.console.contents(),
        ["M1 appears!", "M1 appears!"]
    );
    advance(&mut battle, 68, Controls::default());
    assert_eq!(battle.phase, Phase::Encounter);
    advance(&mut battle, 1, Controls::default());
    assert_eq!(battle.phase, Phase::PartyCommand);
    assert!(battle.messages.console.contents().is_empty());
}

#[test]
fn encounter_pages_never_scroll_or_wordwrap_and_first_strike_has_its_own_page() {
    let mut battle = build_party2();
    battle.enemies = std::iter::repeat_with(|| build_party2().enemies.pop().unwrap())
        .take(5)
        .collect();
    battle.text.encounter = " appears with a deliberately long unwrapped original message".into();
    battle.text.special_combat = "First strike!".into();
    battle.first_strike = true;
    battle.begin_encounter();
    let fast = Controls {
        fast: true,
        hold: false,
    };
    advance(&mut battle, 25, fast);
    assert_eq!(battle.messages.console.contents().len(), 4);
    assert_eq!(battle.messages.console.visible().len(), 3);
    assert!(battle.messages.console.visible()[0].len() > 50);
    advance(&mut battle, 28, fast);
    assert_eq!(battle.messages.console.visible().len(), 4);
    advance(&mut battle, 1, fast);
    assert!(battle.messages.console.contents().is_empty());
    assert_eq!(battle.messages.console.visible().len(), 4);
    advance(&mut battle, 3, fast);
    assert_eq!(battle.messages.console.contents().len(), 1);
    advance(&mut battle, 29, fast);
    assert_eq!(battle.messages.console.contents(), ["First strike!"]);
    advance(&mut battle, 29, fast);
    assert_eq!(battle.phase, Phase::PartyCommand);
}

#[test]
fn encounter_clock_pauses_and_does_not_run_on_extra_render_updates() {
    let mut app = App::new();
    let mut battle = build_party2();
    battle.begin_encounter();
    app.insert_resource(battle)
        .init_resource::<crate::timing::SceneFrames>()
        .init_resource::<crate::timing::SceneWait>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, tick);
    for _ in 0..120 {
        app.update();
    }
    assert!(
        app.world()
            .resource::<Battle>()
            .messages
            .console
            .visible()
            .is_empty()
    );
    app.world_mut().resource_mut::<crate::timing::SceneWait>().0 = true;
    app.world_mut()
        .resource_mut::<crate::timing::SceneFrames>()
        .frame = 100;
    app.update();
    app.world_mut().resource_mut::<crate::timing::SceneWait>().0 = false;
    for frame in 101..=104 {
        app.world_mut()
            .resource_mut::<crate::timing::SceneFrames>()
            .frame = frame;
        app.update();
    }
    assert_eq!(
        app.world()
            .resource::<Battle>()
            .messages
            .console
            .visible()
            .len(),
        1
    );
}
