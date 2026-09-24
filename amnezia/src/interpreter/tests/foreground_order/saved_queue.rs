use super::*;
use crate::interpreter::saved::{State, restore};
use crate::world::MapRebuilt;

fn waiting() -> App {
    let mut app = interp_app();
    app.add_message::<MapRebuilt>();
    app.world_mut().resource_mut::<MapData>().map_id = 3;
    let mut events = vec![];
    for id in 1..=2 {
        let mut commands = append(id as i32);
        commands.push(switch_cmd(id as i32 + 4, 1, 0));
        if id == 1 {
            commands.insert(0, cmd(11410, 0, vec![100]));
        }
        let mut event = map_event(id, 3, commands);
        event.pages[0].condition.flags = 1;
        event.pages[0].condition.switch_a = id + 4;
        events.push(event);
        app.world_mut().resource_mut::<Switches>().set(id + 4, true);
    }
    app.insert_resource(MapEvents { events });
    app.update();
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(1));
    app
}

#[test]
fn saved_pending_events_survive_the_load_rebuild_and_resume_in_order() {
    let mut app = waiting();
    let state = app.world().resource::<RunningEvent>().snapshot().unwrap();
    assert!(state.valid());
    let serialized = ron::to_string(&state).unwrap();
    let restored = ron::from_str::<State>(&serialized).unwrap();
    assert_eq!(restored, state);
    *app.world_mut().resource_mut::<RunningEvent>() = default();
    restore(app.world_mut(), Some(restored));
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert_eq!(value(&app), 12);
    assert!(app.world().resource::<RunningEvent>().snapshot().is_none());
}

#[test]
fn an_ordinary_same_map_rebuild_discards_old_pending_events() {
    let mut app = waiting();
    for event in &mut app.world_mut().resource_mut::<MapEvents>().events {
        event.pages[0].trigger = 0;
    }
    app.world_mut().write_message(MapRebuilt);
    app.world_mut().resource_mut::<RunningEvent>().frame.wait = 0.0;
    app.update();
    assert_eq!(value(&app), 1);
}

#[test]
fn queued_saves_validate_their_map_and_page_owners() {
    let app = waiting();
    let state = app.world().resource::<RunningEvent>().snapshot().unwrap();
    let mut map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0003.ron",
        crate::assets::asset_root(),
    ));
    assert!(state.valid_map(3, &map));
    assert!(!state.valid_map(4, &map));
    map.events.retain(|event| event.id != 2);
    assert!(!state.valid_map(3, &map));
}

#[test]
fn legacy_foreground_saves_default_to_an_empty_queue() {
    let app = waiting();
    let frame = &app.world().resource::<RunningEvent>().frame;
    let serialized = format!("(frame: {})", ron::to_string(frame).unwrap());
    let state = ron::from_str::<State>(&serialized).unwrap();
    assert!(state.valid());
    let mut restored = interp_app();
    restore(restored.world_mut(), Some(state));
    assert!(restored.world().resource::<RunningEvent>().queue.empty());
}
