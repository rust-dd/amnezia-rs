//! Deterministic battle fixtures shared by the model, resolve, and input tests.

use super::*;

pub fn actor(id: u32, level: u32, hp: u32, sp: u32) -> ActorDef {
    ActorDef {
        id,
        name: format!("A{id}"),
        title: String::new(),
        level,
        max_level: 50,
        hp,
        sp,
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
        face_name: String::new(),
        face_index: 0,
    }
}

pub fn monster(id: u32, hp: u32, exp: u32, gold: u32) -> MonsterDef {
    MonsterDef {
        id,
        name: format!("M{id}"),
        battler: String::new(),
        max_hp: hp,
        max_sp: 9999,
        attack: 20,
        defense: 8,
        spirit: 0,
        agility: 8,
        exp,
        gold,
        attribute_ranks: vec![],
        state_ranks: vec![],
        actions: vec![amnezia_data::EnemyActionDef {
            basic: 0,
            ..Default::default()
        }],
    }
}

pub fn item(id: u32, atk: u32, def: u32, hit: u32, crit: u32, element: u32) -> ItemDef {
    ItemDef {
        id,
        name: String::new(),
        description: String::new(),
        item_type: 1,
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
        atk,
        def,
        spi: 0,
        agi: 0,
        attribute_defense: if element == 0 { vec![] } else { vec![element] },
        state_defense: vec![],
        two_handed: false,
        hit,
        crit,
        weapon_animation: 0,
    }
}

/// An actor's starting five-slot loadout, for a `Battle::build` test caller
/// that wants the runtime equipment to match the actor's `ActorDef` gear.
pub fn slots(a: &ActorDef) -> [u32; 5] {
    [a.weapon, a.shield, a.armor, a.helmet, a.accessory]
}

pub fn troop(members: &[(u32, u32, u32)]) -> TroopDef {
    TroopDef {
        id: 1,
        name: "T".into(),
        pages: Vec::new(),
        members: members
            .iter()
            .map(|&(enemy_id, x, y)| amnezia_data::TroopMemberDef { enemy_id, x, y })
            .collect(),
    }
}

/// One level-2 hero versus two 30-HP bandits, a deterministic fixture.
pub fn build_1v2() -> Battle {
    let monsters = vec![monster(1, 30, 10, 30)];
    let ron = actor(1, 2, 63, 37);
    let actors = vec![&ron];
    let equipped = [slots(&ron)];
    let troop = troop(&[(1, 100, 100), (1, 200, 100)]);
    Battle::build(
        &troop,
        &monsters,
        &actors,
        &equipped,
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        42,
    )
}

/// A two-member party (heroes 1 and 2) versus one 30-HP bandit, for the
/// ally-target selection tests.
pub fn build_party2() -> Battle {
    let ron = actor(1, 2, 63, 37);
    let tiff = actor(2, 3, 38, 75);
    let actors = vec![&ron, &tiff];
    let equipped = [slots(&ron), slots(&tiff)];
    let monsters = vec![monster(1, 30, 10, 30)];
    let troop = troop(&[(1, 100, 100)]);
    Battle::build(
        &troop,
        &monsters,
        &actors,
        &equipped,
        &[],
        &[],
        &[],
        &[],
        &Vitals::default(),
        &Progression::default(),
        "Cave1".into(),
        7,
    )
}
