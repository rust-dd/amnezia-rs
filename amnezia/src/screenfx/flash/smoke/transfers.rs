use super::*;
use crate::screenfx::{FlashOverlay, ScreenEffect, saved};
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapData;

#[derive(Resource, Default)]
struct Trace {
    closing: Option<Flashing>,
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
            let closing = world.resource::<Fx>().flash.clone().unwrap();
            world.resource_mut::<Trace>().closing = Some(closing);
            world.resource_mut::<PendingTeleport>().0 = Some((98, 10, 12));
        }
        831 => {
            assert!(world.resource::<Fade>().busy());
            let before = world.resource_mut::<Trace>().closing.take().unwrap();
            verify_closing_tick(&before, world.resource::<Fx>().flash.as_ref().unwrap());
            let held = saved::snapshot(world);
            let mut trace = world.resource_mut::<Trace>();
            trace.held = Some(held);
            trace.frozen += 1;
        }
        832..=909 => {
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
                    .query_filtered::<&Sprite, With<FlashOverlay>>()
                    .single(world)
                    .unwrap()
                    .color
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
                    .query_filtered::<&Sprite, With<FlashOverlay>>()
                    .single(world)
                    .unwrap()
                    .color,
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

fn verify_closing_tick(before: &Flashing, after: &Flashing) {
    // Render-time requests enter the erase after the next closing-map update.
    assert_eq!(after.frames_left + 1, before.frames_left);
    assert_eq!(after.rgb, before.rgb);
    assert_eq!(
        after.level,
        before.level - before.level / f64::from(before.frames_left)
    );
    assert_eq!(
        after.fraction,
        (before.fraction + f64::from(1.0_f32 / 60.0) * 60.0 - 1.0).max(0.0)
    );
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Trace>().checks, 7);
    info!(
        "map screen flash: one closing-map tick, frozen same-map transfer, cross-map clearing and return verified"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flash() -> Flashing {
        let mut flash = Flashing::new(31, 10, 5, 20, 40.0);
        for _ in 0..30 {
            flash.step(1.0 / 60.0);
        }
        flash
    }

    #[test]
    fn transfer_probe_includes_exactly_the_closing_map_tick() {
        let before = flash();
        let mut after = before.clone();
        after.step(1.0 / 60.0);
        verify_closing_tick(&before, &after);
    }

    #[test]
    fn transfer_probe_rejects_early_freezing_and_extra_flash_updates() {
        let before = flash();
        for ticks in [0, 2] {
            let mut after = before.clone();
            for _ in 0..ticks {
                after.step(1.0 / 60.0);
            }
            assert!(std::panic::catch_unwind(|| verify_closing_tick(&before, &after)).is_err());
        }
    }
}
