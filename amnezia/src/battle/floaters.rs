//! Floating battle numbers: the RM2000 damage/heal pop that rises off a struck
//! battler and fades. [`super::resolve`] collects a `PendingNumber` per landed hit
//! or heal (Bevy-free); this module drains them into overlay [`Text2d`] that rises
//! about half a tile and fades over ~half a second, tinted by kind (white damage,
//! green heal, pale Miss/0), and despawns each once it is spent. The text draws on
//! the shared effect overlay (render layer 1), just under the animation cells, so
//! a number sits over the battlers in the same screen space the hit landed in.

use super::model::{Battle, NumberKind};
use crate::animation::{overlay_layer, overlay_translation};
use crate::font::GameFont;
use bevy::prelude::*;
use bevy::text::FontSource;

/// Seconds a floating number rises and fades before it is removed — RM2000's brisk
/// half-second damage pop.
const NUMBER_SECS: f32 = 0.5;

/// How far up (RM2000 px) a number drifts over its life: about half a 16px tile.
const RISE_PX: f32 = 8.0;

/// World z of a floating number on the overlay: above the battlers (200) and the
/// screen flash (300), just under the animation cells (510) so a big effect can
/// still cover it, matching RM2000's number-behind-effect layering.
const NUMBER_Z: f32 = 500.0;

/// Point size of the number glyphs on the fixed 320×240 overlay (1 unit = 1 px).
const NUMBER_FONT_PX: f32 = 12.0;

/// A live floating number: `base` its RM2000 screen offset from centre (the point
/// it popped at) and `elapsed` its age, driving the rise and fade.
#[derive(Component)]
struct FloatingNumber {
    elapsed: f32,
    base: Vec2,
}

/// Drain [`Battle::pending_numbers`] into rising overlay text — one [`Text2d`] per
/// queued pop, at the target's screen position and tinted by kind. Guarded on
/// non-empty so an idle fight never marks [`Battle`] changed.
fn spawn_pending_numbers(mut commands: Commands, mut battle: ResMut<Battle>, font: Res<GameFont>) {
    if battle.pending_numbers.is_empty() {
        return;
    }
    for number in std::mem::take(&mut battle.pending_numbers) {
        let base = Vec2::new(number.pos.0, number.pos.1);
        commands.spawn((
            Text2d::new(number.text),
            TextFont {
                font: FontSource::Handle(font.0.clone()),
                font_size: FontSize::Px(NUMBER_FONT_PX),
                ..default()
            },
            TextColor(number_color(number.kind)),
            Transform::from_translation(overlay_translation(base, NUMBER_Z)),
            overlay_layer(),
            FloatingNumber { elapsed: 0.0, base },
            super::scene::SceneEntity,
        ));
    }
}

/// Rise each floating number by up to [`RISE_PX`] and fade its alpha to zero over
/// [`NUMBER_SECS`], despawning it once spent.
fn animate_floating_numbers(
    time: Res<Time>,
    mut commands: Commands,
    mut numbers: Query<(Entity, &mut FloatingNumber, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut number, mut transform, mut color) in &mut numbers {
        number.elapsed += dt;
        if number.elapsed >= NUMBER_SECS {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = number.elapsed / NUMBER_SECS;
        let risen = Vec2::new(number.base.x, number.base.y - RISE_PX * progress);
        transform.translation = overlay_translation(risen, NUMBER_Z);
        color.0 = color.0.with_alpha(1.0 - progress);
    }
}

/// The tint of a floating number by kind: white HP damage, green healing, and a
/// pale grey for a "Miss"/"0", mirroring RM2000's damage-pop colouring.
fn number_color(kind: NumberKind) -> Color {
    match kind {
        NumberKind::Damage => Color::WHITE,
        NumberKind::Heal => Color::srgb(0.45, 1.0, 0.45),
        NumberKind::Miss => Color::srgb(0.78, 0.78, 0.78),
    }
}

/// Register the floating-number systems: spawn queued pops and animate them.
pub fn register(app: &mut App) {
    app.add_systems(Update, (spawn_pending_numbers, animate_floating_numbers));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_colours_distinguish_damage_heal_and_miss() {
        assert_eq!(number_color(NumberKind::Damage), Color::WHITE);
        // Healing reads green; a miss is a neutral pale grey.
        let heal = number_color(NumberKind::Heal).to_srgba();
        assert!(heal.green > heal.red && heal.green > heal.blue);
        let miss = number_color(NumberKind::Miss).to_srgba();
        assert!((miss.red - miss.green).abs() < 1e-6 && miss.red < 1.0);
    }
}
