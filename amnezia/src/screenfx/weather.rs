//! RM2000 ambient weather (`Weather` 11070) as real particles and overlays,
//! replacing the old flat tint-wash. Faithful to EasyRPG's `Weather`/`Game_Screen`:
//!
//! - **Rain** and **snow** are drawn as many small [`Sprite`] particles whose
//!   count comes from the strength (`{20, 60, 100}` for weak/medium/strong, the
//!   `num_rain_or_snow_particles` table). Rain is a fast diagonal streak; snow is
//!   a slow flake with a gentle horizontal wobble. Both recycle to the top when
//!   they fall off the bottom (see [`WeatherParticle::advance`]).
//! - **Fog** is a two-layer semi-transparent grey overlay that slowly scrolls,
//!   its opacity taken from the strength (the `fog_opacity` table). Not particles.
//! - **Sand** (RM2000 weather type 4) is intentionally omitted: the RM2000 Weather
//!   event command only exposes rain/snow/fog, [`Weather::from_code`] never yields
//!   it, and no map in this game sets it. See the crate report.
//!
//! The particles/overlays are plain layer-0 sprites screen-pinned to the
//! [`MainCamera`] each frame, so the camera's [`crate::screenfx::ScreenTone`]
//! post-process tints them (as RPG_RT tones weather) and they draw above the map
//! yet below the front camera's pictures and the message/menu UI. Motion pauses
//! while a map-suspending scene is up (battle/menu/shop/title/game-over), matching
//! how RPG_RT freezes the map's screen update, and clears on a map change.

use crate::battle::BattleActive;
use crate::gameover::GameOverActive;
use crate::menu::MenuOpen;
use crate::shop::ShopOpen;
use crate::title::TitleActive;
use crate::world::{MainCamera, MapChanged};
use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The RM2000 native viewport the particles live in (1 world unit = 1 pixel).
const SCREEN_W: f32 = 320.0;
const SCREEN_H: f32 = 240.0;

/// World draw-Z of the weather, above every map tile/character (all ≤ ~9) and
/// within the 2D camera's ±1000 range. The two fog layers sit just under it.
const WEATHER_Z: f32 = 100.0;

/// Particles per strength 0/1/2 (weak/medium/strong), EasyRPG's
/// `num_rain_or_snow_particles`. Strength is clamped into this range.
const PARTICLE_COUNT: [usize; 3] = [20, 60, 100];

/// Per-strength fog layer opacities (0..255), EasyRPG's `fog_opacity` rows: the
/// back layer fades out entirely at the hacked strength 3, the front peaks opaque.
const FOG_BACK_OPACITY: [u8; 4] = [32, 64, 96, 0];
const FOG_FRONT_OPACITY: [u8; 4] = [64, 80, 160, 255];

/// The scrolling fog texture size. It is twice [`SCREEN_W`] wide with its right
/// half a copy of its left, giving a horizontal period of [`SCREEN_W`] so a layer
/// can scroll and wrap seamlessly; the extra height covers the front layer's bob.
const FOG_W: usize = 640;
const FOG_H: usize = 280;
const FOG_HALF: usize = 320;

/// The active ambient weather (`Weather` 11070, `params[0]`). Persisted by the
/// save so a load restores it; the interpreter maps a command through
/// [`Self::from_code`] and the render systems react to changes.
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Weather {
    #[default]
    None,
    Rain,
    Snow,
    Fog,
}

impl Weather {
    /// The RM2000 weather code (`params[0]`): 0 none, 1 rain, 2 snow, 3 fog. The
    /// interpreter and the save share this mapping. Type 4 (sandstorm) is not
    /// producible by the RM2000 Weather command, so it folds to `None`.
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => Weather::Rain,
            2 => Weather::Snow,
            3 => Weather::Fog,
            _ => Weather::None,
        }
    }

    /// This weather's RM2000 code, the inverse of [`Self::from_code`], for a save.
    pub fn code(self) -> i32 {
        match self {
            Weather::None => 0,
            Weather::Rain => 1,
            Weather::Snow => 2,
            Weather::Fog => 3,
        }
    }
}

/// The active weather's strength (`Weather` 11070, `params[1]`): 0/1/2 =
/// weak/medium/strong, scaling the particle count and fog opacity. Kept as a
/// separate resource because RM2000's save stores it apart from the type.
#[derive(Resource, Default)]
pub struct WeatherStrength(pub i32);

