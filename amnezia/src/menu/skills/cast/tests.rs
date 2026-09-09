use super::*;
use crate::assets::{asset_root, load_ron};

struct Field {
    data: GameData,
    party: Party,
    progression: Progression,
    equipment: Equipment,
    vitals: Vitals,
    rng: EventRng,
}

impl Field {
    fn original() -> Self {
        let data = GameData {
            actors: load_ron(&format!("{}/actors.ron", asset_root())),
            items: load_ron(&format!("{}/items.ron", asset_root())),
            skills: load_ron(&format!("{}/skills.ron", asset_root())),
        };
        let mut party = Party::default();
        party.restore(vec![1, 2, 5, 10]);
        let mut progression = Progression::default();
        let mut equipment = Equipment::default();
        for actor in &data.actors {
            progression.set_level(actor, 30);
            for slot in 0..5 {
                equipment.set_slot(actor, slot, 0);
            }
        }
        Self {
            data,
            party,
            progression,
            equipment,
            vitals: Vitals::default(),
            rng: EventRng::seeded(1),
        }
    }

    fn full(&self, id: u32) -> (i32, i32) {
        let actor = self.data.actor(id).unwrap();
        derive::max_hp_sp(actor, self.progression.level(actor))
    }

    fn cast(&mut self, caster: usize, target: usize, skill: u32) -> bool {
        apply_field_skill(
            caster,
            target,
            skill,
            &self.data,
            &self.party,
            &self.progression,
            &mut self.vitals,
            &self.equipment,
            &mut self.rng,
        )
    }
}

#[test]
fn field_cast_checks_required_weapon_attributes_before_healing_or_spending() {
    let mut field = Field::original();
    field
        .data
        .skills
        .iter_mut()
        .find(|skill| skill.id == 7)
        .unwrap()
        .attributes = vec![1];
    field.vitals.set(1, 1, field.full(1).1);
    assert!(!field.cast(0, 0, 7));
    assert_eq!(field.vitals.get_stored(1), Some((1, field.full(1).1)));
    field.equipment.set_slot(field.data.actor(1).unwrap(), 0, 1);
    assert!(field.cast(0, 0, 7));
    assert_eq!(field.vitals.get_stored(1).unwrap().0, 51);
}

#[test]
fn original_life_song_and_full_revival_restore_their_percentage_not_one_hp() {
    for (skill, percentage) in [(11, 200), (39, 999)] {
        let mut field = Field::original();
        let def = field.data.skills.iter().find(|s| s.id == skill).unwrap();
        assert_eq!(def.power, percentage);
        assert!(!def.affect_hp && def.affected_states.contains(&1));
        let cost = def.sp_cost as i32;
        let sp = field.full(2).1;
        field.vitals.set(1, 0, 0);
        assert!(field.cast(1, 0, skill));
        assert_eq!(field.vitals.get_stored(1).unwrap().0, field.full(1).0);
        assert_eq!(field.vitals.get_stored(2).unwrap().1, sp - cost);
    }
    let mut field = Field::original();
    field
        .data
        .skills
        .iter_mut()
        .find(|s| s.id == 11)
        .unwrap()
        .power = 25;
    field.vitals.set(1, 0, 0);
    assert!(field.cast(1, 0, 11));
    assert_eq!(field.vitals.get_stored(1).unwrap().0, field.full(1).0 / 4);
}

#[test]
fn original_aria_uses_each_targets_element_rank_and_pays_once_for_the_party() {
    let mut field = Field::original();
    for id in field.party.snapshot() {
        field.vitals.set(id, 1, field.full(id).1);
    }
    let sp = field.full(2).1;
    assert!(field.cast(1, usize::MAX, 9));
    for (id, amount) in [(1, 200), (2, 200), (5, 300), (10, 0)] {
        let context = CastContext {
            data: &field.data,
            progression: &field.progression,
            equipment: &field.equipment,
        };
        assert_eq!(
            context.amount(
                field.data.actor(2).unwrap(),
                field.data.actor(id).unwrap(),
                field
                    .data
                    .skills
                    .iter()
                    .find(|skill| skill.id == 9)
                    .unwrap(),
                &field.vitals,
                &mut field.rng
            ),
            amount
        );
        assert_eq!(
            field.vitals.get_stored(id).unwrap().0,
            (1 + amount).min(field.full(id).0)
        );
    }
    assert_eq!(field.vitals.get_stored(2).unwrap().1, sp - 60);
    let mut armor = crate::menu::testkit::blank_item(999, 3);
    armor.attribute_defense = vec![9];
    field.data.items.push(armor);
    field
        .equipment
        .set_slot(field.data.actor(1).unwrap(), 2, 999);
    field
        .equipment
        .set_slot(field.data.actor(1).unwrap(), 3, 999);
    field.vitals.set(1, 1, field.full(1).1);
    assert!(field.cast(1, 0, 9));
    assert_eq!(field.vitals.get_stored(1).unwrap().0, 101);
}

