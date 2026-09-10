use super::*;
use crate::interpreter::RunningEvent;
use crate::player::CameraPan;
use crate::world::{MapScene, RouteStepper};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;
pub(crate) use pixels::snapshot;

const LABELS: [&str; 18] = [
    "map-flash-hero-a",
    "map-flash-hero-b",
    "map-flash-hero-restored",
    "map-flash-event-a",
    "map-flash-event-b",
    "map-flash-event-restored",
    "map-flash-toned-a",
    "map-flash-toned-b",
    "map-flash-toned-restored",
    "map-flash-alpha-a",
    "map-flash-alpha-b",
    "map-flash-alpha-restored",
    "map-flash-bush-a",
    "map-flash-bush-b",
    "map-flash-bush-restored",
    "map-flash-mixed-a",
    "map-flash-mixed-b",
    "map-flash-mixed-restored",
];

#[derive(Resource)]
struct Fixture {
    tiles: [(i32, i32); 2],
    complete: Arc<AtomicUsize>,
    menu_hold: Option<(usize, [u8; 4])>,
    menu_checks: u8,
}

fn tone(case: usize) -> [f32; 4] {
    if matches!(case, 2 | 5) {
        [50.0, 100.0, 150.0, 0.0]
    } else {
        [100.0; 4]
    }
}

fn opacity(case: usize) -> u8 {
    if matches!(case, 3 | 5) { 128 } else { 255 }
}

fn target(case: usize) -> AnimTarget {
    if matches!(case, 1 | 5) {
        AnimTarget::Event(9000)
    } else {
        AnimTarget::Hero
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        let scene = world
            .query_filtered::<Entity, With<MapScene>>()
            .iter(world)
            .collect::<Vec<_>>();
        for entity in scene {
            world.despawn(entity);
        }
        let data = world.resource::<MapData>();
        assert_eq!(data.map_id, 13);
        let tiles = [0, 1].map(|depth| {
            (10..data.height - 10)
                .flat_map(|y| (10..data.width - 10).map(move |x| (x, y)))
                .find(|&(x, y)| {
                    data.terrain_at(x, y)
                        .is_some_and(|terrain| terrain.bush_depth == depth)
                })
                .expect("original map 13 supplies bare and forest terrain")
        });
        world.insert_resource(Fixture {
            tiles,
            complete: Arc::new(AtomicUsize::new(0)),
            menu_hold: None,
            menu_checks: 0,
        });
        world.spawn((
            EventSprite {
                id: 9000,
                tile_x: 60,
                tile_y: 60,
                dir: 2,
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
    if (300..=900).contains(&frame) && (frame - 300).is_multiple_of(120) {
        start_case(world, ((frame - 300) / 120) as usize);
    }
    if (341..=990).contains(&frame) {
        let case = ((frame - 300) / 120) as usize;
        let phase = match (frame - 300) % 120 {
            41 => Some(0),
            42 => Some(1),
            90 => Some(2),
            _ => None,
        };
        return phase.map(|phase| LABELS[case * 3 + phase]);
    }
    menu_case(world, frame)
}

fn menu_case(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1040 {
        start_case(world, 0);
    }
    if frame == 1070 {
        world.resource_mut::<crate::menu::MenuOpen>().0 = true;
    }
    if matches!(frame, 1072 | 1130 | 1180) {
        let animation = world
            .query::<&playback::LiveAnimation>()
            .single(world)
            .unwrap();
        let current = animation.frame;
        assert!(current > 5);
        let cells = animation.cells.clone();
        let flash = world
            .query_filtered::<&crate::legacy_colors::flash::SpriteFlash, With<Player>>()
            .single(world)
            .unwrap()
            .0;
        let hidden = frame != 1180;
        for cell in cells {
            assert_eq!(
                world.get::<InheritedVisibility>(cell).unwrap().get(),
                !hidden
            );
        }
        let mut fixture = world.resource_mut::<Fixture>();
        if frame == 1072 {
            assert!(flash[3] > 0);
            fixture.menu_hold = Some((current, flash));
            fixture.menu_checks |= 1;
        } else if hidden {
            assert_eq!(fixture.menu_hold, Some((current, flash)));
            fixture.menu_checks |= 2;
        } else {
            assert!(current > fixture.menu_hold.unwrap().0);
            fixture.menu_checks |= 4;
        }
        return Some(if hidden {
            "map-animation-menu-held"
        } else {
            "map-animation-menu-resumed"
        });
    }
    if frame == 1160 {
        world.resource_mut::<crate::menu::MenuOpen>().0 = false;
    }
    None
}

fn start_case(world: &mut World, case: usize) {
    assert!(!world.resource::<RunningEvent>().active());
    let tile = world.resource::<Fixture>().tiles[usize::from(case >= 4)];
    let target = target(case);
    let hero_tile = if target == AnimTarget::Hero {
        tile
    } else {
        (tile.0 - 6, tile.1)
    };
    let event_tile = if target == AnimTarget::Hero {
        (tile.0 + 6, tile.1)
    } else {
        tile
    };
    let data = world.resource::<MapData>();
    let (hero_x, hero_y) = data.tile_center(hero_tile.0, hero_tile.1);
    let (event_x, event_y) = data.tile_center(event_tile.0, event_tile.1);
    let (camera_x, camera_y) = data.tile_center(tile.0, tile.1);
    {
        let (mut hero, mut queue, mut route, mut sprite, mut transform) = world
            .query::<(
                &mut Player,
                &mut MoveQueue,
                &mut RouteStepper,
                &mut Sprite,
                &mut Transform,
            )>()
            .single_mut(world)
            .unwrap();
        hero.set_tile(hero_tile.0, hero_tile.1);
        hero.dir = 2;
        hero.frame = 1;
        *queue = default();
        *route = default();
        sprite.color = Color::WHITE.with_alpha(f32::from(opacity(case)) / 255.0);
        transform.translation.x = hero_x;
        transform.translation.y = hero_y + hero.y_offset();
    }
    for (mut event, mut sprite, mut transform) in world
        .query::<(&mut EventSprite, &mut Sprite, &mut Transform)>()
        .iter_mut(world)
    {
        if event.id == 9000 {
            event.set_tile(event_tile.0, event_tile.1);
            sprite.color = Color::WHITE.with_alpha(f32::from(opacity(case)) / 255.0);
            transform.translation.x = event_x;
            transform.translation.y = event_y + event.y_offset();
        }
    }
    let mut pan = CameraPan::default();
    pan.locked = true;
    pan.position = Some(Vec2::new(camera_x, camera_y));
    world.insert_resource(pan);
    world
        .resource_mut::<crate::screenfx::TintState>()
        .set_tone(tone(case));
    world
        .resource_mut::<crate::state::Switches>()
        .set(4501, false);
    world.resource_mut::<RunningEvent>().start(
        0,
        vec![
            amnezia_data::EventCommand {
                code: 11210,
                params: vec![
                    62,
                    if target == AnimTarget::Hero {
                        10001
                    } else {
                        9000
                    },
                    0,
                    0,
                ],
                indent: 0,
                string: String::new(),
            },
            amnezia_data::EventCommand {
                code: 10210,
                params: vec![0, 4501, 4501, 0],
                indent: 0,
                string: String::new(),
            },
        ],
    );
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Fixture>().complete.load(Ordering::Relaxed),
        LABELS.len()
    );
    assert_eq!(world.resource::<Fixture>().menu_checks, 7);
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    assert_eq!(world.resource::<ActiveAnimations>().total, 0);
}
