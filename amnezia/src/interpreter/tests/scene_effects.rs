use super::*;
use crate::screenfx::TintState;
use crate::world::MainCamera;

pub(super) fn app() -> App {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        crate::player::PlayerPlugin,
        crate::timer::GameClockPlugin,
        crate::screenfx::ScreenFxPlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ))
    .insert_resource(MapData::for_test(40, 30));
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    {
        let mut hero = world.get_mut::<Player>(entity).unwrap();
        hero.tile_x = 20;
        hero.tile_y = 15;
    }
    world
        .entity_mut(entity)
        .insert((Sprite::default(), Transform::default()));
    world.spawn((
        MainCamera,
        Transform::default(),
        Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-160.0, -120.0, 160.0, 120.0),
            ..OrthographicProjection::default_2d()
        }),
    ));
    app.update();
    app
}

fn start(app: &mut App, parallel: bool, mut commands: Vec<EventCommand>) {
    if parallel {
        commands.push(cmd(11410, 0, vec![100]));
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, commands)],
        });
    } else {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(0, commands);
    }
}

#[test]
fn tone_commands_advance_only_if_the_screen_update_has_not_happened_yet() {
    for parallel in [false, true] {
        let mut app = app();
        start(
            &mut app,
            parallel,
            vec![cmd(11030, 0, vec![40, 100, 100, 100, 10, 0])],
        );
        app.update();
        assert_eq!(
            app.world().resource::<TintState>().tone()[0],
            if parallel { 99.0 } else { 100.0 }
        );
        app.update();
        assert_eq!(
            app.world().resource::<TintState>().tone()[0],
            if parallel { 98.0 } else { 99.0 }
        );
    }
}

#[test]
fn foreground_tone_replaces_the_parallel_command_without_replaying_it() {
    let mut app = app();
    start(
        &mut app,
        true,
        vec![cmd(11030, 0, vec![40, 100, 100, 100, 10, 0])],
    );
    start(
        &mut app,
        false,
        vec![cmd(11030, 0, vec![80, 100, 100, 100, 0, 0])],
    );
    app.update();
    assert_eq!(app.world().resource::<TintState>().tone()[0], 80.0);
    app.update();
    assert_eq!(app.world().resource::<TintState>().tone()[0], 80.0);
}

#[test]
fn timer_queries_straddle_the_map_timer_update() {
    let mut app = app();
    {
        let mut clock = app.world_mut().resource_mut::<GameClock>();
        clock.remaining = 2.005;
        clock.start();
    }
    let query = |var| cmd(10220, 0, vec![0, var, var, 0, 7, 1]);
    start(&mut app, true, vec![query(1)]);
    start(&mut app, false, vec![query(2)]);
    app.update();
    let variables = app.world().resource::<Variables>();
    assert_eq!(variables.get(1), 2);
    assert_eq!(variables.get(2), 1);
}

#[test]
fn starting_a_timer_after_the_timer_update_preserves_its_first_tick() {
    for parallel in [false, true] {
        let mut app = app();
        start(
            &mut app,
            parallel,
            vec![cmd(10230, 0, vec![0, 0, 10]), cmd(10230, 0, vec![1])],
        );
        app.update();
        let expected = 10.0 + (59.0 - f32::from(u8::from(parallel))) / 60.0;
        assert!((app.world().resource::<GameClock>().remaining - expected).abs() < 0.00001);
        app.update();
        assert!(
            (app.world().resource::<GameClock>().remaining - expected + 1.0 / 60.0).abs() < 0.00001
        );
    }
}

#[test]
fn a_pending_save_selector_holds_the_map_timer_before_its_fade() {
    let mut app = app();
    {
        let mut clock = app.world_mut().resource_mut::<GameClock>();
        clock.remaining = 10.0;
        clock.start();
    }
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<GameClock>().remaining, 10.0);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = false;
    app.update();
    assert!(app.world().resource::<GameClock>().remaining < 10.0);
}

#[test]
fn a_foreground_pan_starts_next_update_before_foreground_screen_queries() {
    let mut app = app();
    start(&mut app, false, vec![cmd(11060, 0, vec![2, 1, 1, 4, 0])]);
    app.update();
    assert_eq!(app.world().resource::<CameraPan>().offset, Vec2::ZERO);
    let query = |var| cmd(10220, 0, vec![0, var, var, 0, 6, 10001, 4]);
    start(&mut app, true, vec![query(1)]);
    start(&mut app, false, vec![query(2)]);
    app.update();
    assert_eq!(
        app.world().resource::<CameraPan>().offset,
        Vec2::new(2.0, 0.0)
    );
    let variables = app.world().resource::<Variables>();
    assert_eq!(variables.get(1), 152);
    assert_eq!(variables.get(2), 150);
}

#[test]
fn a_parallel_shake_is_visible_to_foreground_screen_queries_in_the_same_update() {
    let mut app = app();
    start(&mut app, true, vec![cmd(11050, 0, vec![3, 5, 10, 0])]);
    start(
        &mut app,
        false,
        vec![cmd(10220, 0, vec![0, 1, 1, 0, 6, 10001, 4])],
    );
    app.update();
    assert_eq!(app.world().resource::<Variables>().get(1), 154);
}
