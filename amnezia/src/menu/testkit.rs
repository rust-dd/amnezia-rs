//! Shared menu fixtures; the default actor has no curves or gear to exercise fallbacks.

use crate::gamedata::GameData;
use amnezia_data::{ActorDef, ItemDef, SkillDef};

pub(super) const ITEM_HERB: u32 = 5;

/// A bare level-2 hero (Ron), empty stat curves so derivations fall back to the
/// starting HP/SP and battle's linear stat formula, no equipment.
pub(super) fn actor() -> ActorDef {
    ActorDef {
        character_name: String::new(),
        character_index: 0,
        rename_skill: false,
        skill_name: String::new(),
        critical_hit: false,
        critical_hit_chance: 30,
        state_ranks: Vec::new(),
        attribute_ranks: Vec::new(),
        id: 1,
        name: "Ron".into(),
        title: "Zsoldos".into(),
        level: 2,
        max_level: 50,
        hp: 63,
        sp: 37,
        curves: Default::default(),
        learnings: Vec::new(),
        exp_base: 30,
        exp_inflation: 30,
        exp_correction: 0,
        weapon: 0,
        shield: 0,
        armor: 0,
        helmet: 0,
        accessory: 0,
        two_weapons: false,
        fix_equipment: false,
        unarmed_animation: 0,
        face_name: "Ron".into(),
        face_index: 6,
    }
}

/// The healing item (id [`ITEM_HERB`]): restores 20 HP flat plus 10% of max HP.
pub(super) fn herb() -> ItemDef {
    let mut item = blank_item(ITEM_HERB, 6);
    item.name = "Gyógyfű".into();
    item.recover_hp = 20;
    item.recover_hp_rate = 10;
    item
}

pub(super) fn weapon(id: u32, name: &str, atk: u32) -> ItemDef {
    let mut item = blank_item(id, 1);
    item.name = name.into();
    item.atk = atk;
    item
}

/// An item with every effect field zeroed, ready to specialise.
pub(super) fn blank_item(id: u32, item_type: u32) -> ItemDef {
    ItemDef {
        prevent_critical: false,
        raise_evasion: false,
        half_sp_cost: false,
        actor_set: Vec::new(),
        state_chance: 0,
        id,
        name: "Tárgy".into(),
        description: String::new(),
        item_type,
        price: 0,
        recover_hp: 0,
        recover_hp_rate: 0,
        recover_sp: 0,
        recover_sp_rate: 0,
        cure_states: vec![],
        scope: 0,
        only_field: false,
        ko_only: false,
        uses: 0,
        atk: 0,
        def: 0,
        spi: 0,
        agi: 0,
        attribute_defense: vec![],
        state_defense: vec![],
        two_handed: false,
        hit: 0,
        crit: 0,
        weapon_animation: 0,
    }
}

/// A plain single-enemy attack skill (scope `0`, `skill_type` 0): never
/// field-usable, so the skill tests can assert the greyed/inert branch.
pub(super) fn skill(id: u32, name: &str, sp_cost: u32) -> SkillDef {
    SkillDef {
        using_message1: String::new(),
        using_message2: String::new(),
        affect_stats: [false; 4],
        ignore_defense: false,
        id,
        name: name.into(),
        description: String::new(),
        sp_cost,
        power: 50,
        hit: 0,
        skill_type: 0,
        failure_message: 0,
        scope: 0,
        animation_id: 0,
        physical_rate: 0,
        magical_rate: 3,
        variance: 4,
        affect_hp: true,
        affect_sp: false,
        absorb: false,
        attributes: vec![],
        affected_states: vec![],
    }
}

/// A flat, variance-free ally heal, matching the original Gyógyító dallam.
pub(super) fn heal_skill(id: u32, name: &str, sp_cost: u32, power: u32) -> SkillDef {
    let mut s = skill(id, name, sp_cost);
    s.scope = 3;
    s.power = power;
    s.magical_rate = 0;
    s.variance = 0;
    s
}

/// The default database: the hero, the healing item, and one attack skill.
pub(super) fn data() -> GameData {
    GameData {
        actors: vec![actor()],
        items: vec![herb()],
        skills: vec![skill(1, "X-Csapás", 20)],
    }
}
