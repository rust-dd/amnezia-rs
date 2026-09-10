use super::render::{overlay_layer, overlay_translation};
use amnezia_data::{AnimationCellDef, AnimationDef};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

const CELL: f32 = 96.0;
const CELL_Z: f32 = 510.0;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct CellMaterial {
    #[uniform(0)]
    pub channels: Vec4,
    #[uniform(1)]
    pub source: Vec4,
    /// Drawn width/height, opacity and whether tone or flash needs a cropped source.
    #[uniform(2)]
    pub sampling: Vec4,
    #[texture(3)]
    pub image: Handle<Image>,
    #[uniform(4)]
    pub flash: Vec4,
}

impl Material2d for CellMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/animation_cell.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Resource)]
pub(super) struct CellMesh(Handle<Mesh>);

#[derive(SystemParam)]
pub(super) struct CellRenderer<'w> {
    server: Res<'w, AssetServer>,
    mesh: Res<'w, CellMesh>,
    materials: ResMut<'w, Assets<CellMaterial>>,
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(Material2dPlugin::<CellMaterial>::default())
        .add_systems(Startup, setup_mesh)
        .add_systems(
            PostUpdate,
            sync_flash.before(bevy::transform::TransformSystems::Propagate),
        );
}

pub(super) fn sync_flash(
    flashes: Query<&Sprite, With<super::render::FlashQuad>>,
    cells: Query<&MeshMaterial2d<CellMaterial>>,
    mut materials: ResMut<Assets<CellMaterial>>,
) {
    let color = flashes
        .iter()
        .next()
        .map_or([0; 4], |sprite| sprite.color.to_srgba().to_u8_array());
    let flash = Vec4::from_array(color.map(f32::from));
    for handle in &cells {
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        if material.flash == flash {
            continue;
        }
        let mut material = materials.get_mut(&handle.0).unwrap();
        material.flash = flash;
        material.sampling.w =
            u8::from(material.channels != Vec4::splat(128.0) || color[3] != 0) as f32;
    }
}

pub(super) fn setup_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(CellMesh(meshes.add(Rectangle::new(1.0, 1.0))));
}

fn cell_rect(id: u32) -> Vec4 {
    Vec4::new((id % 5) as f32 * CELL, (id / 5) as f32 * CELL, CELL, CELL)
}

fn alpha(transparency: u32) -> f32 {
    (255 * (100 - transparency.min(100)) / 100) as f32 / 255.0
}

fn geometry(cell: &AnimationCellDef, base: Vec2) -> (Vec2, f32) {
    let zoom = cell.scale as f64 / 100.0;
    let size = (CELL as f64 * zoom).floor() as f32;
    let origin = (base + Vec2::new(cell.x as f32, cell.y as f32)).trunc()
        - Vec2::splat((CELL as f64 / 2.0 * zoom).floor() as f32);
    (origin + Vec2::splat(size / 2.0), size)
}

impl CellRenderer<'_> {
    pub(super) fn spawn_frame(
        &mut self,
        commands: &mut Commands,
        def: &AnimationDef,
        frame: usize,
        base: Vec2,
    ) -> Vec<Entity> {
        let image = self
            .server
            .load(crate::assets::resolve_png("Battle", &def.animation_name));
        let mut cells = Vec::new();
        for (index, cell) in def.frames[frame].cells.iter().enumerate() {
            if !cell.valid {
                continue;
            }
            let (center, size) = geometry(cell, base);
            if size == 0.0 || cell.transparency >= 100 {
                continue;
            }
            let channels = crate::legacy_colors::tone::uniform(
                [
                    cell.tone_red,
                    cell.tone_green,
                    cell.tone_blue,
                    cell.tone_gray,
                ]
                .map(|v| v as f32),
            );
            let material = self.materials.add(CellMaterial {
                channels,
                source: cell_rect(cell.cell_id),
                sampling: Vec4::new(
                    size,
                    size,
                    alpha(cell.transparency),
                    u8::from(channels != Vec4::splat(128.0)) as f32,
                ),
                image: image.clone(),
                flash: Vec4::ZERO,
            });
            cells.push(
                commands
                    .spawn((
                        Mesh2d(self.mesh.0.clone()),
                        MeshMaterial2d(material),
                        crate::transitions::SnapshotImage(image.clone()),
                        Transform::from_translation(overlay_translation(
                            center,
                            CELL_Z + index as f32 * 0.1,
                        ))
                        .with_scale(Vec3::new(size, size, 1.0)),
                        overlay_layer(),
                    ))
                    .id(),
            );
        }
        cells
    }
}

#[cfg(test)]
mod tests;
