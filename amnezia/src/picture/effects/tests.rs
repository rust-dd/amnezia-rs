use super::*;
use crate::picture::{Anim, Picture, PictureCommand, Tone};

fn picture(effect: Effect) -> Picture {
    Picture {
        id: 1,
        name: "Fog".into(),
        x: 0.0,
        y: 0.0,
        transparency: 0.0,
        zoom: 100.0,
        tone: Tone::NEUTRAL,
        use_transparent_color: false,
        fixed_to_map: false,
        world_anchor: None,
        base_size: None,
        tween: None,
        effect: EffectState::show(effect),
        frame_fraction: 0.0,
    }
}

#[test]
fn all_original_fog_commands_have_the_same_wave_and_ignore_unused_garbage() {
    let mut fog = Vec::new();
    for id in 1..=276 {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_{id:04}.ron",
            crate::assets::asset_root()
        ));
        for command in map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .flat_map(|p| &p.commands)
        {
            if !matches!(command.code, 11110 | 11120) {
                continue;
            }
            let effect = Effect::from_params(&command.params);
            if command.params[12] == 0 {
                assert_eq!(effect, Effect::default());
                continue;
            }
            let PictureCommand::Show {
                name,
                zoom,
                transparency,
                effect,
                ..
            } = PictureCommand::show(1, &command.string, 160.0, 120.0, &command.params)
            else {
                unreachable!()
            };
            assert_eq!(name, "Fog");
            assert_eq!((zoom, transparency), (120.0, 60.0));
            assert_eq!(
                effect,
                Effect {
                    mode: 2,
                    strength: 1
                }
            );
            fog.push(id);
        }
    }
    assert_eq!(fog, [36, 37, 38, 39, 46, 47, 48, 231]);
}

#[test]
fn wave_and_move_tween_share_original_frames_at_every_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut pic = picture(Effect {
            mode: 2,
            strength: 1,
        });
        pic.retarget(
            Anim {
                x: 60.0,
                ..pic.anim()
            },
            Effect {
                mode: 2,
                strength: 3,
            },
            1.0,
        );
        for _ in 0..fps / 2 {
            pic.advance(1.0 / fps as f32);
        }
        assert_eq!(pic.x, 30.0 * (fps / 2) as f32 / (fps as f32 / 2.0));
        // An odd 15 Hz half-second stops at seven whole render frames.
        if fps != 15 {
            assert_eq!(pic.effect.phase, 240);
            assert!((pic.effect.strength - 2.0).abs() < 1e-8);
        }
        for _ in fps / 2..fps {
            pic.advance(1.0 / fps as f32);
        }
        assert_eq!(pic.x, 60.0);
        assert!(pic.tween.is_none());
        assert_eq!(pic.effect.strength, 3.0);
        assert_eq!(pic.effect.phase, 224);
    }
    let mut pic = picture(Effect {
        mode: 2,
        strength: 1,
    });
    pic.advance(1.0 / 120.0);
    assert_eq!(pic.effect.phase, 0);
    pic.advance(1.0 / 120.0);
    assert_eq!(pic.effect.phase, 8);
    for _ in 1..32 {
        pic.advance(1.0 / 60.0);
    }
    assert_eq!(pic.effect.phase, 0);
}

#[test]
fn move_cannot_introduce_an_effect_but_can_fade_and_restart_existing_waves() {
    let mut plain = picture(Effect::default());
    plain.retarget(
        Anim {
            x: 10.0,
            ..plain.anim()
        },
        Effect {
            mode: 2,
            strength: 10,
        },
        0.0,
    );
    plain.advance(1.0 / 60.0);
    assert_eq!(plain.x, 10.0);
    assert_eq!(
        (plain.effect.mode, plain.effect.strength, plain.effect.phase),
        (0, 0.0, 0)
    );

    let mut pic = picture(Effect {
        mode: 2,
        strength: 4,
    });
    pic.retarget(pic.anim(), Effect::default(), 1.0);
    pic.advance(0.5);
    assert!((pic.effect.strength - 2.0).abs() < 1e-8);
    pic.advance(0.5);
    assert_eq!((pic.effect.mode, pic.effect.strength), (2, 0.0));
    pic.retarget(
        pic.anim(),
        Effect {
            mode: 2,
            strength: 4,
        },
        0.0,
    );
    pic.advance(1.0 / 60.0);
    assert_eq!(pic.effect.strength, 4.0);
    assert_eq!(pic.effect.phase, 232);
}

#[test]
fn wave_geometry_preserves_native_rows_clipping_zoom_and_side_padding() {
    let effect = EffectState::show(Effect {
        mode: 2,
        strength: 1,
    });
    let wave = effect
        .wave(Vec2::new(320.0, 242.0), 1.2, Vec2::new(160.0, 120.0))
        .unwrap();
    assert_eq!(wave.size, Vec2::new(394.0, 290.0));
    assert_eq!(wave.center, Vec2::new(160.0, 120.0));
    assert_eq!(wave.uniform, Vec4::new(0.0, 4.8, 1.2, 25.0));
    let wave = effect
        .wave(Vec2::new(3.0, 3.0), 1.5, Vec2::new(20.8, 20.8))
        .unwrap();
    assert_eq!(wave.center, Vec2::splat(20.0));
    assert_eq!(wave.size, Vec2::new(16.0, 4.0));
    assert!(effect.wave(Vec2::ONE, 0.0, Vec2::ZERO).is_none());
}

#[test]
fn picture_clock_pauses_in_other_scenes_but_not_during_dialogue() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(crate::menu::MenuOpen(true))
        .init_resource::<crate::dialogue::Dialogue>()
        .add_systems(Update, crate::picture::drive_tweens);
    let entity = app
        .world_mut()
        .spawn(picture(Effect {
            mode: 2,
            strength: 1,
        }))
        .id();
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(app.world().get::<Picture>(entity).unwrap().effect.phase, 0);
    app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
    app.world_mut()
        .resource_mut::<crate::dialogue::Dialogue>()
        .active = true;
    app.update();
    assert_eq!(app.world().get::<Picture>(entity).unwrap().effect.phase, 8);
}

#[test]
fn stopping_rotation_finishes_the_current_revolution() {
    let mut effect = EffectState::show(Effect {
        mode: 1,
        strength: 8,
    });
    effect.tick(0);
    assert_eq!(effect.rotation, 8.0);
    effect.retarget(Effect::default());
    for _ in 1..32 {
        effect.tick(0);
    }
    assert_eq!(effect.rotation, 0.0);
    effect.tick(0);
    assert_eq!(effect.rotation, 0.0);
    let mut effect = EffectState::show(Effect {
        mode: 1,
        strength: 8,
    });
    for _ in 0..32 {
        effect.tick(0);
    }
    effect.retarget(Effect::default());
    effect.tick(0);
    assert_eq!(effect.rotation, 8.0);
}
