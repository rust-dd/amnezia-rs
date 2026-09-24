use super::*;
use crate::battle::model::{BattleSe, testkit::build_party2};

fn app() -> App {
    let data = GameData {
        actors: vec![],
        items: crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root())),
        skills: crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root())),
    };
    let mut battle = build_party2();
    battle.skills = data.skills.clone();
    battle.phase = Phase::Command;
    battle.menu = MenuLevel::Skill;
    battle.members[0].known_skills = data.skills.iter().take(12).map(|skill| skill.id).collect();
    let mut inventory = Inventory::default();
    for item in data.items.iter().take(12) {
        inventory.add_item(item.id, 1);
    }
    let mut app = App::new();
    app.insert_resource(battle)
        .insert_resource(data)
        .insert_resource(inventory)
        .init_resource::<Windows>()
        .init_resource::<Terms>()
        .init_resource::<GameFrames>()
        .init_resource::<DirectionInput>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(
            Update,
            (
                crate::menu::update_directions,
                tick,
                crate::battle::input::command_input.run_if(motion::ready),
                motion::observe,
                observe,
            )
                .chain(),
        );
    motion::register(&mut app);
    app.update();
    app
}

fn step(app: &mut App, keys: &[KeyCode]) -> Vec<BattleSe> {
    for &key in keys {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    std::mem::take(&mut app.world_mut().resource_mut::<Battle>().pending_se)
}

fn list(app: &App, panel: Panel) -> &List {
    app.world().resource::<Windows>().get(panel)
}

#[test]
fn battle_scroll_holds_cursor_and_description_for_four_updates_in_both_directions() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().cursor = 6;
    step(&mut app, &[]);
    assert_eq!(step(&mut app, &[KeyCode::ArrowDown]), [BattleSe::Cursor]);
    assert_eq!(
        (
            list(&app, Panel::Skill).index,
            list(&app, Panel::Skill).offset
        ),
        (8, 0)
    );
    for offset in [4, 8, 12, 16] {
        step(&mut app, &[]);
        let list = list(&app, Panel::Skill);
        assert_eq!(list.offset, offset);
        assert_eq!(list.cursor_y, 48);
        assert_eq!(list.help_index, if offset == 16 { 8 } else { 6 });
        assert_eq!(list.cursor_index, list.help_index);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut().resource_mut::<Battle>().cursor = 2;
    step(&mut app, &[]);
    assert_eq!(list(&app, Panel::Skill).offset, 16);
    assert_eq!(step(&mut app, &[KeyCode::ArrowUp]), [BattleSe::Cursor]);
    for offset in [12, 8, 4, 0] {
        step(&mut app, &[]);
        let list = list(&app, Panel::Skill);
        assert_eq!(list.offset, offset);
        assert_eq!(list.cursor_y, 0);
        assert_eq!(list.help_index, if offset == 0 { 0 } else { 2 });
    }
}

#[test]
fn battle_navigation_uses_global_repetition_through_pauses_and_wraps_held_commands() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().phase = Phase::PartyCommand;
    app.insert_resource(crate::transitions::Transition::default());
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .start_for(crate::transitions::Kind::Fade, false, 0, IVec2::ZERO, 1000);
    for _ in 0..23 {
        assert!(step(&mut app, &[KeyCode::ArrowDown]).is_empty());
    }
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    assert_eq!(step(&mut app, &[]), [BattleSe::Cursor]);
    assert_eq!(app.world().resource::<Battle>().cursor, 1);
    for expected in [2, 0, 1] {
        for _ in 0..3 {
            assert!(step(&mut app, &[]).is_empty());
        }
        assert_eq!(step(&mut app, &[]), [BattleSe::Cursor]);
        assert_eq!(app.world().resource::<Battle>().cursor, expected);
    }
}

#[test]
fn battle_navigation_sounds_each_direction_before_decision_and_decision_beats_cancel() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().phase = Phase::PartyCommand;
    assert_eq!(
        step(
            &mut app,
            &[
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
                KeyCode::PageUp
            ]
        ),
        [BattleSe::Cursor; 4]
    );
    assert_eq!(app.world().resource::<Battle>().cursor, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    assert_eq!(
        step(
            &mut app,
            &[KeyCode::ArrowDown, KeyCode::Enter, KeyCode::Escape]
        ),
        [BattleSe::Cursor, BattleSe::Decision]
    );
    assert!(app.world().resource::<Battle>().phase == Phase::Resolve);
}

#[test]
fn inactive_lists_keep_viewports_and_clocks_until_a_new_battle() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().cursor = 11;
    step(&mut app, &[]);
    assert_eq!(list(&app, Panel::Skill).offset, 32);
    let frame = list(&app, Panel::Skill).cursor_frame;
    {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.menu_cursors[MenuLevel::Skill as usize] = 11;
        battle.menu = MenuLevel::Target;
        battle.pending_skill = Some(1);
        battle.cursor = 0;
    }
    step(&mut app, &[]);
    assert_eq!(list(&app, Panel::Skill).offset, 32);
    assert_eq!(list(&app, Panel::Skill).cursor_frame, frame);
    {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.menu = MenuLevel::Item;
        battle.cursor = 9;
    }
    step(&mut app, &[]);
    assert_eq!(list(&app, Panel::Item).offset, 16);
    assert_eq!(list(&app, Panel::Skill).offset, 32);
    {
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.generation += 1;
        battle.cursor = 0;
    }
    step(&mut app, &[]);
    assert!(
        app.world()
            .resource::<Windows>()
            .lists
            .iter()
            .all(|list| list.offset == 0)
    );
}

#[test]
fn empty_battle_lists_have_no_selectable_cursor_or_navigation_sound() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().members[0]
        .known_skills
        .clear();
    assert!(step(&mut app, &[KeyCode::ArrowDown, KeyCode::ArrowRight]).is_empty());
    assert_eq!(list(&app, Panel::Skill).count(), 0);
    assert_eq!(step(&mut app, &[KeyCode::Enter]), [BattleSe::Buzzer]);
}

#[test]
fn command_navigation_activates_after_the_slide_handoff_update() {
    let mut app = app();
    app.world_mut().resource_mut::<Battle>().phase = Phase::PartyCommand;
    step(&mut app, &[]);
    assert_eq!(step(&mut app, &[KeyCode::Enter]), [BattleSe::Decision]);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    for _ in 0..8 {
        assert!(step(&mut app, &[]).is_empty());
    }
    assert!(step(&mut app, &[KeyCode::ArrowDown]).is_empty());
    assert_eq!(app.world().resource::<Battle>().cursor, 0);
    assert_eq!(list(&app, Panel::Command).cursor_frame, 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    assert_eq!(step(&mut app, &[KeyCode::ArrowDown]), [BattleSe::Cursor]);
    assert_eq!(app.world().resource::<Battle>().cursor, 1);
    assert_eq!(list(&app, Panel::Command).cursor_frame, 1);
}
