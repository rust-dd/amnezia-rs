use super::*;
use crate::menu::{MenuOpen, save_files::SaveFiles};
use crate::save::{SaveLocation, slots::ActiveSlot};
use crate::transitions::Transition;

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let directory =
        std::env::temp_dir().join(format!("amnezia_title_{tag}_{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let mut app = crate::save::tests::save_resources(directory.join("slot1.ron"));
    app.add_plugins((
        AssetPlugin::default(),
        crate::save::SavePlugin,
        crate::gamedata::GameDataPlugin,
        crate::transitions::TransitionPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<crate::interpreter::RunningEvent>()
    .init_resource::<TitleActive>()
    .init_resource::<TitleState>()
    .init_resource::<NewGameRequest>()
    .init_resource::<MenuOpen>()
    .init_resource::<crate::menu::DirectionInput>()
    .init_resource::<view::clock::Clock>()
    .init_resource::<SystemMusic>()
    .add_message::<AudioRequest>()
    .add_message::<AppExit>()
    .add_message::<MapChanged>()
    .add_systems(
        Update,
        crate::menu::update_directions.before(crate::menu::save_files::FileInput),
    )
    .add_systems(
        Update,
        (
            flow::entered,
            flow::loaded,
            view::clock::tick,
            flow::input,
            flow::drive,
            files::update,
        )
            .chain()
            .after(crate::menu::save_files::FileInput),
    );
    crate::menu::save_files::register_flow(&mut app);
    let mut sounds = SystemSounds::default();
    sounds.buzzer.name = "BUZZER".into();
    app.insert_resource(sounds);
    (app, directory)
}

fn step(app: &mut App, at: u32, key: Option<KeyCode>) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    frame(app, at);
}

fn slot(directory: &std::path::Path, number: u8, map: u32) -> (std::path::PathBuf, String) {
    let path = directory.join(format!("slot{number}.ron"));
    let text = format!(
        "(format_version:15,map_id:{map},x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:17,hero_name:\"Álmos\",vitals:[(1,(17,12))])"
    );
    std::fs::write(&path, &text).unwrap();
    (path, text)
}

fn open(app: &mut App) {
    step(app, 0, None);
    step(app, 35, None);
    step(app, 43, None);
    assert_eq!(app.world().resource::<TitleState>().cursor, CONTINUE);
    step(app, 44, Some(KeyCode::Enter));
    step(app, 50, None);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Files);
    step(app, 51, None);
    step(app, 56, None);
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<LoadRequest>().0);
    assert!(!app.world().resource::<Transition>().busy());
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .clear();
}

