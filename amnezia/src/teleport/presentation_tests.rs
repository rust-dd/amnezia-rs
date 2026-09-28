use super::*;
use crate::animation::{ActiveAnimations, AnimationPlugin};
use crate::interpreter::{CommonEvents, InterpreterStep};
use crate::screenfx::{ScreenFxPlugin, TintState};
use amnezia_data::{CommonEvent, EventCommand};

fn command(code: u32, string: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: string.into(),
        params,
    }
}

fn app() -> App {
    let mut app = crate::interpreter::tests::interp_app();
    app.register_required_components::<Mesh2d, Visibility>();
    app.add_plugins((
        AssetPlugin {
            file_path: crate::assets::asset_root().into(),
            ..default()
        },
        ImagePlugin::default_nearest(),
        crate::transitions::TransitionPlugin,
        TeleportPlugin,
        ScreenFxPlugin,
        crate::picture::PicturePlugin,
        AnimationPlugin,
    ))
    .init_asset::<Mesh>()
    .init_asset::<bevy::audio::AudioSource>()
    .init_resource::<crate::audio::CurrentBgm>()
    .init_resource::<crate::audio::MemorizedBgm>()
    .add_systems(Update, crate::audio::flush.after(InterpreterStep))
    .add_message::<MapChanged>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    let world = app.world_mut();
    world.resource_mut::<MapData>().map_id = 3;
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world.entity_mut(hero).insert(Transform::default());
    world.spawn((crate::world::MainCamera, Transform::default()));
    app.update();
    app
}

fn arrive(app: &mut App, mut commands: Vec<EventCommand>) {
    commands.push(command(11410, "", vec![100]));
    app.insert_resource(CommonEvents(vec![CommonEvent {
        id: 900,
        name: "Destination graphics".into(),
        trigger: 4,
        switch_flag: false,
        switch_id: 0,
        commands,
    }]));
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .hold_black();
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .reserve((3, 7, 8), true);
    flow::begin_pending(app.world_mut());
    assert!(app.world().resource::<Fade>().busy());
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .busy()
    );
}

fn pictures(world: &mut World) -> Vec<crate::picture::saved::PictureState> {
    world
        .run_system_cached(|capture: crate::picture::saved::Capture| capture.snapshot())
        .unwrap()
}

#[test]
fn late_destination_pictures_and_animations_exist_before_show_without_aging() {
    let mut app = app();
    arrive(
        &mut app,
        vec![
            command(
                11110,
                "Cross",
                vec![1, 0, 96, 60, 0, 100, 0, 1, 100, 100, 100, 100, 0, 0],
            ),
            command(11210, "", vec![62, 10001, 0, 0]),
        ],
    );
    let before = pictures(app.world_mut());
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].frame_fraction, 0.0);
    let animation = crate::animation::saved::snapshot(app.world_mut());
    assert_eq!(animation.cast.as_ref().unwrap().elapsed, 0);
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 1);
    app.update();
    assert_eq!(pictures(app.world_mut()), before);
    assert_eq!(
        crate::animation::saved::snapshot(app.world_mut()),
        animation
    );
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.insert_resource(Fade::default());
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 1;
    app.update();
    let advanced = crate::animation::saved::snapshot(app.world_mut());
    assert_eq!(advanced.cast.as_ref().unwrap().elapsed, 1);
    let advanced_pictures = pictures(app.world_mut());
    assert_eq!(advanced_pictures[0].visual, before[0].visual);
    for _ in 0..3 {
        presentation::flush(app.world_mut());
        assert_eq!(crate::animation::saved::snapshot(app.world_mut()), advanced);
        assert_eq!(pictures(app.world_mut()), advanced_pictures);
    }
}

#[test]
fn late_destination_screen_and_audio_commands_arrive_before_show_without_aging() {
    let mut app = app();
    app.insert_resource(crate::audio::CurrentBgm::with_track(
        "Previous map",
        1.0,
        1.0,
    ));
    arrive(
        &mut app,
        vec![
            command(11030, "", vec![70, 90, 110, 50, 0, 0]),
            command(11040, "", vec![31, 10, 5, 20, 100, 0]),
            command(11050, "", vec![3, 5, 100, 0]),
            command(11510, "(OFF)", vec![0, 100, 100, 50]),
        ],
    );
    assert_eq!(
        app.world().resource::<TintState>().tone(),
        [70.0, 90.0, 110.0, 50.0]
    );
    assert!(
        app.world()
            .resource::<crate::audio::CurrentBgm>()
            .track()
            .is_none()
    );
    let before = crate::screenfx::saved::snapshot(app.world());
    app.update();
    assert_eq!(crate::screenfx::saved::snapshot(app.world()), before);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.insert_resource(Fade::default());
    app.update();
    let advanced = crate::screenfx::saved::snapshot(app.world());
    assert_ne!(advanced, before);
    for _ in 0..3 {
        presentation::flush(app.world_mut());
        assert_eq!(crate::screenfx::saved::snapshot(app.world()), advanced);
    }
}
