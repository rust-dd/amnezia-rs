use super::*;
use crate::animation::{BattlerFlash, flash_power_level};
use crate::battle::model::{MenuLevel, Source};
use crate::legacy_colors::flash::SpriteFlash;
use crate::timing::GameFrames;

#[derive(Clone, Copy)]
enum Flash {
    Selection { remaining: u32 },
    Action { remaining: u32 },
    Animation { rgb: [u8; 3], power: u32, age: u32 },
}

impl Flash {
    fn advance(&mut self, delta: u32) {
        match self {
            Self::Selection { remaining } | Self::Action { remaining } => {
                *remaining = remaining.saturating_sub(delta);
            }
            Self::Animation { age, .. } => *age = age.saturating_add(delta),
        }
    }

    fn color(self) -> [u8; 4] {
        match self {
            Self::Selection { remaining } => [248, 248, 248, (remaining * 12) as u8],
            Self::Action { remaining } => [248, 248, 248, (remaining * 8) as u8],
            Self::Animation { rgb, power, age } if age <= 10 => [
                rgb[0],
                rgb[1],
                rgb[2],
                (flash_power_level(age, power) * 8) as u8,
            ],
            _ => [0; 4],
        }
    }
}

#[derive(Component, Default)]
pub(super) struct Effects {
    flash: Option<Flash>,
    blink: u32,
}

impl Effects {
    fn advance(&mut self, delta: u32) {
        self.blink = self.blink.saturating_sub(delta);
        if let Some(flash) = &mut self.flash {
            flash.advance(delta);
            if flash.color()[3] == 0 {
                self.flash = None;
            }
        }
    }
}

#[derive(Resource, Default)]
struct Clock {
    generation: u64,
    last: Option<u32>,
    selection: u32,
}

impl Clock {
    fn advance(&mut self, frame: u32, generation: u64, paused: bool) -> u32 {
        if self.generation != generation {
            *self = Self {
                generation,
                ..default()
            };
        }
        let delta = self
            .last
            .replace(frame)
            .map_or(0, |last| frame.wrapping_sub(last));
        if paused { 0 } else { delta }
    }

    fn select(&mut self, battle: &Battle, delta: u32) -> Option<(usize, u32)> {
        if battle.phase != Phase::Command || battle.menu != MenuLevel::Target {
            self.selection = 0;
            return None;
        }
        if battle.events.blocks_action() {
            return None;
        }
        let total = u64::from(self.selection) + u64::from(delta);
        self.selection = (total % 60) as u32;
        if total < 60 || self.selection >= 16 {
            return None;
        }
        let living = battle.living_enemies();
        living
            .get(battle.cursor.min(living.len().saturating_sub(1)))
            .map(|&index| (index, 16 - self.selection))
    }
}

fn step(
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    mut clock: ResMut<Clock>,
    mut battle: ResMut<Battle>,
    mut messages: MessageReader<BattlerFlash>,
    mut battlers: Query<(&Battler, &mut Effects)>,
) {
    let paused = pause.paused() || battle.phase == Phase::Inactive;
    let delta = clock.advance(frames.frame, battle.generation, paused);
    if paused {
        return;
    }
    for (_, mut effects) in &mut battlers {
        effects.advance(delta);
    }
    for source in std::mem::take(&mut battle.pending_action_flashes) {
        if let Source::Enemy(index) = source
            && let Some((_, mut effects)) = battlers.iter_mut().find(|(b, _)| b.index == index)
        {
            effects.flash = Some(Flash::Action { remaining: 10 });
        }
    }
    for message in messages.read() {
        if let Some((_, mut effects)) = battlers
            .iter_mut()
            .find(|(battler, _)| battler.base.distance_squared(message.pos) < FLASH_MATCH_EPS)
        {
            effects.flash = Some(Flash::Animation {
                rgb: message.rgb,
                power: message.power,
                age: message.age,
            });
        }
    }
    if let Some((target, remaining)) = clock.select(&battle, delta)
        && let Some((_, mut effects)) = battlers.iter_mut().find(|(b, _)| b.index == target)
    {
        effects.flash = Some(Flash::Selection { remaining });
    }
    if !battle.pending_blinks.is_empty() {
        for (x, y) in std::mem::take(&mut battle.pending_blinks) {
            if let Some((_, mut effects)) = battlers.iter_mut().find(|(battler, _)| {
                battler.base.distance_squared(Vec2::new(x, y)) < FLASH_MATCH_EPS
            }) {
                effects.blink = 20;
            }
        }
    }
}

fn paint(
    battle: Res<Battle>,
    mut battlers: Query<(
        &Battler,
        &Effects,
        &mut Sprite,
        &mut Transform,
        &mut SpriteFlash,
    )>,
) {
    for (battler, effects, mut sprite, mut transform, mut flash) in &mut battlers {
        let foe = battle.enemies.get(battler.index);
        let (mut color, zoom) = battler_look(
            foe.is_some_and(|foe| foe.alive()),
            foe.and_then(|foe| foe.dying.as_ref()),
        );
        if effects.blink % 10 >= 5 {
            color.set_alpha(0.0);
        }
        if sprite.color != color {
            sprite.color = color;
        }
        if transform.scale != Vec3::splat(zoom) {
            transform.scale = Vec3::splat(zoom);
        }
        flash.set_if_neq(SpriteFlash(effects.flash.map_or([0; 4], Flash::color)));
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<Clock>().add_systems(
        PostUpdate,
        paint
            .after(step)
            .in_set(EffectsSet)
            .before(bevy::transform::TransformSystems::Propagate)
            .before(crate::legacy_colors::hue::HueSet),
    );
    crate::timing::logical::post(app, || {
        step.in_set(EffectsSet)
            .before(bevy::transform::TransformSystems::Propagate)
            .before(crate::legacy_colors::hue::HueSet)
    });
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EffectsSet;

#[cfg(test)]
mod tests;
