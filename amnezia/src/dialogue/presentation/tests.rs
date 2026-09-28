use super::*;
use crate::dialogue::{Dialogue, MessageOptions, MessagePosition, MessageTransparent, PromptClock};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Dialogue>()
        .init_resource::<MessageOptions>()
        .init_resource::<MessagePosition>()
        .init_resource::<MessageTransparent>()
        .init_resource::<PromptClock>()
        .init_resource::<crate::battle::BattleActive>()
        .insert_resource(crate::timing::GameFrames::default())
        .add_systems(Startup, view::spawn_ui);
    register_windows(&mut app);
    app.update();
    app
}

fn open(world: &mut World) {
    let mut dialogue = world.resource_mut::<Dialogue>();
    dialogue.open(vec![crate::events::MessageBox {
        face: None,
        face_index: 0,
        lines: vec!["Destination message".into()],
    }]);
    dialogue.from_foreground = false;
    dialogue.lifecycle.message.open(false);
}

#[test]
fn an_inline_window_refresh_paints_closing_without_advancing_its_clock() {
    let mut app = app();
    let world = app.world_mut();
    open(world);
    flush(world);
    world.resource_mut::<Dialogue>().finish_parallel(0);
    for _ in 0..2 {
        flush(world);
        let (pixels, visibility) = world
            .query_filtered::<
                (&crate::windowskin::motion::Pixels, &Visibility),
                With<view::motion::AnimatedFrame>,
            >()
            .single(world)
            .unwrap();
        assert_eq!(pixels.half, 34);
        assert_eq!(*visibility, Visibility::Visible);
        assert_eq!(
            world
                .resource::<Dialogue>()
                .lifecycle
                .message
                .half_height(80),
            34
        );
        assert_eq!(world.resource::<crate::timing::GameFrames>().frame, 0);
    }
}

#[test]
fn inline_and_scheduled_refreshes_share_the_latched_window_position() {
    let mut app = app();
    let world = app.world_mut();
    world.insert_resource(crate::world::MapData::for_test(20, 15));
    world.spawn((crate::world::MainCamera, Transform::default()));
    let hero = world
        .spawn((
            crate::player::Player {
                tile_x: 5,
                tile_y: 12,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            Transform::from_xyz(0.0, -88.0 + crate::tiles::CHAR_Y_OFFSET, 0.0),
        ))
        .id();
    open(world);
    flush(world);
    let panel = world
        .query_filtered::<Entity, With<view::DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(world.get::<Node>(panel).unwrap().top, Val::Px(0.0));
    world.get_mut::<Transform>(hero).unwrap().translation.y = 24.0 + crate::tiles::CHAR_Y_OFFSET;
    app.update();
    assert_eq!(app.world().get::<Node>(panel).unwrap().top, Val::Px(0.0));
    open(app.world_mut());
    flush(app.world_mut());
    assert_eq!(app.world().get::<Node>(panel).unwrap().bottom, Val::Px(0.0));
}
