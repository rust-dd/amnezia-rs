use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        crate::terms::TermsPlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(BitmapFont::from_id(0))
    .init_resource::<SaveFiles>()
    .add_systems(Startup, spawn)
    .add_systems(Update, update);
    app
}

#[test]
fn layout_uses_fifteen_full_width_rows_and_original_header_and_portrait_offsets() {
    let mut app = app();
    app.update();
    let world = app.world_mut();
    let rows = world
        .query::<(&Row, &Node)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 15);
    for (row, node) in rows {
        assert_eq!((node.width, node.height), (Val::Px(960.0), Val::Px(192.0)));
        assert_eq!(node.top, Val::Px(row.0 as f32 * 192.0));
    }
    let faces = world
        .query::<(&Face, &Node)>()
        .iter(world)
        .collect::<Vec<_>>();
    assert_eq!(faces.len(), 60);
    for (face, node) in faces {
        assert_eq!(node.left, Val::Px((96 + face.1 * 56) as f32 * 3.0));
        assert_eq!(
            (node.top, node.width, node.height),
            (Val::Px(24.0), Val::Px(144.0), Val::Px(144.0))
        );
    }
}

#[test]
fn save_rows_keep_empty_slots_enabled_and_render_no_invented_completion_or_playtime_text() {
    let mut app = app();
    app.world_mut().resource_mut::<SaveFiles>().entries = Some(vec![
        crate::save::preview::Entry {
            contents: Contents::Empty,
            modified: None,
        };
        15
    ]);
    app.update();
    let world = app.world_mut();
    for (kind, text) in world.query::<(&Text, &PixelText)>().iter(world) {
        match kind {
            Text::Help => assert_eq!(text.runs, [Run::new("Hova mentesz?", 0, 2, 0)]),
            Text::Slot(index) => assert_eq!(
                text.runs,
                [
                    Run::new("File", 4, 2, 0),
                    Run::new(format!("{:>2}", index + 1), 31, 2, 0)
                ]
            ),
        }
    }
}
