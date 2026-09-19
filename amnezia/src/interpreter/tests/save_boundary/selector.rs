use super::*;
use crate::menu::save_files::{SaveFiles, register_flow};

fn press(app: &mut App, key: Option<KeyCode>) {
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame += 1;
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    app.update();
}

fn original_crystals() -> Vec<(u32, u32, Vec<EventCommand>)> {
    let mut pages = Vec::new();
    for entry in std::fs::read_dir(format!("{}/maps", crate::assets::asset_root())).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let map = crate::assets::load_ron::<amnezia_data::Map>(path.to_str().unwrap());
        let map_id = path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_start_matches("map_")
            .parse::<u32>()
            .unwrap();
        for event in map.events {
            for page in event.pages {
                if page.commands.iter().any(|command| command.code == 11910) {
                    pages.push((map_id, event.id, page.commands));
                }
            }
        }
    }
    assert_eq!(pages.len(), 16);
    pages
}

#[derive(serde::Deserialize)]
struct Saved {
    switches: Vec<(u32, bool)>,
    save_access: bool,
    foreground: Option<crate::interpreter::saved::State>,
}

#[test]
fn every_original_crystal_can_be_cancelled_or_saved_then_resumes_exactly_once() {
    for (map, event, commands) in original_crystals() {
        for save in [false, true] {
            let (mut app, path) = super::storage::app(&format!("selector-{map}-{event}-{save}"));
            app.add_plugins(crate::transitions::TransitionPlugin);
            register_flow(&mut app);
            let mut commands = commands.clone();
            commands.push(switch_cmd(9998, 2, 0));
            app.world_mut()
                .resource_mut::<RunningEvent>()
                .start(event, commands);
            for _ in 0..20 {
                press(&mut app, None);
            }
            assert!(app.world().resource::<SaveFiles>().active());
            assert!(app.world().resource::<EventSaveRequest>().0);
            assert!(app.world().resource::<MenuOpen>().0);
            assert!(!app.world().resource::<SaveAccess>().0);
            assert!(!switch_on(&app, 9998));
            assert!(!path.exists());
            let before = app.world().resource::<RunningEvent>().snapshot();
            press(
                &mut app,
                Some(if save {
                    KeyCode::Enter
                } else {
                    KeyCode::Escape
                }),
            );
            assert!(app.world().resource::<EventSaveRequest>().0);
            assert!(!path.exists());
            press(&mut app, None);
            assert!(app.world().resource::<SaveFiles>().active());
            assert!(!switch_on(&app, 9998));
            assert_eq!(path.exists(), save);
            for _ in 0..12 {
                press(&mut app, None);
            }
            assert!(!app.world().resource::<SaveFiles>().active());
            assert!(!app.world().resource::<MenuOpen>().0);
            assert!(!app.world().resource::<EventSaveRequest>().0);
            assert!(!app.world().resource::<RunningEvent>().active());
            assert!(switch_on(&app, 9998));
            let bytes = std::fs::read(&path).ok();
            assert_eq!(bytes.is_some(), save);
            if let Some(bytes) = &bytes {
                let saved = ron::de::from_bytes::<Saved>(bytes).unwrap();
                assert_eq!(saved.foreground, before);
                assert!(!saved.switches.contains(&(9998, true)));
                assert!(!saved.save_access);
            }
            for _ in 0..4 {
                press(&mut app, None);
                assert!(switch_on(&app, 9998));
                assert_eq!(std::fs::read(&path).ok(), bytes);
            }
            if save {
                std::fs::remove_file(path).unwrap();
            }
        }
    }
}
