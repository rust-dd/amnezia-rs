use super::*;
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapData;

#[derive(Resource, Default)]
struct Trace {
    held: Vec<model::Drop>,
    fraction: f64,
    frozen: u32,
    checks: u8,
}

fn hold(world: &mut World) {
    let drops = world.resource::<Rain>().drops.clone();
    let fraction = world.resource::<Rain>().fraction;
    let mut trace = world.resource_mut::<Trace>();
    trace.held = drops;
    trace.fraction = fraction;
    trace.frozen = 0;
}

fn assert_held(world: &World) {
    let trace = world.resource::<Trace>();
    assert_eq!(world.resource::<Rain>().drops, trace.held);
    assert_eq!(world.resource::<Rain>().fraction, trace.fraction);
}

fn hold_after_transfer_tick(world: &mut World) {
    fixture(world);
    hold(world);
    let mut trace = world.resource_mut::<Trace>();
    for drop in &mut trace.held {
        assert!(drop.life > 0);
        drop.life -= 1;
        drop.x -= 1;
        drop.y += 4;
    }
    trace.fraction = std::time::Duration::from_secs_f64(1.0 / 60.0).as_secs_f64() * 60.0 - 1.0;
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        420 => {
            world.init_resource::<Trace>();
            let backing = world
                .query_filtered::<Entity, With<Backing>>()
                .iter(world)
                .collect::<Vec<_>>();
            for entity in backing {
                world.despawn(entity);
            }
            world.write_message(crate::screenfx::ScreenEffect::tint(&[
                100, 100, 100, 100, 0, 0,
            ]));
            world.write_message(crate::screenfx::ScreenEffect::shake(&[0, 0, 0, 0]));
        }
        451 => {
            assert!(!world.resource::<crate::menu::SceneFlow>().active());
            hold(world);
        }
        452 => {
            assert!(world.resource::<crate::menu::SceneFlow>().active());
            assert_ne!(
                world.resource::<Rain>().drops,
                world.resource::<Trace>().held
            );
            hold(world);
        }
        470 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            assert_held(world);
        }
        500 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            assert_held(world);
            world.resource_mut::<Trace>().checks |= 1;
        }
        530 => {
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            assert_ne!(
                world.resource::<Rain>().drops,
                world.resource::<Trace>().held
            );
            world.resource_mut::<Trace>().checks |= 2;
        }
        550 | 650 => {
            hold_after_transfer_tick(world);
            world.resource_mut::<PendingTeleport>().0 = Some(if frame == 550 {
                (13, 61, 60)
            } else {
                (38, 15, 10)
            });
        }
        551..=629 | 651..=729 => {
            if world.resource::<Fade>().busy() {
                assert_held(world);
                world.resource_mut::<Trace>().frozen += 1;
            }
        }
        630 | 730 => {
            assert!(!world.resource::<Fade>().busy());
            assert!(world.resource::<Trace>().frozen > 60);
            assert_ne!(
                world.resource::<Rain>().drops,
                world.resource::<Trace>().held
            );
            assert_eq!(
                world.resource::<MapData>().map_id,
                if frame == 630 { 13 } else { 38 }
            );
            world.resource_mut::<Trace>().checks |= if frame == 630 { 4 } else { 8 };
        }
        770 => {
            assert_eq!(world.resource::<MapData>().map_id, 38);
            assert_eq!(*world.resource::<Weather>(), Weather::Rain);
            return Some("weather-original-forest");
        }
        800 => {
            let map = crate::assets::load_ron::<Map>(&format!(
                "{}/maps/map_0038.ron",
                crate::assets::asset_root()
            ));
            let encounter = map
                .events
                .iter()
                .flat_map(|event| &event.pages)
                .flat_map(|page| &page.commands)
                .find(|command| command.code == 10710)
                .unwrap()
                .clone();
            assert!(
                !world
                    .resource::<crate::interpreter::RunningEvent>()
                    .active()
            );
            world
                .resource_mut::<crate::interpreter::RunningEvent>()
                .start(0, vec![encounter]);
        }
        1000 => {
            assert!(world.resource::<crate::battle::BattleActive>().0);
            assert!(!world.resource::<crate::transitions::Transition>().busy());
            hold(world);
        }
        1020 => {
            assert!(world.resource::<crate::battle::BattleActive>().0);
            assert_ne!(
                world.resource::<Rain>().drops,
                world.resource::<Trace>().held
            );
            let (layers, visible) = world
                .query_filtered::<(&RenderLayers, &InheritedVisibility), With<Canvas>>()
                .single(world)
                .unwrap();
            assert_eq!(*layers, crate::animation::overlay_layer());
            assert!(visible.get());
            world.resource_mut::<Trace>().checks |= 16;
            return Some("weather-battle");
        }
        _ => {}
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Trace>().checks, 31);
    info!(
        "rain: original weather commands, real menu pause/resume, local/cross-map transfers and active battle rendering verified"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transfer_fixture_checks_one_exact_tick_for_every_rain_drop() {
        let mut world = World::new();
        world.init_resource::<Rain>();
        world.init_resource::<Trace>();
        hold_after_transfer_tick(&mut world);
        assert_eq!(world.resource::<Trace>().held.len(), 100);
        world
            .resource_mut::<Rain>()
            .advance(std::time::Duration::from_secs_f64(1.0 / 60.0).as_secs_f64());
        assert_held(&world);
    }
}
