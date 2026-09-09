//! Rendering for on-screen pictures: the tone [`PictureMaterial`] (an RGB
//! multiply + saturation desaturate + opacity, per pixel in `picture_tone.wgsl`)
//! drawn on a per-picture textured quad, plus the systems that spawn, size, and
//! place those quads. Kept apart from the command/state logic in the parent
//! module so the tone maths and the screen/map placement unit-test in isolation.

use super::{Picture, Tone};
use crate::assets::resolve_png;
use crate::picture::PictureCommand;
use crate::screenfx::PICTURE_LAYER;
use crate::world::MainCamera;
use bevy::asset::Asset;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

/// RM2000 screen centre in its 320×240 viewport; a picture's `(x, y)` is its
/// centre, so this maps to the camera centre.
const CENTER_X: f32 = 160.0;
const CENTER_Y: f32 = 120.0;

/// World z of picture 0; each picture adds its id, so higher ids draw on top and
/// every picture sits above the map and characters (z < 4) yet within the 2D
/// camera's range.
const PICTURE_Z_BASE: f32 = 100.0;

/// The tone material for one picture: an RGB multiply, a saturation blend toward
/// luminance, and an opacity, evaluated per pixel by `picture_tone.wgsl`. The
/// RM2000 screen tint is deliberately absent — pictures are not tinted by it.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct PictureMaterial {
    /// `xyz` RGB multipliers (1 = neutral), `w` saturation (1 = neutral, 0 = gray).
    #[uniform(0)]
    pub rgb_sat: Vec4,
    /// `x` opacity (0..1), `y` enables palette-index-zero transparency.
    #[uniform(1)]
    pub extra: Vec4,
    #[texture(2)]
    #[sampler(3)]
    pub image: Handle<Image>,
}

impl Material2d for PictureMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/picture_tone.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// The shared unit quad every picture is drawn on; its transform scale carries
/// the texture size × zoom.
#[derive(Resource)]
pub(super) struct PictureMesh(Handle<Mesh>);

/// Register the shared unit quad once at startup.
pub(super) fn setup_picture_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(PictureMesh(meshes.add(Rectangle::new(1.0, 1.0))));
}

/// Spawn, retarget, or despawn picture quads as commands arrive. A map-fixed
/// picture records its world anchor from the (pre-shake) camera here so it stays
/// pinned to the map; a `Move` on a legacy map-fixed picture keeps its position
/// (RM2000 ignores the target coordinates for those).
pub(super) fn apply_commands(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<PictureCommand>>,
) {
    let Some(mesh) = world
        .get_resource::<PictureMesh>()
        .map(|mesh| mesh.0.clone())
    else {
        return;
    };
    let camera_base = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .map(|t| t.translation.truncate())
        .ok();
    let requests = cursor
        .read(world.resource::<Messages<PictureCommand>>())
        .cloned()
        .collect::<Vec<_>>();
    for request in requests {
        match request {
            PictureCommand::Show {
                id,
                name,
                x,
                y,
                fixed_to_map,
                use_transparent_color,
                transparency,
                zoom,
                tone,
            } => {
                for entity in picture_entities(world, id) {
                    world.despawn(entity);
                }
                let anchor =
                    fixed_to_map.then(|| camera_base.unwrap_or_default() + screen_offset(x, y));
                let image = world
                    .resource::<AssetServer>()
                    .load(resolve_png("Picture", &name));
                let material =
                    world
                        .resource_mut::<Assets<PictureMaterial>>()
                        .add(PictureMaterial {
                            rgb_sat: tone_rgb_sat(tone),
                            extra: opacity_extra(transparency, use_transparent_color),
                            image,
                        });
                world.spawn((
                    Picture {
                        id,
                        x,
                        y,
                        transparency,
                        zoom,
                        tone,
                        use_transparent_color,
                        fixed_to_map,
                        world_anchor: anchor,
                        base_size: None,
                        tween: None,
                    },
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(material),
                    Transform::from_translation(screen_offset(x, y).extend(picture_z(id)))
                        .with_scale(Vec3::ZERO),
                    RenderLayers::layer(PICTURE_LAYER),
                ));
            }
            PictureCommand::Move {
                id,
                x,
                y,
                transparency,
                zoom,
                tone,
                secs,
            } => {
                for entity in picture_entities(world, id) {
                    let mut pic = world.get_mut::<Picture>(entity).unwrap();
                    // Legacy map-fixed pictures ignore MovePicture coordinates.
                    let (tx, ty) = if pic.fixed_to_map {
                        (pic.x, pic.y)
                    } else {
                        (x, y)
                    };
                    pic.retarget(tx, ty, transparency, zoom, tone, secs);
                }
            }
            PictureCommand::Erase { id } => {
                for entity in picture_entities(world, id) {
                    world.despawn(entity);
                }
            }
        }
    }
}

