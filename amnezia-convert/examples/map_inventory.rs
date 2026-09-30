use amnezia_data::{Chipset, Map};
use anyhow::{Context, Result, ensure};
use std::path::Path;

fn audit_maps(root: &Path) -> Result<usize> {
    let mut paths = std::fs::read_dir(root.join("assets/maps"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    let mut count = 0;
    for path in paths {
        if path.extension().is_none_or(|ext| ext != "ron") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().context("map filename")?;
        let id = stem.strip_prefix("map_").context("map filename prefix")?;
        let map = ron::from_str::<Map>(&std::fs::read_to_string(&path)?)?;
        let source = lcf::parse_map(&std::fs::read(root.join(format!("original/Map{id}.lmu")))?)?;
        ensure!(
            (map.chipset_id, map.width, map.height, map.scroll_type)
                == (
                    source.chipset_id,
                    source.width,
                    source.height,
                    source.scroll_type
                ),
            "map {id}: geometry or chipset differs from original LMU"
        );
        ensure!(
            map.lower == source.lower_layer,
            "map {id}: lower tiles differ"
        );
        ensure!(
            map.upper == source.upper_layer,
            "map {id}: upper tiles differ"
        );
        if matches!(id, "0220" | "0222") {
            let edges = map
                .lower
                .iter()
                .filter(|&&id| (5108..=5110).contains(&id))
                .count();
            let black = map.lower.iter().filter(|&&id| id == 5143).count();
            println!(
                "Map{id}: {}x{}, {edges} original grass-ledge tiles (5108..5110), {black} original blank tiles (5143)",
                map.width, map.height
            );
            ensure!(edges > 0, "original grass ledge must be exercised");
        }
        count += 1;
    }
    Ok(count)
}

fn audit_chipsets(root: &Path) -> Result<usize> {
    let chipsets =
        ron::from_str::<Vec<Chipset>>(&std::fs::read_to_string(root.join("assets/chipsets.ron"))?)?;
    for chip in &chipsets {
        let source = xyz::decode(
            &std::fs::read(root.join(format!("original/ChipSet/{}.xyz", chip.graphic)))?,
            true,
        )?;
        let converted =
            image::open(root.join(format!("assets/graphics/ChipSet/{}.png", chip.graphic)))?
                .to_rgba8();
        ensure!(
            converted.dimensions() == (u32::from(source.width), u32::from(source.height))
                && converted.as_raw() == &source.rgba,
            "{}: converted chipset pixels differ from original XYZ",
            chip.graphic
        );
    }
    Ok(chipsets.len())
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let maps = audit_maps(&root)?;
    let chipsets = audit_chipsets(&root)?;
    println!(
        "Verified {maps} original map tile grids and {chipsets} pixel-identical chipset entries."
    );
    Ok(())
}
