use super::*;

#[test]
fn a_real_reload_keeps_restored_cells_and_flashes_frozen_until_the_fade_finishes() {
    let (mut app, path) = app("real_reload");
    app.add_plugins((
        crate::transitions::TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .init_resource::<crate::player::CameraPan>()
    .init_resource::<crate::world::MapEvents>();
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(hero).insert((
        crate::world::MoveQueue::default(),
        crate::world::RouteStepper::default(),
    ));
    play(&mut app);
    let expected = saved::snapshot(app.world_mut());
    let flash = app
        .world()
        .get::<crate::legacy_colors::flash::SpriteFlash>(hero)
        .unwrap()
        .0;
    save(&mut app);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    let mut held = 0;
    let mut cells = None;
    for _ in 0..90 {
        step(&mut app, 1);
        if !app.world().resource::<crate::teleport::Fade>().busy() {
            break;
        }
        if app.world().contains_resource::<saved::Pending>() {
            continue;
        }
        held += 1;
        assert_eq!(saved::snapshot(app.world_mut()), expected);
        assert_eq!(
            app.world()
                .get::<crate::legacy_colors::flash::SpriteFlash>(hero)
                .unwrap()
                .0,
            flash
        );
        let animation = app
            .world_mut()
            .query::<&playback::LiveAnimation>()
            .single(app.world())
            .unwrap();
        if let Some(previous) = &cells {
            assert_eq!(&animation.cells, previous);
        } else {
            cells = Some(animation.cells.clone());
        }
        for &cell in &animation.cells {
            assert_ne!(
                *app.world().get::<Visibility>(cell).unwrap(),
                Visibility::Hidden
            );
        }
        assert_eq!(app.world().resource::<ActiveAnimations>().total, 1);
    }
    std::fs::remove_file(path).unwrap();
    assert!(held > 30);
    assert!(!app.world().resource::<crate::teleport::Fade>().busy());
    step(&mut app, 1);
    assert!(
        saved::snapshot(app.world_mut()).cast.unwrap().elapsed > expected.cast.unwrap().elapsed
    );
}

#[test]
fn session_cleanup_discards_a_restore_that_has_not_arrived_yet() {
    let (mut app, _) = app("aborted");
    play(&mut app);
    let state = saved::snapshot(app.world_mut());
    saved::prepare(app.world_mut(), 3, state);
    assert!(app.world().contains_resource::<saved::Pending>());
    crate::session::clear_transient(app.world_mut());
    assert!(!app.world().contains_resource::<saved::Pending>());
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert_eq!(saved::snapshot(app.world_mut()), default());
}
