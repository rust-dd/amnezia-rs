use super::*;
use crate::interpreter::RunningEvent;
use crate::player::CameraPan;
use crate::world::{MapScene, RouteAction, RouteStepper};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const LABELS: [&str; 10] = [
    "map-animation-walk-a",
    "map-animation-walk-b",
    "map-animation-jump-a",
    "map-animation-jump-b",
    "map-animation-event-a",
    "map-animation-event-b",
    "map-animation-pan-a",
    "map-animation-pan-b",
    "map-animation-shake-a",
    "map-animation-shake-b",
];

#[derive(Resource, Default)]
struct Checks {
    pixels: Arc<AtomicUsize>,
    camera_modes: u32,
}

fn command(code: u32, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world.init_resource::<Checks>();
        assert_eq!(world.resource::<MapData>().map_id, 13);
        world
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone([100.0; 4]);
        world.spawn((
            EventSprite {
                id: 9000,
                tile_x: 60,
                tile_y: 60,
                dir: 1,
                frame: 1,
                charset: "Chara1".into(),
                index: 1,
                layer: 1,
            },
            MoveQueue::default(),
            Sprite::default(),
            Transform::default(),
            MapScene,
        ));
    }
    if (300..=940).contains(&frame) && (frame - 300).is_multiple_of(160) {
        start_case(world, (frame - 300) / 160);
    }
    if (321..=962).contains(&frame) {
        let since = frame - 321;
        if since % 160 <= 1 {
            return Some(LABELS[(since / 160 * 2 + since % 160) as usize]);
        }
    }
    if frame == 1100 {
        start_case(world, 0);
    }
    if frame == 1110 {
        assert_eq!(world.resource::<ActiveAnimations>().0, 1);
        world
            .resource_mut::<crate::teleport::PendingTeleport>()
            .reload(3, 15, 12);
    }
    if frame == 1200 {
        assert_eq!(world.resource::<MapData>().map_id, 3);
        assert_eq!(world.resource::<ActiveAnimations>().0, 0);
        return Some("map-animation-transferred");
    }
    None
}

fn start_case(world: &mut World, case: u32) {
    assert!(!world.resource::<RunningEvent>().active());
    let (x, y) = world.resource::<MapData>().tile_center(60, 60);
    {
        let (mut hero, mut queue, mut route, mut transform) = world
            .query::<(
                &mut Player,
                &mut MoveQueue,
                &mut RouteStepper,
                &mut Transform,
            )>()
            .single_mut(world)
            .unwrap();
        hero.set_tile(60, 60);
        *queue = motion(case == 1, case != 2);
        *route = default();
        transform.translation.x = x;
        transform.translation.y = y + hero.y_offset();
    }
    for (mut event, mut queue, mut transform) in world
        .query::<(&mut EventSprite, &mut MoveQueue, &mut Transform)>()
        .iter_mut(world)
    {
        if event.id != 9000 {
            continue;
        }
        event.set_tile(60, 60);
        *queue = motion(false, case == 2);
        transform.translation.x = x;
        transform.translation.y = y + event.y_offset();
    }
    let mut pan = CameraPan::default();
    pan.locked = true;
    world.insert_resource(pan);
    world
        .resource_mut::<crate::state::Switches>()
        .set(4500, false);
    let mut script = vec![
        command(11210, vec![62, if case == 2 { 9000 } else { 10001 }, 0, 0]),
        command(10210, vec![0, 4500, 4500, 0]),
    ];
    if case == 3 {
        script.push(command(11060, vec![2, 1, 3, 3, 0]));
    }
    if case == 4 {
        world.write_message(crate::screenfx::ScreenEffect::Shake {
            power: 3,
            speed: 5,
            secs: 1.0,
        });
    }
    world.resource_mut::<RunningEvent>().start(0, script);
}

