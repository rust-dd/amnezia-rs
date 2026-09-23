use super::*;
use crate::battle::{BattleActive, model::testkit::build_party2};
use crate::dialogue::testing;
use crate::timing::GameFrames;

fn app(outcome: BattleOutcome) -> App {
    let mut app = App::new();
    testing::register_playback(&mut app);
    let mut battle = build_party2();
    battle.text.victory = "V".into();
    battle.text.exp_received.clear();
    battle.text.defeat = "D".into();
    battle.enemies[0].exp = 1;
    battle.enemies[0].gold = 0;
    battle.finish(outcome);
    app.insert_resource(battle)
        .insert_resource(BattleActive(true))
        .init_resource::<BattleFlow>()
        .init_resource::<Vitals>()
        .init_resource::<MessageOptions>()
        .init_resource::<MessagePosition>()
        .init_resource::<MessageTransparent>()
        .add_systems(Update, outcome_input.after(crate::dialogue::MessageUpdate));
    app
}

fn step(app: &mut App, key: Option<KeyCode>) {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    app.update();
}

#[test]
fn victory_types_original_control_codes_before_accepting_the_last_page() {
    let mut app = app(BattleOutcome::Victory);
    step(&mut app, Some(KeyCode::Enter));
    assert!(app.world().resource::<Dialogue>().active);
    assert_eq!(
        app.world().resource::<Dialogue>().boxes[0].lines,
        ["V\\|", "1\\."]
    );
    let mut spans = Vec::<(String, u32)>::new();
    for _ in 0..150 {
        if testing::ready(app.world()) {
            break;
        }
        step(&mut app, Some(KeyCode::Enter));
        let text = testing::text(app.world());
        if let Some((previous, frames)) = spans.last_mut()
            && previous == text
        {
            *frames += 1;
        } else {
            spans.push((text.to_owned(), 1));
        }
        assert!(!app.world().resource::<BattleFlow>().busy());
    }
    assert!(testing::ready(app.world()), "{spans:?}");
    assert_eq!(testing::text(app.world()), "V\n1");
    assert!(
        spans
            .iter()
            .any(|(text, frames)| text == "V" && *frames >= 61),
        "{spans:?}"
    );
    assert!(
        spans
            .iter()
            .any(|(text, frames)| text == "V\n1" && *frames >= 16),
        "{spans:?}"
    );
    step(&mut app, Some(KeyCode::Escape));
    assert!(!app.world().resource::<Dialogue>().active);
    for _ in 0..2 {
        step(&mut app, None);
    }
    assert!(app.world().resource::<BattleFlow>().busy());
}

#[test]
fn defeat_resets_message_options_but_waits_for_the_typed_message_to_close() {
    let mut app = app(BattleOutcome::Defeat);
    app.world_mut()
        .resource_mut::<MessagePosition>()
        .clone_from(&MessagePosition::Top);
    app.world_mut().resource_mut::<MessageTransparent>().0 = true;
    step(&mut app, Some(KeyCode::Enter));
    assert!(app.world().resource::<MessageOptions>().fixed);
    assert_eq!(
        *app.world().resource::<MessagePosition>(),
        MessagePosition::Bottom
    );
    assert!(!app.world().resource::<MessageTransparent>().0);
    assert!(!app.world().resource::<BattleFlow>().busy());
    for _ in 0..30 {
        if testing::ready(app.world()) {
            break;
        }
        step(&mut app, None);
    }
    assert!(testing::ready(app.world()));
    step(&mut app, Some(KeyCode::Enter));
    for _ in 0..2 {
        step(&mut app, None);
    }
    assert!(app.world().resource::<BattleFlow>().busy());
}

#[test]
fn escape_and_abort_leave_without_a_second_confirmation_or_reward_box() {
    for outcome in [BattleOutcome::Escape, BattleOutcome::Abort] {
        let mut app = app(outcome);
        step(&mut app, None);
        assert!(app.world().resource::<BattleFlow>().busy());
        assert!(!app.world().resource::<Dialogue>().active);
    }
}
