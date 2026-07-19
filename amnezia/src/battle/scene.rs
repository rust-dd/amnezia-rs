//! The front-view battle scene as world sprites: a full-screen backdrop and one
//! battler per enemy, drawn on the shared effect overlay (render layer 1) so they
//! sit in the same coordinate space as the effect animations — a battler's centre
//! is exactly the point [`super::resolve`] plays a hit animation on, so effects
//! land on their target by construction. The scene is rebuilt per fight (keyed on
//! [`Battle::generation`]) and despawned when the fight ends. The HUD windows live
//! in the sibling [`super::hud`] on a higher-order camera above this overlay.

use super::model::{Battle, Dying, MenuLevel, Phase};
use crate::animation::{BattlerFlash, overlay_layer, overlay_translation};
use crate::assets::resolve_png;
use bevy::prelude::*;

/// World z of the full-screen backdrop on the overlay: below the battlers.
const BACKDROP_Z: f32 = 100.0;

/// World z of the enemy battlers on the overlay: above the backdrop, below the
/// screen flash (300) and animation cells (510). Later battlers add a sliver so a
/// fixed placement order is stable.
const BATTLER_Z: f32 = 200.0;

/// How close (squared, in RM2000 px) a [`BattlerFlash`] must land to a battler's
/// base to tint it. Both are integer-derived, so this only guards float noise.
const FLASH_MATCH_EPS: f32 = 0.5;

/// The battler height an animation's `position` anchor assumes until the battler
/// image has loaded and its real pixel height is known: EasyRPG falls back to
/// `GetAnimationCellHeight() / 2` = 48 (`battle_animation.cpp`).
const FALLBACK_BATTLER_HEIGHT: f32 = 48.0;

/// A backdrop, battler, or floating-number sprite belonging to the live fight;
/// despawned together when the fight ends (or a new one starts).
#[derive(Component)]
pub(super) struct SceneEntity;

/// Seconds a guaranteed per-hit blink brightens a struck foe before it clears
/// (RM2000 `SetBlinkTimer(20)` at 60 fps).
const BLINK_SECS: f32 = 20.0 / 60.0;

/// Peak amount a blink adds to each colour channel at its start. It brightens
/// (adds) rather than tints (lerps), so an already-white sprite still flashes —
/// a multiply tint toward white is a no-op.
const BLINK_BRIGHTEN: f32 = 0.9;

/// An enemy battler sprite: its index into [`Battle::enemies`], its RM2000
/// screen-offset base — the anchor animations play on and the point a target
/// flash is matched against — and its measured pixel height, from which an
/// animation's `position` anchor derives its vertical offset (see
/// [`battler_height_at`]). Height starts at [`FALLBACK_BATTLER_HEIGHT`] and is
/// updated once the battler image loads.
#[derive(Component)]
pub(super) struct Battler {
    index: usize,
    base: Vec2,
    pub(super) height: f32,
}

/// A running target-flash tint on a battler: `rgb`/`power` the flash colour and
/// peak strength, decaying back to the base tint over `secs`. Fired by an
/// animation's own `flash_scope=1` timings.
#[derive(Component)]
struct BattlerTint {
    elapsed: f32,
    secs: f32,
    rgb: [f32; 3],
    power: f32,
}

/// A guaranteed per-hit whitening blink on a foe sprite: started for every landed
/// blow (RM2000 `SetBlinkTimer`) independent of the animation's own flash. It
/// brightens the sprite toward white, decaying over `secs`.
#[derive(Component)]
struct BattlerBlink {
    elapsed: f32,
    secs: f32,
}

/// A foe's placement `(x, y)` on the RM2000 320×240 backdrop re-centred to a
/// screen offset from centre (y downward) — the same point `resolve::foe_anim_pos`
/// uses, so an animation and its target flash land on this battler's centre.
fn battler_base(x: u32, y: u32) -> Vec2 {
    Vec2::new(x as f32 - 160.0, y as f32 - 120.0)
}

