use super::*;
use crate::legacy_colors::flash::SpriteFlash;

pub(super) fn fixture(fps: u32) -> (App, Entity, Entity) {
    let (mut app, hero, _) = super::map_targets::map_app(fps);
    app.world_mut().entity_mut(hero).insert(Sprite {
        color: Color::WHITE.with_alpha(0.4),
        ..default()
    });
    let event = app
        .world_mut()
        .spawn((
            EventSprite {
                id: 7,
                tile_x: 5,
                tile_y: 5,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 1,
                layer: 1,
            },
            Transform::from_xyz(32.0, 24.0, 0.0),
            Sprite::default(),
        ))
        .id();
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].timings = vec![AnimationTimingDef {
        frame: 1,
        flash_scope: FLASH_SCOPE_TARGET,
        flash_red: 31,
        flash_green: 20,
        flash_blue: 10,
        flash_power: 31,
        ..tests::se_timing("")
    }];
    (app, hero, event)
}

pub(super) fn play(app: &mut App, target: AnimTarget) -> u32 {
    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 1,
        target,
        global: false,
    });
    app.update();
    app.world().resource::<GameFrames>().frame
}

pub(super) fn color(app: &App, entity: Entity) -> [u8; 4] {
    app.world()
        .get::<SpriteFlash>(entity)
        .map_or([0; 4], |flash| flash.0)
}

#[test]
fn map_target_envelope_uses_logical_frames_and_preserves_character_opacity() {
    let levels = [31, 31, 31, 25, 25, 20, 20, 15, 15, 10, 10];
    for fps in [15, 30, 60, 144] {
        let (mut app, hero, event) = fixture(fps);
        let start = play(&mut app, AnimTarget::Hero);
        assert_eq!(color(&app, hero), [0; 4]);
        for _ in 0..fps {
            app.update();
            let elapsed = app
                .world()
                .resource::<GameFrames>()
                .frame
                .wrapping_sub(start);
            let expected = elapsed
                .checked_sub(1)
                .and_then(|age| levels.get(age as usize))
                .map_or([0; 4], |level| [248, 160, 80, level * 8]);
            assert_eq!(color(&app, hero), expected, "{fps} FPS at {elapsed}");
            assert_eq!(color(&app, event), [0; 4]);
            assert_eq!(app.world().get::<Sprite>(hero).unwrap().color.alpha(), 0.4);
        }
    }
}

#[test]
fn a_moving_map_event_is_flashed_by_identity_not_its_initial_screen_position() {
    let (mut app, hero, event) = fixture(60);
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].scope = SCOPE_SCREEN;
    play(&mut app, AnimTarget::Event(7));
    app.world_mut()
        .get_mut::<Transform>(event)
        .unwrap()
        .translation
        .x += 96.0;
    app.update();
    assert_eq!(color(&app, event), [248, 160, 80, 248]);
    assert_eq!(color(&app, hero), [0; 4]);
    let flashes = app.world().resource::<Messages<BattlerFlash>>();
    assert_eq!(flashes.get_cursor().read(flashes).count(), 0);
}

#[test]
fn map_flash_holds_without_a_logical_tick_and_during_a_transition() {
    let (mut app, hero, _) = fixture(60);
    play(&mut app, AnimTarget::Hero);
    app.update();
    assert_eq!(color(&app, hero), [248, 160, 80, 248]);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    for _ in 0..20 {
        app.update();
        assert_eq!(color(&app, hero), [248, 160, 80, 248]);
    }
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for _ in 0..120 {
        app.update();
        assert_eq!(color(&app, hero), [248, 160, 80, 248]);
    }
    app.world_mut()
        .remove_resource::<crate::transitions::Transition>();
    app.update();
    assert_eq!(color(&app, hero), [248, 160, 80, 248]);
    app.update();
    app.update();
    assert_eq!(color(&app, hero), [248, 160, 80, 200]);
}

#[test]
fn final_animation_tick_flashes_once_without_leaving_an_independent_tail() {
    for fps in [15, 30, 60, 144] {
        let (mut app, hero, _) = fixture(fps);
        app.world_mut().resource_mut::<AnimationLibrary>().0[0]
            .frames
            .truncate(1);
        let start = play(&mut app, AnimTarget::Hero);
        for _ in 0..fps / 2 {
            app.update();
            let elapsed = app
                .world()
                .resource::<GameFrames>()
                .frame
                .wrapping_sub(start);
            let expected = if (1..=2).contains(&elapsed) {
                [248, 160, 80, 248]
            } else {
                [0; 4]
            };
            assert_eq!(color(&app, hero), expected, "{fps} FPS at {elapsed}");
        }
    }
}

#[test]
fn replacing_the_target_or_canceling_a_map_animation_clears_the_old_character() {
    let (mut app, hero, event) = fixture(60);
    play(&mut app, AnimTarget::Hero);
    app.update();
    assert_ne!(color(&app, hero), [0; 4]);
    play(&mut app, AnimTarget::Event(7));
    app.update();
    assert_eq!(color(&app, hero), [0; 4]);
    assert_ne!(color(&app, event), [0; 4]);
    play(&mut app, AnimTarget::Event(999));
    app.update();
    assert_eq!(color(&app, hero), [0; 4]);
    assert_eq!(color(&app, event), [0; 4]);
}

#[test]
fn map_effect_resets_and_new_sessions_clear_target_flashes_even_without_a_clock_tick() {
    for session in [false, true] {
        let (mut app, hero, _) = fixture(60);
        play(&mut app, AnimTarget::Hero);
        app.update();
        assert_ne!(color(&app, hero), [0; 4]);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
        if session {
            playback::reset_transient(app.world_mut());
        } else {
            app.world_mut().write_message(crate::world::MapEffectsReset);
        }
        app.update();
        assert_eq!(color(&app, hero), [0; 4]);
    }
}

#[test]
fn overlapping_target_timings_use_the_last_matching_entry_and_ignore_other_scopes() {
    let (mut app, hero, _) = fixture(60);
    let initial = app.world().resource::<AnimationLibrary>().0[0].timings[0].clone();
    let mut first = initial.clone();
    first.frame = 2;
    first.flash_red = 3;
    let mut last = first.clone();
    last.flash_red = 5;
    let mut screen = initial.clone();
    screen.frame = 3;
    screen.flash_scope = FLASH_SCOPE_SCREEN;
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].timings =
        vec![first, screen, initial, last];
    play(&mut app, AnimTarget::Hero);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(color(&app, hero), [40, 160, 80, 248]);
    app.update();
    app.update();
    assert_eq!(color(&app, hero), [40, 160, 80, 248]);
    app.update();
    assert_eq!(color(&app, hero), [40, 160, 80, 200]);
}
