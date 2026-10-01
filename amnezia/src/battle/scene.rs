//! Front-view backdrop and enemy sprites in the animation overlay.

mod effects;
pub(super) mod smoke;
#[cfg(test)]
mod tests;

pub(crate) use effects::EffectsSet;

use super::model::{Battle, Dying, Phase};
use crate::animation::{overlay_layer, overlay_translation};
use crate::assets::resolve_png;
use crate::legacy_colors::{hue::HueShift, tone::SpriteTone};
use bevy::prelude::*;

/// World z of the full-screen backdrop on the overlay: below the battlers.
const BACKDROP_Z: f32 = 100.0;

/// Battlers sit above the backdrop and below flashes/cells; tiny offsets preserve placement order.
const BATTLER_Z: f32 = 200.0;

/// How close (squared, in RM2000 px) an animation flash must land to a battler's
/// base to tint it. Both are integer-derived, so this only guards float noise.
const FLASH_MATCH_EPS: f32 = 0.5;

/// EasyRPG uses half an animation cell (48 px) until the battler bitmap is loaded.
const FALLBACK_BATTLER_HEIGHT: f32 = 48.0;

/// A backdrop or battler sprite belonging to the live fight;
/// despawned together when the fight ends (or a new one starts).
#[derive(Component)]
pub(super) struct SceneEntity;

#[derive(Component)]
struct Canvas;

type MovingScene = (With<SceneEntity>, Without<Canvas>);

/// Enemy sprite with a shared animation/flash anchor and measured pixel height.
/// Uses [`FALLBACK_BATTLER_HEIGHT`] until the image loads.
#[derive(Component)]
#[require(effects::Effects, crate::legacy_colors::flash::SpriteFlash)]
pub(super) struct Battler {
    index: usize,
    base: Vec2,
    pub(super) height: f32,
}

/// Re-centre backdrop coordinates on the animation/flash overlay (y down).
fn battler_base(x: u32, y: u32) -> Vec2 {
    Vec2::new(x as f32 - 160.0, y as f32 - 120.0)
}

/// Rebuild the backdrop and battler sprites when a new fight starts (keyed on the
/// per-fight generation stamp), and clear them when it ends.
fn sync_scene(
    mut commands: Commands,
    battle: Res<Battle>,
    tint: Option<Res<crate::screenfx::TintState>>,
    asset_server: Res<AssetServer>,
    scene: Query<Entity, With<SceneEntity>>,
    mut synced: Local<u64>,
) {
    if !battle.is_changed() || battle.generation == *synced {
        return;
    }
    *synced = battle.generation;
    for entity in &scene {
        commands.entity(entity).despawn();
    }
    if battle.phase == Phase::Inactive {
        return;
    }
    commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::new(320.0, 240.0)),
        Transform::from_xyz(0.0, 0.0, BACKDROP_Z - 1.0),
        overlay_layer(),
        SceneEntity,
        Canvas,
    ));
    let tone = SpriteTone(tint.as_ref().map_or([100.0; 4], |tint| tint.tone()));
    let image = asset_server.load(resolve_png("Backdrop", &battle.background));
    commands.spawn((
        Sprite {
            image: image.clone(),
            custom_size: Some(Vec2::new(320.0, 240.0)),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0,
            },
            ..default()
        },
        Transform::from_translation(overlay_translation(Vec2::ZERO, BACKDROP_Z)),
        overlay_layer(),
        SceneEntity,
        HueShift {
            original: image,
            degrees: 0,
        },
        tone,
    ));
    for (index, foe) in battle.enemies.iter().enumerate() {
        let base = battler_base(foe.x, foe.y);
        let image = asset_server.load(resolve_png("Monster", &foe.battler));
        commands.spawn((
            Sprite {
                image: image.clone(),
                color: battler_look(foe.alive(), None).0,
                ..default()
            },
            Transform::from_translation(overlay_translation(base, BATTLER_Z + index as f32 * 0.1)),
            overlay_layer(),
            SceneEntity,
            HueShift {
                original: image,
                degrees: foe.battler_hue,
            },
            tone,
            Battler {
                index,
                base,
                height: FALLBACK_BATTLER_HEIGHT,
            },
        ));
    }
}

/// Replace fallback heights after loading so head/feet animations align with the bitmap.
fn measure_battlers(images: Res<Assets<Image>>, mut battlers: Query<(&mut Battler, &Sprite)>) {
    for (mut battler, sprite) in &mut battlers {
        if let Some(image) = images.get(&sprite.image) {
            let height = image.height() as f32;
            if battler.height != height {
                battler.height = height;
            }
        }
    }
}

/// Height at the target centre; undrawn party targets return `None`.
pub(super) fn battler_height_at(battlers: &Query<&Battler>, pos: Vec2) -> Option<f32> {
    battlers
        .iter()
        .find(|b| b.base.distance_squared(pos) < FLASH_MATCH_EPS)
        .map(|b| b.height)
}

fn battler_look(alive: bool, dying: Option<&Dying>) -> (Color, f32) {
    if alive {
        return (Color::WHITE, 1.0);
    }
    match dying {
        Some(d) => {
            let progress = (d.elapsed / d.secs).clamp(0.0, 1.0);
            let fade = 1.0 - progress;
            if d.explode {
                (
                    Color::srgba(1.0, 1.0, 1.0, fade * 240.0 / 255.0),
                    1.0 + progress,
                )
            } else {
                (Color::srgba(1.0, 1.0, 1.0, fade * 252.0 / 255.0), 1.0)
            }
        }
        None => (Color::srgba(1.0, 1.0, 1.0, 0.0), 1.0),
    }
}

fn sync_tone(
    tint: Option<Res<crate::screenfx::TintState>>,
    mut sprites: Query<&mut SpriteTone, With<SceneEntity>>,
) {
    let Some(tint) = tint else {
        return;
    };
    for mut tone in &mut sprites {
        tone.set_if_neq(SpriteTone(tint.tone()));
    }
}

fn apply_shake(
    shake: crate::screenfx::ScreenShake,
    mut scene: Query<(Option<&Battler>, &mut Transform), MovingScene>,
) {
    for (battler, mut transform) in &mut scene {
        let base = battler.map_or(Vec2::ZERO, |battler| battler.base);
        transform.translation = overlay_translation(base + shake.offset(), transform.translation.z);
    }
}

pub fn register(app: &mut App) {
    effects::register(app);
    app.add_systems(
        Update,
        (
            sync_scene.after(super::flow::drive),
            measure_battlers.after(sync_scene),
        ),
    )
    .add_systems(
        PostUpdate,
        (
            sync_tone.before(crate::legacy_colors::hue::HueSet),
            apply_shake
                .after(EffectsSet)
                .before(bevy::transform::TransformSystems::Propagate),
        ),
    );
}
