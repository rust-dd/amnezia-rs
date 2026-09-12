use super::*;
use crate::font::bitmap::{DEFAULT, PixelText, Run};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<Dialogue>()
        .init_resource::<MessageTransparent>()
        .add_systems(Startup, spawn_ui)
        .add_systems(Update, (render_box, render_reveal).chain());
    app.update();
    app
}

#[test]
fn dialogue_uses_the_original_bitmap_contents_instead_of_font_metrics() {
    let mut app = app();
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<DialogueText>>()
        .single(world)
        .unwrap();
    let text = world
        .get::<PixelText>(entity)
        .expect("bitmap dialogue text");
    assert_eq!(text.size, UVec2::new(304, 64));
    assert!(text.runs.is_empty());
    assert!(world.get::<TextFont>(entity).is_none());
    let node = world.get::<Node>(entity).unwrap();
    assert_eq!((node.left, node.top), (Val::Px(24.0), Val::Px(24.0)));
    assert_eq!((node.width, node.height), (Val::Px(912.0), Val::Px(192.0)));
}

#[test]
fn dialogue_frame_has_one_full_background_and_eight_fixed_border_pieces() {
    let mut app = app();
    let world = app.world_mut();
    let frames = world
        .query_filtered::<Entity, With<DialogueFrame>>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 1);
    let children = world.get::<Children>(frames[0]).unwrap();
    assert_eq!(children.len(), 9);
    let rects = children
        .iter()
        .map(|child| world.get::<ImageNode>(child).unwrap().rect.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rects[0], Rect::new(0.0, 0.0, 32.0, 32.0));
    assert_eq!(
        rects
            .iter()
            .filter(|rect| rect.size() == Vec2::splat(8.0))
            .count(),
        4
    );
    assert!(!rects.contains(&Rect::new(32.0, 0.0, 64.0, 32.0)));
}

fn show(app: &mut App, raw: &str, face: bool) {
    let mut dialogue = app.world_mut().resource_mut::<Dialogue>();
    dialogue.open(vec![crate::events::MessageBox {
        face: face.then(|| "Ron".into()),
        face_index: 0,
        lines: raw.lines().map(str::to_string).collect(),
    }]);
    let mut reveal =
        crate::dialogue::Typewriter::new(raw, "Ron", &crate::state::Variables::default());
    for _ in 0..200 {
        if reveal.is_complete() {
            break;
        }
        reveal.tick();
    }
    assert!(reveal.is_complete());
    dialogue.reveal = Some(reveal);
    app.update();
}

fn text(app: &mut App) -> PixelText {
    let world = app.world_mut();
    world
        .query_filtered::<&PixelText, With<DialogueText>>()
        .single(world)
        .unwrap()
        .clone()
}

#[test]
fn face_changes_preserve_the_original_four_line_origin_and_palette() {
    let mut app = app();
    let raw = "\\N[1]\nÁrvíztűrő tükörfúrógép.\nÁÉÍÓÖŐÚÜŰ\n0123456789";
    for face in [false, true, false] {
        show(&mut app, raw, face);
        assert_eq!(
            text(&mut app),
            PixelText {
                size: UVec2::new(304, 64),
                runs: vec![Run::new(
                    "Ron\nÁrvíztűrő tükörfúrógép.\nÁÉÍÓÖŐÚÜŰ\n0123456789",
                    if face { 72 } else { 0 },
                    2,
                    DEFAULT,
                )],
            }
        );
    }
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(text(&mut app).runs.is_empty());
}

#[test]
fn transparency_hides_only_the_skin_and_closing_clears_the_bitmap() {
    let mut app = app();
    show(&mut app, "Árvíztűrő", true);
    let expected = text(&mut app);
    for transparent in [true, false] {
        app.world_mut().resource_mut::<MessageTransparent>().0 = transparent;
        app.update();
        assert_eq!(text(&mut app), expected);
        let world = app.world_mut();
        let skin = world
            .query_filtered::<&Visibility, With<DialogueFrame>>()
            .single(world)
            .unwrap();
        assert_eq!(*skin, visible_if(!transparent));
        let face = world
            .query_filtered::<&Visibility, With<DialogueFace>>()
            .single(world)
            .unwrap();
        assert_eq!(*face, Visibility::Visible);
    }
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert!(text(&mut app).runs.is_empty());
}

#[test]
fn unchanged_reveal_does_not_invalidate_the_bitmap_cache() {
    let mut app = app();
    show(&mut app, "Ron", false);
    app.world_mut().clear_trackers();
    app.update();
    let world = app.world_mut();
    assert_eq!(
        world
            .query_filtered::<Entity, Changed<PixelText>>()
            .iter(world)
            .count(),
        0
    );
}
