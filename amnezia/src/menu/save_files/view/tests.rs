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
fn save_and_load_windows_have_native_backgrounds_without_gpu_resampling() {
    let mut app = app();
    app.update();
    let world = app.world_mut();
    let mut sizes = world
        .query::<&crate::windowskin::background::Pixels>()
        .iter(world)
        .map(|pixels| (pixels.0.x, pixels.0.y))
        .collect::<Vec<_>>();
    sizes.sort_unstable();
    assert_eq!(
        sizes,
        [(320, 32)]
            .into_iter()
            .chain([(320, 64); 15])
            .collect::<Vec<_>>()
    );
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
            timestamp: None,
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

#[test]
fn load_uses_its_original_prompt_and_disables_empty_and_corrupt_file_labels() {
    let mut app = app();
    let mut entries = vec![
        crate::save::preview::Entry {
            contents: Contents::Empty,
            timestamp: None,
        };
        15
    ];
    entries[14].contents = Contents::Corrupt;
    let mut files = app.world_mut().resource_mut::<SaveFiles>();
    files.mode = Mode::Load;
    files.entries = Some(entries);
    app.update();
    let world = app.world_mut();
    for (kind, text) in world.query::<(&Text, &PixelText)>().iter(world) {
        match kind {
            Text::Help => assert_eq!(text.runs, [Run::new("Honnan töltesz?", 0, 2, 0)]),
            Text::Slot(index) => {
                assert_eq!(
                    text.runs[0],
                    Run::new("File", 4, 2, crate::font::bitmap::DISABLED)
                );
                assert_eq!(
                    text.runs[1],
                    Run::new(
                        format!("{:>2}", index + 1),
                        31,
                        2,
                        crate::font::bitmap::DISABLED
                    )
                );
                assert_eq!(text.runs.len(), if *index == 14 { 3 } else { 2 });
            }
        }
    }
}

#[test]
fn file_arrows_wait_for_the_first_tick_and_update_before_the_list_moves() {
    let mut app = app();
    let mut files = app.world_mut().resource_mut::<SaveFiles>();
    files.entries = Some(vec![
        crate::save::preview::Entry {
            contents: Contents::Empty,
            timestamp: None,
        };
        15
    ]);
    files.navigation = crate::menu::save_files::navigation::Navigation::new(2);
    app.update();
    assert_arrows(&mut app, [false, false]);
    app.world_mut().resource_mut::<SaveFiles>().navigation.tick(
        [true, false, false, false, false, false],
        [true, false],
        true,
    );
    app.update();
    assert_arrows(&mut app, [false, true]);
    app.world_mut()
        .resource_mut::<SaveFiles>()
        .navigation
        .tick([false; 6], [false; 2], true);
    app.update();
    assert_arrows(&mut app, [true, true]);
    for _ in 0..18 {
        app.world_mut()
            .resource_mut::<SaveFiles>()
            .navigation
            .tick([false; 6], [false; 2], true);
    }
    app.update();
    assert_arrows(&mut app, [false, false]);
    for _ in 0..20 {
        app.world_mut()
            .resource_mut::<SaveFiles>()
            .navigation
            .tick([false; 6], [false; 2], true);
    }
    app.update();
    assert_arrows(&mut app, [true, true]);
}

fn assert_arrows(app: &mut App, expected: [bool; 2]) {
    let world = app.world_mut();
    for (arrow, visibility) in world.query::<(&Arrow, &Visibility)>().iter(world) {
        assert_eq!(
            *visibility != Visibility::Hidden,
            expected[usize::from(!arrow.0)]
        );
    }
}
