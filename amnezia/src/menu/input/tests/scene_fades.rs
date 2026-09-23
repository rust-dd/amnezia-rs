use super::*;
use crate::menu::{MemberAction, MenuView, SceneFlow, scene};
use crate::timing::{FrameClockSet, GameFrames, SceneWait};
use crate::transitions::{Transition, TransitionPlugin};

#[derive(Resource, Default)]
struct MapPaused(bool);

fn observe_pause(pause: crate::world::ScenePause, mut recorded: ResMut<MapPaused>) {
    recorded.0 = pause.paused();
}

fn fixture(open: bool, cursor: usize, screen: MenuScreen) -> App {
    let mut app = app_on(cursor, screen);
    app.world_mut().resource_mut::<MenuOpen>().0 = open;
    app.world_mut()
        .resource_mut::<Inventory>()
        .add_item(testkit::ITEM_HERB, 1);
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.skills = vec![testkit::heal_skill(1, "Heal", 1, 20)];
    data.actors[0].learnings = vec![amnezia_data::Learning {
        level: 1,
        skill_id: 1,
    }];
    app.add_plugins(TransitionPlugin)
        .init_resource::<SceneWait>()
        .init_resource::<MapPaused>()
        .init_resource::<crate::menu::view::clocks::Clock>()
        .add_systems(PreUpdate, capture_wait.in_set(FrameClockSet))
        .add_systems(
            Update,
            (crate::menu::view::clocks::update, observe_pause)
                .in_set(MenuView)
                .after(crate::menu::MenuInput),
        );
    scene::register(&mut app);
    app.update();
    app
}

fn capture_wait(transition: Res<Transition>, flow: Res<SceneFlow>, mut wait: ResMut<SceneWait>) {
    wait.0 = transition.busy() || flow.active();
}

