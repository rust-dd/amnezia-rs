use super::*;

fn original_battle() -> Battle {
    let mut battle = build_party2();
    messages::original_text(&mut battle);
    battle.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    battle.members[0].name = "Ron".into();
    battle.members[1].name = "Tiffany".into();
    battle
}

#[test]
fn original_medicine_uses_the_item_term_and_reports_full_hp_as_zero_recovered() {
    let mut battle = original_battle();
    battle.members[1].hp = battle.members[1].max_hp - 3;
    assert_eq!(
        battle.apply_item(0, 105, 1),
        "Ron Gyógyital használata\nTiffany HP 3 visszatért"
    );
    assert_eq!(
        battle.apply_item(0, 105, 1),
        "Ron Gyógyital használata\nTiffany HP 0 visszatért"
    );
    assert!(battle.apply_item(0, u32::MAX, 1).is_empty());
}

#[test]
fn original_revival_and_antitoxin_items_use_the_states_own_recovery_messages() {
    let mut battle = original_battle();
    battle.members[1].hp = 0;
    battle.members[1].states = vec![(1, 0)];
    assert_eq!(
        battle.apply_item(0, 112, 1),
        "Ron Életital használata\nTiffany visszanyeri eszméletét!"
    );
    assert_eq!(battle.members[1].hp, battle.members[1].max_hp / 2);
    assert_eq!(battle.apply_item(0, 112, 1), "Ron Életital használata");
    battle.members[1].states = vec![(2, 0)];
    assert_eq!(
        battle.apply_item(0, 111, 1),
        "Ron Antitoxin használata\nTiffany szervezetéből eltűnt a méreg"
    );
    assert!(battle.members[1].states.is_empty());
}

#[test]
fn mixed_medicine_reports_hp_then_sp_then_states_without_invented_zero_sp() {
    let mut battle = original_battle();
    battle.items = vec![medicine(1, 20, 15, vec![2])];
    battle.members[1].hp = battle.members[1].max_hp - 8;
    battle.members[1].sp = battle.members[1].max_sp - 9;
    battle.members[1].states = vec![(2, 0)];
    assert_eq!(
        battle.apply_item(0, 1, 1),
        "Ron Gyógyfű használata\nTiffany HP 8 visszatért\nTiffany SP 9 visszatért\nTiffany szervezetéből eltűnt a méreg"
    );
    assert_eq!(
        battle.apply_item(0, 1, 1),
        "Ron Gyógyfű használata\nTiffany HP 0 visszatért"
    );
}
