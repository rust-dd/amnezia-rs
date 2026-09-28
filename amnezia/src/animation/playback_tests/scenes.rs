use super::*;

#[test]
fn an_owed_animation_tick_runs_in_the_callback_before_the_next_raw_tick() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, _, _) = map_flashes::fixture(60);
    map_flashes::play(&mut app, AnimTarget::Hero);
    let raw = app.world().resource::<GameFrames>().frame;
    let elapsed = saved::snapshot(app.world_mut()).cast.unwrap().elapsed;
    app.init_resource::<crate::timing::logical::Step>();
    app.world_mut()
        .resource_mut::<crate::timing::logical::Step>()
        .callback = true;
    app.world_mut().run_system_once(step_animations).unwrap();
    assert_eq!(app.world().resource::<GameFrames>().frame, raw);
    assert_eq!(
        saved::snapshot(app.world_mut()).cast.unwrap().elapsed,
        elapsed + 1
    );
    app.world_mut()
        .resource_mut::<crate::timing::logical::Step>()
        .callback = false;
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.world_mut().run_system_once(step_animations).unwrap();
    assert_eq!(
        saved::snapshot(app.world_mut()).cast.unwrap().elapsed,
        elapsed + 2
    );
}

fn cells(app: &mut App) -> Vec<(Entity, Visibility)> {
    app.world_mut()
        .query_filtered::<(Entity, &Visibility), With<MeshMaterial2d<cells::CellMaterial>>>()
        .iter(app.world())
        .map(|(entity, visibility)| (entity, *visibility))
        .collect()
}

fn frame(app: &mut App) -> Option<usize> {
    app.world_mut()
        .query::<&LiveAnimation>()
        .iter(app.world())
        .next()
        .map(|animation| animation.frame)
}

#[test]
fn menu_shop_title_and_gameover_hold_the_animation_clock_without_restarting_cells() {
    for scene in 0..4 {
        let mut app = app(60);
        let initial = cells(&mut app)[0].0;
        match scene {
            0 => {
                app.insert_resource(crate::menu::MenuOpen(true));
            }
            1 => {
                app.insert_resource(crate::shop::ShopOpen(true));
            }
            2 => {
                app.insert_resource(crate::title::TitleActive(true));
            }
            _ => {
                app.insert_resource(crate::gameover::GameOverActive(true));
            }
        }
        for _ in 0..120 {
            app.update();
        }
        assert_eq!(frame(&mut app), Some(0), "scene {scene}");
        assert_eq!(cells(&mut app), [(initial, Visibility::Hidden)]);
        app.world_mut().remove_resource::<crate::menu::MenuOpen>();
        app.world_mut().remove_resource::<crate::shop::ShopOpen>();
        app.world_mut()
            .remove_resource::<crate::title::TitleActive>();
        app.world_mut()
            .remove_resource::<crate::gameover::GameOverActive>();
        app.update();
        assert_eq!(frame(&mut app), Some(0));
        assert_eq!(cells(&mut app), [(initial, Visibility::Inherited)]);
        app.update();
        assert_eq!(frame(&mut app), Some(1));
    }
}

#[test]
fn a_map_cast_continues_its_sounds_in_battle_without_drawing_over_the_battlers() {
    let mut app = app(60);
    app.insert_resource(crate::battle::BattleActive(true));
    app.update();
    assert_eq!(frame(&mut app), Some(0));
    assert_eq!(cells(&mut app)[0].1, Visibility::Hidden);
    let messages = app.world().resource::<Messages<AudioRequest>>();
    assert_eq!(messages.get_cursor().read(messages).count(), 1);
    app.update();
    assert_eq!(frame(&mut app), Some(1));
    assert_eq!(cells(&mut app)[0].1, Visibility::Hidden);
    app.world_mut()
        .resource_mut::<crate::battle::BattleActive>()
        .0 = false;
    app.update();
    assert_eq!(frame(&mut app), Some(1));
    assert_eq!(cells(&mut app)[0].1, Visibility::Inherited);
}

#[test]
fn the_battle_scene_not_the_early_encounter_flag_controls_map_cell_visibility() {
    let mut app = app(60);
    app.insert_resource(crate::battle::BattleActive(true));
    app.init_resource::<crate::battle::Battle>();
    app.update();
    assert_eq!(cells(&mut app)[0].1, Visibility::Inherited);
    app.world_mut()
        .resource_mut::<crate::battle::Battle>()
        .phase = crate::battle::Phase::PartyCommand;
    app.update();
    assert_eq!(cells(&mut app)[0].1, Visibility::Hidden);
    app.world_mut()
        .resource_mut::<crate::battle::Battle>()
        .phase = crate::battle::Phase::Inactive;
    app.update();
    assert_eq!(cells(&mut app)[0].1, Visibility::Inherited);
}

#[test]
fn a_menu_preserves_the_current_character_flash_until_the_next_map_update() {
    let (mut app, hero, _) = map_flashes::fixture(60);
    map_flashes::play(&mut app, AnimTarget::Hero);
    app.update();
    assert_eq!(map_flashes::color(&app, hero), [248, 160, 80, 248]);
    app.insert_resource(crate::menu::MenuOpen(true));
    for _ in 0..120 {
        app.update();
        assert_eq!(map_flashes::color(&app, hero), [248, 160, 80, 248]);
    }
    app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(map_flashes::color(&app, hero), [248, 160, 80, 200]);
}

#[test]
fn only_battle_slots_hold_battle_actions_while_a_map_cast_is_still_alive() {
    let mut app = app(60);
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 1);
    assert_eq!(app.world().resource::<ActiveAnimations>().battle, 0);
    let mut short = app.world().resource::<AnimationLibrary>().0[0].clone();
    short.id = 2;
    short.frames.truncate(1);
    app.world_mut()
        .resource_mut::<AnimationLibrary>()
        .0
        .push(short);
    app.world_mut().write_message(PlayAnimation {
        slot: AnimationSlot::Enemies,
        map_target: None,
        anim_id: 2,
        targets: vec![AnimAnchor {
            pos: Vec2::ZERO,
            height: 48.0,
        }],
        screen_center: Vec2::ZERO,
        global: false,
        sound_only: false,
    });
    app.update();
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 2);
    assert_eq!(app.world().resource::<ActiveAnimations>().battle, 1);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 1);
    assert_eq!(app.world().resource::<ActiveAnimations>().battle, 0);
}

#[test]
fn screen_flashes_hold_their_envelope_and_are_hidden_while_a_menu_owns_the_scene() {
    let mut app = app(60);
    app.add_systems(Update, fade_flashes.before(step_animations));
    let mut library = app.world_mut().resource_mut::<AnimationLibrary>();
    library.0[0].timings.truncate(1);
    library.0[0].timings[0].flash_scope = FLASH_SCOPE_SCREEN;
    app.update();
    let quad = app
        .world_mut()
        .query_filtered::<Entity, With<render::FlashQuad>>()
        .single(app.world())
        .unwrap();
    app.insert_resource(crate::menu::MenuOpen(true));
    for _ in 0..30 {
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(quad).unwrap().color.alpha(),
            248.0 / 255.0
        );
        assert_eq!(
            app.world().get::<Visibility>(quad),
            Some(&Visibility::Hidden)
        );
    }
    app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(quad).unwrap().color.alpha(),
        248.0 / 255.0
    );
    assert_eq!(
        app.world().get::<Visibility>(quad),
        Some(&Visibility::Inherited)
    );
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<Sprite>(quad).unwrap().color.alpha(),
        200.0 / 255.0
    );
}
