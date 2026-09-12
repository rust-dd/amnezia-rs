use super::*;
use crate::picture::{
    PictureCommand,
    saved::{Capture, Pending, PictureState},
};
use bevy::ecs::system::RunSystemOnce;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

mod pixels;
pub(crate) use pixels::snapshot;

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Vec<PictureState>,
    checks: u8,
    pixels: Arc<AtomicU32>,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-pictures");
    app.insert_resource(Fixture {
        slot,
        saved: Vec::new(),
        checks: 0,
        pixels: Arc::new(AtomicU32::new(0)),
    });
}

fn current(world: &mut World) -> Vec<PictureState> {
    world
        .run_system_once(|capture: Capture| capture.snapshot())
        .unwrap()
}

fn cross(world: &mut World, id: u32, x: f32, fixed: bool) {
    world.write_message(PictureCommand::show(
        id,
        "Cross",
        x,
        60.0,
        &[
            0,
            0,
            0,
            0,
            i32::from(fixed),
            800,
            0,
            0,
            100,
            100,
            100,
            100,
            0,
            0,
        ],
    ));
}

fn fog(world: &mut World) {
    world.write_message(PictureCommand::show(
        1,
        "Fog",
        80.0,
        140.0,
        &[0, 0, 0, 0, 0, 50, 35, 0, 60, 140, 100, 0, 2, 1],
    ));
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let path = world.resource::<Fixture>().slot.path(world);
    if frame > 330
        && world.resource::<Fixture>().checks & 1 == 0
        && !world.contains_resource::<Pending>()
    {
        assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
        assert!(world.resource::<Fade>().busy());
        let pictures = current(world);
        assert_eq!(pictures, world.resource::<Fixture>().saved);
        world.resource_mut::<Fixture>().checks |= 1;
        info!(
            "saved pictures: exact image, anchor, tween, tone and effect state restored after scene cleanup"
        );
    }
    match frame {
        270 => {
            fog(world);
            cross(world, 2, 230.0, false);
            cross(world, 3, 280.0, true);
            world.resource_mut::<RunningEvent>().start(
                0,
                vec![
                    amnezia_data::EventCommand {
                        code: 11060,
                        indent: 0,
                        string: String::new(),
                        params: vec![0],
                    },
                    amnezia_data::EventCommand {
                        code: 11060,
                        indent: 0,
                        string: String::new(),
                        params: vec![2, 2, 4, 1, 0],
                    },
                ],
            );
        }
        280 => {
            world.write_message(PictureCommand::move_to(
                1,
                100.0,
                150.0,
                &[1, 0, 100, 150, 0, 75, 60, 0, 80, 120, 100, 50, 2, 4, 60],
            ));
        }
        300 | 460 => world.resource_mut::<EventSaveRequest>().0 = true,
        303 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(
                game.pictures
                    .iter()
                    .map(|p| (p.id, p.name.as_str()))
                    .collect::<Vec<_>>(),
                [(1, "Fog"), (2, "Cross"), (3, "Cross")]
            );
            world.resource_mut::<Fixture>().saved = game.pictures;
        }
        310 | 450 => {
            for id in 1..=3 {
                world.write_message(PictureCommand::erase(id));
            }
            if frame == 310 {
                cross(world, 9, 160.0, false);
            }
        }
        330 | 470 | 590 => world.resource_mut::<LoadRequest>().0 = true,
        430 => {
            assert!(!world.resource::<Fade>().busy());
            let pictures = current(world);
            let saved = &world.resource::<Fixture>().saved;
            assert_eq!(pictures.len(), 3);
            assert!(pictures[0].visual.transparency > saved[0].visual.transparency);
            assert_ne!(pictures[0].effect, saved[0].effect);
            assert!(pictures[0].tween.is_some());
            for (picture, saved) in pictures[1..].iter().zip(&saved[1..]) {
                assert_eq!(picture.visual, saved.visual);
                assert_eq!(picture.world_anchor, saved.world_anchor);
                assert_eq!(picture.fixed_to_map, saved.fixed_to_map);
                assert_eq!(picture.use_transparent_color, saved.use_transparent_color);
            }
            world.resource_mut::<Fixture>().checks |= 2;
            return Some("save-pictures-restored");
        }
        463 => {
            assert!(read_save(&path).unwrap().pictures.is_empty());
            cross(world, 7, 160.0, false);
        }
        560 | 680 => {
            assert!(!world.resource::<Fade>().busy());
            assert!(current(world).is_empty());
            world.resource_mut::<Fixture>().checks |= if frame == 560 { 4 } else { 8 };
            if frame == 680 {
                assert_eq!(read_save(&path).unwrap().format_version, 4);
            }
            return Some(if frame == 560 {
                "save-pictures-empty"
            } else {
                "save-pictures-legacy"
            });
        }
        570 => fog(world),
        580 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 4;
            game.pictures.clear();
            write_save(&path, &game).unwrap();
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 15);
    assert_eq!(fixture.pixels.load(Ordering::SeqCst), 1);
    fixture.slot.finish(world);
    info!(
        "saved pictures: restored animation, screen/map anchoring, empty and legacy slots verified"
    );
}
