use super::*;
use crate::player::saved_camera::CameraState;
use crate::world::saved::hero::{HeroState, snapshot};

#[test]
fn saved_jumps_keep_landing_phase_and_independent_effect_scroll() {
    for before in [2, 4] {
        let (mut original, original_hero, _) = super::super::walking::fixture((20, 15));
        original
            .world_mut()
            .resource_mut::<CameraPan>()
            .command(&[2, 1, 1, 1, 0]);
        for _ in 0..29 {
            original.update();
        }
        let mut route = original
            .world_mut()
            .get_mut::<RouteStepper>(original_hero)
            .unwrap();
        route.set_speed(6);
        route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 25]));
        for _ in 0..before {
            original.update();
        }
        let saved = ron::to_string(&(
            original.world().resource::<CameraPan>().snapshot(),
            snapshot(original.world_mut()).unwrap(),
        ))
        .unwrap();
        let (camera, state) = ron::from_str::<(CameraState, HeroState)>(&saved).unwrap();
        assert!(camera.valid() && state.valid());
        let (mut restored, restored_hero, _) = super::super::walking::fixture((20, 15));
        restored.insert_resource(camera.into_pan());
        restored
            .world_mut()
            .entity_mut(restored_hero)
            .insert((state.motion.into_queue(), state.route));
        {
            let mut hero = restored
                .world_mut()
                .get_mut::<Player>(restored_hero)
                .unwrap();
            hero.frame = state.frame;
            hero.dir = crate::tiles::DIR_DOWN;
        }
        for _ in 0..40 {
            original.update();
            restored.update();
            assert_eq!(
                original.world().resource::<CameraPan>().snapshot(),
                restored.world().resource::<CameraPan>().snapshot()
            );
            assert_eq!(
                snapshot(original.world_mut()),
                snapshot(restored.world_mut())
            );
        }
    }
}