/// The last `(weather, strength)` the render systems built entities for. RPG_RT
/// only re-initialises particles when the type or strength actually changes (some
/// games set the same weather from a parallel process every frame); comparing
/// against this avoids respawning — and so freezing — the effect each tick.
#[derive(Resource, Default)]
struct AppliedWeather {
    weather: Weather,
    strength: i32,
}

/// Tags every weather sprite (particle or fog layer) so one query clears them all
/// on a change or map swap.
#[derive(Component)]
struct WeatherEntity;

/// A single rain streak or snow flake in RM2000 screen space (`x` right, `y`
/// down, origin top-left). Carries its own motion so [`Self::advance`] is a pure,
/// testable step, and a small LCG state for respawning across the top.
#[derive(Component)]
struct WeatherParticle {
    x: f32,
    y: f32,
    fall: f32,
    drift: f32,
    wobble_amp: f32,
    wobble_freq: f32,
    phase: f32,
    rng: u32,
}

impl WeatherParticle {
    /// The on-screen x including the snow wobble (rain has zero amplitude).
    fn draw_x(&self) -> f32 {
        self.x + self.wobble_amp * self.phase.sin()
    }

    /// Advance one frame of `dt` seconds: fall, drift, and wobble. When the flake
    /// passes the bottom edge it recycles to the top with a fresh random x,
    /// returning `true` (RPG_RT respawns spent particles; here they wrap). Pure and
    /// deterministic given the particle's state — the unit tests exercise it.
    fn advance(&mut self, dt: f32) -> bool {
        self.y += self.fall * dt;
        self.x += self.drift * dt;
        self.phase += self.wobble_freq * dt;
        if self.y >= SCREEN_H {
            self.y -= SCREEN_H;
            self.x = self.respawn_x();
            return true;
        }
        false
    }

    /// A fresh spawn x across the screen width, advancing the LCG.
    fn respawn_x(&mut self) -> f32 {
        next_frac(&mut self.rng) * SCREEN_W
    }
}

/// One fog overlay layer: how far it has scrolled, its leftward speed, and its
/// vertical bob (the front layer bobs; the back layer does not).
#[derive(Component)]
struct FogLayer {
    scroll: f32,
    speed: f32,
    bob_amp: f32,
    bob_freq: f32,
    time: f32,
    z: f32,
}

/// The generated scrolling fog texture, built once at startup.
#[derive(Resource)]
struct FogAssets {
    texture: Handle<Image>,
}

/// The resources [`rebuild_weather`] reads and updates, bundled so the system
/// stays within Bevy's parameter count.
#[derive(SystemParam)]
struct WeatherInput<'w> {
    weather: Res<'w, Weather>,
    strength: Res<'w, WeatherStrength>,
    fog: Res<'w, FogAssets>,
    applied: ResMut<'w, AppliedWeather>,
}

/// The immutable per-kind motion and look of a particle.
#[derive(Clone, Copy)]
struct Kind {
    fall: f32,
    drift: f32,
    wobble_amp: f32,
    wobble_freq: f32,
    w: f32,
    h: f32,
    alpha: f32,
    rotate: bool,
}

/// Rain: fast (4 px/frame ≈ 240 px/s) with a 1 px/frame leftward slant, a thin
/// near-vertical streak leaning ~14° along its fall, at moderate opacity.
const RAIN: Kind = Kind {
    fall: 240.0,
    drift: -60.0,
    wobble_amp: 0.0,
    wobble_freq: 0.0,
    w: 1.5,
    h: 16.0,
    alpha: 0.5,
    rotate: true,
};

/// Snow: slow (≈ 2.5 px/frame ≈ 150 px/s) with a small leftward drift plus a
/// gentle sine wobble, a tiny bright flake.
const SNOW: Kind = Kind {
    fall: 150.0,
    drift: -30.0,
    wobble_amp: 5.0,
    wobble_freq: 2.0,
    w: 2.5,
    h: 2.5,
    alpha: 0.85,
    rotate: false,
};

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Weather>()
            .init_resource::<WeatherStrength>()
            .init_resource::<AppliedWeather>()
            .add_systems(Startup, init_fog)
            .add_systems(
                Update,
                (
                    rebuild_weather,
                    (step_particles, scroll_fog).run_if(weather_running),
                ),
            );
    }
}