fn tick(app: &mut App, keys: &[KeyCode]) {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

fn screen(app: &App) -> (bool, MenuScreen) {
    (
        app.world().resource::<MenuOpen>().0,
        app.world().resource::<MenuState>().screen,
    )
}

#[test]
fn every_menu_scene_changes_only_between_two_six_frame_fades() {
    use MenuScreen::*;
    let items = ItemList { cursor: 0 };
    let item_target = ItemTarget {
        item_id: testkit::ITEM_HERB,
        cursor: 0,
    };
    let skills = SkillList {
        member: 0,
        cursor: 0,
    };
    let skill_target = SkillTarget {
        member: 0,
        skill_id: 1,
        cursor: 0,
    };
    let member = |action| MemberSelect { action, cursor: 0 };
    let equip = Equip {
        member: 0,
        slot: 0,
        picking: None,
    };
    let cases = [
        (false, 0, Command, KeyCode::Escape, true, Command),
        (true, 0, Command, KeyCode::Escape, false, Command),
        (true, 0, Command, KeyCode::Enter, true, items),
        (true, 0, items, KeyCode::Escape, true, Command),
        (true, 0, items, KeyCode::Enter, true, item_target),
        (true, 0, item_target, KeyCode::Escape, true, items),
        (
            true,
            1,
            member(MemberAction::Skill),
            KeyCode::Enter,
            true,
            skills,
        ),
        (true, 1, skills, KeyCode::Escape, true, Command),
        (true, 1, skills, KeyCode::Enter, true, skill_target),
        (true, 1, skill_target, KeyCode::Escape, true, skills),
        (
            true,
            2,
            member(MemberAction::Equip),
            KeyCode::Enter,
            true,
            equip,
        ),
        (true, 2, equip, KeyCode::Escape, true, Command),
        (
            true,
            4,
            Command,
            KeyCode::Enter,
            true,
            EndGame { cursor: 1 },
        ),
        (
            true,
            4,
            EndGame { cursor: 1 },
            KeyCode::Enter,
            true,
            Command,
        ),
        (
            true,
            4,
            EndGame { cursor: 0 },
            KeyCode::Escape,
            true,
            Command,
        ),
        (
            true,
            2,
            member(MemberAction::Status),
            KeyCode::Enter,
            true,
            Status { member: 0 },
        ),
        (
            true,
            2,
            Status { member: 0 },
            KeyCode::Escape,
            true,
            member(MemberAction::Status),
        ),
    ];
    for (open, cursor, before, key, next_open, next) in cases {
        let mut app = fixture(open, cursor, before);
        tick(&mut app, &[key]);
        for age in 0..6 {
            assert!(app.world().resource::<MapPaused>().0);
            assert_eq!(screen(&app), (open, before));
            assert_eq!(app.world().resource::<Transition>().age(), age);
            assert!(app.world().resource::<Transition>().busy());
            assert!(app.world().resource::<SceneFlow>().active());
            tick(&mut app, &[]);
        }
        for age in 0..6 {
            assert!(app.world().resource::<MapPaused>().0);
            assert_eq!(screen(&app), (next_open, next));
            assert_eq!(app.world().resource::<Transition>().age(), age);
            assert!(app.world().resource::<Transition>().busy());
            tick(&mut app, &[]);
        }
        assert_eq!(screen(&app), (next_open, next));
        assert!(!app.world().resource::<Transition>().busy());
        assert!(!app.world().resource::<SceneFlow>().active());
        assert!(app.world().resource::<SceneWait>().0);
        assert!(app.world().resource::<MapPaused>().0);
        tick(&mut app, &[]);
        assert!(!app.world().resource::<SceneWait>().0);
        assert_eq!(app.world().resource::<MapPaused>().0, next_open);
    }
}

#[test]
fn member_selection_and_equipment_focus_changes_do_not_fade() {
    let mut app = fixture(true, 1, MenuScreen::Command);
    tick(&mut app, &[KeyCode::Enter]);
    assert!(matches!(screen(&app).1, MenuScreen::MemberSelect { .. }));
    assert!(!app.world().resource::<SceneFlow>().active());
    tick(&mut app, &[KeyCode::Escape]);
    assert_eq!(screen(&app).1, MenuScreen::Command);
    assert!(!app.world().resource::<Transition>().busy());
    let mut app = fixture(
        true,
        2,
        MenuScreen::Equip {
            member: 0,
            slot: 0,
            picking: None,
        },
    );
    tick(&mut app, &[KeyCode::Enter]);
    assert!(matches!(
        screen(&app).1,
        MenuScreen::Equip {
            picking: Some(0),
            ..
        }
    ));
    assert!(!app.world().resource::<SceneFlow>().active());
    tick(&mut app, &[KeyCode::Escape]);
    assert!(matches!(
        screen(&app).1,
        MenuScreen::Equip { picking: None, .. }
    ));
    assert!(!app.world().resource::<Transition>().busy());
}

#[test]
fn both_fades_and_the_final_frame_discard_decisions_without_replaying_them() {
    let mut app = fixture(true, 0, MenuScreen::Command);
    tick(&mut app, &[KeyCode::Enter]);
    for age in 1..=12 {
        tick(
            &mut app,
            &[
                KeyCode::Escape,
                KeyCode::Enter,
                KeyCode::Space,
                KeyCode::KeyS,
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
            ],
        );
        assert_eq!(
            screen(&app).1,
            if age < 6 {
                MenuScreen::Command
            } else {
                MenuScreen::ItemList { cursor: 0 }
            }
        );
        assert!(!app.world().resource::<SaveFiles>().active());
        assert_eq!(
            app.world()
                .resource::<Inventory>()
                .count(testkit::ITEM_HERB),
            1
        );
    }
    tick(&mut app, &[]);
    assert_eq!(screen(&app).1, MenuScreen::ItemList { cursor: 0 });
    assert!(!app.world().resource::<Transition>().busy());
    tick(&mut app, &[KeyCode::Enter]);
    assert!(app.world().resource::<SceneFlow>().active());
}

#[test]
fn zero_tick_updates_neither_finish_the_fade_nor_advance_window_clocks() {
    let mut app = fixture(true, 0, MenuScreen::Command);
    for _ in 0..11 {
        tick(&mut app, &[]);
    }
    tick(&mut app, &[KeyCode::Enter]);
    let color = app
        .world()
        .resource::<crate::menu::view::clocks::Clock>()
        .source_x(crate::menu::view::CursorId::Command);
    assert_eq!(color, 96.0);
    for _ in 0..30 {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(screen(&app).1, MenuScreen::Command);
        assert_eq!(app.world().resource::<Transition>().age(), 0);
        assert_eq!(
            app.world()
                .resource::<crate::menu::view::clocks::Clock>()
                .source_x(crate::menu::view::CursorId::Command),
            color
        );
    }
}
