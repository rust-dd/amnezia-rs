use super::*;
use amnezia_data::AnimationFrameDef;

fn cell() -> AnimationCellDef {
    AnimationCellDef {
        valid: true,
        cell_id: 7,
        x: 0,
        y: 0,
        scale: 100,
        tone_red: 100,
        tone_green: 100,
        tone_blue: 100,
        tone_gray: 100,
        transparency: 0,
    }
}

#[test]
fn cell_rect_indexes_a_five_column_sheet() {
    for (id, x, y) in [
        (0, 0.0, 0.0),
        (4, 384.0, 0.0),
        (5, 0.0, 96.0),
        (12, 192.0, 192.0),
    ] {
        assert_eq!(cell_rect(id), Vec4::new(x, y, 96.0, 96.0));
    }
}

#[test]
fn transparency_uses_the_original_integer_opacity() {
    for (percent, expected) in [
        (0, 255),
        (1, 252),
        (40, 153),
        (50, 127),
        (60, 102),
        (99, 2),
        (100, 0),
        (150, 0),
    ] {
        assert_eq!(alpha(percent), expected as f32 / 255.0);
    }
}

#[test]
fn scaled_cells_have_integer_edges_and_original_center_rounding() {
    let mut cell = cell();
    for (scale, center, size) in [
        (0, 0.0, 0.0),
        (25, 0.0, 24.0),
        (50, 0.0, 48.0),
        (75, 0.0, 72.0),
        (99, 0.5, 95.0),
        (100, 0.0, 96.0),
        (101, 0.0, 96.0),
        (150, 0.0, 144.0),
    ] {
        cell.scale = scale;
        let base = Vec2::new(-33.0, 27.0);
        assert_eq!(geometry(&cell, base), (base + Vec2::splat(center), size));
    }
}

fn render_cell(cell: AnimationCellDef) -> App {
    let mut deleted = cell.clone();
    deleted.valid = false;
    let def = AnimationDef {
        id: 1,
        name: String::new(),
        animation_name: "Sword1".into(),
        scope: 0,
        position: 1,
        frames: vec![AnimationFrameDef {
            cells: vec![deleted, cell],
        }],
        timings: vec![],
    };
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<CellMaterial>()
        .add_systems(Startup, setup_mesh)
        .add_systems(
            Update,
            move |mut renderer: CellRenderer, mut commands: Commands| {
                let cells = renderer.spawn_frame(&mut commands, &def, 0, Vec2::ZERO);
                assert_eq!(cells.len(), 1);
            },
        );
    app.update();
    app
}

#[test]
fn valid_cells_use_quantized_tone_gray_and_preserve_deleted_cell_depth() {
    let mut cell = cell();
    cell.tone_red = 50;
    cell.tone_blue = 150;
    cell.tone_gray = 0;
    cell.transparency = 50;
    let mut app = render_cell(cell);
    let world = app.world_mut();
    let (handle, transform) = world
        .query::<(&MeshMaterial2d<CellMaterial>, &Transform)>()
        .single(world)
        .unwrap();
    let material = world
        .resource::<Assets<CellMaterial>>()
        .get(&handle.0)
        .unwrap();
    assert_eq!(material.channels, Vec4::new(64.0, 128.0, 192.0, 0.0));
    assert_eq!(material.source, Vec4::new(192.0, 96.0, 96.0, 96.0));
    assert_eq!(material.sampling, Vec4::new(96.0, 96.0, 127.0 / 255.0, 1.0));
    assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 510.1));
}

#[test]
fn neutral_cells_keep_full_sheet_sampling() {
    let mut app = render_cell(cell());
    let world = app.world_mut();
    let handle = world
        .query::<&MeshMaterial2d<CellMaterial>>()
        .single(world)
        .unwrap();
    let material = world
        .resource::<Assets<CellMaterial>>()
        .get(&handle.0)
        .unwrap();
    assert_eq!(material.channels, Vec4::splat(128.0));
    assert_eq!(material.sampling, Vec4::new(96.0, 96.0, 1.0, 0.0));
}

#[test]
fn screen_flashes_repaint_existing_cells_and_restore_their_sampling_after_expiry() {
    use super::super::render::{FlashQuad, FlashStamp, spawn_screen_flash};
    use bevy::ecs::system::RunSystemOnce;
    for toned in [false, true] {
        let mut cell = cell();
        cell.transparency = 40;
        if toned {
            cell.tone_gray = 0;
        }
        let mut app = render_cell(cell);
        let world = app.world_mut();
        let handle = world
            .query::<&MeshMaterial2d<CellMaterial>>()
            .single(world)
            .unwrap()
            .0
            .clone();
        for (age, alpha) in [(0, 248.0), (3, 200.0), (10, 80.0)] {
            world
                .run_system_once(move |mut commands: Commands| {
                    spawn_screen_flash(
                        &mut commands,
                        [248, 160, 80],
                        31,
                        FlashStamp { age, frame: 1 },
                    );
                })
                .unwrap();
            world.run_system_once(sync_flash).unwrap();
            let material = world
                .resource::<Assets<CellMaterial>>()
                .get(&handle)
                .unwrap();
            assert_eq!(material.flash, Vec4::new(248.0, 160.0, 80.0, alpha));
            assert_eq!(material.sampling.z, 153.0 / 255.0);
            assert_eq!(material.sampling.w, 1.0);
        }
        let flash = world
            .query_filtered::<Entity, With<FlashQuad>>()
            .single(world)
            .unwrap();
        world.despawn(flash);
        world.run_system_once(sync_flash).unwrap();
        let material = world
            .resource::<Assets<CellMaterial>>()
            .get(&handle)
            .unwrap();
        assert_eq!(material.flash, Vec4::ZERO);
        assert_eq!(material.sampling.w, u8::from(toned) as f32);
    }
}
