use super::*;
use crate::screenfx::{ScreenEffect, apply_effect, step_flash, step_shake, tone::step_tint};

fn advance(world: &mut World, dt: f32) {
    step_tint(&mut world.resource_mut::<TintState>(), dt);
    let mut fx = world.resource_mut::<Fx>();
    step_flash(&mut fx, dt);
    step_shake(&mut fx, dt);
}

fn original() -> World {
    let mut world = World::new();
    let tone = ron::from_str::<TintState>("(current:(100.0,100.0,100.0,100.0),target:(70.0,90.0,110.0,50.0),frames_left:120,fraction:0.0)").unwrap();
    let mut fx = Fx::default();
    apply_effect(&mut fx, &ScreenEffect::flash(&[31, 10, 5, 20, 20, 0]));
    apply_effect(&mut fx, &ScreenEffect::shake(&[3, 5, 20, 0]));
    world.insert_resource(tone);
    world.insert_resource(fx);
    world
}

#[test]
fn serialized_effects_continue_exactly_through_completion_at_15_to_144_fps() {
    for fps in [15, 30, 60, 120, 144] {
        let mut original = original();
        advance(&mut original, 0.4375);
        let state = snapshot(&original);
        assert!(state.valid());
        let serialized = ron::to_string(&state).unwrap();
        let mut restored = World::new();
        ron::from_str::<ScreenState>(&serialized)
            .unwrap()
            .restore(&mut restored);
        for _ in 0..fps * 3 {
            advance(&mut original, 1.0 / fps as f32);
            advance(&mut restored, 1.0 / fps as f32);
            assert_eq!(snapshot(&restored), snapshot(&original));
            assert!(snapshot(&restored).valid());
        }
        assert_eq!(
            restored.resource::<TintState>().tone(),
            [70.0, 90.0, 110.0, 50.0]
        );
        assert!(restored.resource::<Fx>().flash.is_none());
        assert_eq!(restored.resource::<Fx>().shake_offset, Vec2::ZERO);
    }
}

#[test]
fn session_cleanup_discards_a_pending_screen_restore() {
    let mut world = original();
    let state = snapshot(&world);
    prepare(&mut world, 2, Some(state));
    crate::session::clear_transient(&mut world);
    assert!(!world.contains_resource::<Pending>());
    assert!(world.resource::<Fx>().flash.is_none());
    assert_eq!(world.resource::<Fx>().shake_offset, Vec2::ZERO);
}
