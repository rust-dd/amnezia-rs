use super::*;

fn flash_timing(scope: u32) -> AnimationTimingDef {
    AnimationTimingDef {
        frame: 1,
        se_name: String::new(),
        se_volume: 100,
        se_tempo: 100,
        flash_scope: scope,
        flash_red: 31,
        flash_green: 20,
        flash_blue: 10,
        flash_power: 31,
    }
}

#[test]
fn a_screen_scope_flash_spawns_a_fullscreen_quad_and_no_battler_flash() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<BattlerFlash>();
    app.add_systems(
        Update,
        |mut commands: Commands, mut bf: MessageWriter<BattlerFlash>| {
            emit_flash(
                &mut commands,
                &mut bf,
                &flash_timing(2),
                &[Vec2::new(16.0, -24.0)],
                FlashStamp::default(),
            );
        },
    );
    app.update();
    let mut quads = app.world_mut().query::<&render::FlashQuad>();
    assert_eq!(quads.iter(app.world()).count(), 1, "one full-screen quad");
    let messages = app.world().resource::<Messages<BattlerFlash>>();
    let mut cursor = messages.get_cursor();
    assert_eq!(cursor.read(messages).count(), 0, "no target tint");
}

#[test]
fn a_screen_flash_spawns_one_quad_even_with_many_target_anchors() {
    // A screen flash is a single full-screen tint; a multi-target cast must
    // not stack one quad per target.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<BattlerFlash>();
    app.add_systems(
        Update,
        |mut commands: Commands, mut bf: MessageWriter<BattlerFlash>| {
            emit_flash(
                &mut commands,
                &mut bf,
                &flash_timing(2),
                &[Vec2::new(-60.0, -20.0), Vec2::new(40.0, -20.0)],
                FlashStamp::default(),
            );
        },
    );
    app.update();
    let mut quads = app.world_mut().query::<&render::FlashQuad>();
    assert_eq!(quads.iter(app.world()).count(), 1, "one full-screen quad");
}

#[test]
fn a_target_scope_flash_publishes_a_battler_flash_not_a_box() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<BattlerFlash>();
    app.add_systems(
        Update,
        |mut commands: Commands, mut bf: MessageWriter<BattlerFlash>| {
            emit_flash(
                &mut commands,
                &mut bf,
                &flash_timing(1),
                &[Vec2::new(16.0, -24.0)],
                FlashStamp::default(),
            );
        },
    );
    app.update();
    let mut quads = app.world_mut().query::<&render::FlashQuad>();
    assert_eq!(quads.iter(app.world()).count(), 0, "no drawn box");
    let messages = app.world().resource::<Messages<BattlerFlash>>();
    let mut cursor = messages.get_cursor();
    let flashes: Vec<Vec2> = cursor.read(messages).map(|f| f.pos).collect();
    assert_eq!(flashes, vec![Vec2::new(16.0, -24.0)]);
}

#[test]
fn a_target_flash_fires_once_per_anchor() {
    // A multi-target scope-0 cast tints every target battler, one flash each.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<BattlerFlash>();
    app.add_systems(
        Update,
        |mut commands: Commands, mut bf: MessageWriter<BattlerFlash>| {
            emit_flash(
                &mut commands,
                &mut bf,
                &flash_timing(1),
                &[Vec2::new(-60.0, -20.0), Vec2::new(40.0, -20.0)],
                FlashStamp::default(),
            );
        },
    );
    app.update();
    let messages = app.world().resource::<Messages<BattlerFlash>>();
    let mut cursor = messages.get_cursor();
    let flashes: Vec<Vec2> = cursor.read(messages).map(|f| f.pos).collect();
    assert_eq!(
        flashes,
        vec![Vec2::new(-60.0, -20.0), Vec2::new(40.0, -20.0)]
    );
}

#[test]
fn flash_channel_normalises_and_clamps_the_0_31_scale() {
    assert_eq!(flash_channel(0), 0);
    assert_eq!(flash_channel(31), 248);
    assert_eq!(flash_channel(62), 248);
    assert_eq!(flash_channel(15), 120);
}

#[test]
fn target_screen_offset_centres_and_negates_y() {
    // A target on the camera centre maps to the screen centre.
    assert_eq!(
        target_screen_offset(Vec2::new(50.0, -20.0), Vec2::new(50.0, -20.0)),
        Vec2::ZERO
    );
    // A target +32 right and +16 *up* in world (y-up) reads as RM2000
    // `(32, -16)` — right, and above centre (RM2000 y grows downward).
    assert_eq!(
        target_screen_offset(Vec2::new(32.0, 16.0), Vec2::ZERO),
        Vec2::new(32.0, -16.0)
    );
    // A target below the camera has positive RM2000 y.
    assert_eq!(
        target_screen_offset(Vec2::new(0.0, -40.0), Vec2::ZERO),
        Vec2::new(0.0, 40.0)
    );
}

#[test]
fn resolver_projects_hero_and_event_onto_screen() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<ShowMapAnimation>();
    app.add_message::<PlayAnimation>();
    app.insert_resource(AnimationLibrary(vec![
        anim_def(62, 0, 1, vec![]),
        anim_def(63, 0, 1, vec![]),
    ]));
    app.add_systems(Update, resolve_map_animation);

    app.world_mut()
        .spawn((MainCamera, Transform::from_xyz(100.0, 50.0, 0.0)));
    app.world_mut().spawn((
        Player {
            tile_x: 0,
            tile_y: 0,
            dir: 0,
            frame: 0,
            charset: String::new(),
            index: 0,
        },
        Transform::from_xyz(132.0, 66.0, 3.0),
    ));
    app.world_mut().spawn((
        EventSprite {
            id: 7,
            tile_x: 0,
            tile_y: 0,
            dir: 0,
            frame: 0,
            charset: String::new(),
            index: 0,
            layer: 1,
        },
        Transform::from_xyz(90.0, 30.0, 3.0),
    ));

    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 62,
        target: AnimTarget::Hero,
        global: false,
    });
    app.update();
    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 63,
        target: AnimTarget::Event(7),
        global: false,
    });
    app.update();

    let messages = app.world().resource::<Messages<PlayAnimation>>();
    let mut cursor = messages.get_cursor();
    let plays = cursor
        .read(messages)
        .map(|p| {
            let a = p.targets[0];
            (p.anim_id, a.pos.x, a.pos.y)
        })
        .collect::<Vec<_>>();
    assert_eq!(plays, vec![(62, 32.0, -12.0), (63, -10.0, 16.0)]);
}

