use super::*;
use crate::interpreter::RunningEvent;
use crate::save::EventSaveRequest;
use crate::state::Switches;

#[derive(Resource)]
struct Probe {
    before: Vec<u8>,
    checks: u8,
}

fn start(world: &mut World, map_id: u32) {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    let mut commands = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .find(|page| page.commands.iter().any(|command| command.code == 11910))
        .unwrap()
        .commands
        .clone();
    commands.push(amnezia_data::EventCommand {
        code: 10210,
        indent: 0,
        string: String::new(),
        params: vec![0, 9998, 9998, 2],
    });
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<MenuOpen>().0 = false;
    world.resource_mut::<SaveAccess>().0 = false;
    world.resource_mut::<Switches>().set(9998, false);
    world.resource_mut::<RunningEvent>().start(0, commands);
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 800 {
        world.insert_resource(Probe {
            before: std::fs::read(path(world, 15)).unwrap(),
            checks: 0,
        });
        start(world, 2);
    }
    if frame == 900 {
        start(world, 260);
        let mut transition = world.resource_mut::<crate::transitions::Transition>();
        transition.hold_black();
        transition.event_erased = true;
    }
    if matches!(frame, 835 | 935) {
        assert!(world.resource::<SaveFiles>().event_menu.is_some());
        assert!(world.resource::<EventSaveRequest>().0);
        assert!(!world.resource::<SaveAccess>().0);
        assert!(!world.resource::<Switches>().get(9998));
        assert_eq!(
            std::fs::read(path(world, 15)).unwrap(),
            world.resource::<Probe>().before
        );
        return Some(if frame == 835 {
            "save-crystal-cancel"
        } else {
            "save-crystal-confirm"
        });
    }
    if matches!(frame, 880 | 980) {
        assert!(!world.resource::<SaveFiles>().active());
        assert!(!world.resource::<EventSaveRequest>().0);
        assert!(!world.resource::<RunningEvent>().active());
        assert!(!world.resource::<MenuOpen>().0);
        assert!(!world.resource::<SaveAccess>().0);
        assert!(world.resource::<Switches>().get(9998));
        let bytes = std::fs::read(path(world, 15)).unwrap();
        if frame == 880 {
            assert_eq!(bytes, world.resource::<Probe>().before);
        } else {
            assert_ne!(bytes, world.resource::<Probe>().before);
        }
        world.resource_mut::<Probe>().checks |= if frame == 880 { 1 } else { 2 };
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().checks, 3);
    assert!(world.resource::<Switches>().get(9998));
    assert!(!world.resource::<MenuOpen>().0);
    assert!(!world.resource::<SaveAccess>().0);
    info!(
        "save crystals: original map 2/260 commands cancel without writing and save before resuming once"
    );
}
