//! Screen presentation effects driven by the event interpreter: the erase/show
//! black fade, the color tint, the brief flash, and the camera shake. The
//! interpreter stays decoupled by emitting a [`ScreenEffect`] message; this
//! plugin consumes it and drives three fullscreen overlays (tint, flash, fade)
//! plus a camera-shake offset. Mirrors the audio/shop producer/consumer
//! contract.
//!
//! The shake avoids touching `player.rs`: [`apply_camera_shake`] runs in
//! `PostUpdate` (after the `Update` `camera_follow` has set the base position)
//! and adds an offset the follow overwrites again next frame, so it never
//! accumulates.

use crate::world::MainCamera;
use bevy::prelude::*;
use bevy::transform::TransformSystems;

/// Seconds an erase/show fade takes to reach full black / full clear. The
/// interpreter waits this long after an `EraseScreen`/`ShowScreen` so the next
/// command runs against the settled screen.
pub const SCREEN_FADE_SECS: f32 = 0.25;

/// Alpha per second for the erase/show fade, so a full fade takes
/// [`SCREEN_FADE_SECS`].
const FADE_SPEED: f32 = 1.0 / SCREEN_FADE_SECS;

/// World units of horizontal camera displacement per unit of shake `power`.
const SHAKE_AMPLITUDE: f32 = 1.5;

/// Radians per second of shake oscillation per unit of shake `speed`.
const SHAKE_FREQUENCY: f32 = 4.0;

/// A screen effect the interpreter emits; consumed by [`step_effects`].
#[derive(Message, Debug, Clone, PartialEq)]
pub enum ScreenEffect {
    /// Fade to black and hold (`EraseScreen` 11010).
    Erase,
    /// Fade from black back to the scene (`ShowScreen` 11020).
    Show,
    /// Shift the screen color over `secs` (`TintScreen` 11030). `r,g,b,sat` are
    /// RM2000 0..200, 100 = neutral.
    Tint {
        r: i32,
        g: i32,
        b: i32,
        sat: i32,
        secs: f32,
    },
    /// Flash `r,g,b` (0..31) at `intensity` (0..31), decaying over `secs`
    /// (`FlashScreen` 11040).
    Flash {
        r: i32,
        g: i32,
        b: i32,
        intensity: i32,
        secs: f32,
    },
    /// Shake the screen at `power`/`speed` for `secs` (`ShakeScreen` 11050).
    Shake { power: i32, speed: i32, secs: f32 },
}

impl ScreenEffect {
    /// Map a `TintScreen` command `[r, g, b, saturation, duration, wait]`; the
    /// duration is in tenths of a second.
    pub fn tint(params: &[i32]) -> Self {
        Self::Tint {
            r: param(params, 0, 100),
            g: param(params, 1, 100),
            b: param(params, 2, 100),
            sat: param(params, 3, 100),
            secs: tenths(param(params, 4, 0)),
        }
    }

    /// Map a `FlashScreen` command `[r, g, b, intensity, duration, wait]`.
    pub fn flash(params: &[i32]) -> Self {
        Self::Flash {
            r: param(params, 0, 0),
            g: param(params, 1, 0),
            b: param(params, 2, 0),
            intensity: param(params, 3, 0),
            secs: tenths(param(params, 4, 0)),
        }
    }

    /// Map a `ShakeScreen` command `[power, speed, duration, wait]`.
    pub fn shake(params: &[i32]) -> Self {
        Self::Shake {
            power: param(params, 0, 0),
            speed: param(params, 1, 0),
            secs: tenths(param(params, 2, 0)),
        }
    }
}

/// The value at `index`, or `fallback` when the list is too short.
fn param(params: &[i32], index: usize, fallback: i32) -> i32 {
    params.get(index).copied().unwrap_or(fallback)
}

/// Convert an RM2000 tenths-of-a-second duration to seconds.
fn tenths(v: i32) -> f32 {
    v as f32 / 10.0
}

/// The set holding [`apply_camera_shake`], so pictures can order their placement
/// after the shake offset lands (they track the shaken camera).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScreenShakeSet;

/// Which overlay a fullscreen node is, so one query drives all three.
#[derive(Component, Clone, Copy)]
enum FxLayer {
    Tint,
    Flash,
    Fade,
}

/// The live effect state: fade progress, the lerping tint, the decaying flash,
/// and the running shake with its current offset.
#[derive(Resource)]
struct Fx {
    fade_alpha: f32,
    fade_target: f32,
    tint_cur: [f32; 4],
    tint_to: [f32; 4],
    tint_secs_left: f32,
    flash: Option<Flashing>,
    shake: Option<Shaking>,
    shake_offset: Vec2,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            fade_alpha: 0.0,
            fade_target: 0.0,
            tint_cur: NEUTRAL_TINT,
            tint_to: NEUTRAL_TINT,
            tint_secs_left: 0.0,
            flash: None,
            shake: None,
            shake_offset: Vec2::ZERO,
        }
    }
}

