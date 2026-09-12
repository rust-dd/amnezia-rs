use super::*;
use crate::screenfx::{saved, weather_smoke as rain};

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<saved::ScreenState>,
    legacy: String,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-weather");
    app.insert_resource(Fixture {
        slot,
        saved: None,
        legacy: String::new(),
        checks: 0,
    });
}

fn restored(world: &mut World, frame: u32) {
    let bit = match frame {
        331..=429 => 1,
        511..=619 => 4,
        671..=769 => 16,
        _ => return,
    };
    if world.resource::<Fixture>().checks & bit != 0 || world.contains_resource::<saved::Pending>()
    {
        return;
    }
    assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
    assert!(world.resource::<Fade>().busy());
    if bit == 4 {
        assert_eq!(saved::snapshot(world).weather_pan, [0.0; 2]);
        rain::saved::reset_reference(world);
    } else {
        assert_eq!(
            Some(saved::snapshot(world)),
            world.resource::<Fixture>().saved
        );
    }
    assert_eq!(
        *world.resource::<Weather>(),
        if bit == 16 {
            Weather::None
        } else {
            Weather::Rain
        }
    );
    rain::saved::verify_fresh(world);
    world.resource_mut::<Fixture>().checks |= bit;
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    restored(world, frame);
    let path = world.resource::<Fixture>().slot.path(world);
    match frame {
        260 => {
            rain::setup(world);
            rain::start(world, 28, 1);
        }
        270 => {
            world
                .resource_mut::<crate::player::CameraPan>()
                .command(&[2, 2, 4, 1, 0]);
        }
        300 | 650 => world.resource_mut::<EventSaveRequest>().0 = true,
        303 | 653 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(game.weather, i32::from(frame == 303));
            assert_ne!(game.screen.as_ref().unwrap().weather_pan, [0.0; 2]);
            world.resource_mut::<Fixture>().saved = game.screen;
        }
        310 => {
            *world.resource_mut::<Weather>() = Weather::None;
            world.resource_mut::<WeatherStrength>().0 = 0;
            world.insert_resource(crate::player::CameraPan::default());
            rain::saved::poison(world);
        }
        330 | 510 | 670 => world.resource_mut::<LoadRequest>().0 = true,
        430 => {
            assert!(!world.resource::<Fade>().busy());
            assert_ne!(
                saved::snapshot(world).weather_pan,
                world
                    .resource::<Fixture>()
                    .saved
                    .as_ref()
                    .unwrap()
                    .weather_pan
            );
            world.resource_mut::<Fixture>().checks |= 2;
            return Some("weather-pixels-save-restored");
        }
        500 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 6;
            let pan = ron::to_string(&game.screen.as_ref().unwrap().weather_pan).unwrap();
            let legacy = ron::to_string(&game)
                .unwrap()
                .replace(&format!(",weather_pan:{pan}"), "");
            assert!(!legacy.contains("weather_pan"));
            std::fs::write(&path, &legacy).unwrap();
            world.resource_mut::<Fixture>().legacy = legacy;
            rain::saved::poison(world);
        }
        620 => {
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(read_save(&path).unwrap().format_version, 6);
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                world.resource::<Fixture>().legacy
            );
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("weather-pixels-save-legacy");
        }
        640 => *world.resource_mut::<Weather>() = Weather::None,
        660 => {
            *world.resource_mut::<Weather>() = Weather::Rain;
            rain::saved::poison(world);
        }
        770 => {
            assert!(!world.resource::<Fade>().busy());
            assert_eq!(*world.resource::<Weather>(), Weather::None);
            rain::saved::verify_hidden(world);
            world.resource_mut::<Fixture>().checks |= 32;
            return Some("save-weather-disabled");
        }
        800 => rain::start(world, 38, 2),
        850 => {
            assert_eq!(world.resource::<WeatherStrength>().0, 2);
            world.resource_mut::<Fixture>().checks |= 64;
            return Some("weather-pixels-save-resumed");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 127);
    rain::saved::verify_finished(world);
    fixture.slot.finish(world);
    info!(
        "saved weather: restored scroll, fresh particles, continued pan, legacy defaults and saved inactive weather verified"
    );
}
