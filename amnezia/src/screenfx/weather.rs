//! Weather commands with original rain and compatibility snow/fog renderers.

mod fog;
pub(crate) mod rain;

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
use fog::{FogAssets, init_fog, scroll_fog, spawn_fog};

/// The RM2000 native viewport the particles live in (1 world unit = 1 pixel).
const SCREEN_W: f32 = 320.0;
const SCREEN_H: f32 = 240.0;

/// World draw-Z of the weather, above every map tile/character (all ≤ ~9) and
/// within the 2D camera's ±1000 range. The two fog layers sit just under it.
const WEATHER_Z: f32 = 100.0;

/// Particles per strength 0/1/2 (weak/medium/strong), EasyRPG's
/// `num_rain_or_snow_particles`. Strength is clamped into this range.
const PARTICLE_COUNT: [usize; 3] = [20, 60, 100];

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

/// Last snow/fog presentation built by the compatibility renderer.
/// Rain keeps its own persistent state.
#[derive(Resource, Default)]
struct AppliedWeather {
    weather: Weather,
    strength: i32,
}

/// Tags compatibility weather sprites so one query clears them all
/// on a change or map swap.
#[derive(Component)]
struct WeatherEntity;

/// A compatibility snow flake in screen space (`x` right, `y`
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
    /// The on-screen x including the snow wobble.
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
        rain::register(app);
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

/// Whether the weather should keep animating: paused while a map-suspending scene
/// is on top, mirroring how RPG_RT freezes the map's screen update there.
fn weather_running(
    transition: crate::transitions::TransitionPause,
    battle: Res<BattleActive>,
    menu: Res<MenuOpen>,
    shop: Res<ShopOpen>,
    title: Res<TitleActive>,
    gameover: Res<GameOverActive>,
) -> bool {
    !(transition.paused() || battle.0 || menu.0 || shop.0 || title.0 || gameover.0)
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
        Weather::Rain => {}
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

#[cfg(test)]
mod tests;
