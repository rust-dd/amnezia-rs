use amnezia_data::Map;
use std::path::Path;

fn main() {
    let selected = std::env::args()
        .skip(1)
        .map(|arg| arg.parse::<u32>().expect("map IDs must be integers"))
        .collect::<Vec<_>>();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/maps");
    let mut paths = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let id = path.file_stem().unwrap().to_str().unwrap()[4..]
            .parse::<u32>()
            .unwrap();
        if !selected.is_empty() && !selected.contains(&id) {
            continue;
        }
        let map = ron::from_str::<Map>(&std::fs::read_to_string(path).unwrap()).unwrap();
        for event in &map.events {
            for (page, definition) in event.pages.iter().enumerate() {
                let interesting = definition.animation_type != 0
                    || definition.translucent
                    || definition
                        .commands
                        .iter()
                        .any(|c| matches!(c.code, 10850 | 11910));
                if selected.is_empty() && !interesting {
                    continue;
                }
                println!(
                    "map={id} event={} page={} {:?} at={},{} graphic={:?}:{} anim={} alpha={} trigger={} layer={} condition={:?}",
                    event.id,
                    page + 1,
                    event.name,
                    event.x,
                    event.y,
                    definition.graphic_name,
                    definition.graphic_index,
                    definition.animation_type,
                    definition.translucent,
                    definition.trigger,
                    definition.layer,
                    definition.condition
                );
                for (index, command) in definition.commands.iter().enumerate() {
                    if !selected.is_empty() || matches!(command.code, 10850 | 11910) {
                        println!(
                            "  {index}: {} {:?} {:?}",
                            command.code, command.params, command.string
                        );
                    }
                }
            }
        }
    }
}
