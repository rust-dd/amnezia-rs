use super::npcs::npc;
use super::*;
use crate::world::{AutoMove, EventSprite};

#[test]
fn an_autorun_pauses_its_own_autonomy_without_stopping_later_npcs_early() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![cmd(11410, 0, vec![100])], &[]);
    let second = npc(&mut app, 2, 6, vec![], &[]);
    app.world_mut()
        .entity_mut(first)
        .insert(AutoMove::new(3, 8, 4, 1));
    app.world_mut()
        .entity_mut(second)
        .insert(AutoMove::new(3, 8, 4, 2));
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        events.events[0].pages[0].trigger = 3;
        events.events[1].pages[0].trigger = 0;
    }
    app.update();
    assert_eq!(app.world().get::<EventSprite>(first).unwrap().tile_x, 1);
    assert_eq!(app.world().get::<EventSprite>(second).unwrap().tile_x, 7);
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
}

#[test]
fn an_autorun_can_queue_after_its_forced_route_starts_moving() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![switch_cmd(10, 0, 0)], &[1, 8, 0, 0, 1]);
    app.world_mut().resource_mut::<MapEvents>().events[0].pages[0].trigger = 3;
    app.update();
    assert!(switch_on(&app, 10));
    assert_eq!(app.world().get::<EventSprite>(first).unwrap().tile_x, 2);
    assert!(app.world().get::<MoveQueue>(first).unwrap().busy());
}

#[test]
fn a_character_already_in_motion_does_not_queue_an_autorun_until_its_next_stopped_update() {
    let mut app = app();
    let first = npc(&mut app, 1, 1, vec![], &[1, 8, 0, 0, 1]);
    app.update();
    {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        let page = &mut events.events[0].pages[0];
        page.trigger = 3;
        page.commands = vec![switch_cmd(10, 0, 0)];
    }
    app.update();
    assert!(app.world().get::<MoveQueue>(first).unwrap().busy());
    assert!(!switch_on(&app, 10));
    for _ in 0..20 {
        app.update();
    }
    assert!(switch_on(&app, 10));
}

#[test]
fn a_queued_autorun_owns_player_movement_actions_and_boarding_before_foreground_execution() {
    for mode in 0..3 {
        let mut app = app();
        npc(&mut app, 1, 1, vec![cmd(11410, 0, vec![100])], &[]);
        app.world_mut().resource_mut::<MapEvents>().events[0].pages[0].trigger = 3;
        crate::dialogue::testing::register_actions(&mut app);
        let mut action = map_event(2, 0, vec![switch_cmd(20, 0, 0)]);
        action.x = 5;
        action.y = 6;
        action.pages[0].layer = 1;
        app.world_mut()
            .resource_mut::<MapEvents>()
            .events
            .push(action);
        if mode == 2 {
            app.add_plugins(crate::vehicles::VehiclePlugin)
                .init_resource::<crate::audio::CurrentBgm>();
            app.world_mut()
                .resource_mut::<crate::vehicles::Vehicles>()
                .set_location(0, 0, 5, 6);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(if mode == 0 {
                KeyCode::ArrowRight
            } else {
                KeyCode::Enter
            });
        app.update();
        assert_eq!(hero_x(&mut app), 5);
        assert!(!switch_on(&app, 20));
        assert!(!app.world().resource::<crate::vehicles::Vehicles>().riding());
        assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
    }
}
