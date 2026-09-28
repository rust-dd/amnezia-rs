use super::Fx;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct Applied(Vec2);

#[derive(SystemParam)]
pub(crate) struct CameraShake<'w> {
    fx: Option<Res<'w, Fx>>,
    applied: Option<ResMut<'w, Applied>>,
}

impl CameraShake<'_> {
    pub(crate) fn project(&mut self, base: Vec2) -> Vec2 {
        let offset = self.fx.as_ref().map_or(Vec2::ZERO, |fx| fx.shake_offset);
        if let Some(applied) = self.applied.as_mut() {
            applied.0 = offset;
        }
        base + offset
    }

    pub(super) fn reproject(&mut self, current: Vec2) -> Vec2 {
        let previous = self
            .applied
            .as_ref()
            .map_or(Vec2::ZERO, |applied| applied.0);
        self.project(current - previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_shake_reprojection_replaces_signed_offsets_and_removes_a_stopped_shake() {
        let mut world = World::new();
        world.init_resource::<Applied>();
        world.init_resource::<Fx>();
        let base = Vec2::new(72.0, -131.0);
        let mut point = base;
        for offset in [Vec2::new(5.0, -2.0), Vec2::new(-3.0, 4.0), Vec2::ZERO] {
            world.resource_mut::<Fx>().shake_offset = offset;
            for _ in 0..3 {
                point = world
                    .run_system_cached_with(
                        |In(current): In<Vec2>, mut shake: CameraShake| shake.reproject(current),
                        point,
                    )
                    .unwrap();
                assert_eq!(point, base + offset);
            }
        }
    }

    #[test]
    fn a_new_camera_base_includes_the_current_shake_before_later_projection() {
        let mut world = World::new();
        world.init_resource::<Applied>();
        world.insert_resource(Fx {
            shake_offset: Vec2::new(-3.0, 0.0),
            ..default()
        });
        for base in [Vec2::new(72.0, -131.0), Vec2::new(-11.0, 13.0)] {
            let point = world
                .run_system_cached_with(
                    |In(base): In<Vec2>, mut shake: CameraShake| shake.project(base),
                    base,
                )
                .unwrap();
            assert_eq!(point, base + Vec2::new(-3.0, 0.0));
            assert_eq!(
                world
                    .run_system_cached_with(
                        |In(current): In<Vec2>, mut shake: CameraShake| shake.reproject(current),
                        point,
                    )
                    .unwrap(),
                point
            );
        }
    }
}
