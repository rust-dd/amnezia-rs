use super::maps;

#[test]
fn all_original_overlap_forbidden_pages_survive_conversion() {
    let mut pages = Vec::new();
    for (map_id, map) in maps() {
        for event in map.events {
            for (index, page) in event.pages.iter().enumerate() {
                if page.overlap_forbidden {
                    pages.push((map_id, event.id, index + 1));
                }
            }
        }
    }
    assert_eq!(pages, [(42, 10, 1), (49, 239, 1), (84, 51, 1)]);
}
