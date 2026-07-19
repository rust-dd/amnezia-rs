//! On-screen pictures (RM2000 `ShowPicture`/`MovePicture`/`ErasePicture`): a
//! numbered picture (1..50) drawn from `graphics/Picture/*.png`, positioned in
//! the 320×240 screen and optionally tweened. The interpreter emits a
//! [`PictureCommand`]; this plugin spawns/moves/despawns a textured quad per id.
//!
//! Each picture carries a colour [`Tone`] (RGB multiply + saturation, so a
//! grayscale or tinted picture renders as one — see [`render`]) and honours the
//! RM2000 fixed-to-map flag: a screen-pinned picture re-centres on the (shaken)
//! camera every frame, a map-fixed one holds a world anchor and scrolls with the
//! map. Pictures shake with the screen (they track the shaken camera) but are
//! never touched by the screen tint, and all pictures are cleared on a map
//! change ([`clear_on_map_change`]), matching RPG Maker 2000's transfer default.

use crate::screenfx::ScreenShakeSet;
use crate::world::MapChanged;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy::transform::TransformSystems;

mod render;

/// An RM2000 picture colour tone: per-channel RGB and a saturation, each a
/// percent with 100 neutral (`saturation = 0` is full grayscale). Interpolated
/// by [`PictureCommand::Move`] over its duration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub sat: f32,
}

impl Tone {
    /// The no-op tone (every channel neutral). Used as a test reference point.
    #[cfg(test)]
    pub(crate) const NEUTRAL: Tone = Tone {
        r: 100.0,
        g: 100.0,
        b: 100.0,
        sat: 100.0,
    };
}

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
        fixed_to_map: bool,
        transparency: f32,
        zoom: f32,
        tone: Tone,
    },
    /// Tween picture `id` to `(x, y)`/opacity/zoom/tone over `secs`.
    Move {
        id: u32,
        x: f32,
        y: f32,
        transparency: f32,
        zoom: f32,
        tone: Tone,
        secs: f32,
    },
    /// Remove picture `id`.
    Erase { id: u32 },
}

impl PictureCommand {
    /// Map a `ShowPicture` (11110) with `name` from the command string and
    /// already-resolved `(x, y)`. `params[4]` is the fixed-to-map flag,
    /// `params[5]` zoom %, `params[6]` transparency, `params[8..12]` the tone.
    pub fn show(id: u32, name: &str, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Show {
            id,
            name: name.to_string(),
            x,
            y,
            fixed_to_map: fixed_param(params),
            transparency: transparency_param(params),
            zoom: zoom_param(params),
            tone: tone_param(params),
        }
    }