fn anim_def(id: u32, scope: u32, position: u32, timings: Vec<AnimationTimingDef>) -> AnimationDef {
    AnimationDef {
        id,
        name: String::new(),
        animation_name: String::new(),
        scope,
        position,
        frames: vec![amnezia_data::AnimationFrameDef { cells: Vec::new() }],
        timings,
    }
}

pub(super) fn se_timing(name: &str) -> AnimationTimingDef {
    AnimationTimingDef {
        frame: 1,
        se_name: name.to_string(),
        se_volume: 100,
        se_tempo: 100,
        flash_scope: FLASH_SCOPE_TARGET,
        flash_red: 31,
        flash_green: 31,
        flash_blue: 31,
        flash_power: 31,
    }
}

#[test]
fn frame_cadence_shows_each_data_frame_for_two_game_frames() {
    // RM2000 holds each data frame for two 60 fps game-frames: 1/30 s.
    assert_eq!(FRAME_SECS, 1.0 / 30.0);
}

#[test]
fn position_offset_matches_easyrpg_calculate_offset() {
    // Feet/down (2) drops by h/2, head/up (0) lifts by h/2, centre (1) and any
    // other value leave the effect on the target centre.
    assert_eq!(position_offset(POSITION_DOWN, 48.0), 24.0);
    assert_eq!(position_offset(POSITION_UP, 48.0), -24.0);
    assert_eq!(position_offset(1, 48.0), 0.0);
    assert_eq!(position_offset(7, 48.0), 0.0);
}

#[test]
fn a_screen_scope_animation_draws_its_cells_once_centred() {
    // A screen-scope (1) animation ignores the per-target anchors for drawing
    // and paints once at the caller's screen centre.
    let def = anim_def(1, SCOPE_SCREEN, 1, Vec::new());
    let targets = [
        AnimAnchor {
            pos: Vec2::new(-60.0, -20.0),
            height: 48.0,
        },
        AnimAnchor {
            pos: Vec2::new(40.0, -20.0),
            height: 48.0,
        },
    ];
    let center = Vec2::new(0.0, -40.0);
    assert_eq!(draw_anchors(&def, &targets, center, false), vec![center]);
}

#[test]
fn a_global_animation_tiles_its_cells_3x3_across_the_screen() {
    // The global flag (opcode 11210 params[3]) overrides scope: the cells draw at
    // the screen centre plus its eight neighbours, one screen-width/height apart.
    let def = anim_def(1, SCOPE_SCREEN, 1, Vec::new());
    let anchors = draw_anchors(&def, &[], Vec2::ZERO, true);
    assert_eq!(anchors.len(), 9);
    assert!(anchors.contains(&Vec2::ZERO));
    assert!(anchors.contains(&Vec2::new(-320.0, -240.0)));
    assert!(anchors.contains(&Vec2::new(320.0, 240.0)));
    assert!(anchors.contains(&Vec2::new(320.0, 0.0)));
}

#[test]
fn a_target_scope_animation_draws_at_each_target_with_the_position_offset() {
    // A target-scope (0), feet-anchored (2) animation draws at every target,
    // each dropped by half that target's height.
    let def = anim_def(1, 0, POSITION_DOWN, Vec::new());
    let targets = [
        AnimAnchor {
            pos: Vec2::new(-60.0, -20.0),
            height: 48.0,
        },
        AnimAnchor {
            pos: Vec2::new(40.0, -20.0),
            height: 24.0,
        },
    ];
    assert_eq!(
        draw_anchors(&def, &targets, Vec2::ZERO, false),
        vec![Vec2::new(-60.0, 4.0), Vec2::new(40.0, -8.0)]
    );
}

#[test]
fn a_multi_target_cast_plays_its_sound_effect_once() {
    // One cast is one timeline: three target anchors must not play the SE
    // three times.
    let def = anim_def(1, 0, POSITION_DOWN, vec![se_timing("Punch")]);
    let anchors = vec![
        Vec2::new(-60.0, -20.0),
        Vec2::new(40.0, -20.0),
        Vec2::new(0.0, 80.0),
    ];
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<AudioRequest>();
    app.add_message::<BattlerFlash>();
    app.add_systems(
        Update,
        move |mut commands: Commands,
              mut audio: MessageWriter<AudioRequest>,
              mut bf: MessageWriter<BattlerFlash>| {
            fire_timings(
                &mut commands,
                &mut audio,
                &mut bf,
                &def,
                0,
                &anchors,
                Some(FlashStamp::default()),
            );
        },
    );
    app.update();
    let sounds = app.world().resource::<Messages<AudioRequest>>();
    let mut sc = sounds.get_cursor();
    assert_eq!(sc.read(sounds).count(), 1, "SE plays once for the cast");
    // The target flash, by contrast, still fires once per target.
    let flashes = app.world().resource::<Messages<BattlerFlash>>();
    let mut fc = flashes.get_cursor();
    assert_eq!(fc.read(flashes).count(), 3, "one target flash per anchor");
}
