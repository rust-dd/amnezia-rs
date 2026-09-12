use super::*;

fn sounds(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .filter(|request| matches!(request, AudioRequest::Sound { .. }))
        .collect()
}

fn flash(app: &mut App) -> [u8; 4] {
    app.world_mut()
        .query_filtered::<&crate::legacy_colors::flash::SpriteFlash, With<Player>>()
        .single(app.world())
        .map_or([0; 4], |flash| flash.0)
}

#[test]
fn restored_playback_matches_uninterrupted_frames_flashes_and_future_sounds_at_every_fps() {
    for fps in [15, 30, 60, 120, 144] {
        let (mut reference, _) = app(&format!("reference_{fps}"));
        let (mut loaded, path) = app(&format!("resume_{fps}"));
        play(&mut reference);
        play(&mut loaded);
        assert_eq!(sounds(&mut reference), sounds(&mut loaded));
        let expected_flash = flash(&mut reference);
        assert_ne!(expected_flash, [0; 4]);
        save(&mut loaded);
        reload(&mut loaded);
        std::fs::remove_file(path).unwrap();
        assert!(
            sounds(&mut loaded).is_empty(),
            "loading must not replay past sounds"
        );
        assert_eq!(flash(&mut loaded), expected_flash);
        let mut remaining = 0;
        for _ in 0..fps * 2 {
            for app in [&mut reference, &mut loaded] {
                app.world_mut()
                    .resource_mut::<GameFrames>()
                    .advance(1.0 / fps as f64);
                app.update();
            }
            assert_eq!(
                saved::snapshot(loaded.world_mut()),
                saved::snapshot(reference.world_mut())
            );
            assert_eq!(flash(&mut loaded), flash(&mut reference));
            let expected = sounds(&mut reference);
            remaining += expected.len();
            assert_eq!(sounds(&mut loaded), expected, "{fps} FPS");
        }
        assert_eq!(remaining, 3);
        assert!(saved::snapshot(loaded.world_mut()).cast.is_none());
    }
}

#[test]
fn a_saved_event_target_and_global_flag_restore_without_old_entity_handles() {
    let (mut app, path) = app("event");
    let event = || EventSprite {
        id: 7,
        tile_x: 6,
        tile_y: 7,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 1,
        layer: 1,
    };
    let old = app
        .world_mut()
        .spawn((event(), Sprite::default(), Transform::default()))
        .id();
    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 62,
        target: AnimTarget::Event(7),
        global: true,
    });
    app.update();
    step(&mut app, 13);
    let expected = saved::snapshot(app.world_mut());
    save(&mut app);
    app.world_mut().despawn(old);
    let next = app
        .world_mut()
        .spawn((
            event(),
            Sprite::default(),
            Transform::from_xyz(25.0, 40.0, 0.0),
        ))
        .id();
    reload(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(saved::snapshot(app.world_mut()), expected);
    assert_ne!(next, old);
    assert_ne!(
        app.world()
            .get::<crate::legacy_colors::flash::SpriteFlash>(next)
            .unwrap()
            .0,
        [0; 4]
    );
    let animation = app
        .world_mut()
        .query::<&playback::LiveAnimation>()
        .single(app.world())
        .unwrap();
    assert_eq!(animation.cells.len(), 9);
}
