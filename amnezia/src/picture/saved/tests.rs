use super::*;
use crate::picture::{Effect, Tone};

fn picture(mode: i32) -> Picture {
    PictureState {
        id: 1,
        name: "Fog".into(),
        visual: Anim {
            x: 160.0,
            y: 120.0,
            transparency: 60.0,
            zoom: 120.0,
            tone: Tone::NEUTRAL,
        },
        use_transparent_color: false,
        fixed_to_map: true,
        world_anchor: Some([120.5, -80.25]),
        tween: None,
        effect: EffectState::show(Effect { mode, strength: 1 }),
        frame_fraction: 0.0,
    }
    .into_picture()
}

#[test]
fn serialized_picture_moves_rotation_and_waves_resume_at_their_exact_frame() {
    for fps in [15, 30, 60, 120, 144] {
        for mode in [1, 2] {
            let mut original = picture(mode);
            original.base_size = Some(Vec2::new(320.0, 240.0));
            original.retarget(
                Anim {
                    x: 200.0,
                    y: 40.0,
                    transparency: 10.0,
                    zoom: 75.0,
                    tone: Tone {
                        r: 70.0,
                        g: 130.0,
                        b: 40.0,
                        sat: 50.0,
                    },
                },
                Effect { mode, strength: 5 },
                3.0,
            );
            for _ in 0..25 {
                original.advance(1.0 / fps as f32);
            }
            original.advance(1.0 / 144.0);
            let state = original.snapshot();
            assert!(valid(std::slice::from_ref(&state)));
            let encoded = ron::to_string(&state).unwrap();
            let mut restored = ron::from_str::<PictureState>(&encoded)
                .unwrap()
                .into_picture();
            assert!(restored.base_size.is_none());
            for _ in 0..200 {
                original.advance(1.0 / fps as f32);
                restored.advance(1.0 / fps as f32);
                assert_eq!(original.snapshot(), restored.snapshot());
            }
        }
    }
}

#[test]
fn duplicate_ids_invalid_interpolation_and_nonfinite_picture_state_are_rejected() {
    let original = picture(2).snapshot();
    assert!(valid(&[]));
    assert!(!valid(&[original.clone(), original.clone()]));
    for case in 0..10 {
        let mut state = original.clone();
        match case {
            0 => state.id = 0,
            1 => state.name.clear(),
            2 => state.visual.zoom = f32::NAN,
            3 => state.world_anchor = Some([f32::INFINITY, 0.0]),
            4 => state.fixed_to_map = false,
            5 => state.frame_fraction = 1.0,
            6 => state.frame_fraction = f64::NAN,
            7 => {
                state.effect = EffectState::show(Effect {
                    mode: 99,
                    strength: 1,
                })
            }
            8 => {
                state.tween = Some(Tween {
                    from: state.visual,
                    to: state.visual,
                    elapsed: 0,
                    frames: 0,
                })
            }
            _ => {
                state.tween = Some(Tween {
                    from: state.visual,
                    to: state.visual,
                    elapsed: 10,
                    frames: 10,
                })
            }
        }
        assert!(!valid(&[state]));
    }
}

#[test]
fn session_cleanup_cancels_old_picture_restores() {
    let mut world = World::new();
    prepare(&mut world, 2, vec![picture(2).snapshot()]);
    assert!(world.contains_resource::<Pending>());
    crate::session::clear_transient(&mut world);
    assert!(!world.contains_resource::<Pending>());
}
