//! Assembling a fresh encounter: [`Battle::build`] instantiates both sides.

use super::battle::RESOLVE_STEP_SECS;
use super::{Battle, Fighter, Foe, Phase, Progression, Stats, Vitals, logic};
use amnezia_data::{ActorDef, AttributeDef, ItemDef, MonsterDef, SkillDef, StateDef, TroopDef};
use bevy::prelude::*;

impl Battle {
    /// Assemble a fresh encounter: instantiate each troop member as a live
    /// [`Foe`], each party actor as a live [`Fighter`] (HP/SP from `vitals`, or
    /// full on a first fight), and enter the command phase.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        troop: &TroopDef,
        monsters: &[MonsterDef],
        actors: &[&ActorDef],
        equipped: &[[u32; 5]],
        items: &[ItemDef],
        attributes: &[AttributeDef],
        states: &[StateDef],
        skills: &[SkillDef],
        vitals: &Vitals,
        progression: &Progression,
        background: String,
        seed: u64,
    ) -> Self {
        let enemies: Vec<Foe> = troop
            .members
            .iter()
            .filter_map(|m| {
                monsters.iter().find(|d| d.id == m.enemy_id).map(|d| Foe {
                    name: d.name.clone(),
                    battler: d.battler.clone(),
                    hp: d.max_hp as i32,
                    max_hp: d.max_hp as i32,
                    sp: d.max_sp as i32,
                    max_sp: d.max_sp as i32,
                    stats: Stats::from_monster(d),
                    exp: d.exp,
                    gold: d.gold,
                    x: m.x,
                    y: m.y,
                    attribute_ranks: d.attribute_ranks.clone(),
                    state_ranks: d.state_ranks.clone(),
                    states: Vec::new(),
                    defending: false,
                    fled: false,
                    charging: false,
                    actions: d.actions.clone(),
                    switch_on_after_action: None,
                    switch_off_after_action: None,
                    dying: None,
                })
            })
            .collect();
        let members = actors
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                let slots = equipped.get(idx).copied().unwrap_or([
                    a.weapon,
                    a.shield,
                    a.armor,
                    a.helmet,
                    a.accessory,
                ]);
                Fighter::build(a, slots, items, vitals, progression)
            })
            .collect::<Vec<_>>();
        // The RM2000 escape chance is fixed at battle start from the two sides'
        // AVERAGE agilities (EasyRPG `InitEscapeChance`), then only nudged by +10
        // per failed attempt — never recomputed as combatants fall.
        let party_avg =
            logic::average_agility(&members.iter().map(|f| f.stats.agility).collect::<Vec<_>>());
        let enemy_avg =
            logic::average_agility(&enemies.iter().map(|e| e.stats.agility).collect::<Vec<_>>());
        Battle {
            events: crate::battle::events::BattleEvents::new(&troop.pages),
            phase: Phase::PartyCommand,
            background,
            allow_escape: true,
            members,
            enemies,
            attributes: attributes.to_vec(),
            states: states.to_vec(),
            skills: skills.to_vec(),
            items: items.to_vec(),
            timer: Timer::from_seconds(RESOLVE_STEP_SECS, TimerMode::Repeating),
            log: vec![format!("{} rátok támad!", troop.name)],
            rng: seed | 1,
            generation: seed | 1,
            round: 1,
            escape_chance: logic::init_escape_chance(party_avg, enemy_avg),
            ..default()
        }
    }
}

impl Fighter {
    pub(in crate::battle) fn build(
        actor: &ActorDef,
        slots: [u32; 5],
        items: &[ItemDef],
        vitals: &Vitals,
        progression: &Progression,
    ) -> Self {
        let level = progression.level(actor);
        let (max_hp, max_sp) = logic::actor_hp_sp_at(&actor.curves, level, actor.hp, actor.sp);
        let (max_hp, max_sp) = (max_hp as i32, max_sp as i32);
        let (hp, sp) = vitals.get_stored(actor.id).unwrap_or((max_hp, max_sp));
        let mut stats = logic::actor_stats_at(&actor.curves, level);
        let bonus = logic::equipment_bonus_slots(slots, items);
        stats.attack += bonus.attack;
        stats.defense += bonus.defense;
        stats.spirit += bonus.spirit;
        stats.agility += bonus.agility;
        let weapon = items.iter().find(|i| i.id == slots[0]);
        Self {
            actor_id: actor.id,
            level,
            name: actor.name.clone(),
            hp: hp.clamp(0, max_hp),
            max_hp,
            sp: sp.clamp(0, max_sp),
            max_sp,
            stats,
            defending: false,
            command: None,
            weapon_hit: logic::effective_hit(weapon.map(|w| w.hit)),
            weapon_crit: weapon.map_or(0, |w| w.crit),
            weapon_element: weapon.and_then(|w| w.attribute_defense.first().copied()),
            attack_animation: weapon.map_or(actor.unarmed_animation, |w| w.weapon_animation),
            states: vitals
                .states(actor.id)
                .into_iter()
                .map(|id| (id, 0))
                .collect(),
            state_ranks: actor.state_ranks.clone(),
            state_guards: logic::equipment_state_guards(slots, items),
            resist_attributes: logic::equipment_resist_slots(slots, items),
            known_skills: progression.known_skill_ids(actor),
        }
    }
}