/// RM2000 neutral tint (100 = no change on every channel).
const NEUTRAL_TINT: [f32; 4] = [100.0, 100.0, 100.0, 100.0];

struct Flashing {
    r: i32,
    g: i32,
    b: i32,
    intensity: i32,
    elapsed: f32,
    secs: f32,
}

struct Shaking {
    power: i32,
    speed: i32,
    elapsed: f32,
    secs: f32,
}

/// The ambient weather (`Weather` 11070). A static translucent wash over the
/// scene rather than a particle sim; [`render_weather`] paints it. `Rain`/`Snow`/
/// `Fog` are only produced by the interpreter's Weather arm, which lands
/// separately.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
#[allow(dead_code)]
pub enum Weather {
    #[default]
    None,
    Rain,
    Snow,
    Fog,
}

/// The fullscreen node that carries the weather wash, kept separate from the
/// [`FxLayer`] overlays so the effect query never touches it.
#[derive(Component)]
struct WeatherOverlay;

pub struct ScreenFxPlugin;

impl Plugin for ScreenFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ScreenEffect>()
            .init_resource::<Fx>()
            .init_resource::<Weather>()
            .add_systems(Startup, spawn_overlays)
            .add_systems(Update, (step_effects, render_weather))
            .add_systems(
                PostUpdate,
                apply_camera_shake
                    .in_set(ScreenShakeSet)
                    .before(TransformSystems::Propagate),
            );
    }
}

/// The three fullscreen overlays. Tint and flash sit below the message/menu UI
/// (negative z) so those windows stay untinted, yet above every 2D sprite
/// (pictures, the map) because UI always composites over the world. The erase
/// fade sits above everything (even the teleport fade) so a black-out truly
/// covers the screen.
fn spawn_overlays(mut commands: Commands) {
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(-20),
        FxLayer::Tint,
    ));
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(-10),
        FxLayer::Flash,
    ));
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(1003),
        FxLayer::Fade,
    ));
    // Between the color tint (-20) and the flash (-10): an ambient wash that
    // sits over the world but under the flash and every UI window.
    commands.spawn((
        full_screen(),
        transparent(),
        GlobalZIndex(-15),
        WeatherOverlay,
    ));
}

/// An absolutely-positioned node filling the whole screen.
fn full_screen() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// A fully transparent background, the resting state of every overlay.
fn transparent() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0))
}

/// Ingest new effects, advance every running effect, and repaint the overlays.
fn step_effects(
    time: Res<Time>,
    mut effects: MessageReader<ScreenEffect>,
    mut fx: ResMut<Fx>,
    mut layers: Query<(&mut BackgroundColor, &FxLayer)>,
) {
    for effect in effects.read() {
        apply_effect(&mut fx, effect);
    }
    let dt = time.delta_secs();
    step_fade(&mut fx, dt);
    step_tint(&mut fx, dt);
    step_flash(&mut fx, dt);
    step_shake(&mut fx, dt);
    for (mut background, layer) in &mut layers {
        background.0 = overlay_color(&fx, *layer);
    }
}

/// Fold one incoming effect into the live state.
fn apply_effect(fx: &mut Fx, effect: &ScreenEffect) {
    match *effect {
        ScreenEffect::Erase => fx.fade_target = 1.0,
        ScreenEffect::Show => fx.fade_target = 0.0,
        ScreenEffect::Tint { r, g, b, sat, secs } => {
            fx.tint_to = [r as f32, g as f32, b as f32, sat as f32];
            fx.tint_secs_left = secs;
            if secs <= 0.0 {
                fx.tint_cur = fx.tint_to;
            }
        }
        ScreenEffect::Flash {
            r,
            g,
            b,
            intensity,
            secs,
        } => {
            fx.flash = Some(Flashing {
                r,
                g,
                b,
                intensity,
                elapsed: 0.0,
                secs,
            });
        }
        ScreenEffect::Shake { power, speed, secs } => {
            fx.shake = Some(Shaking {
                power,
                speed,
                elapsed: 0.0,
                secs,
            });
        }
    }
}

fn step_fade(fx: &mut Fx, dt: f32) {
    fx.fade_alpha = approach(fx.fade_alpha, fx.fade_target, FADE_SPEED * dt);
}

fn step_tint(fx: &mut Fx, dt: f32) {
    if fx.tint_secs_left <= 0.0 {
        fx.tint_cur = fx.tint_to;
        return;
    }
    let t = (dt / fx.tint_secs_left).clamp(0.0, 1.0);
    for i in 0..4 {
        fx.tint_cur[i] += (fx.tint_to[i] - fx.tint_cur[i]) * t;
    }
    fx.tint_secs_left -= dt;
}

