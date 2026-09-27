use super::*;

#[derive(Resource)]
pub(super) struct Reference {
    pub(super) phase: [i64; 2],
    speed: [i64; 2],
    checks: u32,
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 600 {
        let phase = world.resource::<Panorama>().motion.as_ref().unwrap().phase;
        world.insert_resource(Reference {
            phase,
            speed: [-2, 2],
            checks: 0,
        });
        change(world, &[1, 1, 1, 1, 1, -1]);
    } else if (601..=1200).contains(&frame) {
        let actual = world.resource::<Panorama>().motion.as_ref().unwrap().phase;
        let mut reference = world.resource_mut::<Reference>();
        for axis in 0..2 {
            reference.phase[axis] = (reference.phase[axis] + reference.speed[axis])
                .rem_euclid([640 * 32, 480 * 32][axis]);
        }
        assert_eq!(
            actual, reference.phase,
            "panorama phase at smoke frame {frame}"
        );
        reference.checks += 1;
    }
    match frame {
        606 => Some("panorama-phase-slow"),
        610 => {
            change(world, &[0; 6]);
            world.resource_mut::<Reference>().speed = [0; 2];
            None
        }
        615 => Some("panorama-phase-bounded"),
        620 => {
            change(world, &[1, 1, 1, -1, 1, 1]);
            world.resource_mut::<Reference>().speed = [2, -2];
            None
        }
        626 => Some("panorama-phase-reverse"),
        630 => {
            world.write_message(crate::screenfx::ScreenEffect::Shake {
                power: 5,
                speed: 5,
                secs: 0.3,
            });
            None
        }
        632 => Some("panorama-phase-shake"),
        660 => {
            change(world, &[1, 1, 1, 8, 1, -6]);
            world.resource_mut::<Reference>().speed = [-256, 64];
            None
        }
        _ => None,
    }
}

fn change(world: &mut World, params: &[i32]) {
    let before = world.resource::<Panorama>().motion.as_ref().unwrap().phase;
    world.resource_scope(|world, mut panorama: Mut<Panorama>| {
        world.resource_scope(|world, mut camera: Mut<crate::player::CameraPan>| {
            panorama.change(
                world.resource::<MapData>(),
                &mut camera,
                PanoramaDef::from_command("Sky".into(), params),
            );
        });
    });
    assert_eq!(
        world.resource::<Panorama>().motion.as_ref().unwrap().phase,
        before
    );
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Reference>().checks, 600);
    info!(
        "panorama: 600 exact phase states verified through speed, direction and loop-mode changes"
    );
}
