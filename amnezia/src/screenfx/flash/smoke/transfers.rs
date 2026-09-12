use super::*;
use crate::screenfx::{FlashOverlay, ScreenEffect, saved};
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapData;

#[derive(Resource, Default)]
struct Trace {
    held: Option<saved::ScreenState>,
    frozen: u32,
    checks: u8,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        800 => {
            world.init_resource::<Trace>();
            world.write_message(ScreenEffect::flash(&[31, 10, 5, 20, 400, 0]));
        }
        830 => {
            assert_eq!(world.resource::<MapData>().map_id, 98);
            let current = saved::snapshot(world);
            assert!(world.resource::<Fx>().flash.is_some());
            world.resource_mut::<Trace>().held = Some(current);
            world.resource_mut::<PendingTeleport>().0 = Some((98, 10, 12));
        }
        831..=909 => {
            if world.resource::<Fade>().busy() {
                assert_eq!(Some(saved::snapshot(world)), world.resource::<Trace>().held);
                world.resource_mut::<Trace>().frozen += 1;
            }
        }
        910 => {
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(world.resource::<MapData>().map_id, 98);
            assert!(world.resource::<Fx>().flash.is_some());
            assert!(world.resource::<Trace>().frozen > 60);
            assert!(
                world
                    .query_filtered::<&BackgroundColor, With<FlashOverlay>>()
                    .single(world)
                    .unwrap()
                    .0
                    .alpha()
                    > 0.0
            );
            world.resource_mut::<Trace>().checks |= 1;
            return Some("map-effect-transfer-same");
        }
        920 => {
            world.resource_mut::<PendingTeleport>().0 = Some((99, 9, 12));
        }
        1000 => {
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(world.resource::<MapData>().map_id, 99);
            assert!(world.resource::<Fx>().flash.is_none());
            assert_eq!(
                world
                    .query_filtered::<&BackgroundColor, With<FlashOverlay>>()
                    .single(world)
                    .unwrap()
                    .0,
                Color::NONE
            );
            world.resource_mut::<Trace>().checks |= 2;
            return Some("map-effect-transfer-cleared");
        }
        1010 => {
            world.resource_mut::<PendingTeleport>().0 = Some((98, 9, 12));
        }
        1110 => {
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(world.resource::<MapData>().map_id, 98);
            assert!(world.resource::<Fx>().flash.is_none());
            world.resource_mut::<Trace>().checks |= 4;
        }
        _ => {}
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Trace>().checks, 7);
    info!("map screen flash: frozen same-map transfer, cross-map clearing and return verified");
}