fn step_flash(fx: &mut Fx, dt: f32) {
    if let Some(flash) = &mut fx.flash {
        flash.elapsed += dt;
        if flash.elapsed >= flash.secs {
            fx.flash = None;
        }
    }
}

fn step_shake(fx: &mut Fx, dt: f32) {
    let Some(shake) = &mut fx.shake else {
        fx.shake_offset = Vec2::ZERO;
        return;
    };
    shake.elapsed += dt;
    if shake.elapsed >= shake.secs {
        fx.shake = None;
        fx.shake_offset = Vec2::ZERO;
        return;
    }
    fx.shake_offset = Vec2::new(shake_offset(shake.power, shake.speed, shake.elapsed), 0.0);
}

/// The color an overlay should paint given the current state.
fn overlay_color(fx: &Fx, layer: FxLayer) -> Color {
    match layer {
        FxLayer::Fade => Color::srgba(0.0, 0.0, 0.0, fx.fade_alpha),
        FxLayer::Tint => {
            let [r, g, b, a] = tint_overlay(
                fx.tint_cur[0] as i32,
                fx.tint_cur[1] as i32,
                fx.tint_cur[2] as i32,
                fx.tint_cur[3] as i32,
            );
            Color::srgba(r, g, b, a)
        }
        FxLayer::Flash => match &fx.flash {
            Some(flash) => {
                let (r, g, b) = flash_color(flash.r, flash.g, flash.b);
                Color::srgba(
                    r,
                    g,
                    b,
                    flash_alpha(flash.intensity, flash.elapsed, flash.secs),
                )
            }
            None => Color::srgba(0.0, 0.0, 0.0, 0.0),
        },
    }
}

/// Add the current shake offset to the camera after `camera_follow` set its base
/// position. Runs every frame; the follow re-centres next frame, so the offset
/// never accumulates.
fn apply_camera_shake(fx: Res<Fx>, mut cameras: Query<&mut Transform, With<MainCamera>>) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    camera.translation.x += fx.shake_offset.x;
    camera.translation.y += fx.shake_offset.y;
}

/// Repaint the weather overlay when [`Weather`] changes (and once at startup).
fn render_weather(
    weather: Res<Weather>,
    mut overlays: Query<&mut BackgroundColor, With<WeatherOverlay>>,
) {
    if !weather.is_changed() {
        return;
    }
    for mut background in &mut overlays {
        background.0 = weather_color(*weather);
    }
}

/// The translucent wash a weather kind paints: rain a blue-grey, snow a
/// near-white, fog a flat grey; `None` is fully transparent (hidden).
fn weather_color(weather: Weather) -> Color {
    match weather {
        Weather::None => Color::srgba(0.0, 0.0, 0.0, 0.0),
        Weather::Rain => Color::srgba(0.35, 0.42, 0.55, 0.28),
        Weather::Snow => Color::srgba(0.90, 0.92, 0.98, 0.30),
        Weather::Fog => Color::srgba(0.62, 0.62, 0.66, 0.42),
    }
}

/// Step `cur` toward `target` by at most `step`.
fn approach(cur: f32, target: f32, step: f32) -> f32 {
    if cur < target {
        (cur + step).min(target)
    } else {
        (cur - step).max(target)
    }
}

/// The overlay color+alpha (0..1 each) approximating an RM2000 multiply tint
/// `(r,g,b,sat)` in 0..200 (100 = neutral). The alpha darkens by the darkest
/// channel; the color adds the brighter channels back so a colored tint keeps
/// its hue. Brightening (values > 100) and saturation aren't representable by an
/// alpha overlay and read as neutral (first pass).
fn tint_overlay(r: i32, g: i32, b: i32, _sat: i32) -> [f32; 4] {
    let fr = (r as f32 / 100.0).clamp(0.0, 1.0);
    let fg = (g as f32 / 100.0).clamp(0.0, 1.0);
    let fb = (b as f32 / 100.0).clamp(0.0, 1.0);
    let min_f = fr.min(fg).min(fb);
    let alpha = 1.0 - min_f;
    if alpha <= f32::EPSILON {
        return [0.0, 0.0, 0.0, 0.0];
    }
    [
        (fr - min_f) / alpha,
        (fg - min_f) / alpha,
        (fb - min_f) / alpha,
        alpha,
    ]
}

/// The flash overlay's RGB (0..1) from RM2000 0..31 channels.
fn flash_color(r: i32, g: i32, b: i32) -> (f32, f32, f32) {
    (
        (r as f32 / 31.0).clamp(0.0, 1.0),
        (g as f32 / 31.0).clamp(0.0, 1.0),
        (b as f32 / 31.0).clamp(0.0, 1.0),
    )
}

