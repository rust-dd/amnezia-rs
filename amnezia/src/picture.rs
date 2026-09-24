//! On-screen pictures (RM2000 `ShowPicture`/`MovePicture`/`ErasePicture`): a
//! numbered picture (1..50) drawn from `graphics/Picture/*.png`, positioned in
//! the 320×240 screen and optionally tweened. The interpreter emits a
//! [`PictureCommand`]; this plugin spawns/moves/despawns a textured quad per id.
//!
//! Each picture carries a palette-space colour [`Tone`] (saturation + hard light, so a
//! grayscale or tinted picture renders as one — see [`render`]) and honours the
//! RM2000 fixed-to-map flag: a screen-pinned picture re-centres on the unshaken
//! camera every frame, a map-fixed one holds a world anchor and scrolls with the
//! map. Pictures inherit screen shake but not screen tint. Rebuilding the map
//! clears the previous scene's pictures; same-map repositioning preserves them.

use crate::screenfx::ScreenShakeSet;
use crate::world::MapRebuilt;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy::transform::TransformSystems;

mod effects;
mod render;
pub(crate) mod saved;
pub(crate) mod smoke;
pub use effects::Effect;
#[cfg(test)]
mod tests;

/// An RM2000 picture colour tone: per-channel RGB and a saturation, each a
/// percent with 100 neutral (`saturation = 0` is full grayscale). Interpolated
/// by [`PictureCommand::Move`] over its duration.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        use_transparent_color: bool,
        transparency: f32,
        zoom: f32,
        tone: Tone,
        effect: Effect,
    },
    /// Tween picture `id` to `(x, y)`/opacity/zoom/tone over `secs`.
    Move {
        id: u32,
        x: f32,
        y: f32,
        transparency: f32,
        zoom: f32,
        tone: Tone,
        effect: Effect,
        secs: f32,
    },
    /// Remove picture `id`.
    Erase { id: u32 },
}

impl PictureCommand {
    /// Map a `ShowPicture` (11110) with `name` from the command string and
    /// already-resolved `(x, y)`. `params[4]` is the fixed-to-map flag,
    /// `params[5]` zoom %, `params[6]` transparency, `params[7]` colour key,
    /// `params[8..12]` the tone, and `params[12..14]` the effect.
    pub fn show(id: u32, name: &str, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Show {
            id,
            name: name.to_string(),
            x,
            y,
            fixed_to_map: fixed_param(params),
            use_transparent_color: params.get(7).copied().unwrap_or(0) > 0,
            transparency: transparency_param(params),
            zoom: zoom_param(params),
            tone: tone_param(params),
            effect: Effect::from_params(params),
        }
    }

    /// Map a `MovePicture` (11120): same fields as `Show` (minus the graphic and
    /// the fixed/colour-key flags, which stay from the original show) plus `params[14]`
    /// duration in tenths of a second.
    pub fn move_to(id: u32, x: f32, y: f32, params: &[i32]) -> Self {
        Self::Move {
            id,
            x,
            y,
            transparency: transparency_param(params),
            zoom: zoom_param(params),
            tone: tone_param(params),
            effect: Effect::from_params(params),
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
    name: String,
    x: f32,
    y: f32,
    transparency: f32,
    zoom: f32,
    tone: Tone,
    use_transparent_color: bool,
    /// Anchored to the map (scrolls with it) rather than pinned to the screen.
    fixed_to_map: bool,
    /// The fixed world position of a map-anchored picture, sampled at show time.
    world_anchor: Option<Vec2>,
    /// The native texture size, filled once the image loads.
    base_size: Option<Vec2>,
    tween: Option<Tween>,
    effect: effects::EffectState,
    frame_fraction: f64,
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

    /// Apply one interpolation sample.
    fn apply(&mut self, state: Anim) {
        self.x = state.x;
        self.y = state.y;
        self.transparency = state.transparency;
        self.zoom = state.zoom;
        self.tone = state.tone;
    }

    /// Retarget this picture to a new visual state over `secs`.
    fn retarget(&mut self, to: Anim, effect: Effect, secs: f32) {
        self.effect.retarget(effect);
        if secs <= 0.0 {
            self.apply(to);
            self.tween = None;
        } else {
            self.tween = Some(Tween {
                from: self.anim(),
                to,
                elapsed: 0,
                frames: (secs * 60.0).round().max(1.0) as u32,
            });
        }
    }

    fn advance(&mut self, dt: f32) {
        self.frame_fraction += f64::from(dt.max(0.0)) * 60.0;
        let frames = (self.frame_fraction + 1e-6).floor() as u32;
        self.frame_fraction = (self.frame_fraction - f64::from(frames)).max(0.0);
        for _ in 0..frames {
            let remaining = if let Some(mut tween) = self.tween {
                tween.elapsed += 1;
                self.apply(Anim::lerp(
                    tween.from,
                    tween.to,
                    tween.elapsed as f32 / tween.frames as f32,
                ));
                self.tween = (tween.elapsed < tween.frames).then_some(tween);
                tween.frames - tween.elapsed
            } else {
                0
            };
            self.effect.tick(remaining);
        }
    }
}

/// A picture's interpolated visual state.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Anim {
    pub x: f32,
    pub y: f32,
    pub transparency: f32,
    pub zoom: f32,
    pub tone: Tone,
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
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Tween {
    from: Anim,
    to: Anim,
    elapsed: u32,
    frames: u32,
}

pub struct PicturePlugin;

impl Plugin for PicturePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PictureCommand>()
            .add_message::<MapRebuilt>()
            .add_plugins(Material2dPlugin::<render::PictureMaterial>::default())
            .add_systems(Startup, render::setup_picture_mesh)
            .add_systems(
                PostUpdate,
                (render::size_pictures, render::place_pictures)
                    .chain()
                    .after(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
        register_timeline(app);
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ParallelPictures;

fn register_timeline(app: &mut App) {
    app.add_systems(
        Update,
        (
            (clear_on_map_change, saved::restore)
                .chain()
                .after(crate::teleport::MapTransfer)
                .before(crate::interpreter::ParallelStep)
                .before(ParallelPictures),
            render::apply_commands
                .in_set(ParallelPictures)
                .after(crate::interpreter::ParallelStep)
                .before(crate::player::PlayerStep)
                .before(crate::player::CameraFollow),
            drive_tweens
                .after(ParallelPictures)
                .after(crate::screenfx::ScreenAdvance)
                .after(crate::animation::AnimationSet::Advance)
                .before(crate::interpreter::InterpreterStep),
            render::apply_commands.after(crate::interpreter::InterpreterStep),
        ),
    );
}

/// Advance picture moves and effects on their shared 60 Hz clock.
fn drive_tweens(
    time: Res<Time>,
    scenes: crate::world::ScenePause,
    mut pictures: Query<&mut Picture>,
) {
    if scenes.paused() {
        return;
    }
    let dt = time.delta_secs();
    for mut pic in &mut pictures {
        pic.advance(dt);
    }
}

/// Clear the previous scene before destination picture commands can run.
fn clear_on_map_change(
    mut commands: Commands,
    mut changed: MessageReader<MapRebuilt>,
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