/// Generate the fog texture (a subtle near-white noise, horizontally periodic)
/// and stash its handle for the fog layers to share.
fn init_fog(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(FogAssets {
        texture: images.add(fog_image()),
    });
}

/// Whether the weather should keep animating: paused while a map-suspending scene
/// is on top, mirroring how RPG_RT freezes the map's screen update there.
fn weather_running(
    transition: Option<Res<crate::transitions::Transition>>,
    battle: Res<BattleActive>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    title: Res<TitleActive>,
    gameover: Res<GameOverActive>,
) -> bool {
    !(transition.as_ref().is_some_and(|t| t.busy())
        || battle.0
        || menu.0
        || shop.0
        || title.0
        || gameover.0)
}

/// Rebuild the weather entities when the type/strength changes or the map swaps:
/// clear the old sprites and spawn the new set (nothing for `None`). Only fires on
/// an actual change so a per-frame parallel re-set does not restart the effect.
fn rebuild_weather(
    mut commands: Commands,
    mut input: WeatherInput,
    mut map_changed: MessageReader<MapChanged>,
    cameras: Query<&Transform, With<MainCamera>>,
    existing: Query<Entity, With<WeatherEntity>>,
) {
    let map_changed = map_changed.read().count() > 0;
    let weather = *input.weather;
    let strength = input.strength.0;
    let differs = weather != input.applied.weather || strength != input.applied.strength;
    if !map_changed && !differs {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let base = cameras
        .single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    match weather {
        Weather::None => {}
        Weather::Rain => spawn_particles(&mut commands, RAIN, particle_count(strength), base),
        Weather::Snow => spawn_particles(&mut commands, SNOW, particle_count(strength), base),
        Weather::Fog => spawn_fog(&mut commands, &input.fog, strength, base),
    }
    input.applied.weather = weather;
    input.applied.strength = strength;
}

/// The particle count for a strength, clamped into the RM2000 0..2 range.
fn particle_count(strength: i32) -> usize {
    PARTICLE_COUNT[strength.clamp(0, 2) as usize]
}

/// Spawn `count` particles of `kind`, spread across the screen and placed
/// relative to the camera `base` so they appear on-screen from the first frame.
fn spawn_particles(commands: &mut Commands, kind: Kind, count: usize, base: Vec2) {
    let rotation = if kind.rotate {
        Quat::from_rotation_z((kind.drift / kind.fall).atan())
    } else {
        Quat::IDENTITY
    };
    for i in 0..count {
        let particle = new_particle(kind, seed_for(i));
        let translation = world_pos(base, particle.draw_x(), particle.y);
        commands.spawn((
            Sprite::from_color(
                Color::srgba(1.0, 1.0, 1.0, kind.alpha),
                Vec2::new(kind.w, kind.h),
            ),
            Transform {
                translation,
                rotation,
                ..default()
            },
            WeatherEntity,
            particle,
        ));
    }
}

/// Build a particle of `kind` with a seeded spread of start position and phase.
fn new_particle(kind: Kind, seed: u32) -> WeatherParticle {
    let mut rng = seed | 1;
    let x = next_frac(&mut rng) * SCREEN_W;
    let y = next_frac(&mut rng) * SCREEN_H;
    let phase = next_frac(&mut rng) * std::f32::consts::TAU;
    WeatherParticle {
        x,
        y,
        fall: kind.fall,
        drift: kind.drift,
        wobble_amp: kind.wobble_amp,
        wobble_freq: kind.wobble_freq,
        phase,
        rng,
    }
}

/// A distinct, well-spread seed per particle index.
fn seed_for(i: usize) -> u32 {
    (i as u32)
        .wrapping_mul(2_654_435_761)
        .wrapping_add(0x9E37_79B9)
}

/// Spawn the two fog overlay layers at strength-derived opacities.
fn spawn_fog(commands: &mut Commands, fog: &FogAssets, strength: i32, base: Vec2) {
    let level = strength.clamp(0, 3) as usize;
    commands.spawn(fog_sprite(
        fog,
        FOG_BACK_OPACITY[level] as f32 / 255.0,
        FogLayer {
            scroll: 0.0,
            speed: 15.0,
            bob_amp: 0.0,
            bob_freq: 0.0,
            time: 0.0,
            z: WEATHER_Z - 2.0,
        },
        base,
    ));
    commands.spawn(fog_sprite(
        fog,
        FOG_FRONT_OPACITY[level] as f32 / 255.0,
        FogLayer {
            scroll: 0.0,
            speed: 7.5,
            bob_amp: 8.0,
            bob_freq: 0.6,
            time: 0.0,
            z: WEATHER_Z - 1.0,
        },
        base,
    ));
}

/// One fog layer's sprite bundle: the shared texture at `alpha`, screen-pinned to
/// the camera `base` at its initial scroll offset.
fn fog_sprite(
    fog: &FogAssets,
    alpha: f32,
    layer: FogLayer,
    base: Vec2,
) -> (Sprite, Transform, WeatherEntity, FogLayer) {
    let sx = base.x + layer.scroll.rem_euclid(SCREEN_W) - SCREEN_W / 2.0;
    let z = layer.z;
    (
        Sprite {
            image: fog.texture.clone(),
            custom_size: Some(Vec2::new(FOG_W as f32, FOG_H as f32)),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            ..default()
        },
        Transform::from_xyz(sx, base.y, z),
        WeatherEntity,
        layer,
    )
}

/// Advance and place every particle, screen-pinned to the (followed, shaken)
/// camera. Runs only while [`weather_running`], so it freezes with the map.
fn step_particles(
    time: Res<Time>,
    cameras: Query<&Transform, (With<MainCamera>, Without<WeatherParticle>)>,
    mut particles: Query<(&mut WeatherParticle, &mut Transform)>,
) {
    let Ok(base) = cameras.single().map(|t| t.translation.truncate()) else {
        return;
    };
    let dt = time.delta_secs();
    for (mut particle, mut transform) in &mut particles {
        particle.advance(dt);
        transform.translation = world_pos(base, particle.draw_x(), particle.y);
    }
}

/// Scroll and place the fog layers, screen-pinned to the camera. The horizontal
/// wrap uses the texture's [`SCREEN_W`] period so it is seamless.
fn scroll_fog(
    time: Res<Time>,
    cameras: Query<&Transform, (With<MainCamera>, Without<FogLayer>)>,
    mut layers: Query<(&mut FogLayer, &mut Transform)>,
) {
    let Ok(base) = cameras.single().map(|t| t.translation.truncate()) else {
        return;
    };
    let dt = time.delta_secs();
    for (mut layer, mut transform) in &mut layers {
        layer.scroll -= layer.speed * dt;
        layer.time += dt;
        let sx = base.x + layer.scroll.rem_euclid(SCREEN_W) - SCREEN_W / 2.0;
        let bob = layer.bob_amp * (layer.time * layer.bob_freq).sin();
        transform.translation = Vec3::new(sx, base.y + bob, layer.z);
    }
}

/// The world translation of a particle at RM2000 screen `(sx, sy)` (y down) when
/// the camera is centred at `base`: screen centre maps to the camera centre and y
/// flips for world y-up.
fn world_pos(base: Vec2, sx: f32, sy: f32) -> Vec3 {
    Vec3::new(
        base.x + sx - SCREEN_W / 2.0,
        base.y + SCREEN_H / 2.0 - sy,
        WEATHER_Z,
    )
}

/// Advance an LCG and return its next value as a `[0, 1)` fraction.
fn next_frac(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 8) as f32 / (1u32 << 24) as f32
}

/// Build the fog texture: near-white noise (EasyRPG's grey fog colours) whose
/// right half copies its left, so it tiles horizontally with a [`SCREEN_W`]
/// period for a seamless scroll.
fn fog_image() -> Image {
    let mut data = vec![0u8; FOG_W * FOG_H * 4];
    let greys = [230u8, 240, 255];
    let mut rng = 0x9E37_79B9u32;
    for y in 0..FOG_H {
        for x in 0..FOG_HALF {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let grey = greys[(rng >> 16) as usize % greys.len()];
            let left = (y * FOG_W + x) * 4;
            let right = (y * FOG_W + x + FOG_HALF) * 4;
            for start in [left, right] {
                data[start] = grey;
                data[start + 1] = grey;
                data[start + 2] = grey;
                data[start + 3] = 255;
            }
        }
    }
    Image::new(
        Extent3d {
            width: FOG_W as u32,
            height: FOG_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

#[cfg(test)]
mod tests;
