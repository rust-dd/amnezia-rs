use super::*;
use crate::interpreter::flow::call_event_page;

#[test]
fn all_original_event_calls_resolve_to_their_map_page() {
    let mut calls = 0;
    let mut choices = 0;
    let mut nested_choices = 0;
    for entry in std::fs::read_dir(format!("{}/maps", crate::assets::asset_root())).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let map = crate::assets::load_ron::<amnezia_data::Map>(path.to_str().unwrap());
        for event in &map.events {
            for page in &event.pages {
                let mut scopes = std::collections::HashSet::new();
                for command in &page.commands {
                    match command.code {
                        10140 => {
                            scopes.insert(command.indent);
                        }
                        20141 => {
                            scopes.remove(&command.indent);
                        }
                        12330 => {
                            assert_eq!(command.params[0], 1);
                            let (callee, _) =
                                call_event_page(&map.events, &command.params, event.id)
                                    .unwrap_or_else(|| {
                                        panic!(
                                            "{} event {}: {:?}",
                                            path.display(),
                                            event.id,
                                            command.params
                                        )
                                    });
                            calls += 1;
                            if scopes.iter().any(|indent| command.indent > *indent) {
                                choices += 1;
                                if callee.iter().any(|command| command.code == 10140) {
                                    nested_choices += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    assert_eq!((calls, choices, nested_choices), (5, 0, 0));
}
