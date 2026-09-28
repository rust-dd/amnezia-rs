use super::*;
use crate::save::{EventSaveRequest, LoadRequest, SavePlugin};
use crate::timing::GameFrames;
use crate::world::{MapData, MapRebuilt};

mod resume;
mod screen_flash;
mod transfers;
mod validation;

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "amnezia_saved_animation_{tag}_{}.ron",
        std::process::id()
    ));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.register_required_components::<Mesh2d, Visibility>();
    app.add_plugins((AssetPlugin::default(), SavePlugin))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<cells::CellMaterial>()
        .init_resource::<crate::interpreter::RunningEvent>()
        .init_resource::<ActiveAnimations>()
        .add_message::<PlayAnimation>()
        .add_message::<ShowMapAnimation>()
        .add_message::<crate::world::MapChanged>()
        .add_message::<AudioRequest>()
        .add_message::<BattlerFlash>()
        .insert_resource(AnimationLibrary(load_ron(&format!(
            "{}/animations.ron",
            asset_root()
        ))))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ))
        .add_systems(Startup, cells::setup_mesh)
        .add_systems(
            Update,
            ((fade_flashes, step_animations, track_active_animations)
                .chain()
                .in_set(AnimationSet::Advance)
                .after(crate::teleport::MapTransfer),),
        )
        .add_systems(
            PostUpdate,
            (playback::follow_map_animations, cells::sync_flash),
        );
    map::flash::register(&mut app);
    start::register(&mut app);
    scene::register(&mut app);
    saved::register(&mut app);
    crate::teleport::rebuild::register(
        &mut app,
        crate::teleport::rebuild::Stage::Reset,
        playback::clear_map_animations,
    );
    let mut map = MapData::for_test(20, 15);
    map.map_id = 3;
    app.insert_resource(map);
    app.world_mut().spawn((MainCamera, Transform::default()));
    app.world_mut().spawn((
        Player {
            tile_x: 8,
            tile_y: 7,
            dir: 2,
            frame: 1,
            charset: "Chara1".into(),
            index: 0,
        },
        Sprite::default(),
        Transform::from_xyz(24.0, 32.0, 0.0),
    ));
    app.update();
    (app, path)
}

fn play(app: &mut App) {
    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 62,
        target: AnimTarget::Hero,
        global: false,
    });
    app.update();
    step(app, 13);
}

fn step(app: &mut App, ticks: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = app
        .world()
        .resource::<GameFrames>()
        .frame
        .wrapping_add(ticks);
    app.update();
}

fn save(app: &mut App) {
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
}

fn reload(app: &mut App) {
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapRebuilt);
    app.update();
}

#[test]
fn the_file_records_the_active_map_animation() {
    let (mut app, path) = app("file");
    play(&mut app);
    save(&mut app);
    let contents = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(contents.contains("map_animation"));
}

#[test]
fn a_loaded_map_restores_the_playing_cells_without_restarting() {
    let (mut app, path) = app("restore");
    play(&mut app);
    let before = app
        .world_mut()
        .query::<&playback::LiveAnimation>()
        .single(app.world())
        .unwrap()
        .frame;
    assert_eq!(before, 6);
    let expected = saved::snapshot(app.world_mut());
    save(&mut app);
    reload(&mut app);
    std::fs::remove_file(path).unwrap();
    let animation = app
        .world_mut()
        .query::<&playback::LiveAnimation>()
        .single(app.world())
        .expect("the saved map cast must be restored");
    assert_eq!(animation.frame, before);
    assert!(!animation.cells.is_empty());
    assert_eq!(animation.map_target, Some(AnimTarget::Hero));
    assert_eq!(saved::snapshot(app.world_mut()), expected);
}
