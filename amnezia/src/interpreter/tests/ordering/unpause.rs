use super::*;
use crate::dialogue::MessageOptions;
use crate::world::{AutoMove, EventSprite};

fn moving_npc(app: &mut App, id: u32, commands: Vec<EventCommand>) -> Entity {
    let entity = super::npcs::npc(app, id, id as i32 * 2, commands, &[]);
    let route = {
        let mut events = app.world_mut().resource_mut::<MapEvents>();
        let page = &mut events.events.last_mut().unwrap().pages[0];
        page.trigger = 0;
        page.direction = crate::tiles::DIR_RIGHT;
        page.move_type = 3;
        page.move_frequency = 8;
        page.move_speed = 4;
        RouteStepper::from_event_page(Some(page))
    };
    app.world_mut()
        .entity_mut(entity)
        .insert((route, AutoMove::new(3, 8, 4, id)));
    app.world_mut()
        .resource_mut::<MessageOptions>()
        .continue_events = true;
    entity
}

fn force(target: i32, commands: &[i32]) -> EventCommand {
    let mut params = vec![target, 8, 0, 0];
    params.extend_from_slice(commands);
    cmd(11330, 0, params)
}

fn queue(app: &mut App, id: u32, decision: bool) {
    let map_id = app.world().resource::<MapData>().map_id;
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .queue_event(map_id, id, 0, decision);
}

#[test]
fn a_forced_route_unpauses_its_own_queued_foreground_event() {
    for commands in [&[][..], &[32, 5][..]] {
        let mut app = app();
        let entity = moving_npc(
            &mut app,
            1,
            vec![force(10005, commands), cmd(11410, 0, vec![100])],
        );
        queue(&mut app, 1, true);
        app.update();
        let running = app.world().resource::<RunningEvent>();
        assert!(running.active());
        assert!(!running.event_paused(1));
        assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 2);
        app.update();
        assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 3);
    }
}

#[test]
fn a_forced_route_also_unpauses_a_directly_started_foreground_event() {
    let mut app = app();
    let entity = moving_npc(&mut app, 1, vec![]);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![force(10005, &[]), cmd(11410, 0, vec![100])]);
    app.update();
    assert!(!app.world().resource::<RunningEvent>().event_paused(1));
    app.update();
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 3);
}

#[test]
fn parallel_forced_routes_unpause_pending_events_without_removing_their_commands() {
    let mut app = app();
    let entity = moving_npc(&mut app, 1, vec![switch_cmd(10, 0, 0)]);
    queue(&mut app, 1, true);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11410, 0, vec![100])]);
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![force(1, &[]), cmd(11410, 0, vec![100])],
    )]));
    app.update();
    let running = app.world().resource::<RunningEvent>();
    assert!(running.active());
    assert!(!running.event_paused(1));
    assert_eq!(running.queued_ids(), vec![1]);
    assert_eq!(app.world().get::<EventSprite>(entity).unwrap().tile_x, 3);
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert!(switch_on(&app, 10));
    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn a_called_frames_self_route_unpauses_only_its_resolved_target() {
    let mut app = app();
    moving_npc(&mut app, 1, vec![cmd(12330, 0, vec![1, 2, 1])]);
    moving_npc(
        &mut app,
        2,
        vec![force(10005, &[]), cmd(11410, 0, vec![100])],
    );
    moving_npc(&mut app, 3, vec![switch_cmd(12, 0, 0)]);
    for id in 1..=3 {
        queue(&mut app, id, true);
    }
    app.update();
    let running = app.world().resource::<RunningEvent>();
    assert_eq!(running.debug_id(), Some(2));
    assert!(running.event_paused(1));
    assert!(!running.event_paused(2));
    assert!(running.event_paused(3));
    assert_eq!(running.queued_ids(), vec![2, 3]);
}

#[test]
fn unpaused_active_and_pending_events_roundtrip_without_repausing_or_losing_origin() {
    for pending in [false, true] {
        let mut app = app();
        moving_npc(
            &mut app,
            1,
            vec![force(10005, &[]), cmd(11410, 0, vec![100])],
        );
        queue(&mut app, 1, true);
        if pending {
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(0, vec![force(1, &[]), cmd(11410, 0, vec![100])]);
        }
        app.update();
        let saved = app.world().resource::<RunningEvent>().snapshot().unwrap();
        assert!(saved.valid());
        let encoded = ron::to_string(&saved).unwrap();
        let decoded = ron::from_str::<crate::interpreter::saved::State>(&encoded).unwrap();
        assert_eq!(decoded, saved);
        crate::interpreter::saved::restore(app.world_mut(), Some(decoded));
        let running = app.world().resource::<RunningEvent>();
        assert!(!running.event_paused(1));
        assert_eq!(running.queued_ids(), if pending { vec![1] } else { vec![] });
        if pending {
            app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
            app.update();
        }
        let running = app.world().resource::<RunningEvent>();
        assert_eq!(running.debug_id(), Some(1));
        assert!(running.frame.decision);
    }
}

#[test]
fn an_unknown_forced_route_target_does_not_unpause_the_calling_event() {
    let mut app = app();
    moving_npc(&mut app, 1, vec![force(99, &[]), cmd(11410, 0, vec![100])]);
    queue(&mut app, 1, true);
    app.update();
    let running = app.world().resource::<RunningEvent>();
    assert!(running.active());
    assert!(running.event_paused(1));
}
