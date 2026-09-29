//! Troop pages run once per turn, at command-entry and action boundaries.

mod actors;
mod commands;
mod conditions;
#[cfg(test)]
mod tests;

use super::model::{Battle, Phase};
use crate::animation::ActiveAnimations;
use crate::audio::AudioRequest;
use crate::dialogue::Dialogue;
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::{Party, Switches, Variables};
use crate::vitals::Vitals;
use amnezia_data::TroopPageDef;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(Default)]
pub(in crate::battle) struct BattleEvents {
    pages: Vec<TroopPageDef>,
    executed: Vec<bool>,
    page: Option<usize>,
    ip: usize,
    pub turn: u32,
    checkpoint: bool,
    blocked_frame: bool,
    wait: f32,
    wait_animation: bool,
}

impl BattleEvents {
    pub fn new(pages: &[TroopPageDef]) -> Self {
        Self {
            pages: pages.to_vec(),
            executed: vec![false; pages.len()],
            checkpoint: !pages.is_empty(),
            ..default()
        }
    }

    pub fn blocks_action(&self) -> bool {
        self.checkpoint || self.page.is_some() || self.blocked_frame
    }

    pub fn holds_resolution(&self) -> bool {
        self.checkpoint || self.page.is_some()
    }

    pub fn presenting(&self) -> bool {
        self.page.is_some() || self.blocked_frame
    }

    pub fn check_pages(&mut self) {
        self.checkpoint = !self.pages.is_empty();
    }

    pub fn next_turn(&mut self) {
        self.turn += 1;
        self.executed.fill(false);
        self.check_pages();
    }

    fn schedule(&mut self, battle: &Battle, world: &EventWorld) -> bool {
        self.page = self.pages.iter().enumerate().position(|(index, page)| {
            !self.executed[index] && conditions::matches(&page.condition, self.turn, battle, world)
        });
        if let Some(index) = self.page {
            self.executed[index] = true;
            self.ip = 0;
            true
        } else {
            self.checkpoint = false;
            false
        }
    }
}

#[derive(SystemParam)]
pub(super) struct EventWorld<'w> {
    data: Res<'w, GameData>,
    equipment: Res<'w, Equipment>,
    party: ResMut<'w, Party>,
    progression: ResMut<'w, Progression>,
    vitals: ResMut<'w, Vitals>,
    dialogue: ResMut<'w, Dialogue>,
    switches: ResMut<'w, Switches>,
    variables: Res<'w, Variables>,
    audio: MessageWriter<'w, AudioRequest>,
}

impl EventWorld<'_> {
    pub(super) fn action_boundary(&mut self, battle: &mut Battle) -> bool {
        for (id, enabled) in battle.pending_switches.drain(..) {
            self.switches.set(id, enabled);
        }
        let mut events = std::mem::take(&mut battle.events);
        let scheduled = events.schedule(battle, self);
        battle.events = events;
        scheduled
    }

    pub(super) fn outcome_music(
        &mut self,
        system: &amnezia_data::SystemDef,
        overrides: Option<&crate::system_bgm::SystemBgm>,
        outcome: super::BattleOutcome,
    ) {
        super::systems::play_outcome_music(&mut self.audio, system, overrides, outcome);
    }
}

pub(super) fn drive(
    time: Res<Time>,
    animation: Option<Res<ActiveAnimations>>,
    battle_data: Res<super::BattleData>,
    system_bgm: Option<Res<crate::system_bgm::SystemBgm>>,
    mut battle: ResMut<Battle>,
    mut world: EventWorld,
) {
    if battle.phase != Phase::Inactive && (world.switches.is_changed() || battle.is_changed()) {
        let switches = world
            .switches
            .entries()
            .into_iter()
            .filter_map(|(id, enabled)| enabled.then_some(id))
            .collect();
        if battle.ai_switches != switches {
            battle.ai_switches = switches;
        }
    }
    if matches!(
        battle.phase,
        Phase::Inactive | Phase::Encounter | Phase::Escape | Phase::Outcome
    ) || !battle.events.blocks_action()
    {
        return;
    }
    let mut events = std::mem::take(&mut battle.events);
    events.blocked_frame = false;
    if events.page.is_some() {
        events.blocked_frame = true;
        if world.dialogue.active {
            battle.events = events;
            return;
        }
        if events.wait > 0.0 {
            events.wait = (events.wait - time.delta_secs()).max(0.0);
            battle.events = events;
            return;
        }
        if events.wait_animation {
            if battle.tick_anim_hold(animation.is_some_and(|a| a.battle > 0)) {
                battle.events = events;
                return;
            }
            events.wait_animation = false;
        }
    }
    for _ in 0..1000 {
        if events.page.is_none() {
            if battle.death_in_progress() {
                break;
            }
            if let Some(outcome) = battle.end_state() {
                battle.finish(outcome);
                super::systems::play_outcome_music(
                    &mut world.audio,
                    &battle_data.system,
                    system_bgm.as_deref(),
                    outcome,
                );
                break;
            }
            if events.checkpoint {
                events.schedule(&battle, &world);
            }
            break;
        }
        events.blocked_frame = true;
        if !commands::step(&mut events, &mut battle, &mut world) {
            break;
        }
    }
    battle.events = events;
}

pub(super) fn sync_switches(mut battle: ResMut<Battle>, mut switches: ResMut<Switches>) {
    if battle.pending_switches.is_empty() {
        return;
    }
    for (id, enabled) in battle.pending_switches.drain(..) {
        switches.set(id, enabled);
    }
}
