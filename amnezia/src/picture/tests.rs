use super::*;

mod transfers;

#[test]
fn show_maps_position_flag_tone_transparency_and_zoom() {
    let params = [2, 1, 93, 94, 0, 100, 0, 1, 100, 100, 100, 100, 0, 60];
    assert_eq!(
        PictureCommand::show(2, "Cross", 93.0, 94.0, &params),
        PictureCommand::Show {
            id: 2,
            name: "Cross".into(),
            x: 93.0,
            y: 94.0,
            fixed_to_map: false,
            use_transparent_color: true,
            transparency: 0.0,
            zoom: 100.0,
            tone: Tone::NEUTRAL,
            effect: Effect::default(),
        }
    );
}

#[test]
fn show_reads_the_fixed_to_map_flag_and_a_grayscale_tone() {
    let fixed = [1, 1, 12, 13, 1, 150, 60, 1, 100, 100, 100, 100, 0, 60];
    let gray = [1, 0, 74, 120, 0, 100, 100, 0, 100, 100, 100, 0, 0, 60];
    match PictureCommand::show(1, "AirshipShadow", 12.0, 13.0, &fixed) {
        PictureCommand::Show { fixed_to_map, .. } => assert!(fixed_to_map),
        _ => panic!("expected a Show"),
    }
    match PictureCommand::show(1, "Intro1", 74.0, 120.0, &gray) {
        PictureCommand::Show {
            tone, fixed_to_map, ..
        } => {
            assert!(!fixed_to_map);
            assert_eq!(tone.sat, 0.0);
            assert_eq!((tone.r, tone.g, tone.b), (100.0, 100.0, 100.0));
        }
        _ => panic!("expected a Show"),
    }
}

#[test]
fn move_reads_duration_and_tone_from_the_command() {
    let params = [1, 0, 246, 120, 0, 100, 0, 0, 100, 100, 100, 0, 0, 0, 30, 1];
    assert_eq!(
        PictureCommand::move_to(1, 246.0, 120.0, &params),
        PictureCommand::Move {
            id: 1,
            x: 246.0,
            y: 120.0,
            transparency: 0.0,
            zoom: 100.0,
            tone: Tone {
                r: 100.0,
                g: 100.0,
                b: 100.0,
                sat: 0.0,
            },
            secs: 3.0,
            effect: Effect::default(),
        }
    );
}

#[test]
fn lerp_interpolates_endpoints_and_midpoint() {
    assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
}

#[test]
fn tween_interpolates_the_tone_toward_grayscale() {
    let mut pic = Picture {
        id: 1,
        x: 0.0,
        y: 0.0,
        transparency: 0.0,
        zoom: 100.0,
        tone: Tone::NEUTRAL,
        fixed_to_map: false,
        use_transparent_color: false,
        world_anchor: None,
        base_size: None,
        tween: None,
        effect: effects::EffectState::default(),
        frame_fraction: 0.0,
    };
    pic.retarget(
        Anim {
            x: 0.0,
            y: 0.0,
            transparency: 0.0,
            zoom: 100.0,
            tone: Tone {
                r: 100.0,
                g: 100.0,
                b: 100.0,
                sat: 0.0,
            },
        },
        Effect::default(),
        2.0,
    );
    let tween = pic.tween.expect("a tween");
    let mid = Anim::lerp(tween.from, tween.to, 0.5);
    assert_eq!(mid.tone.sat, 50.0);
}

#[test]
fn a_map_rebuild_despawns_every_picture() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<MapRebuilt>();
    app.add_systems(Update, clear_on_map_change);
    app.world_mut().spawn(test_picture(1));
    app.world_mut().spawn(test_picture(2));

    app.update();
    assert_eq!(count_pictures(&mut app), 2);

    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
}

fn test_picture(id: u32) -> Picture {
    Picture {
        id,
        x: 0.0,
        y: 0.0,
        transparency: 0.0,
        zoom: 100.0,
        tone: Tone::NEUTRAL,
        fixed_to_map: false,
        use_transparent_color: false,
        world_anchor: None,
        base_size: None,
        tween: None,
        effect: effects::EffectState::default(),
        frame_fraction: 0.0,
    }
}

