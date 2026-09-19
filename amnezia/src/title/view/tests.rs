use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::terms::Terms;
use crate::timing::GameFrames;

fn app() -> App {
    let mut terms = Terms::default();
    terms.0.new_game = "Új játék".into();
    terms.0.load_game = "Betöltés".into();
    terms.0.exit_game = "Kilépés".into();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(BitmapFont::from_id(0))
        .init_resource::<clock::Clock>()
        .insert_resource(terms)
        .init_resource::<GameFrames>()
        .init_resource::<TitleActive>()
        .init_resource::<crate::save::SaveLocation>()
        .insert_resource(TitleState {
            stage: flow::Stage::Showing,
            ..default()
        })
        .add_systems(Startup, spawn)
        .add_systems(Update, (clock::tick, update).chain());
    app.update();
    app.world_mut().resource_mut::<TitleState>().stage = flow::Stage::Ready;
    app.world_mut().resource_mut::<GameFrames>().frame = 8;
    app.update();
    app
}

#[test]
fn title_uses_the_original_centered_64_by_64_command_window() {
    let mut app = app();
    let world = app.world_mut();
    assert!(world.query::<&Node>().iter(world).any(|node| {
        (node.left, node.top, node.width, node.height)
            == (
                Val::Px(384.0),
                Val::Px(444.0),
                Val::Px(192.0),
                Val::Px(192.0),
            )
    }));
}

#[test]
fn title_commands_are_bitmap_terms_without_an_invented_text_arrow() {
    let mut app = app();
    let world = app.world_mut();
    let rows = world
        .query::<(&TitleRow, &PixelText, &Node)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    for (row, text, node) in rows {
        let enabled =
            row.0 != CONTINUE || world.resource::<crate::save::SaveLocation>().has_saves();
        assert_eq!(text.size, UVec2::new(48, 16));
        assert_eq!(
            text.runs,
            [Run::new(
                ROWS[row.0],
                0,
                0,
                if enabled { DEFAULT } else { DISABLED }
            )]
        );
        assert_eq!(node.left, Val::Px(24.0));
        assert_eq!(node.top, Val::Px((10 + row.0 * 16) as f32 * 3.0));
    }
}

#[test]
fn only_continue_uses_the_disabled_palette_when_no_save_exists() {
    for has_save in [false, true] {
        assert_eq!(command_color(NEW_GAME, has_save), DEFAULT);
        assert_eq!(command_color(SHUTDOWN, has_save), DEFAULT);
        assert_eq!(
            command_color(CONTINUE, has_save),
            if has_save { DEFAULT } else { DISABLED }
        );
    }
}

#[test]
fn load_browsing_hides_the_title_without_reopening_or_aging_its_command_window() {
    let mut app = app();
    let phase = app.world().resource::<clock::Clock>().source_x();
    for (stage, frame, visible) in [
        (flow::Stage::Files, 1000, false),
        (flow::Stage::FileLeaving(false), 1006, false),
        (flow::Stage::FileReturning, 1012, true),
    ] {
        app.world_mut().resource_mut::<TitleState>().stage = stage;
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<clock::Clock>().opened, 8);
        assert_eq!(world.resource::<clock::Clock>().source_x(), phase);
        let visibility = world
            .query_filtered::<&Visibility, With<TitleRoot>>()
            .single(world)
            .unwrap();
        assert_eq!(*visibility == Visibility::Visible, visible);
    }
}

#[test]
fn title_cursor_uses_nine_original_phase_pieces_and_tracks_all_three_rows() {
    let mut app = app();
    app.world_mut().resource_mut::<GameFrames>().frame = 11;
    app.world_mut().resource_mut::<TitleState>().cursor = SHUTDOWN;
    app.update();
    let world = app.world_mut();
    let (node, visibility, children) = world
        .query_filtered::<(&Node, &Visibility, &Children), With<TitleCursor>>()
        .single(world)
        .unwrap();
    assert_eq!(
        (node.left, node.top, node.width, node.height),
        (Val::Px(12.0), Val::Px(120.0), Val::Px(168.0), Val::Px(48.0))
    );
    assert_eq!(*visibility, Visibility::Inherited);
    assert_eq!(children.len(), 9);
    for child in children {
        let rect = world.get::<ImageNode>(*child).unwrap().rect.unwrap();
        assert!(rect.min.x >= 96.0 && rect.max.x <= 128.0);
    }
}

#[test]
fn title_terms_resize_the_existing_window_without_rebuilding_the_ui() {
    let mut app = app();
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<TitleWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut().resource_mut::<Terms>().0.new_game = "Másik játék".into();
    app.update();
    let node = app.world().get::<Node>(entity).unwrap();
    assert_eq!((node.left, node.width), (Val::Px(357.0), Val::Px(246.0)));
    let world = app.world_mut();
    assert!(
        world
            .query::<&PixelText>()
            .iter(world)
            .any(|text| text.runs[0].text == "Másik játék" && text.size.x == 66)
    );
}

#[test]
fn returning_to_the_title_hides_contents_until_the_eighth_opening_frame() {
    let mut app = app();
    app.world_mut().resource_mut::<TitleActive>().0 = false;
    app.update();
    app.world_mut().resource_mut::<TitleActive>().0 = true;
    app.world_mut().resource_mut::<TitleState>().stage = flow::Stage::Showing;
    app.world_mut().resource_mut::<GameFrames>().frame = 100;
    app.update();
    app.world_mut().resource_mut::<TitleState>().stage = flow::Stage::Ready;
    for frame in 100..=108 {
        app.world_mut().resource_mut::<GameFrames>().frame = frame;
        app.update();
        let world = app.world_mut();
        for visibility in world
            .query_filtered::<&Visibility, Or<(With<TitleRow>, With<TitleCursor>)>>()
            .iter(world)
        {
            assert_eq!(
                *visibility,
                if frame == 108 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                }
            );
        }
    }
}
