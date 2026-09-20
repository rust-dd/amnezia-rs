use super::*;

fn terms() -> Terms {
    let mut terms = Terms::default();
    terms.0.exit_game_message = "Játék vége?".into();
    terms.0.yes = "Igen".into();
    terms.0.no = "Nem".into();
    terms
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(terms())
        .insert_resource(BitmapFont::from_id(0))
        .insert_resource(MenuOpen(true))
        .insert_resource(MenuState {
            cursor: 4,
            screen: MenuScreen::EndGame { cursor: 1 },
        })
        .init_resource::<Clock>()
        .init_resource::<GameFrames>()
        .add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|panel| spawn(panel, &Handle::default()));
        })
        .add_systems(Update, update);
    app.update();
    app
}

#[test]
fn original_end_game_windows_fit_the_database_labels_and_keep_native_margins() {
    let mut app = app();
    let world = app.world_mut();
    for (kind, node, visibility) in world
        .query::<(&EndWindow, &Node, &Visibility)>()
        .iter(world)
    {
        let rect = match kind {
            EndWindow::Help => (119, 72, 82, 32),
            EndWindow::Commands => (140, 120, 40, 48),
        };
        assert_eq!(
            (node.left, node.top, node.width, node.height),
            (
                Val::Px(rect.0 as f32 * 3.0),
                Val::Px(rect.1 as f32 * 3.0),
                Val::Px(rect.2 as f32 * 3.0),
                Val::Px(rect.3 as f32 * 3.0)
            )
        );
        assert_eq!(*visibility, Visibility::Inherited);
    }
    let labels = world
        .query::<(&EndText, &PixelText, &Node)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 3);
    for (label, text, node) in labels {
        assert_eq!(
            text.runs,
            [Run::new(
                ["Játék vége?", "Igen", "Nem"][label.0],
                0,
                0,
                DEFAULT
            )]
        );
        assert_eq!(node.left, Val::Px(24.0));
        assert_eq!(node.top, Val::Px(if label.0 == 2 { 78.0 } else { 30.0 }));
    }
}

#[test]
fn end_game_cursor_moves_between_both_original_rows_and_all_nine_phase_pieces() {
    let mut app = app();
    app.world_mut().resource_mut::<GameFrames>().frame = 12;
    app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::EndGame { cursor: 0 };
    app.update();
    let world = app.world_mut();
    let (node, children) = world
        .query_filtered::<(&Node, &Children), With<EndCursor>>()
        .single(world)
        .unwrap();
    assert_eq!(
        (node.left, node.top, node.width, node.height),
        (Val::Px(12.0), Val::Px(24.0), Val::Px(96.0), Val::Px(48.0))
    );
    assert_eq!(children.len(), 9);
    for entity in children {
        let rect = world.get::<ImageNode>(*entity).unwrap().rect.unwrap();
        assert!(rect.min.x >= 96.0 && rect.max.x <= 128.0);
    }
}

#[test]
fn changing_terms_resizes_the_existing_end_game_windows_and_bitmap_content() {
    let mut app = app();
    app.world_mut().resource_mut::<Terms>().0.yes = "Folytasd".into();
    app.update();
    let world = app.world_mut();
    let (_, node) = world
        .query::<(&EndWindow, &Node)>()
        .iter(world)
        .find(|(kind, _)| **kind == EndWindow::Commands)
        .unwrap();
    assert_eq!((node.left, node.width), (Val::Px(384.0), Val::Px(192.0)));
    for (label, text) in world.query::<(&EndText, &PixelText)>().iter(world) {
        if label.0 != 0 {
            assert_eq!(text.size.x, 48);
        }
    }
}

#[test]
fn end_game_clock_uses_logical_frames_and_resets_on_reentry_without_pause_catchup() {
    for fps in [15, 30, 60, 144] {
        let mut frames = GameFrames::default();
        let mut clock = Clock::default();
        clock.advance(0, true, false);
        for _ in 0..fps * 3 {
            frames.advance(1.0 / fps as f64);
            clock.advance(frames.frame, true, false);
            assert_eq!(clock.phase, frames.frame % 21);
        }
        let held = clock.phase;
        clock.advance(frames.frame + 500, true, true);
        assert_eq!(clock.phase, held);
        clock.advance(frames.frame + 501, true, false);
        assert_eq!(clock.phase, (held + 1) % 21);
        clock.advance(frames.frame + 1000, false, false);
        clock.advance(frames.frame + 2000, true, false);
        assert_eq!(clock.phase, 0);
    }
}

#[test]
fn end_game_windows_disappear_when_cancelled_or_when_the_menu_closes() {
    for close_menu in [false, true] {
        let mut app = app();
        if close_menu {
            app.world_mut().resource_mut::<MenuOpen>().0 = false;
        } else {
            app.world_mut().resource_mut::<MenuState>().screen = MenuScreen::Command;
        }
        app.update();
        let world = app.world_mut();
        assert!(
            world
                .query_filtered::<&Visibility, With<EndWindow>>()
                .iter(world)
                .all(|visibility| *visibility == Visibility::Hidden)
        );
    }
}

#[test]
fn both_end_windows_use_native_background_dimensions_and_update_without_replacing_entities() {
    let mut app = app();
    let mut original = Vec::new();
    for changed in [false, true] {
        if changed {
            app.world_mut().resource_mut::<Terms>().0.yes = "Folytasd".into();
            app.update();
        }
        let world = app.world_mut();
        let backgrounds = world
            .query::<(Entity, &EndWindow, &Children)>()
            .iter(world)
            .map(|(entity, kind, children)| {
                let pixels = children
                    .iter()
                    .filter_map(|child| {
                        world
                            .get::<crate::windowskin::background::Pixels>(child)
                            .map(|pixels| (child, pixels))
                    })
                    .collect::<Vec<_>>();
                assert_eq!(pixels.len(), 1);
                let expected = match kind {
                    EndWindow::Help => UVec2::new(82, 32),
                    EndWindow::Commands => UVec2::new(if changed { 64 } else { 40 }, 48),
                };
                assert_eq!(pixels[0].1.0, expected);
                (entity, pixels[0].0)
            })
            .collect::<Vec<_>>();
        if changed {
            assert_eq!(backgrounds, original);
        } else {
            original = backgrounds;
        }
    }
}
