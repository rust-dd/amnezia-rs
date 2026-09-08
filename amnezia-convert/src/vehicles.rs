use amnezia_data::VehicleDef;
use anyhow::Result;
use std::path::Path;

pub fn convert_vehicles(input: &Path, output: &Path) -> Result<()> {
    let database = std::fs::read(input.join("RPG_RT.ldb"))?;
    let tree = std::fs::read(input.join("RPG_RT.lmt"))?;
    let vehicles = lcf::parse_vehicles(&database, &tree)?.map(|vehicle| VehicleDef {
        charset: vehicle.charset,
        index: vehicle.index,
        map_id: vehicle.map_id,
        x: vehicle.x,
        y: vehicle.y,
    });
    std::fs::create_dir_all(output)?;
    std::fs::write(output.join("vehicles.ron"), ron::to_string(&vehicles)?)?;
    Ok(())
}