/// The flash alpha, decaying linearly from `intensity/31` at `elapsed == 0` to 0
/// at `elapsed == secs`.
fn flash_alpha(intensity: i32, elapsed: f32, secs: f32) -> f32 {
    if secs <= 0.0 {
        return 0.0;
    }
    let peak = (intensity as f32 / 31.0).clamp(0.0, 1.0);
    (peak * (1.0 - elapsed / secs)).clamp(0.0, 1.0)
}

/// The horizontal camera offset (world units) for a shake of `power`/`speed` at
/// `elapsed` seconds: a sine whose amplitude scales with power and frequency
/// with speed. Zero at `elapsed == 0`, so the screen starts centred.
fn shake_offset(power: i32, speed: i32, elapsed: f32) -> f32 {
    let amplitude = power as f32 * SHAKE_AMPLITUDE;
    let frequency = speed as f32 * SHAKE_FREQUENCY;
    amplitude * (elapsed * frequency).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tint_neutral_is_fully_transparent() {
        assert_eq!(tint_overlay(100, 100, 100, 100), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn tint_gray_darkens_with_black_overlay() {
        // A uniform 70/100 tint = a 30%-opaque black overlay (screen * 0.7).
        let [r, g, b, a] = tint_overlay(70, 70, 70, 70);
        assert_eq!((r, g, b), (0.0, 0.0, 0.0));
        assert!((a - 0.3).abs() < 1e-6);
    }

    #[test]
    fn tint_colored_keeps_hue_in_the_overlay() {
        // Green darkest -> alpha from green; red/blue brighter -> tinted overlay.
        let [r, g, b, a] = tint_overlay(70, 60, 70, 100);
        assert!((a - 0.4).abs() < 1e-6);
        assert!(r > 0.0 && b > 0.0 && g == 0.0);
    }

    #[test]
    fn tint_black_is_full_black_overlay() {
        assert_eq!(tint_overlay(0, 0, 0, 100), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn flash_alpha_decays_from_peak_to_zero() {
        assert!((flash_alpha(31, 0.0, 0.5) - 1.0).abs() < 1e-6);
        assert!((flash_alpha(31, 0.25, 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(flash_alpha(31, 0.5, 0.5), 0.0);
        assert_eq!(flash_alpha(31, 1.0, 0.5), 0.0);
    }

    #[test]
    fn flash_alpha_scales_with_intensity() {
        assert!((flash_alpha(20, 0.0, 0.5) - 20.0 / 31.0).abs() < 1e-6);
    }

    #[test]
    fn flash_color_normalises_0_31_channels() {
        assert_eq!(flash_color(31, 0, 31), (1.0, 0.0, 1.0));
    }

    #[test]
    fn shake_starts_centred_and_stays_bounded() {
        assert_eq!(shake_offset(5, 5, 0.0), 0.0);
        for i in 0..200 {
            let t = i as f32 * 0.01;
            assert!(shake_offset(5, 5, t).abs() <= 5.0 * SHAKE_AMPLITUDE + 1e-4);
        }
    }

    #[test]
    fn approach_moves_toward_target_without_overshoot() {
        assert_eq!(approach(0.0, 1.0, 0.3), 0.3);
        assert_eq!(approach(0.9, 1.0, 0.3), 1.0);
        assert_eq!(approach(1.0, 0.0, 0.3), 0.7);
        assert_eq!(approach(0.1, 0.0, 0.3), 0.0);
    }

    #[test]
    fn tint_command_maps_params_and_tenths_duration() {
        assert_eq!(
            ScreenEffect::tint(&[70, 60, 70, 100, 5, 1]),
            ScreenEffect::Tint {
                r: 70,
                g: 60,
                b: 70,
                sat: 100,
                secs: 0.5,
            }
        );
    }

    #[test]
    fn flash_command_maps_intensity_and_duration() {
        assert_eq!(
            ScreenEffect::flash(&[31, 31, 31, 20, 5, 0]),
            ScreenEffect::Flash {
                r: 31,
                g: 31,
                b: 31,
                intensity: 20,
                secs: 0.5,
            }
        );
    }

    #[test]
    fn shake_command_maps_power_speed_duration() {
        assert_eq!(
            ScreenEffect::shake(&[3, 4, 40, 0]),
            ScreenEffect::Shake {
                power: 3,
                speed: 4,
                secs: 4.0,
            }
        );
    }

    #[test]
    fn weather_none_is_hidden() {
        assert_eq!(
            weather_color(Weather::None),
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn weather_kinds_are_translucent_not_opaque() {
        for kind in [Weather::Rain, Weather::Snow, Weather::Fog] {
            let a = weather_color(kind).alpha();
            assert!(a > 0.0 && a < 1.0);
        }
    }
}
