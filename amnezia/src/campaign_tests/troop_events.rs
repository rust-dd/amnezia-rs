use super::*;

#[test]
fn original_battle_event_pages_are_present_in_the_shipped_data() {
    let troops = load_ron::<Vec<TroopDef>>(&format!("{}/troops.ron", asset_root()));
    assert_eq!(troops.len(), 52);
    assert_eq!(troops.iter().map(|t| t.pages.len()).sum::<usize>(), 69);
    let scripted = troops
        .iter()
        .filter(|t| t.pages.iter().any(|p| !p.commands.is_empty()))
        .count();
    assert_eq!(scripted, 23);
    let pages = troops
        .iter()
        .flat_map(|t| &t.pages)
        .filter(|p| !p.commands.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(pages.len(), 40);
    assert_eq!(pages.iter().map(|p| p.commands.len()).sum::<usize>(), 352);
    assert!(pages.iter().flat_map(|p| &p.commands).all(|c| c.code != 0));
    let troop = |id| troops.iter().find(|t| t.id == id).unwrap();
    let tutorial = &troop(15).pages[0];
    assert_eq!(tutorial.condition.flags, 8);
    assert_eq!(
        (tutorial.condition.turn_a, tutorial.condition.turn_b),
        (0, 0)
    );
    assert!(
        tutorial
            .commands
            .iter()
            .any(|c| c.code == 10440 && c.params == [1, 1, 0, 0, 2])
    );
    assert!(tutorial.commands.iter().any(|c| c.code == 13410));
    let alen = &troop(16).pages[1];
    assert_eq!((alen.condition.turn_a, alen.condition.turn_b), (0, 3));
    assert!(
        alen.commands
            .iter()
            .any(|c| c.code == 10330 && c.params == [0, 0, 4])
    );
    let finale = &troop(31).pages;
    assert_eq!(finale[1].condition.flags, 40);
    assert_eq!(
        (
            finale[1].condition.enemy_index,
            finale[1].condition.enemy_hp_min
        ),
        (3, 1)
    );
    assert_eq!(
        (
            finale[2].condition.enemy_index,
            finale[2].condition.enemy_hp_max
        ),
        (1, 0)
    );
    assert!(
        finale[3]
            .commands
            .iter()
            .any(|c| c.code == 13310 && c.params[1] == 617)
    );
    assert_eq!(troop(50).pages.len(), 6);
}