fn count_pictures(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Picture>()
        .iter(app.world())
        .count()
}

#[test]
fn original_picture_commands_preserve_all_color_key_choices() {
    let mut opaque = 0;
    let mut keyed = 0;
    for id in 1..=276 {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_{id:04}.ron",
            crate::assets::asset_root()
        ));
        for command in map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .flat_map(|p| &p.commands)
        {
            if command.code != 11110 {
                continue;
            }
            let PictureCommand::Show {
                use_transparent_color,
                ..
            } = PictureCommand::show(1, &command.string, 0.0, 0.0, &command.params)
            else {
                unreachable!()
            };
            assert_eq!(use_transparent_color, command.params[7] > 0);
            if use_transparent_color {
                keyed += 1
            } else {
                opaque += 1
            }
        }
    }
    assert_eq!((opaque, keyed), (34, 11));
    assert!(matches!(
        PictureCommand::show(1, "Fog", 0.0, 0.0, &[]),
        PictureCommand::Show {
            use_transparent_color: false,
            ..
        }
    ));
}

fn render_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: crate::assets::asset_root().into(),
            ..default()
        },
    ))
    .init_asset::<Image>()
    .init_asset::<Mesh>()
    .init_asset::<render::PictureMaterial>()
    .add_message::<PictureCommand>()
    .add_systems(Startup, render::setup_picture_mesh)
    .add_systems(
        Update,
        (render::apply_commands, render::place_pictures).chain(),
    );
    app.world_mut()
        .spawn((crate::world::MainCamera, Transform::default()));
    app
}

#[test]
fn same_frame_show_move_replacement_and_erase_obey_command_order() {
    let mut app = render_app();
    let show = [1, 0, 0, 0, 0, 100, 0, 1, 100, 100, 100, 100, 0, 60];
    let move_to = [1, 0, 0, 0, 0, 125, 25, 0, 100, 100, 100, 100, 0, 0, 0];
    app.world_mut()
        .write_message(PictureCommand::show(1, "Cross", 10.0, 20.0, &show));
    app.world_mut()
        .write_message(PictureCommand::move_to(1, 30.0, 40.0, &move_to));
    app.update();
    let world = app.world_mut();
    let (picture, handle) = world
        .query::<(&Picture, &MeshMaterial2d<render::PictureMaterial>)>()
        .single(world)
        .unwrap();
    assert_eq!((picture.x, picture.y, picture.zoom), (30.0, 40.0, 125.0));
    assert!(picture.use_transparent_color);
    assert_eq!(
        world
            .resource::<Assets<render::PictureMaterial>>()
            .get(&handle.0)
            .unwrap()
            .extra,
        Vec4::new(191.0 / 255.0, 1.0, 0.0, 0.0)
    );

    for name in ["Cross", "RonFace", "StellaFace"] {
        app.world_mut()
            .write_message(PictureCommand::show(1, name, 0.0, 0.0, &[]));
    }
    app.update();
    assert_eq!(count_pictures(&mut app), 1);
    let world = app.world_mut();
    let (picture, handle) = world
        .query::<(&Picture, &MeshMaterial2d<render::PictureMaterial>)>()
        .single(world)
        .unwrap();
    assert!(!picture.use_transparent_color);
    let material = world
        .resource::<Assets<render::PictureMaterial>>()
        .get(&handle.0)
        .unwrap();
    assert_eq!(material.extra.y, 0.0);
    assert!(
        material
            .image
            .path()
            .unwrap()
            .path()
            .ends_with("StellaFace.png")
    );

    app.world_mut()
        .write_message(PictureCommand::show(2, "Cross", 0.0, 0.0, &show));
    app.world_mut().write_message(PictureCommand::erase(2));
    app.world_mut().write_message(PictureCommand::erase(1));
    app.update();
    assert_eq!(count_pictures(&mut app), 0);
}