    /// Map a `MovePicture` (11120): same fields as `Show` (minus the graphic and
    /// the fixed flag, which stay from the original show) plus `params[14]`
    /// duration in tenths of a second.
    pub fn move_to(id: u32, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Move {
            id,
            x,
            y,
            transparency: transparency_param(params),
            zoom: zoom_param(params),
            tone: tone_param(params),
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

/// RM2000 fixed-to-map flag (`params[4]`, > 0 = anchored to the map).
fn fixed_param(params: &[i32]) -> bool {
    params.get(4).copied().unwrap_or(0) > 0
}

/// RM2000 colour tone (`params[8..12]` = red, green, blue, saturation percents).
fn tone_param(params: &[i32]) -> Tone {
    Tone {
        r: params.get(8).copied().unwrap_or(100) as f32,
        g: params.get(9).copied().unwrap_or(100) as f32,
        b: params.get(10).copied().unwrap_or(100) as f32,
        sat: params.get(11).copied().unwrap_or(100) as f32,
    }
}

/// A live on-screen picture: its id, current visual state, map anchor, texture
/// size, and any running move.
#[derive(Component)]
struct Picture {
    id: u32,
    x: f32,
    y: f32,
    transparency: f32,
    zoom: f32,
    tone: Tone,
    /// Anchored to the map (scrolls with it) rather than pinned to the screen.
    fixed_to_map: bool,
    /// The fixed world position of a map-anchored picture, sampled at show time.
    world_anchor: Option<Vec2>,
    /// The native texture size, filled once the image loads.
    base_size: Option<Vec2>,
    tween: Option<Tween>,
}

impl Picture {
    /// This picture's current visual state as an interpolation endpoint.
    fn anim(&self) -> Anim {
        Anim {
            x: self.x,
            y: self.y,
            transparency: self.transparency,
            zoom: self.zoom,
            tone: self.tone,
        }
    }

    /// Apply the visual state `state`, immediately when `secs <= 0`, otherwise as
    /// a tween from the current state.
    fn apply(&mut self, state: Anim) {
        self.x = state.x;
        self.y = state.y;
        self.transparency = state.transparency;
        self.zoom = state.zoom;
        self.tone = state.tone;
    }

    /// Retarget this picture to a new visual state over `secs`.
    fn retarget(&mut self, x: f32, y: f32, transparency: f32, zoom: f32, tone: Tone, secs: f32) {
        let to = Anim {
            x,
            y,
            transparency,
            zoom,
            tone,
        };
        if secs <= 0.0 {
            self.apply(to);
            self.tween = None;
        } else {
            self.tween = Some(Tween {
                from: self.anim(),
                to,
                elapsed: 0.0,
                secs,
            });
        }
    }
}

/// A picture's interpolated visual state.
#[derive(Clone, Copy)]
struct Anim {
    x: f32,
    y: f32,
    transparency: f32,
    zoom: f32,
    tone: Tone,
}

impl Anim {
    /// Linear interpolation of every field.
    fn lerp(from: Anim, to: Anim, t: f32) -> Anim {
        Anim {
            x: lerp(from.x, to.x, t),
            y: lerp(from.y, to.y, t),
            transparency: lerp(from.transparency, to.transparency, t),
            zoom: lerp(from.zoom, to.zoom, t),
            tone: Tone {
                r: lerp(from.tone.r, to.tone.r, t),
                g: lerp(from.tone.g, to.tone.g, t),
                b: lerp(from.tone.b, to.tone.b, t),
                sat: lerp(from.tone.sat, to.tone.sat, t),
            },
        }
    }
}

/// A `MovePicture` in progress, interpolating the whole visual state.
#[derive(Clone, Copy)]
struct Tween {
    from: Anim,
    to: Anim,
    elapsed: f32,
    secs: f32,
}

pub struct PicturePlugin;

impl Plugin for PicturePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PictureCommand>()
            .add_plugins(Material2dPlugin::<render::PictureMaterial>::default())
            .add_systems(Startup, render::setup_picture_mesh)
            .add_systems(
                Update,
                (
                    clear_on_map_change,
                    render::apply_commands,
                    render::size_pictures,
                    drive_tweens,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                render::place_pictures
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
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
        pic.apply(Anim::lerp(tween.from, tween.to, t));
        pic.tween = (tween.elapsed < tween.secs).then_some(tween);
    }
}

/// Erase every picture when the map changes, matching RM2000's transfer default
/// (it clears pictures on transfer). The interpreter is paused across the fade,
/// so the destination map's own `ShowPicture`s run only after this has cleared.
fn clear_on_map_change(
    mut commands: Commands,
    mut changed: MessageReader<MapChanged>,
    pictures: Query<Entity, With<Picture>>,
) {
    if changed.read().last().is_none() {
        return;
    }
    for entity in &pictures {
        commands.entity(entity).despawn();
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_maps_position_flag_tone_transparency_and_zoom() {
        // Cross at (93, 94): [id, method, x, y, fixed, zoom, transp, usetransp,
        // R, G, B, sat, effmode, effpower].
        let params = [2, 1, 93, 94, 0, 100, 0, 1, 100, 100, 100, 100, 0, 60];
        assert_eq!(
            PictureCommand::show(2, "Cross", 93.0, 94.0, &params),
            PictureCommand::Show {
                id: 2,
                name: "Cross".into(),
                x: 93.0,
                y: 94.0,
                fixed_to_map: false,
                transparency: 0.0,
                zoom: 100.0,
                tone: Tone::NEUTRAL,
            }
        );
    }

    #[test]
    fn show_reads_the_fixed_to_map_flag_and_a_grayscale_tone() {
        // A map-fixed shadow, and the intro plate's grayscale (saturation 0).
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
        // Intro fade to grayscale: saturation 0, duration 30 tenths = 3.0s.
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
        // Start in colour (sat 100), retarget to grayscale (sat 0) over 2s.
        let mut pic = Picture {
            id: 1,
            x: 0.0,
            y: 0.0,
            transparency: 0.0,
            zoom: 100.0,
            tone: Tone::NEUTRAL,
            fixed_to_map: false,
            world_anchor: None,
            base_size: None,
            tween: None,
        };
        pic.retarget(
            0.0,
            0.0,
            0.0,
            100.0,
            Tone {
                r: 100.0,
                g: 100.0,
                b: 100.0,
                sat: 0.0,
            },
            2.0,
        );
        let tween = pic.tween.expect("a tween");
        let mid = Anim::lerp(tween.from, tween.to, 0.5);
        assert_eq!(mid.tone.sat, 50.0);
    }

    #[test]
    fn a_map_change_despawns_every_picture() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<MapChanged>();
        app.add_systems(Update, clear_on_map_change);
        app.world_mut().spawn(test_picture(1));
        app.world_mut().spawn(test_picture(2));

        // No map change yet: the pictures survive a frame.
        app.update();
        assert_eq!(count_pictures(&mut app), 2);

        // On a map change every picture despawns.
        app.world_mut().write_message(MapChanged);
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
            world_anchor: None,
            base_size: None,
            tween: None,
        }
    }

    fn count_pictures(app: &mut App) -> usize {
        app.world_mut()
            .query::<&Picture>()
            .iter(app.world())
            .count()
    }
}
