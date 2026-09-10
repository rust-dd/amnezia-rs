use super::*;
use amnezia_data::EventCommand;

#[derive(Resource, Default)]
struct Trace {
    erase: bool,
    hidden_transfer: bool,
    show: bool,
    resumed: bool,
}

pub(crate) fn entry() -> Vec<EventCommand> {
    vec![
        EventCommand {
            code: 10230,
            indent: 0,
            string: String::new(),
            params: vec![0, 0, 1200, 0, 0],
        },
        EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![99, 9, 12],
        },
    ]
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        assert_eq!(world.resource::<crate::world::MapData>().map_id, 99);
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_0099.ron",
            crate::assets::asset_root()
        ));
        let commands = &map.events.iter().find(|e| e.id == 8).unwrap().pages[2].commands;
        assert_eq!(commands[2].code, 11010);
        assert_eq!(commands[6].code, 11020);
        world
            .resource_mut::<crate::interpreter::RunningEvent>()
            .start(8, commands[2..7].to_vec());
        world.insert_resource(Trace::default());
    }
    if frame < 260 {
        return None;
    }
    let transition = world.resource::<Transition>();
    let mosaic = transition
        .effect
        .as_ref()
        .filter(|e| e.kind == Kind::Mosaic);
    let erase = mosaic.is_some_and(|e| e.erase) && transition.frame >= 12;
    let show = mosaic.is_some_and(|e| !e.erase) && transition.frame >= 12;
    let map_id = world.resource::<crate::world::MapData>().map_id;
    let hidden = map_id == 98 && transition.erased() && transition.event_erased;
    let resumed = map_id == 98
        && !transition.busy()
        && !transition.erased()
        && !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
        && !world.resource::<crate::teleport::Fade>().busy();
    let mut trace = world.resource_mut::<Trace>();
    if erase && !trace.erase {
        trace.erase = true;
        return Some("screen-events-mosaic-out");
    }
    if hidden {
        trace.hidden_transfer = true;
    }
    if show && !trace.show {
        trace.show = true;
        return Some("screen-events-mosaic-in");
    }
    if resumed && !trace.resumed {
        trace.resumed = true;
        return Some("screen-events-resumed");
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let trace = world.resource::<Trace>();
    assert!(trace.erase && trace.hidden_transfer && trace.show && trace.resumed);
    let player = world
        .query::<&crate::player::Player>()
        .single(world)
        .unwrap();
    assert_eq!((player.tile_x, player.tile_y), (9, 12));
    info!(
        "original Map0099/e8 mosaic transfer reached Map0098 and returned control without an intermediate show"
    );
}