fn motion(jumping: bool, moving: bool) -> MoveQueue {
    let mut queue = MoveQueue::default();
    queue.set_step_secs(0.5);
    if moving {
        queue.enqueue_route((0..4).map(|_| {
            if jumping {
                RouteAction::Jump {
                    dx: 2,
                    dy: 0,
                    face: 1,
                }
            } else {
                RouteAction::Step {
                    dx: 1,
                    dy: 0,
                    face: 1,
                }
            }
        }));
    }
    queue
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    complete: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let case = LABELS.iter().position(|&name| name == label)? / 2;
    assert!(
        world.resource::<crate::state::Switches>().get(4500),
        "nonwaiting animation must continue its event"
    );
    assert!(!world.resource::<RunningEvent>().active());
    let mut cameras = world.query_filtered::<&Transform, With<MainCamera>>();
    let mut events = world.query::<(&EventSprite, &MoveQueue)>();
    let mut heroes = world.query::<(&Player, &MoveQueue)>();
    let mut animations = world.query::<&playback::LiveAnimation>();
    let data = world.resource::<MapData>();
    let camera = cameras.single(world).unwrap().translation;
    let pan = world.resource::<CameraPan>();
    let camera_moved = (case == 3 && pan.offset.x > 0.0)
        || (case == 4 && (camera.x - pan.position.unwrap().x).abs() > 0.1);
    let (ground, elevated) = if case == 2 {
        let (event, queue) = events
            .iter(world)
            .find(|(event, _)| event.id == 9000)
            .unwrap();
        (
            queue.ground_position(event, data),
            queue.render_position(event, data),
        )
    } else {
        let (hero, queue) = heroes.single(world).unwrap();
        (
            queue.ground_position(hero, data),
            queue.render_position(hero, data),
        )
    };
    let (start_x, _) = data.tile_center(60, 60);
    assert!(
        ground.x > start_x + 8.0,
        "the target must have moved before capture"
    );
    if case == 1 {
        assert!(elevated.y > ground.y);
    }
    let (index, frame) = animations
        .single(world)
        .map(|animation| {
            assert_eq!(
                animation.map_target,
                Some(if case == 2 {
                    AnimTarget::Event(9000)
                } else {
                    AnimTarget::Hero
                })
            );
            (62, animation.frame)
        })
        .unwrap();
    assert_eq!(frame, 10);
    let def = world
        .resource::<AnimationLibrary>()
        .0
        .iter()
        .find(|def| def.id == index)
        .unwrap();
    assert_eq!((def.scope, def.position), (0, 1));
    let cells = def.frames[frame]
        .cells
        .iter()
        .filter(|cell| cell.valid)
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);
    let cell = cells[0];
    assert_eq!(
        (cell.scale, cell.transparency, cell.x, cell.y),
        (100, 0, 0, 0)
    );
    let source = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Battle", &def.animation_name));
    let image = world.resource::<Assets<Image>>().get(&source).unwrap();
    let left = 160 + ground.x.floor() as i32 - camera.x.floor() as i32 - 48;
    let top = 120 + camera.y.ceil() as i32 - ground.y.ceil() as i32 - 4 - 48;
    let mut pixels = Vec::new();
    for y in 0..96 {
        for x in 0..96 {
            let color = image
                .get_color_at(cell.cell_id % 5 * 96 + x, cell.cell_id / 5 * 96 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let (x, y) = (left + x as i32, top + y as i32);
            if color[3] == 255 && (0..320).contains(&x) && (0..240).contains(&y) {
                pixels.push((x as u32, y as u32, color));
            }
        }
    }
    assert!(pixels.len() > 300);
    let mut checks = world.resource_mut::<Checks>();
    if camera_moved {
        checks.camera_modes |= 1 << case;
    }
    Some(Snapshot {
        pixels,
        complete: checks.pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "map animation ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
        self.complete.fetch_add(1, Ordering::Relaxed);
        info!(
            "map animation: {} original effect pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Checks>().pixels.load(Ordering::Relaxed),
        LABELS.len()
    );
    assert_eq!(world.resource::<Checks>().camera_modes, (1 << 3) | (1 << 4));
}
