use super::*;
use crate::animation::saved;

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<saved::MapState>,
    legacy: String,
    held: u32,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-animations");
    saved::smoke::configure(app);
    app.insert_resource(Fixture {
        slot,
        saved: None,
        legacy: String::new(),
        held: 0,
        checks: 0,
    });
}

fn start(world: &mut World) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![amnezia_data::EventCommand {
            code: 11210,
            indent: 0,
            string: String::new(),
            params: vec![62, 10001, 0, 0],
        }],
    );
}

fn restored(world: &mut World, frame: u32) -> Option<&'static str> {
    if !(331..430).contains(&frame) || world.contains_resource::<saved::Pending>() {
        return None;
    }
    let state = saved::snapshot(world);
    assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
    if world.resource::<Fade>().busy() {
        assert_eq!(Some(state), world.resource::<Fixture>().saved);
        saved::smoke::verify_target(world);
        let mut fixture = world.resource_mut::<Fixture>();
        fixture.held += 1;
        fixture.checks |= 1;
    } else if world.resource::<Fixture>().checks & 2 == 0
        && state.cast.is_some_and(|cast| cast.elapsed / 2 == 10)
    {
        assert!(world.resource::<Fixture>().held > 30);
        world.resource_mut::<Fixture>().checks |= 2;
        return Some("saved-animation-restored");
    }
    None
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = restored(world, frame) {
        return Some(label);
    }
    let path = world.resource::<Fixture>().slot.path(world);
    match frame {
        260 => world.resource_mut::<TintState>().set_tone([100.0; 4]),
        300 | 470 | 660 => start(world),
        320 | 650 => world.resource_mut::<EventSaveRequest>().0 = true,
        321 => return Some("saved-animation-before"),
        324 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            let cast = game.map_animation.cast.as_ref().unwrap();
            assert_eq!((cast.id, cast.elapsed), (62, 19));
            world.resource_mut::<Fixture>().saved = Some(game.map_animation);
        }
        330 | 510 | 670 => world.resource_mut::<LoadRequest>().0 = true,
        460 => {
            assert_eq!(saved::snapshot(world), default());
            assert_eq!(
                world.resource::<crate::animation::ActiveAnimations>().total,
                0
            );
            world.resource_mut::<Fixture>().checks |= 4;
        }
        500 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 7;
            let state = ron::to_string(&game.map_animation).unwrap();
            let legacy = ron::to_string(&game)
                .unwrap()
                .replace(&format!(",map_animation:{state}"), "");
            assert!(!legacy.contains("map_animation"));
            std::fs::write(&path, &legacy).unwrap();
            world.resource_mut::<Fixture>().legacy = legacy;
        }
        620 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(saved::snapshot(world), default());
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                world.resource::<Fixture>().legacy
            );
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("saved-animation-legacy");
        }
        653 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(game.map_animation, default());
        }
        760 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(saved::snapshot(world), default());
            assert_eq!(
                world.resource::<crate::animation::ActiveAnimations>().total,
                0
            );
            world.resource_mut::<Fixture>().checks |= 16;
            return Some("saved-animation-empty");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 31);
    saved::smoke::verify_finished(world);
    fixture.slot.finish(world);
    info!(
        "saved map animation: exact restore, frozen reload, target flash, resumed bitmap, completion and legacy/empty slots verified"
    );
}