/// Rebuild the backdrop and battler sprites when a new fight starts (keyed on the
/// per-fight generation stamp), and clear them when it ends.
fn sync_scene(
    mut commands: Commands,
    battle: Res<Battle>,
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
        Sprite {
            image: asset_server.load(resolve_png("Backdrop", &battle.background)),
            custom_size: Some(Vec2::new(320.0, 240.0)),
            ..default()
        },
        Transform::from_translation(overlay_translation(Vec2::ZERO, BACKDROP_Z)),
        overlay_layer(),
        SceneEntity,
    ));
    for (index, foe) in battle.enemies.iter().enumerate() {
        let base = battler_base(foe.x, foe.y);
        commands.spawn((
            Sprite {
                image: asset_server.load(resolve_png("Monster", &foe.battler)),
                color: battler_look(foe.alive(), None, false).0,
                ..default()
            },
            Transform::from_translation(overlay_translation(base, BATTLER_Z + index as f32 * 0.1)),
            overlay_layer(),
            SceneEntity,
            Battler {
                index,
                base,
                height: FALLBACK_BATTLER_HEIGHT,
            },
        ));
    }
}

/// Update each battler's measured pixel height once its image has loaded, so an
/// animation's `position` anchor (feet/head) shifts by the real half-height
/// rather than the fallback. Runs every frame; the height settles as soon as the
/// asset is ready and is otherwise left at [`FALLBACK_BATTLER_HEIGHT`].
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

/// The pixel height of the battler whose base matches `pos` (an animation's
/// target centre), for the animation's `position` anchor offset. `None` when no
/// battler sits there — a party-area target, which the caller heights with a
/// sensible default.
pub(super) fn battler_height_at(battlers: &Query<&Battler>, pos: Vec2) -> Option<f32> {
    battlers
        .iter()
        .find(|b| b.base.distance_squared(pos) < FLASH_MATCH_EPS)
        .map(|b| b.height)
}

/// Route each [`BattlerFlash`] to the battler at its `pos`, starting a decaying
/// tint on that sprite. A party-area flash (RM2000 front view draws no party
/// sprites) matches no battler and is dropped — the correct no-op.
fn apply_battler_flash(
    mut commands: Commands,
    mut flashes: MessageReader<BattlerFlash>,
    battlers: Query<(Entity, &Battler)>,
) {
    for flash in flashes.read() {
        if let Some((entity, _)) = battlers
            .iter()
            .find(|(_, b)| b.base.distance_squared(flash.pos) < FLASH_MATCH_EPS)
        {
            commands.entity(entity).insert(BattlerTint {
                elapsed: 0.0,
                secs: flash.secs,
                rgb: flash.rgb,
                power: flash.power,
            });
        }
    }
}

/// Advance any in-progress foe death/explosion in real time (so the held beat
/// plays out) and turn each queued per-hit blink into a [`BattlerBlink`] on its
/// foe sprite. Guarded so an idle fight never marks [`Battle`] changed.
fn drive_battlers(
    time: Res<Time>,
    mut battle: ResMut<Battle>,
    mut commands: Commands,
    battlers: Query<(Entity, &Battler)>,
) {
    let dying = battle.death_in_progress();
    let has_blinks = !battle.pending_blinks.is_empty();
    if !dying && !has_blinks {
        return;
    }
    if dying {
        battle.advance_deaths(time.delta_secs());
    }
    for pos in std::mem::take(&mut battle.pending_blinks) {
        let target = Vec2::new(pos.0, pos.1);
        if let Some((entity, _)) = battlers
            .iter()
            .find(|(_, b)| b.base.distance_squared(target) < FLASH_MATCH_EPS)
        {
            commands.entity(entity).insert(BattlerBlink {
                elapsed: 0.0,
                secs: BLINK_SECS,
            });
        }
    }
}

