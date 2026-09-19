use super::SaveFiles;
use crate::menu::{MenuOpen, MenuScreen, MenuState};
use crate::save::{SaveAccess, SaveLocation, SaveRequest, slots::ActiveSlot};
use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) use super::view::pixels::snapshot;
mod crystals;
pub(crate) mod load;

#[derive(Resource)]
struct Fixture {
    directory: PathBuf,
    original: PathBuf,
    original_slot: ActiveSlot,
    files: Vec<(u8, Vec<u8>)>,
    checks: u8,
}

#[derive(Resource, Default)]
pub(super) struct Pixels(pub Arc<AtomicUsize>);

pub(crate) fn configure(app: &mut App) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "amnezia-smoke-save-slots-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let original = app.world().resource::<SaveLocation>().0.clone();
    let original_slot = *app.world().resource::<ActiveSlot>();
    app.insert_resource(SaveLocation(directory.join("slot1.ron")))
        .insert_resource(ActiveSlot::default())
        .init_resource::<Pixels>()
        .insert_resource(Fixture {
            directory,
            original,
            original_slot,
            files: Vec::new(),
            checks: 0,
        });
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        310 | 400 | 420 | 450 | 560 | 580 | 600 | 950 => Some(KeyCode::Enter),
        380 | 540 | 650 | 850 => Some(KeyCode::Escape),
        405 => Some(KeyCode::ArrowDown),
        480 => Some(KeyCode::PageUp),
        510 => Some(KeyCode::ArrowUp),
        _ => None,
    }
}

pub(crate) fn event_input(world: &mut World) -> bool {
    let files = world.resource::<SaveFiles>();
    if files.event_menu.is_none() {
        return false;
    }
    let target = world.resource::<ActiveSlot>().index();
    let key = if files.entries.is_none() || files.finished.is_some() || files.navigation.moving() {
        None
    } else if files.navigation.index < target {
        Some(KeyCode::ArrowDown)
    } else if files.navigation.index > target {
        Some(KeyCode::ArrowUp)
    } else {
        Some(KeyCode::Enter)
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    true
}

pub(crate) fn event_active(world: &World) -> bool {
    world.resource::<SaveFiles>().event_menu.is_some()
}

fn path(world: &World, number: u8) -> PathBuf {
    ActiveSlot::new(number)
        .unwrap()
        .path(&world.resource::<SaveLocation>().0)
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = crystals::drive(world, frame) {
        return Some(label);
    }
    if frame == 300 {
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        assert!(!world.resource::<crate::teleport::Fade>().busy());
        world.resource_mut::<MenuOpen>().0 = true;
        *world.resource_mut::<MenuState>() = MenuState {
            cursor: 3,
            screen: MenuScreen::Command,
        };
        world.resource_mut::<SaveAccess>().0 = true;
        world
            .resource_mut::<crate::state::Party>()
            .restore(vec![1, 2, 3, 4]);
        world.resource_mut::<crate::vitals::Vitals>().set(1, 17, 12);
        world.resource_mut::<crate::text::HeroName>().0 = "Álmos".into();
    }
    if frame == 390 {
        assert!(!world.resource::<SaveFiles>().active());
        assert!(world.resource::<MenuOpen>().0);
        assert!(!world.resource::<SaveRequest>().0);
        assert_eq!(
            std::fs::read_dir(&world.resource::<Fixture>().directory)
                .unwrap()
                .count(),
            0
        );
        world.resource_mut::<Fixture>().checks |= 1;
    }
    if frame == 430 {
        seed_previews(world);
    }
    if frame == 550 {
        unchanged(world, false);
        assert!(!world.resource::<SaveFiles>().active());
        world.resource_mut::<crate::text::HeroName>().0 = "Áron".into();
        world.resource_mut::<crate::vitals::Vitals>().set(1, 23, 12);
        world.resource_mut::<Fixture>().checks |= 2;
    }
    if frame == 700 {
        unchanged(world, true);
        assert!(!world.resource::<SaveFiles>().active());
        assert!(world.resource::<MenuOpen>().0);
        assert_eq!(
            *world.resource::<ActiveSlot>(),
            ActiveSlot::new(15).unwrap()
        );
        world.resource_mut::<Fixture>().checks |= 4;
    }
    match frame {
        470 => Some("save-slots-bottom"),
        500 => Some("save-slots-corrupt"),
        511 => Some("save-slots-moving"),
        610 => Some("save-slots-updated"),
        _ => None,
    }
}

fn seed_previews(world: &mut World) {
    assert!(path(world, 2).is_file());
    assert!(!path(world, 1).exists());
    let bytes = std::fs::read(path(world, 2)).unwrap();
    let mut files = Vec::new();
    for number in [1, 2, 12, 14, 15] {
        let contents = if number == 12 {
            b"broken save".to_vec()
        } else {
            bytes.clone()
        };
        let target = path(world, number);
        std::fs::write(&target, &contents).unwrap();
        let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::from(number));
        std::fs::File::open(&target)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        files.push((number, contents));
    }
    world.resource_mut::<Fixture>().files = files;
}

fn unchanged(world: &World, saved_last: bool) {
    for (number, original) in &world.resource::<Fixture>().files {
        let actual = std::fs::read(path(world, *number)).unwrap();
        if *number == 15 && saved_last {
            assert_ne!(&actual, original);
        } else {
            assert_eq!(&actual, original, "slot {number}");
        }
    }
    assert_eq!(
        std::fs::read_dir(&world.resource::<Fixture>().directory)
            .unwrap()
            .count(),
        5
    );
}

pub(crate) fn verify_finished(world: &mut World) {
    crystals::verify_finished(world);
    assert_eq!(world.resource::<Pixels>().0.load(Ordering::Relaxed), 7);
    assert_eq!(world.resource::<Fixture>().checks, 7);
    unchanged(world, true);
    let fixture = world.remove_resource::<Fixture>().unwrap();
    for (number, _) in fixture.files {
        std::fs::remove_file(path(world, number)).unwrap();
    }
    std::fs::remove_dir(fixture.directory).unwrap();
    world.resource_mut::<SaveLocation>().0 = fixture.original;
    *world.resource_mut::<ActiveSlot>() = fixture.original_slot;
    info!("save selector: cancel, independent slot 2/15 saves and unchanged neighbours verified");
}
