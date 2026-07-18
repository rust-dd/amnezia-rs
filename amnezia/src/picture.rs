//! On-screen pictures (RM2000 `ShowPicture`/`MovePicture`/`ErasePicture`): a
//! numbered picture (1..50) drawn from `graphics/Picture/*.png`, positioned in
//! the 320×240 screen and optionally tweened. The interpreter emits a
//! [`PictureCommand`]; this plugin spawns/moves/despawns a sprite per id.
//!
//! Each picture is a top-level, centre-anchored [`Sprite`] kept pinned to the
//! screen by [`place_pictures`], which re-centres it on the camera every frame
//! (so it doesn't scroll with the map and survives map transfers). Positions are
//! read after the screen shake lands, so pictures shake with the view.

use crate::assets::resolve_png;
use crate::screenfx::ScreenShakeSet;
use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

/// RM2000 screen centre in its 320×240 viewport; a picture's `(x, y)` is its
/// centre, so this maps to the camera centre.
const CENTER_X: f32 = 160.0;
const CENTER_Y: f32 = 120.0;

/// World z of picture 0; each picture adds its id, so higher ids draw on top and
/// every picture sits above the map and characters (z < 4) yet within the 2D
/// camera's range.
const PICTURE_Z_BASE: f32 = 100.0;

/// A picture command emitted by the interpreter, consumed by this plugin. The
/// interpreter resolves `x`/`y` (direct or via variables) before sending, so
/// they are always final screen coordinates.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum PictureCommand {
    /// Show (or replace) picture `id` with graphic `name` at `(x, y)`.
    Show {
        id: u32,
        name: String,
        x: f32,
        y: f32,
        transparency: f32,
        zoom: f32,
    },
    /// Tween picture `id` to `(x, y)`/opacity/zoom over `secs`.
    Move {
        id: u32,
        x: f32,
        y: f32,
        transparency: f32,
        zoom: f32,
        secs: f32,
    },
    /// Remove picture `id`.
    Erase { id: u32 },
}

impl PictureCommand {
    /// Map a `ShowPicture` (11110) with `name` from the command string and
    /// already-resolved `(x, y)`. `params[5]` is zoom %, `params[6]` transparency.
    pub fn show(id: u32, name: &str, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Show {
            id,
            name: name.to_string(),
            x,
            y,
            transparency: transparency_param(params),
            zoom: zoom_param(params),
        }
    }

    /// Map a `MovePicture` (11120): same fields as `Show` plus `params[14]`
    /// duration in tenths of a second (the graphic is unchanged, so no name).
    pub fn move_to(id: u32, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Move {
            id,
            x,
            y,
            transparency: transparency_param(params),
            zoom: zoom_param(params),
            secs: params.get(14).copied().unwrap_or(0) as f32 / 10.0,
        }
    }

    /// Map an `ErasePicture` (11130) `[id]`.
    pub fn erase(id: u32) -> Self {
        Self::Erase { id }
    }
}

/// RM2000 transparency percent (`params[6]`, 0 = opaque, 100 = invisible).
fn transparency_param(params: &[i32]) -> f32 {
    params.get(6).copied().unwrap_or(0) as f32
}

/// RM2000 magnification percent (`params[5]`, 100 = native size).
fn zoom_param(params: &[i32]) -> f32 {
    params.get(5).copied().unwrap_or(100) as f32
}

/// A live on-screen picture: its id, current visual state, and any running move.
#[derive(Component)]
struct Picture {
    id: u32,
    x: f32,
    y: f32,
    transparency: f32,
    zoom: f32,
    tween: Option<Tween>,
}

/// A `MovePicture` in progress: interpolating `[x, y, transparency, zoom]`.
#[derive(Clone, Copy)]
struct Tween {
    from: [f32; 4],
    to: [f32; 4],
    elapsed: f32,
    secs: f32,
}

pub struct PicturePlugin;

