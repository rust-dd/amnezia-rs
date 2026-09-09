use super::maps;

#[test]
fn all_original_nondefault_character_animation_modes_survive_conversion() {
    let mut counts = [0; 6];
    for map in maps().values() {
        for event in &map.events {
            for page in &event.pages {
                counts[page.animation_type as usize] += 1;
            }
        }
    }
    assert_eq!(counts[1..], [556, 5, 42, 355, 37]);
}
