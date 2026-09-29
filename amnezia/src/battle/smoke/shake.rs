use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource)]
struct Fixture {
    checks: Arc<AtomicUsize>,
    restored: bool,
    underlay: Entity,
}

pub(in crate::battle) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 883 {
        let underlay = world
            .spawn((
                Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(320.0, 240.0)),
                Transform::from_xyz(0.0, 0.0, 90.0),
                crate::animation::overlay_layer(),
            ))
            .id();
        let fixture = Fixture {
            checks: Arc::new(AtomicUsize::new(0)),
            restored: false,
            underlay,
        };
        world.resource_mut::<Battle>().pending_shake = true;
        world
            .resource_mut::<Battle>()
            .pending_action_flashes
            .push(crate::battle::model::Source::Enemy(0));
        world.insert_resource(fixture);
    }
    if frame == 897 {
        let mut fixture = world.remove_resource::<Fixture>().unwrap();
        world.despawn(fixture.underlay);
        fixture.restored = true;
        world.insert_resource(fixture);
    }
    if (884..=892).contains(&frame) {
        let position = world
            .query_filtered::<&Transform, (
                With<crate::battle::scene::SceneEntity>,
                With<crate::legacy_colors::hue::HueShift>,
                Without<crate::battle::scene::Battler>,
            )>()
            .single(world)
            .unwrap()
            .translation
            .x;
        assert_eq!(
            position,
            [0.0, 5.0, 5.0, 2.0, -2.0, -6.0, -6.0, -4.0, 0.0][(frame - 884) as usize]
        );
    }
    match frame {
        885 => Some("battle-shake-right"),
        889 => Some("battle-shake-left"),
        895 => Some("battle-shake-restored"),
        _ => None,
    }
}

pub(in crate::battle) fn checks(world: &World) -> Arc<AtomicUsize> {
    world.resource::<Fixture>().checks.clone()
}

pub(super) fn verify_finished(world: &World) {
    let fixture = world.resource::<Fixture>();
    assert!(fixture.restored);
    assert_eq!(fixture.checks.load(Ordering::Relaxed), 3);
}
