use crate::interpreter::RunningEvent;
use crate::transitions::Transition;
use bevy::prelude::*;

#[derive(Resource)]
struct Probe {
    origin: Vec2,
    frozen: Option<Vec2>,
    checks: u32,
    frozen_checks: u32,
}

fn position(world: &mut World) -> Vec2 {
    world
        .query_filtered::<&Transform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate()
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 720 {
        assert!(!world.resource::<RunningEvent>().active());
        let origin = position(world);
        world.insert_resource(Probe {
            origin,
            frozen: None,
            checks: 0,
            frozen_checks: 0,
        });
        world.resource_mut::<RunningEvent>().start(
            0,
            vec![
                super::command(11050, vec![3, 5, 100, 0]),
                super::command(11410, vec![2]),
                super::command(11010, vec![0]),
                super::command(11020, vec![0]),
                super::command(11050, vec![0, 0, 0, 0]),
            ],
        );
    }
    if !(721..=880).contains(&frame) {
        return;
    }
    let point = position(world);
    let offset = world
        .run_system_cached(|shake: crate::screenfx::ScreenShake| shake.offset())
        .unwrap();
    let waiting = world.resource::<Transition>().busy();
    let mut probe = world.resource_mut::<Probe>();
    assert_eq!(point, probe.origin + offset, "shaken camera frame {frame}");
    probe.checks += 1;
    if waiting {
        assert_ne!(offset, Vec2::ZERO);
        if let Some(frozen) = probe.frozen {
            assert_eq!(offset, frozen);
        }
        probe.frozen = Some(offset);
        probe.frozen_checks += 1;
    }
    if frame == 880 {
        assert_eq!(offset, Vec2::ZERO);
        assert!(!waiting);
        assert!(!world.resource::<RunningEvent>().active());
        super::super::capture(world, "camera-shake-returned");
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.checks, 160);
    assert!(probe.frozen_checks >= 68);
    info!(
        "camera shake: 160 exact projections, including {} frozen asynchronous frames, verified",
        probe.frozen_checks
    );
}