/// Paint every battler each frame: its base look (natural / targeted / dying), the
/// decaying animation flash blended over it ([`BattlerTint`]), and the guaranteed
/// per-hit brighten ([`BattlerBlink`]) added on top — plus the death/explosion
/// zoom on the sprite's scale. Spent tints and blinks are removed.
#[allow(clippy::type_complexity)]
fn paint_battlers(
    time: Res<Time>,
    battle: Res<Battle>,
    mut commands: Commands,
    mut battlers: Query<(
        Entity,
        &Battler,
        &mut Sprite,
        &mut Transform,
        Option<&mut BattlerTint>,
        Option<&mut BattlerBlink>,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, battler, mut sprite, mut transform, tint, blink) in &mut battlers {
        let foe = battle.enemies.get(battler.index);
        let alive = foe.is_some_and(|f| f.alive());
        let dying = foe.and_then(|f| f.dying.as_ref());
        let (base, zoom) = battler_look(alive, dying, targeted(&battle, battler.index));
        let mut color = match tint {
            Some(mut tint) => {
                tint.elapsed += dt;
                if tint.elapsed >= tint.secs {
                    commands.entity(entity).remove::<BattlerTint>();
                    base
                } else {
                    let amount = tint.power * (1.0 - tint.elapsed / tint.secs);
                    blend(base, tint.rgb, amount)
                }
            }
            None => base,
        };
        if let Some(mut blink) = blink {
            blink.elapsed += dt;
            if blink.elapsed >= blink.secs {
                commands.entity(entity).remove::<BattlerBlink>();
            } else {
                let amount = BLINK_BRIGHTEN * (1.0 - blink.elapsed / blink.secs);
                color = brighten(color, amount);
            }
        }
        if sprite.color != color {
            sprite.color = color;
        }
        let scale = Vec3::splat(zoom);
        if transform.scale != scale {
            transform.scale = scale;
        }
    }
}

/// Whether the target cursor currently rests on enemy `index`.
fn targeted(battle: &Battle, index: usize) -> bool {
    if battle.phase != Phase::Command || battle.menu != MenuLevel::Target {
        return false;
    }
    let living = battle.living_enemies();
    living.get(battle.cursor.min(living.len().saturating_sub(1))) == Some(&index)
}

