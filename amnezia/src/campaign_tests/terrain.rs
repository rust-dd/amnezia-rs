use crate::assets::{asset_root, load_ron};
use amnezia_data::{Chipset, TerrainDef};

#[test]
fn original_terrain_flags_and_chipset_tags_survive_conversion() {
    let chips = load_ron::<Vec<Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    assert_eq!(
        chips.iter().filter(|c| !c.terrain_data.is_empty()).count(),
        18
    );
    for chip in &chips {
        assert_eq!(
            chip.terrain_data.len(),
            if [6, 12].contains(&chip.id) { 0 } else { 162 }
        );
    }
    let terrains = load_ron::<Vec<TerrainDef>>(&format!("{}/terrains.ron", asset_root()));
    assert_eq!(terrains.len(), 10);
    let landing = terrains
        .iter()
        .filter(|t| t.airship_land)
        .map(|t| t.id)
        .collect::<Vec<_>>();
    assert_eq!(landing, [1, 3, 4, 6]);
    let bush = terrains
        .iter()
        .filter(|t| t.bush_depth != 0)
        .map(|t| (t.id, t.bush_depth))
        .collect::<Vec<_>>();
    assert_eq!(bush, [(2, 1), (7, 1)]);
    assert!(terrains.iter().all(|t| t.airship_pass));
    assert_eq!(terrains[8].name, "Sea : Beach");
    assert!(terrains[8].boat_pass && terrains[8].ship_pass);
    assert!(!terrains[9].boat_pass && terrains[9].ship_pass);
    assert_eq!(
        chips[10].terrain_data.iter().filter(|&&t| t == 11).count(),
        1
    );
}