fn picture_entities(world: &mut World, id: u32) -> Vec<Entity> {
    world
        .query::<(Entity, &Picture)>()
        .iter(world)
        .filter_map(|(entity, picture)| (picture.id == id).then_some(entity))
        .collect()
}

/// Record each picture's native texture size once its image has loaded, so
/// [`place_pictures`] can scale the unit quad to it.
pub(super) fn size_pictures(
    images: Res<Assets<Image>>,
    materials: Res<Assets<PictureMaterial>>,
    mut pictures: Query<(&mut Picture, &MeshMaterial2d<PictureMaterial>)>,
) {
    for (mut pic, handle) in &mut pictures {
        if pic.base_size.is_some() {
            continue;
        }
        let Some(material) = materials.get(&handle.0) else {
            continue;
        };
        let Some(image) = images.get(&material.image) else {
            continue;
        };
        pic.base_size = Some(image.size().as_vec2());
    }
}

/// Place each picture (screen-pinned to the shaken camera, or fixed to its map
/// anchor), scale it to texture size × zoom, and push its tone/opacity into the
/// material.
pub(super) fn place_pictures(
    cameras: Query<&Transform, (With<MainCamera>, Without<Picture>)>,
    mut materials: ResMut<Assets<PictureMaterial>>,
    mut pictures: Query<(&Picture, &mut Transform, &MeshMaterial2d<PictureMaterial>)>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let base = camera.translation.truncate();
    for (pic, mut transform, handle) in &mut pictures {
        let pos = picture_translation(base, pic.x, pic.y, pic.world_anchor);
        transform.translation = pos.extend(picture_z(pic.id));
        if let Some(size) = pic.base_size {
            transform.scale = (size * (pic.zoom / 100.0).max(0.0)).extend(1.0);
        }
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.rgb_sat = tone_rgb_sat(pic.tone);
            material.extra = opacity_extra(pic.transparency, pic.use_transparent_color);
        }
    }
}

/// A picture's world translation: its fixed map anchor when it scrolls with the
/// map, otherwise the (shaken) camera centre plus its screen offset. RM2000 y
/// grows downward, so the offset flips against world y.
fn picture_translation(base: Vec2, x: f32, y: f32, anchor: Option<Vec2>) -> Vec2 {
    match anchor {
        Some(map_anchor) => map_anchor,
        None => base + screen_offset(x, y),
    }
}

/// The camera-relative offset of a picture centred at RM2000 `(x, y)`.
fn screen_offset(x: f32, y: f32) -> Vec2 {
    Vec2::new(x - CENTER_X, CENTER_Y - y)
}

/// The tone uniform (RGB multipliers + saturation) for `tone`. Each RM2000
/// channel is 0..200 with 100 neutral, mapped to a 0..2 multiplier as the
/// animation tone does; the shader turns the `w` saturation into a desaturate.
pub(super) fn tone_rgb_sat(tone: Tone) -> Vec4 {
    Vec4::new(
        channel(tone.r),
        channel(tone.g),
        channel(tone.b),
        channel(tone.sat),
    )
}

