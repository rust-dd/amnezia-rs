//! The front-view battle scene as world sprites: a full-screen backdrop and one
//! battler per enemy, drawn on the shared effect overlay (render layer 1) so they
//! sit in the same coordinate space as the effect animations — a battler's centre
//! is exactly the point [`super::resolve`] plays a hit animation on, so effects
//! land on their target by construction. The scene is rebuilt per fight (keyed on
//! [`Battle::generation`]) and despawned when the fight ends. The HUD windows live
//! in the sibling [`super::hud`] on a higher-order camera above this overlay.

use super::model::{Battle, MenuLevel, Phase};
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

/// A backdrop or battler sprite belonging to the live fight; despawned together
/// when the fight ends (or a new one starts).
#[derive(Component)]
struct SceneEntity;

/// An enemy battler sprite: its index into [`Battle::enemies`] and its RM2000
/// screen-offset base — the anchor animations play on and the point a target
/// flash is matched against.
#[derive(Component)]
struct Battler {
    index: usize,
    base: Vec2,
}

/// A running target-flash tint on a battler: `rgb`/`power` the flash colour and
/// peak strength, decaying back to the base tint over `secs`.
#[derive(Component)]
struct BattlerTint {
    elapsed: f32,
    secs: f32,
    rgb: [f32; 3],
    power: f32,
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
                color: enemy_tint(foe.alive(), false),
                ..default()
            },
            Transform::from_translation(overlay_translation(base, BATTLER_Z + index as f32 * 0.1)),
            overlay_layer(),
            SceneEntity,
            Battler { index, base },
        ));
    }
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

/// Paint every battler each frame: its base tint (dead / targeted / natural) and,
/// while a [`BattlerTint`] runs, the decaying flash blended over that base —
/// removing the tint and settling back to the base once it is spent.
fn paint_battlers(
    time: Res<Time>,
    battle: Res<Battle>,
    mut commands: Commands,
    mut battlers: Query<(Entity, &Battler, &mut Sprite, Option<&mut BattlerTint>)>,
) {
    let dt = time.delta_secs();
    for (entity, battler, mut sprite, tint) in &mut battlers {
        let alive = battle.enemies.get(battler.index).is_some_and(|f| f.alive());
        let base = enemy_tint(alive, targeted(&battle, battler.index));
        let color = match tint {
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
        if sprite.color != color {
            sprite.color = color;
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

/// The base tint of an enemy battler sprite: faded and dark once it's dead, a
/// warm gold glow while it's the current target, and its natural colours (white,
/// no tint) otherwise.
fn enemy_tint(alive: bool, targeted: bool) -> Color {
    if !alive {
        return Color::srgba(0.35, 0.35, 0.35, 0.5);
    }
    if targeted {
        return Color::srgb(1.0, 0.9, 0.55);
    }
    Color::WHITE
}

/// `base` blended toward `rgb` by `amount` (0 = base, 1 = full `rgb`), keeping
/// `base`'s own alpha.
fn blend(base: Color, rgb: [f32; 3], amount: f32) -> Color {
    let b = base.to_srgba();
    Color::srgba(
        b.red + (rgb[0] - b.red) * amount,
        b.green + (rgb[1] - b.green) * amount,
        b.blue + (rgb[2] - b.blue) * amount,
        b.alpha,
    )
}

/// Register the battle scene systems: rebuild the backdrop + battlers per fight,
/// route target flashes onto them, and paint the battlers each frame.
pub fn register(app: &mut App) {
    app.add_systems(Update, (sync_scene, apply_battler_flash, paint_battlers));
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
    fn enemy_tint_marks_dead_and_targeted_states() {
        assert_eq!(enemy_tint(true, false), Color::WHITE);
        assert_eq!(enemy_tint(true, true), Color::srgb(1.0, 0.9, 0.55));
        assert!(enemy_tint(false, false).alpha() < 1.0);
    }

    #[test]
    fn a_target_flash_starts_a_tint_on_the_matching_battler() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<BattlerFlash>();
        app.add_systems(Update, apply_battler_flash);
        let base = battler_base(176, 96);
        let entity = app.world_mut().spawn(Battler { index: 0, base }).id();
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
        let entity = app.world_mut().spawn(Battler { index: 0, base }).id();
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
