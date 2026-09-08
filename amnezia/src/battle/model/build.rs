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
                    dying: None,
                })
            })
            .collect();
        let members: Vec<Fighter> = actors
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                let level = progression.level(a);
                let (max_hp, max_sp) = logic::actor_hp_sp_at(&a.curves, level, a.hp, a.sp);
                let (max_hp, max_sp) = (max_hp as i32, max_sp as i32);
                // Resume stored HP/SP (clamped to the level's maxima), or start
                // full at the current level when this actor has no stored vitals.
                let (hp, sp) = match vitals.get_stored(a.id) {
                    Some((h, s)) => (h.min(max_hp), s.min(max_sp)),
                    None => (max_hp, max_sp),
                };
                // The runtime loadout (`Equipment` resource), resolved per member
                // at the call site; an out-of-range member falls back to the
                // actor's starting gear so a bare test caller stays correct.
                let slots = equipped.get(idx).copied().unwrap_or([
                    a.weapon,
                    a.shield,
                    a.armor,
                    a.helmet,
                    a.accessory,
                ]);
                let mut stats = logic::actor_stats_at(&a.curves, level);
                let bonus = logic::equipment_bonus_slots(slots, items);
                stats.attack += bonus.attack;
                stats.defense += bonus.defense;
                stats.spirit += bonus.spirit;
                stats.agility += bonus.agility;
                // The equipped weapon (slot 0) lends its hit, crit, and first
                // element for later resolution; id `0` matches no real item, so an
                // empty weapon slot resolves to `None`.
                let weapon = items.iter().find(|i| i.id == slots[0]);
                Fighter {
                    actor_id: a.id,
                    name: a.name.clone(),
                    hp,
                    max_hp,
                    sp,
                    max_sp,
                    stats,
                    defending: false,
                    command: None,
                    weapon_hit: weapon.map_or(0, |w| w.hit),
                    weapon_crit: weapon.map_or(0, |w| w.crit),
                    weapon_element: weapon.and_then(|w| w.attribute_defense.first().copied()),
                    // A weapon animates with its own `weapon_animation`; an empty
                    // slot falls back to the actor's bare-handed animation.
                    attack_animation: match weapon {
                        Some(w) => w.weapon_animation,
                        None => a.unarmed_animation,
                    },
                    states: Vec::new(),
                    resist_attributes: logic::equipment_resist_slots(slots, items),
                    known_skills: progression.known_skill_ids(a),
                }
            })
            .collect();
        // The RM2000 escape chance is fixed at battle start from the two sides'
        // AVERAGE agilities (EasyRPG `InitEscapeChance`), then only nudged by +10
        // per failed attempt — never recomputed as combatants fall.
        let party_avg =
            logic::average_agility(&members.iter().map(|f| f.stats.agility).collect::<Vec<_>>());
        let enemy_avg =
            logic::average_agility(&enemies.iter().map(|e| e.stats.agility).collect::<Vec<_>>());
        Battle {
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
