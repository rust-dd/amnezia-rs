use super::*;

#[test]
fn only_the_original_world_map_and_airship_clouds_loop() {
    let maps = maps();
    let loops = maps
        .iter()
        .filter(|(_, map)| map.scroll_type != 0)
        .map(|(&id, map)| (id, map.width, map.height, map.scroll_type))
        .collect::<Vec<_>>();
    assert_eq!(loops, [(13, 140, 140, 3), (87, 20, 30, 1)]);
}