#[test]
fn full_formula_uses_equipment_and_inclusive_variance_without_field_hit_rolls() {
    let mut field = Field::original();
    field.data = crate::menu::testkit::data();
    field.data.actors[0].hp = 1000;
    field.progression = Progression::default();
    field.party.restore(vec![1]);
    let mut skill = crate::menu::testkit::heal_skill(2, "Heal", 5, 10);
    skill.physical_rate = 10;
    skill.magical_rate = 10;
    skill.variance = 4;
    skill.hit = 0;
    field.data.skills = vec![skill];
    let mut weapon = crate::menu::testkit::weapon(900, "Weapon", 20);
    weapon.spi = 40;
    field.data.items = vec![weapon];
    field.equipment.set_slot(&field.data.actors[0], 0, 900);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..=100 {
        field.rng = EventRng::seeded(seed);
        let roll = EventRng::seeded(seed).next_u64();
        field.vitals.set(1, 1, 37);
        assert!(field.cast(0, 0, 2));
        let expected = 38 + (roll % 19) as i32;
        assert_eq!(field.vitals.get_stored(1), Some((1 + expected, 32)));
        seen.insert(expected);
    }
    assert!(seen.contains(&38) && seen.contains(&56));
}

#[test]
fn self_and_team_revival_keep_targets_and_deduct_one_cost_after_recovery() {
    let mut field = Field::original();
    field.vitals.set(1, 1, field.full(1).1);
    let before = field.vitals.get_stored(1).unwrap();
    assert!(field.cast(0, usize::MAX, 49));
    let after = field.vitals.get_stored(1).unwrap();
    assert!(after.0 > before.0);
    assert_eq!(after.1, before.1 - 10);
    field.vitals.set(1, 0, 0);
    field.vitals.set(5, 0, 0);
    let sp = field.full(2).1;
    assert!(field.cast(1, usize::MAX, 35));
    assert_eq!(field.vitals.get_stored(1).unwrap().0, field.full(1).0);
    assert_eq!(field.vitals.get_stored(5).unwrap().0, field.full(5).0);
    assert_eq!(field.vitals.get_stored(2).unwrap().1, sp - 80);
}

#[test]
fn dead_or_full_targets_and_restricted_casters_do_not_spend_sp_and_status_cures_ignore_hit() {
    let mut field = Field::original();
    assert!(!field.cast(1, 0, 7));
    assert_eq!(field.vitals.get_stored(2), None);
    field.vitals.set(1, 0, 0);
    assert!(!field.cast(1, 0, 7));
    assert_eq!(field.vitals.get_stored(1), Some((0, 0)));
    field.vitals.set(1, 1, field.full(1).1);
    field.vitals.set_states(1, vec![4]);
    assert!(!field.cast(0, 0, 49));
    field.vitals.set_states(1, vec![2, 3, 4]);
    field
        .data
        .skills
        .iter_mut()
        .find(|s| s.id == 10)
        .unwrap()
        .hit = 0;
    assert!(field.cast(1, 0, 10));
    assert!(field.vitals.states(1).is_empty());
    let hp = field.vitals.get_stored(1).unwrap().0;
    assert_eq!(hp, 1);
}

#[test]
fn zero_power_stat_scaled_heals_are_field_usable_but_battle_only_state_cures_are_not() {
    let mut skill = crate::menu::testkit::heal_skill(1, "Heal", 0, 0);
    skill.magical_rate = 10;
    assert!(field_usable(&skill));
    skill.affect_hp = false;
    skill.affected_states = vec![9];
    assert!(!field_usable(&skill));
    skill.affected_states = vec![2];
    assert!(field_usable(&skill));
}
