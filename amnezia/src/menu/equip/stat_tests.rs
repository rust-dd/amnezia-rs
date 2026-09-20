use super::*;
use crate::menu::testkit;
use crate::progression::Progression;
use crate::vitals::Vitals;

#[test]
fn equipment_preview_clamps_before_each_original_state_modifier() {
    let mut data = testkit::data();
    let actor = &mut data.actors[0];
    actor.curves.attack = vec![1200; 2];
    actor.curves.defense = vec![1; 2];
    actor.curves.spirit = vec![800; 2];
    actor.curves.agility = vec![999; 2];
    let actor = &data.actors[0];
    let progression = Progression::default();
    let mut vitals = Vitals::default();
    assert_eq!(
        stats(actor, &data, &progression, &vitals, [0; 5]),
        [999, 1, 800, 999]
    );
    for state in crate::conditions::definitions()
        .iter()
        .filter(|state| state.id > 1)
    {
        vitals.set_states(1, vec![state.id]);
        let expected = std::array::from_fn(|i| {
            let base = [999_u32, 1, 800, 999][i];
            if !state.affect_stats[i] {
                base
            } else if state.affect_type == 0 {
                (base / 2).max(1)
            } else if state.affect_type == 1 {
                base * 2
            } else {
                base
            }
        });
        assert_eq!(
            stats(actor, &data, &progression, &vitals, [0; 5]),
            expected,
            "state {}",
            state.id
        );
    }
}

#[test]
fn two_handed_preview_removes_the_other_hands_bonus_without_changing_worn_gear() {
    let mut data = testkit::data();
    let mut weapon = testkit::weapon(10, "Kétkezes", 10);
    weapon.two_handed = true;
    let mut shield = testkit::blank_item(11, 2);
    shield.def = 15;
    data.items = vec![weapon, shield];
    let actor = &data.actors[0];
    let worn = [0, 11, 0, 0, 0];
    let preview = equipment::preview_slots(worn, 0, 10, &data.items);
    assert_eq!(
        stats(
            actor,
            &data,
            &Progression::default(),
            &Vitals::default(),
            preview
        ),
        [38, 16, 14, 12]
    );
    assert_eq!(worn, [0, 11, 0, 0, 0]);
}
