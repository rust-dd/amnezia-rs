use super::*;
use crate::assets::{asset_root, load_ron};

#[test]
fn both_original_slow_dialogue_maps_use_the_exact_name_reveal_timeline() {
    let mut count = 0;
    for (id, expected_count) in [(183, 26), (220, 16)] {
        let map = load_ron::<amnezia_data::Map>(&format!("{}/maps/map_{id:04}.ron", asset_root()));
        let mut found = 0;
        for command in map
            .events
            .iter()
            .flat_map(|event| &event.pages)
            .flat_map(|page| &page.commands)
        {
            if !command.string.starts_with("\\S[5]") {
                continue;
            }
            assert!(matches!(command.code, 10110 | 20110));
            let mut writer = Typewriter::new(&command.string, "Ron", &Variables::default());
            let name = crate::text::substitute(&command.string, "Ron", &Variables::default());
            assert!(matches!(name.as_str(), "Ron" | "Tiffany"));
            let length = name.chars().count();
            for tick in 1..=length * 3 {
                writer.tick();
                let shown = ((tick - 1) / 3 + 1).min(length);
                assert_eq!(writer.text(), name.chars().take(shown).collect::<String>());
                assert_eq!(writer.is_complete(), tick == length * 3);
            }
            found += 1;
        }
        assert_eq!(found, expected_count, "map {id}");
        count += found;
    }
    assert_eq!(count, 42);
}
