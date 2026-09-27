use super::*;
use crate::screenfx::saved::{self, Pending, ScreenState};

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<ScreenState>,
    paused: Option<ScreenState>,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-screen");
    app.insert_resource(Fixture {
        slot,
        saved: None,
        paused: None,
        checks: 0,
    });
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    matches!(frame, 450 | 510).then_some(KeyCode::Escape)
}

fn effects(world: &mut World, active: bool) {
    let params = if active {
        [
            (11030, vec![50, 125, 75, 50, 100, 0]),
            (11040, vec![31, 10, 5, 20, 100, 0]),
            (11050, vec![3, 5, 100, 0]),
        ]
    } else {
        [
            (11030, vec![100, 100, 100, 100, 0, 0]),
            (11040, vec![0, 0, 0, 0, 0, 0]),
            (11050, vec![0, 0, 0, 0]),
        ]
    };
    world.resource_mut::<RunningEvent>().start(
        0,
        params
            .into_iter()
            .map(|(code, params)| amnezia_data::EventCommand {
                code,
                indent: 0,
                string: String::new(),
                params,
            })
            .collect(),
    );
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let path = world.resource::<Fixture>().slot.path(world);
    if frame > 330
        && world.resource::<Fixture>().checks & 1 == 0
        && !world.contains_resource::<Pending>()
    {
        assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
        assert!(world.resource::<Fade>().busy());
        assert_eq!(
            Some(saved::snapshot(world)),
            world.resource::<Fixture>().saved
        );
        saved::smoke::verify_camera(world);
        world.resource_mut::<Fixture>().checks |= 1;
        info!(
            "saved screen: exact fractional tone, pending target, flash and shake restored before showing the map"
        );
    }
    match frame {
        260 => saved::smoke::setup(world),
        270 => effects(world, true),
        300 => world.resource_mut::<EventSaveRequest>().0 = true,
        303 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert!(game.screen.is_some());
            world.resource_mut::<Fixture>().saved = game.screen;
        }
        310 => effects(world, false),
        330 | 560 => world.resource_mut::<LoadRequest>().0 = true,
        430 => {
            assert!(!world.resource::<Fade>().busy());
            let current = saved::snapshot(world);
            let old = world.resource::<Fixture>().saved.as_ref().unwrap();
            assert!(current.tone.tone()[0] < old.tone.tone()[0]);
            assert!(current.valid());
            world.resource_mut::<Fixture>().checks |= 2;
            return Some("save-screen-restored");
        }
        451 => {
            assert!(!world.resource::<crate::menu::SceneFlow>().active());
            let before = saved::snapshot(world);
            world.resource_mut::<Fixture>().paused = Some(before);
        }
        452 => {
            assert!(world.resource::<crate::menu::SceneFlow>().active());
            let paused = saved::snapshot(world);
            assert_ne!(Some(&paused), world.resource::<Fixture>().paused.as_ref());
            world.resource_mut::<Fixture>().paused = Some(paused);
        }
        470 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            assert_eq!(
                Some(saved::snapshot(world)),
                world.resource::<Fixture>().paused
            );
            return Some("save-screen-menu");
        }
        500 => {
            assert!(world.resource::<crate::menu::MenuOpen>().0);
            assert_eq!(
                Some(saved::snapshot(world)),
                world.resource::<Fixture>().paused
            );
            world.resource_mut::<Fixture>().checks |= 4;
        }
        530 => {
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            assert!(
                saved::snapshot(world).tone.tone()[0]
                    < world
                        .resource::<Fixture>()
                        .paused
                        .as_ref()
                        .unwrap()
                        .tone
                        .tone()[0]
            );
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("save-screen-resumed");
        }
        550 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 5;
            game.screen = None;
            write_save(&path, &game).unwrap();
        }
        660 => {
            assert!(!world.resource::<Fade>().busy());
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, 5);
            let expected = [game.tone.0, game.tone.1, game.tone.2, game.tone.3].map(|v| v as f32);
            assert_eq!(world.resource::<TintState>().tone(), expected);
            let current = saved::snapshot(world);
            world.resource_mut::<Fixture>().checks |= 16;
            world.resource_mut::<Fixture>().paused = Some(current);
            return Some("save-screen-legacy");
        }
        700 => assert_eq!(
            Some(saved::snapshot(world)),
            world.resource::<Fixture>().paused
        ),
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 31);
    saved::smoke::verify_finished(world);
    fixture.slot.finish(world);
    info!("saved screen: restored effects, real menu pause/resume and legacy defaults verified");
}
