use super::*;
use crate::player::{
    CameraPan,
    saved_camera::{CameraState, Pending},
};
use crate::world::MainCamera;

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<CameraState>,
    follow_origin: Option<Vec2>,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-camera");
    app.insert_resource(Fixture {
        slot,
        saved: None,
        follow_origin: None,
        checks: 0,
    });
}

fn command(code: u32, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        indent: 0,
        string: String::new(),
        params,
    }
}

fn start(world: &mut World, commands: Vec<amnezia_data::EventCommand>) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(0, commands);
}

fn position(world: &mut World) -> Vec2 {
    let logical = world.resource::<CameraPan>().position.unwrap();
    let rendered = world
        .query_filtered::<&Transform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    assert!(logical.abs_diff_eq(rendered, 0.001));
    logical
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let path = world.resource::<Fixture>().slot.path(world);
    if frame > 370
        && world.resource::<Fixture>().checks & 1 == 0
        && !world.contains_resource::<Pending>()
    {
        assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
        assert!(world.resource::<Fade>().busy());
        let saved = world.resource::<Fixture>().saved.as_ref().unwrap().clone();
        assert_eq!(world.resource::<CameraPan>().snapshot(), saved);
        assert!(position(world).abs_diff_eq(Vec2::from_array(saved.position.unwrap()), 0.001));
        world.resource_mut::<Fixture>().checks |= 1;
        info!("saved camera: exact state restored after map rebuild, before the fade releases");
    }
    match frame {
        260 => start(
            world,
            vec![
                command(11060, vec![0]),
                command(11330, vec![10001, 8, 0, 0, 36, 1, 1, 1, 1, 37]),
                command(11340, vec![]),
                command(11060, vec![2, 2, 12, 1, 0]),
            ],
        ),
        330 => world.resource_mut::<EventSaveRequest>().0 = true,
        333 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            let saved = game.camera.unwrap();
            assert!(saved.locked && saved.offset[1] < 0.0);
            assert_eq!(saved.target, [0.0, -192.0]);
            assert_eq!(saved.speed, 15.0);
            world.resource_mut::<Fixture>().saved = Some(saved);
        }
        350 | 510 => start(
            world,
            vec![command(11060, vec![3, 0, 1, 6, 0]), command(11060, vec![1])],
        ),
        370 | 640 => world.resource_mut::<LoadRequest>().0 = true,
        500 => {
            assert!(!world.resource::<Fade>().busy());
            let saved = world.resource::<Fixture>().saved.as_ref().unwrap();
            let current = world.resource::<CameraPan>();
            assert!(current.locked && current.offset.y < saved.offset[1] - 10.0);
            assert_eq!(current.target.to_array(), saved.target);
            assert_eq!(current.speed, saved.speed);
            let expected = Vec2::from_array(saved.position.unwrap()) + current.offset
                - Vec2::from_array(saved.offset);
            assert!(position(world).abs_diff_eq(expected, 0.001));
            world.resource_mut::<Fixture>().checks |= 2;
            return Some("save-camera-resumed");
        }
        550 => {
            let current = world.resource::<CameraPan>();
            assert!(!current.locked);
            assert_eq!(current.offset, Vec2::ZERO);
            assert_eq!(current.target, Vec2::ZERO);
            let origin = position(world);
            world.resource_mut::<Fixture>().follow_origin = Some(origin);
        }
        560 => start(
            world,
            vec![
                command(11330, vec![10001, 8, 0, 0, 36, 1, 37]),
                command(11340, vec![]),
            ],
        ),
        620 => {
            let origin = world.resource::<Fixture>().follow_origin.unwrap();
            assert!(position(world).abs_diff_eq(origin + Vec2::X * 16.0, 0.001));
            world.resource_mut::<Fixture>().checks |= 4;
            return Some("save-camera-following");
        }
        630 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 3;
            game.camera = None;
            write_save(&path, &game).unwrap();
        }
        730 => {
            assert!(!world.resource::<Fade>().busy());
            let current = world.resource::<CameraPan>();
            assert!(!current.locked);
            assert_eq!(current.offset, Vec2::ZERO);
            assert_eq!(current.target, Vec2::ZERO);
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, 3);
            let expected = Vec2::from(
                world
                    .resource::<MapData>()
                    .tile_center(game.x as i32, game.y as i32),
            ) + Vec2::X * 8.0;
            assert!(position(world).abs_diff_eq(expected, 0.001));
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("save-camera-legacy");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 15);
    fixture.slot.finish(world);
    info!(
        "saved camera: map arrival, continued pan, unlocked following and legacy centering verified"
    );
}
