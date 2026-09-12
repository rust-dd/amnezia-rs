use super::*;
use crate::interpreter::RunningEvent;
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapData;

#[derive(Resource)]
struct Probe {
    original: Vec<Entity>,
    anchor: Vec2,
    checked: u8,
}

fn transfer(world: &mut World, map_id: i32, x: i32, y: i32) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![amnezia_data::EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![map_id, x, y],
        }],
    );
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        610 => {
            for id in 1..=3 {
                world.write_message(PictureCommand::erase(id));
            }
            for (id, x, fixed) in [(1, 230.0, 0), (2, 280.0, 1)] {
                world.write_message(PictureCommand::show(
                    id,
                    "Cross",
                    x,
                    60.0,
                    &[0, 0, 0, 0, fixed, 800, 0, fixed, 100, 100, 100, 100, 0, 0],
                ));
            }
        }
        630 => {
            let pictures = world
                .query::<(Entity, &Picture)>()
                .iter(world)
                .collect::<Vec<_>>();
            assert_eq!(pictures.len(), 2);
            let anchor = pictures
                .iter()
                .find(|(_, p)| p.id == 2)
                .unwrap()
                .1
                .world_anchor
                .unwrap();
            let original = pictures
                .iter()
                .map(|(entity, _)| *entity)
                .collect::<Vec<_>>();
            world.insert_resource(Probe {
                original,
                anchor,
                checked: 0,
            });
        }
        640 => transfer(world, 3, 8, 7),
        740 => {
            assert_eq!(world.resource::<MapData>().map_id, 3);
            assert!(!world.resource::<Fade>().busy());
            let probe = world.resource::<Probe>();
            for entity in &probe.original {
                let picture = world.get::<Picture>(*entity).unwrap();
                if picture.fixed_to_map {
                    assert_eq!(picture.world_anchor, Some(probe.anchor));
                }
            }
            world.resource_mut::<Probe>().checked |= 1;
            return Some("pictures-transfer-same");
        }
        760 => transfer(world, 13, 60, 60),
        860 | 1010 => {
            assert_eq!(world.resource::<MapData>().map_id, 13);
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(world.query::<&Picture>().iter(world).count(), 0);
            world.resource_mut::<Probe>().checked |= if frame == 860 { 2 } else { 4 };
            return Some(if frame == 860 {
                "pictures-transfer-cleared"
            } else {
                "pictures-transfer-reloaded"
            });
        }
        870 => {
            world.write_message(PictureCommand::show(
                3,
                "Cross",
                160.0,
                120.0,
                &[0, 0, 0, 0, 0, 800, 0, 0, 100, 100, 100, 100, 0, 0],
            ));
        }
        900 => assert_eq!(world.query::<&Picture>().iter(world).count(), 1),
        910 => world.resource_mut::<PendingTeleport>().reload(13, 61, 60),
        _ => {}
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().checked, 7);
    info!("picture transfers: same-map retention, cross-map clearing and reload clearing verified");
}