impl Plugin for PicturePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PictureCommand>()
            .add_systems(Update, (apply_commands, drive_tweens))
            .add_systems(
                PostUpdate,
                place_pictures
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// Spawn, retarget, or despawn pictures as commands arrive.
fn apply_commands(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut requests: MessageReader<PictureCommand>,
    mut pictures: Query<(Entity, &mut Picture)>,
) {
    for request in requests.read() {
        match request {
            PictureCommand::Show {
                id,
                name,
                x,
                y,
                transparency,
                zoom,
            } => {
                despawn_picture(&mut commands, &pictures, *id);
                commands.spawn((
                    Picture {
                        id: *id,
                        x: *x,
                        y: *y,
                        transparency: *transparency,
                        zoom: *zoom,
                        tween: None,
                    },
                    Sprite {
                        image: asset_server.load(resolve_png("Picture", name)),
                        color: sprite_color(*transparency),
                        ..default()
                    },
                    Transform::from_translation(screen_offset(*x, *y).extend(picture_z(*id))),
                ));
            }
            PictureCommand::Move {
                id,
                x,
                y,
                transparency,
                zoom,
                secs,
            } => {
                if let Some((_, mut pic)) = pictures.iter_mut().find(|(_, p)| p.id == *id) {
                    if *secs <= 0.0 {
                        pic.x = *x;
                        pic.y = *y;
                        pic.transparency = *transparency;
                        pic.zoom = *zoom;
                        pic.tween = None;
                    } else {
                        pic.tween = Some(Tween {
                            from: [pic.x, pic.y, pic.transparency, pic.zoom],
                            to: [*x, *y, *transparency, *zoom],
                            elapsed: 0.0,
                            secs: *secs,
                        });
                    }
                }
            }
            PictureCommand::Erase { id } => despawn_picture(&mut commands, &pictures, *id),
        }
    }
}

/// Despawn every picture entity with the given id.
fn despawn_picture(commands: &mut Commands, pictures: &Query<(Entity, &mut Picture)>, id: u32) {
    for (entity, pic) in pictures.iter() {
        if pic.id == id {
            commands.entity(entity).despawn();
        }
    }
}

/// Advance running move tweens, updating each picture's current visual state.
fn drive_tweens(time: Res<Time>, mut pictures: Query<&mut Picture>) {
    let dt = time.delta_secs();
    for mut pic in &mut pictures {
        let Some(mut tween) = pic.tween else {
            continue;
        };
        tween.elapsed += dt;
        let t = (tween.elapsed / tween.secs).clamp(0.0, 1.0);
        let [x, y, transparency, zoom] = lerp4(tween.from, tween.to, t);
        pic.x = x;
        pic.y = y;
        pic.transparency = transparency;
        pic.zoom = zoom;
        pic.tween = (tween.elapsed < tween.secs).then_some(tween);
    }
}

/// Pin each picture to the (shaken) camera centre plus its screen offset, and
/// repaint its opacity and zoom.
fn place_pictures(
    cameras: Query<&Transform, (With<MainCamera>, Without<Picture>)>,
    mut pictures: Query<(&Picture, &mut Transform, &mut Sprite)>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let base = camera.translation.truncate();
    for (pic, mut transform, mut sprite) in &mut pictures {
        transform.translation = (base + screen_offset(pic.x, pic.y)).extend(picture_z(pic.id));
        transform.scale = Vec3::splat((pic.zoom / 100.0).max(0.0));
        sprite.color = sprite_color(pic.transparency);
    }
}

/// The camera-relative offset of a picture centred at RM2000 `(x, y)`; RM2000 y
/// grows downward, so it flips against world y.
fn screen_offset(x: f32, y: f32) -> Vec2 {
    Vec2::new(x - CENTER_X, CENTER_Y - y)
}

/// A white sprite tint carrying the picture's opacity (1 - transparency).
fn sprite_color(transparency: f32) -> Color {
    Color::srgba(1.0, 1.0, 1.0, opacity(transparency))
}

/// Opacity (0..1) from RM2000 transparency percent (0 opaque, 100 invisible).
fn opacity(transparency: f32) -> f32 {
    1.0 - transparency.clamp(0.0, 100.0) / 100.0
}

/// The world z of picture `id`.
fn picture_z(id: u32) -> f32 {
    PICTURE_Z_BASE + id as f32
}

/// Linear interpolation of two 4-vectors.
fn lerp4(from: [f32; 4], to: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp(from[0], to[0], t),
        lerp(from[1], to[1], t),
        lerp(from[2], to[2], t),
        lerp(from[3], to[3], t),
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
        assert_eq!(opacity(50.0), 0.5);
        assert_eq!(opacity(150.0), 0.0);
    }

    #[test]
    fn picture_z_orders_by_id_and_stays_above_the_map() {
        assert!(picture_z(2) > picture_z(1));
        assert!(picture_z(1) > 4.0);
    }

    #[test]
    fn lerp_interpolates_endpoints_and_midpoint() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(
            lerp4([0.0, 0.0, 0.0, 100.0], [10.0, 20.0, 100.0, 200.0], 0.5),
            [5.0, 10.0, 50.0, 150.0]
        );
    }

    #[test]
    fn show_maps_id_name_transparency_and_zoom() {
        // Cross at (93, 94): [id, method, x, y, fixed, zoom, transp, ...].
        let params = [2, 1, 93, 94, 0, 100, 0, 1, 100, 100, 100, 100, 0, 60];
        assert_eq!(
            PictureCommand::show(2, "Cross", 93.0, 94.0, &params),
            PictureCommand::Show {
                id: 2,
                name: "Cross".into(),
                x: 93.0,
                y: 94.0,
                transparency: 0.0,
                zoom: 100.0,
            }
        );
    }

    #[test]
    fn move_reads_duration_from_param_14() {
        // Intro fade-in: transparency 0, duration 10 tenths = 1.0s.
        let params = [
            1,
            1,
            12,
            13,
            0,
            100,
            0,
            0,
            100,
            100,
            100,
            0,
            0,
            -2147483640,
            10,
            1,
        ];
        assert_eq!(
            PictureCommand::move_to(1, 12.0, 13.0, &params),
            PictureCommand::Move {
                id: 1,
                x: 12.0,
                y: 13.0,
                transparency: 0.0,
                zoom: 100.0,
                secs: 1.0,
            }
        );
    }
}
