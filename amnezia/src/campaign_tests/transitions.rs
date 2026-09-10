use super::*;

#[test]
fn original_scene_transitions_and_all_event_overrides_are_preserved() {
    let system = load_ron::<amnezia_data::SystemDef>(&format!("{}/system.ron", asset_root()));
    assert_eq!(system.transitions, [0, 0, 16, 17, 17, 16]);
    let mut changes = BTreeMap::new();
    let mut screens = BTreeMap::new();
    for map in maps().into_values() {
        for event in map.events {
            for page in event.pages {
                for command in page.commands {
                    if command.code == 10690 {
                        *changes.entry(command.params).or_insert(0) += 1;
                    } else if matches!(command.code, 11010 | 11020) {
                        *screens.entry((command.code, command.params)).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        changes,
        BTreeMap::from([
            (vec![0, 0], 8),
            (vec![0, 19], 4),
            (vec![0, 20], 4),
            (vec![1, 0], 8),
            (vec![1, 19], 8),
        ])
    );
    assert_eq!(
        screens,
        BTreeMap::from([
            ((11010, vec![0]), 1082),
            ((11010, vec![17]), 33),
            ((11020, vec![0]), 1077),
            ((11020, vec![17]), 33),
        ])
    );
}
