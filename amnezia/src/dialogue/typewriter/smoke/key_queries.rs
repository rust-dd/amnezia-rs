use super::*;
use crate::state::Variables;

#[derive(Resource, Default)]
struct Probe(u8);

fn original(map_id: u32, wait: bool) -> EventCommand {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    map.events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|command| command.code == 11610 && command.params[1] == i32::from(wait))
        .unwrap()
        .clone()
}

fn start(world: &mut World, map_id: u32, flag: i32) {
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<RunningEvent>().start(
        0,
        vec![
            original(map_id, true),
            EventCommand {
                code: 10210,
                indent: 0,
                string: String::new(),
                params: vec![0, flag, flag, 0],
            },
        ],
    );
}

pub(super) fn input(world: &mut World, frame: u32) -> bool {
    if frame < 1030 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    if matches!(frame, 1030 | 1060 | 1100) {
        keys.reset_all();
    }
    if frame == 1040 {
        keys.release(KeyCode::Escape);
    }
    if let Some(key) = match frame {
        1030 | 1070 => Some(KeyCode::Escape),
        1042 | 1100 => Some(KeyCode::Enter),
        1060 => Some(KeyCode::ArrowUp),
        1080 => Some(KeyCode::ArrowRight),
        1110 => Some(KeyCode::Space),
        _ => None,
    } {
        keys.press(key);
    }
    true
}

pub(super) fn drive(world: &mut World, frame: u32) {
    match frame {
        1030 => {
            world.init_resource::<Probe>();
            start(world, 221, 9006);
            world
                .resource_mut::<crate::interpreter::CommonEvents>()
                .0
                .push(amnezia_data::CommonEvent {
                    id: 9000,
                    name: String::new(),
                    trigger: 2,
                    switch_id: 0,
                    commands: vec![original(13, false)],
                });
        }
        1031 | 1038 => {
            assert_eq!(world.resource::<Variables>().get(62), 0);
            assert_eq!(world.resource::<Variables>().get(52), 6);
            assert!(world.resource::<RunningEvent>().active());
            let keys = world.resource::<ButtonInput<KeyCode>>();
            assert!(keys.pressed(KeyCode::Escape));
            assert!(!keys.just_pressed(KeyCode::Escape));
            world.resource_mut::<Probe>().0 |= 1;
        }
        1043 => {
            assert_eq!(world.resource::<Variables>().get(62), 5);
            assert!(world.resource::<Switches>().get(9006));
            assert!(!world.resource::<RunningEvent>().active());
            world
                .resource_mut::<crate::interpreter::CommonEvents>()
                .0
                .retain(|event| event.id != 9000);
            world.resource_mut::<Probe>().0 |= 2;
        }
        1060 => start(world, 118, 9007),
        1071 => {
            assert_eq!(world.resource::<Variables>().get(89), 0);
            assert!(!world.resource::<Switches>().get(9007));
        }
        1081 => {
            assert_eq!(world.resource::<Variables>().get(89), 3);
            assert!(world.resource::<Switches>().get(9007));
            assert!(!world.resource::<RunningEvent>().active());
            world.resource_mut::<Probe>().0 |= 4;
        }
        1100 => start(world, 13, 9008),
        1111 => {
            assert_eq!(world.resource::<Variables>().get(52), 5);
            assert!(world.resource::<Switches>().get(9008));
            assert!(!world.resource::<RunningEvent>().active());
            world.resource_mut::<Probe>().0 |= 8;
        }
        _ => {}
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().0, 15);
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    info!("original key queries: three waits, ignored cancel, held-key parallel polling verified");
}