/// An RM2000 tone channel (0..200, 100 neutral) as a 0..2 multiplier.
fn channel(value: f32) -> f32 {
    (value / 100.0).clamp(0.0, 2.0)
}

fn opacity_extra(transparency: f32, use_transparent_color: bool) -> Vec4 {
    Vec4::new(
        opacity(transparency),
        f32::from(use_transparent_color),
        0.0,
        0.0,
    )
}

/// Opacity (0..1) from RM2000 transparency percent (0 opaque, 100 invisible).
fn opacity(transparency: f32) -> f32 {
    (255.0 * (100.0 - transparency.clamp(0.0, 100.0)) / 100.0).floor() / 255.0
}

/// The world z of picture `id`.
fn picture_z(id: u32) -> f32 {
    PICTURE_Z_BASE + id as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_offset_centres_on_the_viewport_centre() {
        assert_eq!(screen_offset(160.0, 120.0), Vec2::ZERO);
        assert_eq!(screen_offset(0.0, 0.0), Vec2::new(-160.0, 120.0));
        assert_eq!(screen_offset(320.0, 240.0), Vec2::new(160.0, -120.0));
    }

    #[test]
    fn opacity_is_the_complement_of_transparency() {
        assert_eq!(opacity(0.0), 1.0);
        assert_eq!(opacity(100.0), 0.0);
        assert_eq!(opacity(50.0), 127.0 / 255.0);
        assert_eq!(opacity(25.0), 191.0 / 255.0);
        assert_eq!(opacity(150.0), 0.0);
    }

    #[test]
    fn picture_z_orders_by_id_and_stays_above_the_map() {
        assert!(picture_z(2) > picture_z(1));
        assert!(picture_z(1) > 4.0);
    }

    #[test]
    fn neutral_tone_is_the_identity_multiplier() {
        assert_eq!(tone_rgb_sat(Tone::NEUTRAL), Vec4::new(1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn grayscale_tone_drops_saturation_to_zero() {
        // The only non-neutral tone the game uses: neutral RGB, saturation 0.
        let gray = Tone {
            r: 100.0,
            g: 100.0,
            b: 100.0,
            sat: 0.0,
        };
        assert_eq!(tone_rgb_sat(gray), Vec4::new(1.0, 1.0, 1.0, 0.0));
    }

    #[test]
    fn colour_tone_scales_and_clamps_each_channel() {
        let tone = Tone {
            r: 200.0,
            g: 50.0,
            b: 0.0,
            sat: 300.0,
        };
        assert_eq!(tone_rgb_sat(tone), Vec4::new(2.0, 0.5, 0.0, 2.0));
    }

    #[test]
    fn screen_pinned_picture_tracks_the_camera_and_stays_on_screen() {
        // No anchor: the world position follows the camera base (so the picture
        // holds its screen spot as the map scrolls).
        let at_origin = picture_translation(Vec2::ZERO, 160.0, 120.0, None);
        let panned = picture_translation(Vec2::new(48.0, -32.0), 160.0, 120.0, None);
        assert_eq!(at_origin, Vec2::ZERO);
        assert_eq!(panned, Vec2::new(48.0, -32.0));
    }

    #[test]
    fn map_fixed_picture_holds_its_world_anchor_and_scrolls_with_the_map() {
        // With an anchor the world position is fixed regardless of the camera,
        // so the picture scrolls off with the map like a world sprite.
        let anchor = Vec2::new(7.0, 8.0);
        let a = picture_translation(Vec2::ZERO, 160.0, 120.0, Some(anchor));
        let b = picture_translation(Vec2::new(48.0, -32.0), 160.0, 120.0, Some(anchor));
        assert_eq!(a, anchor);
        assert_eq!(b, anchor);
    }
}