#[test]
fn every_slot_can_enable_continue_without_an_existing_first_slot() {
    let (mut app, directory) = app("all_slots_gate");
    assert!(!app.world().resource::<SaveLocation>().has_saves());
    for number in 1..=15 {
        let (path, original) = slot(&directory, number, 2);
        app.world_mut().resource_mut::<TitleState>().stage = Stage::Inactive;
        step(&mut app, u32::from(number), None);
        assert_eq!(app.world().resource::<TitleState>().cursor, CONTINUE);
        assert!(app.world().resource::<SaveLocation>().has_saves());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
        assert!(!app.world().resource::<SaveLocation>().has_saves());
    }
    std::fs::create_dir(directory.join("slot15.ron")).unwrap();
    assert!(!app.world().resource::<SaveLocation>().has_saves());
    std::fs::remove_dir(directory.join("slot15.ron")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn cancelling_load_uses_six_frame_fades_without_restarting_music_or_changing_files() {
    let (mut app, directory) = app("load_cancel");
    let (path, original) = slot(&directory, 15, 2);
    open(&mut app);
    step(&mut app, 57, Some(KeyCode::Escape));
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::FileLeaving(false)
    );
    step(&mut app, 62, Some(KeyCode::Enter));
    assert!(app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<LoadRequest>().0);
    step(&mut app, 63, None);
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::FileReturning
    );
    assert!(!app.world().resource::<SaveFiles>().active());
    assert!(!app.world().resource::<MenuOpen>().0);
    assert!(
        app.world_mut()
            .resource_mut::<Messages<AudioRequest>>()
            .drain()
            .all(|audio| {
                !matches!(
                    audio,
                    AudioRequest::Bgm { .. }
                        | AudioRequest::StopBgm
                        | AudioRequest::FadeOutBgm { .. }
                )
            })
    );
    step(&mut app, 68, Some(KeyCode::Enter));
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::FileReturning
    );
    step(&mut app, 69, None);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
    assert_eq!(app.world().resource::<TitleState>().cursor, CONTINUE);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn choosing_slot_fifteen_loads_it_only_after_the_exit_fade_without_writing() {
    let (mut app, directory) = app("load_selected");
    let (first, original_first) = slot(&directory, 1, 3);
    let (selected, original_selected) = slot(&directory, 15, 2);
    open(&mut app);
    step(&mut app, 57, Some(KeyCode::Enter));
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::FileLeaving(true)
    );
    assert!(
        app.world_mut()
            .resource_mut::<Messages<AudioRequest>>()
            .drain()
            .any(|audio| {
                matches!(audio, AudioRequest::FadeOutBgm { duration } if duration == 0.8)
            })
    );
    assert_eq!(
        *app.world().resource::<ActiveSlot>(),
        ActiveSlot::new(15).unwrap()
    );
    step(&mut app, 62, None);
    assert!(!app.world().resource::<LoadRequest>().0);
    step(&mut app, 63, None);
    assert!(app.world().resource::<LoadRequest>().0);
    assert!(!app.world().resource::<SaveFiles>().active());
    step(&mut app, 64, None);
    assert!(!app.world().resource::<LoadRequest>().0);
    assert!(app.world().resource::<PendingTeleport>().0.is_some());
    assert_eq!(app.world().resource::<crate::text::HeroName>().0, "Álmos");
    assert_eq!(app.world().resource::<crate::state::Inventory>().gold(), 17);
    app.world_mut().resource_mut::<PendingTeleport>().0 = None;
    step(&mut app, 65, None);
    assert!(!app.world().resource::<TitleActive>().0);
    assert!(!app.world().resource::<MenuOpen>().0);
    for (path, original) in [(first, original_first), (selected, original_selected)] {
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn a_failure_after_preview_returns_to_the_selector_without_mutating_the_session() {
    for missing in [false, true] {
        let (mut app, directory) = app(if missing {
            "load_deleted"
        } else {
            "load_invalid_map"
        });
        let (path, original) = slot(&directory, 15, if missing { 2 } else { 999_999 });
        open(&mut app);
        if missing {
            std::fs::remove_file(&path).unwrap();
        }
        step(&mut app, 57, Some(KeyCode::Enter));
        step(&mut app, 63, None);
        step(&mut app, 64, None);
        assert_eq!(app.world().resource::<TitleState>().stage, Stage::Files);
        assert!(app.world().resource::<TitleActive>().0);
        assert!(app.world().resource::<SaveFiles>().active());
        assert!(app.world().resource::<MenuOpen>().0);
        assert!(!app.world().resource::<LoadRequest>().0);
        assert!(app.world().resource::<PendingTeleport>().0.is_none());
        assert_eq!(app.world().resource::<crate::text::HeroName>().0, "Ron");
        assert!(
            app.world_mut()
                .resource_mut::<Messages<AudioRequest>>()
                .drain()
                .any(|audio| {
                    matches!(audio, AudioRequest::Sound { name, .. } if name == "BUZZER")
                })
        );
        step(&mut app, 70, None);
        step(&mut app, 71, Some(KeyCode::Escape));
        step(&mut app, 77, None);
        step(&mut app, 83, None);
        assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
        if !missing {
            assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }
}

#[test]
fn corrupt_and_empty_load_slots_buzz_without_selecting_or_requesting_a_load() {
    let (mut app, directory) = app("disabled_slots");
    let path = directory.join("slot1.ron");
    std::fs::write(&path, "broken save").unwrap();
    open(&mut app);
    for (at, key) in [
        (57, KeyCode::Enter),
        (58, KeyCode::ArrowUp),
        (67, KeyCode::Enter),
    ] {
        if at == 67 {
            step(&mut app, 66, None);
        }
        step(&mut app, at, Some(key));
        assert_eq!(app.world().resource::<TitleState>().stage, Stage::Files);
        assert!(!app.world().resource::<LoadRequest>().0);
        assert_eq!(*app.world().resource::<ActiveSlot>(), ActiveSlot::default());
        if key == KeyCode::Enter {
            assert!(
                app.world_mut()
                    .resource_mut::<Messages<AudioRequest>>()
                    .drain()
                    .any(|audio| {
                        matches!(audio, AudioRequest::Sound { name, .. } if name == "BUZZER")
                    })
            );
        }
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken save");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
