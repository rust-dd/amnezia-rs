use super::*;
use crate::choice::Choice;
use crate::dialogue::view::prompts as presentation;
use crate::inputnumber::InputNumber;

mod embedded;

fn prompt_app() -> App {
    let mut app = app();
    app.init_resource::<Choice>()
        .init_resource::<InputNumber>()
        .init_resource::<presentation::Clock>()
        .init_resource::<MessagePosition>()
        .init_resource::<crate::dialogue::MessageOptions>()
        .add_systems(
            Update,
            (presentation::render_cursor, update_position).after(render_reveal),
        );
    app
}

fn remember_face(app: &mut App, name: &str) {
    crate::events::message_boxes(
        &[amnezia_data::EventCommand {
            code: 10130,
            indent: 0,
            string: name.into(),
            params: vec![6, 0, 0],
        }],
        &mut app.world_mut().resource_mut::<Dialogue>().face,
    );
}

fn cursor(app: &mut App) -> (Node, Visibility) {
    let world = app.world_mut();
    let (node, visibility) = world
        .query_filtered::<(&Node, &Visibility), With<presentation::Cursor>>()
        .single(world)
        .unwrap();
    (node.clone(), *visibility)
}

#[test]
fn choices_share_the_original_message_window_and_bitmap_line_origins() {
    let mut app = app();
    app.init_resource::<Choice>();
    app.world_mut().resource_mut::<Choice>().open(
        ["Első", "Második", "Harmadik", "Negyedik"]
            .map(str::to_string)
            .to_vec(),
        0,
        0,
    );
    app.update();
    let world = app.world_mut();
    let visibility = world
        .query_filtered::<&Visibility, With<DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(*visibility, Visibility::Visible);
    assert_eq!(
        text(&mut app).runs,
        ["Első", "Második", "Harmadik", "Negyedik"]
            .into_iter()
            .enumerate()
            .map(|(row, label)| Run::new(label, 12, 2 + row as i32 * 16, DEFAULT))
            .collect::<Vec<_>>()
    );
}

#[test]
fn number_input_uses_twelve_pixel_digit_columns_without_text_brackets() {
    let mut app = app();
    app.init_resource::<InputNumber>();
    app.world_mut().resource_mut::<InputNumber>().open(4, 76);
    app.update();
    assert_eq!(
        text(&mut app).runs,
        (0..4)
            .map(|digit| Run::new("0", 12 + digit * 12, 2, DEFAULT))
            .collect::<Vec<_>>()
    );
}

#[test]
fn remembered_portraits_shift_both_prompt_text_and_the_original_cursors() {
    let mut app = prompt_app();
    for face in [true, false] {
        remember_face(&mut app, if face { "Ron" } else { "" });
        app.world_mut()
            .resource_mut::<Choice>()
            .open(vec!["Igen".into(), "Nem".into()], 0, 1);
        app.world_mut().resource_mut::<Choice>().cursor = 1;
        app.update();
        assert_eq!(
            text(&mut app).runs[0].position,
            IVec2::new(if face { 84 } else { 12 }, 2)
        );
        let (node, visibility) = cursor(&mut app);
        assert_eq!(visibility, Visibility::Visible);
        assert_eq!(node.left, Val::Px(if face { 246.0 } else { 30.0 }));
        assert_eq!((node.top, node.height), (Val::Px(72.0), Val::Px(48.0)));
        assert_eq!(node.width, Val::Px(if face { 684.0 } else { 900.0 }));
        app.world_mut().resource_mut::<Choice>().active = false;
        app.world_mut().resource_mut::<InputNumber>().open(4, 76);
        app.update();
        let (node, visibility) = cursor(&mut app);
        assert_eq!(visibility, Visibility::Visible);
        assert_eq!(node.left, Val::Px(if face { 372.0 } else { 156.0 }));
        assert_eq!(
            (node.top, node.width, node.height),
            (Val::Px(24.0), Val::Px(42.0), Val::Px(48.0))
        );
        app.world_mut().resource_mut::<InputNumber>().active = false;
    }
}

#[test]
fn transparent_prompts_keep_the_face_text_and_cursor_without_a_second_panel() {
    let mut app = prompt_app();
    remember_face(&mut app, "Ron");
    app.world_mut().resource_mut::<MessageTransparent>().0 = true;
    app.world_mut().resource_mut::<InputNumber>().open(1, 24);
    app.update();
    let world = app.world_mut();
    let (image, visibility) = world
        .query_filtered::<(&ImageNode, &Visibility), With<DialogueFace>>()
        .single(world)
        .unwrap();
    assert_eq!(*visibility, Visibility::Visible);
    assert_eq!(image.rect, Some(Rect::new(96.0, 48.0, 144.0, 96.0)));
    assert_eq!(
        *world
            .query_filtered::<&Visibility, With<DialogueFrame>>()
            .single(world)
            .unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        world
            .query_filtered::<Entity, With<DialoguePanel>>()
            .iter(world)
            .count(),
        1
    );
    assert_eq!(cursor(&mut app).1, Visibility::Visible);
    assert_eq!(text(&mut app).runs, vec![Run::new("0", 84, 2, DEFAULT)]);
    app.world_mut().resource_mut::<InputNumber>().active = false;
    app.update();
    assert_eq!(cursor(&mut app).1, Visibility::Hidden);
    assert!(text(&mut app).runs.is_empty());
}

#[test]
fn prompts_use_fixed_message_positions_and_the_battle_bottom_override() {
    let mut app = prompt_app();
    app.world_mut()
        .resource_mut::<crate::dialogue::MessageOptions>()
        .fixed = true;
    for position in [
        MessagePosition::Top,
        MessagePosition::Middle,
        MessagePosition::Bottom,
    ] {
        app.insert_resource(position);
        app.world_mut()
            .resource_mut::<Choice>()
            .open(vec!["Tovább".into()], 0, 1);
        app.update();
        let world = app.world_mut();
        let node = world
            .query_filtered::<&Node, With<DialoguePanel>>()
            .single(world)
            .unwrap();
        assert_eq!(
            node.top,
            match position {
                MessagePosition::Top => Val::Px(0.0),
                MessagePosition::Middle => Val::Px(240.0),
                MessagePosition::Bottom => Val::Auto,
            }
        );
    }
    app.insert_resource(MessagePosition::Top)
        .insert_resource(crate::battle::BattleActive(true));
    app.update();
    let world = app.world_mut();
    let node = world
        .query_filtered::<&Node, With<DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!((node.top, node.bottom), (Val::Auto, Val::Px(0.0)));
}