/// A battler sprite's base colour and zoom this frame: a living foe shows its
/// natural colours (a warm gold glow while it's the current target); a foe playing
/// its RM2000 death-out fades from full opacity (a self-destruct also zooms out as
/// it fades); any other downed foe is gone (fully transparent). The animation
/// flash and the per-hit blink are layered over this by [`paint_battlers`].
fn battler_look(alive: bool, dying: Option<&Dying>, targeted: bool) -> (Color, f32) {
    if alive {
        let color = if targeted {
            Color::srgb(1.0, 0.9, 0.55)
        } else {
            Color::WHITE
        };
        return (color, 1.0);
    }
    match dying {
        // RM2000 `Sprite_Enemy::Draw`: death alpha `7 * dt` (dt 36 -> 0), explode
        // alpha `12 * et` with zoom `(20 - et) / 20 + 1` (et 20 -> 0), each timer
        // counted down; here `elapsed / secs` runs 0 -> 1 in their place.
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

/// `base` blended toward `rgb` by `amount` (0 = base, 1 = full `rgb`), keeping
/// `base`'s own alpha. Used for an animation's colour flash.
fn blend(base: Color, rgb: [f32; 3], amount: f32) -> Color {
    let b = base.to_srgba();
    Color::srgba(
        b.red + (rgb[0] - b.red) * amount,
        b.green + (rgb[1] - b.green) * amount,
        b.blue + (rgb[2] - b.blue) * amount,
        b.alpha,
    )
}

/// `color` brightened toward white by `amount` added to each channel (a hit
/// blink), keeping its alpha so a fading death sprite still flashes without going
/// opaque. A multiply tint can't whiten a white sprite, so this adds instead.
fn brighten(color: Color, amount: f32) -> Color {
    let c = color.to_srgba();
    Color::srgba(c.red + amount, c.green + amount, c.blue + amount, c.alpha)
}

/// Register the battle scene systems: rebuild the backdrop + battlers per fight,
/// advance deaths and route per-hit blinks and target flashes onto them, and paint
/// the battlers each frame.
pub fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            sync_scene,
            measure_battlers,
            drive_battlers,
            apply_battler_flash,
            paint_battlers,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battler_base_and_translation_map_a_centre_to_the_overlay() {
        // A foe centred at RM2000 (176, 96) sits at screen offset (16, -24), which
        // the origin-fixed overlay draws at world (16, 24).
        let base = battler_base(176, 96);
        assert_eq!(base, Vec2::new(16.0, -24.0));
        assert_eq!(
            overlay_translation(base, BATTLER_Z),
            Vec3::new(16.0, 24.0, BATTLER_Z)
        );
    }

    #[test]
    fn battler_look_covers_alive_targeted_and_death_states() {
        // A living foe is white, or warm gold while it is the current target.
        assert_eq!(battler_look(true, None, false), (Color::WHITE, 1.0));
        assert_eq!(
            battler_look(true, None, true).0,
            Color::srgb(1.0, 0.9, 0.55)
        );
        // A downed foe with no death-out is gone (fully transparent).
        assert_eq!(battler_look(false, None, false).0.alpha(), 0.0);
        // A fresh death-out starts near full opacity and holds its scale.
        let death = Dying {
            elapsed: 0.0,
            secs: 0.6,
            explode: false,
        };
        let (color, zoom) = battler_look(false, Some(&death), false);
        assert!(color.alpha() > 0.9);
        assert_eq!(zoom, 1.0);
        // A finished self-destruct has zoomed out and faded away.
        let boom = Dying {
            elapsed: 0.4,
            secs: 0.4,
            explode: true,
        };
        let (color, zoom) = battler_look(false, Some(&boom), false);
        assert!(zoom > 1.9);
        assert!(color.alpha() < 0.01);
    }

    #[test]
    fn brighten_lifts_channels_and_keeps_alpha() {
        // A white sprite can't be whitened by a multiply tint, so the blink adds.
        let lifted = brighten(Color::srgba(0.4, 0.4, 0.4, 0.7), 0.5);
        let c = lifted.to_srgba();
        assert!((c.red - 0.9).abs() < 1e-6);
        assert!((c.alpha - 0.7).abs() < 1e-6);
    }

    #[test]
    fn a_target_flash_starts_a_tint_on_the_matching_battler() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<BattlerFlash>();
        app.add_systems(Update, apply_battler_flash);
        let base = battler_base(176, 96);
        let entity = app
            .world_mut()
            .spawn(Battler {
                index: 0,
                base,
                height: FALLBACK_BATTLER_HEIGHT,
            })
            .id();
        app.world_mut().write_message(BattlerFlash {
            pos: base,
            rgb: [1.0, 0.6, 0.3],
            power: 1.0,
            secs: 0.2,
        });
        app.update();
        assert!(app.world().entity(entity).get::<BattlerTint>().is_some());
    }

    #[test]
    fn a_party_area_flash_tints_no_battler() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<BattlerFlash>();
        app.add_systems(Update, apply_battler_flash);
        let base = battler_base(176, 96);
        let entity = app
            .world_mut()
            .spawn(Battler {
                index: 0,
                base,
                height: FALLBACK_BATTLER_HEIGHT,
            })
            .id();
        // The party area (below centre, no battler there) matches nothing.
        app.world_mut().write_message(BattlerFlash {
            pos: Vec2::new(0.0, 80.0),
            rgb: [1.0, 1.0, 1.0],
            power: 1.0,
            secs: 0.2,
        });
        app.update();
        assert!(app.world().entity(entity).get::<BattlerTint>().is_none());
    }
}
