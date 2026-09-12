use super::*;
use crate::world::{EventSprite, MapEvents, MapScene, saved};

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Vec<saved::EventState>,
    legacy: String,
    held: u32,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-npcs");
    app.insert_resource(Fixture {
        slot,
        saved: Vec::new(),
        legacy: String::new(),
        held: 0,
        checks: 0,
    });
}

fn start(world: &mut World, code: u32, params: Vec<i32>) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![amnezia_data::EventCommand {
            code,
            params,
            indent: 0,
            string: String::new(),
        }],
    );
}

fn black_stage(world: &mut World) {
    let tiles = world
        .query_filtered::<Entity, (With<MapScene>, Without<EventSprite>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in tiles {
        world.despawn(entity);
    }
    let other_events = world
        .query::<(Entity, &EventSprite)>()
        .iter(world)
        .filter_map(|(entity, event)| (event.id != 1).then_some(entity))
        .collect::<Vec<_>>();
    for entity in other_events {
        world
            .entity_mut(entity)
            .insert(bevy::camera::visibility::RenderLayers::none());
    }
}

fn restored(world: &mut World, frame: u32) -> Option<&'static str> {
    if !(331..450).contains(&frame) || world.contains_resource::<saved::Pending>() {
        return None;
    }
    assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
    if world.resource::<Fade>().busy() {
        assert_eq!(saved::snapshot(world), world.resource::<Fixture>().saved);
        black_stage(world);
        let mut fixture = world.resource_mut::<Fixture>();
        fixture.held += 1;
        fixture.checks |= 1;
    } else if world.resource::<Fixture>().checks & 2 == 0 {
        assert!(world.resource::<Fixture>().held > 30);
        let (_, queue, route) = world
            .query::<(
                &EventSprite,
                &crate::world::MoveQueue,
                &crate::world::RouteStepper,
            )>()
            .iter(world)
            .find(|(event, _, _)| event.id == 1)
            .unwrap();
        assert!(queue.busy() && route.forced() && route.through());
        world.resource_mut::<Fixture>().checks |= 2;
        return Some("saved-npc-restored");
    }
    None
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = restored(world, frame) {
        return Some(label);
    }
    let path = world.resource::<Fixture>().slot.path(world);
    match frame {
        260 => {
            world.resource_mut::<TintState>().set_tone([100.0; 4]);
            black_stage(world);
            start(world, 10860, vec![1, 0, 10, 10]);
        }
        270 => start(
            world,
            11330,
            vec![
                1, 8, 0, 0, 34, 6, 67, 104, 97, 114, 97, 49, 1, 29, 29, 29, 36, 26, 1, 1, 27, 37,
            ],
        ),
        290 => {
            let state = saved::snapshot(world);
            world.resource_mut::<Fixture>().saved = state;
            world.resource_mut::<EventSaveRequest>().0 = true;
            return Some("saved-npc-before");
        }
        294 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(game.map_events, world.resource::<Fixture>().saved);
        }
        305 => start(world, 10860, vec![1, 0, 8, 9]),
        330 | 640 => world.resource_mut::<LoadRequest>().0 = true,
        570 => {
            let (event, queue, route) = world
                .query::<(
                    &EventSprite,
                    &crate::world::MoveQueue,
                    &crate::world::RouteStepper,
                )>()
                .iter(world)
                .find(|(event, _, _)| event.id == 1)
                .unwrap();
            assert_eq!((event.tile_x, event.tile_y), (12, 10));
            assert_eq!((event.charset.as_str(), event.index), ("Chara1", 1));
            assert!(!queue.busy() && !route.pending() && !route.forced() && !route.through());
            let logical = world
                .resource::<MapEvents>()
                .events
                .iter()
                .find(|event| event.id == 1)
                .unwrap();
            assert_eq!((logical.x, logical.y), (12, 10));
            world.resource_mut::<Fixture>().checks |= 4;
            return Some("saved-npc-landed");
        }
        630 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 8;
            let state = ron::to_string(&game.map_events).unwrap();
            let original = ron::to_string(&game)
                .unwrap()
                .replace(&format!(",map_events:{state}"), "");
            assert!(!original.contains("map_events"));
            std::fs::write(&path, &original).unwrap();
            world.resource_mut::<Fixture>().legacy = original;
        }
        750 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(!world.resource::<Fade>().busy());
            let event = world
                .query::<&EventSprite>()
                .iter(world)
                .find(|event| event.id == 1)
                .unwrap();
            assert_eq!((event.tile_x, event.tile_y), (3, 5));
            assert_ne!((event.charset.as_str(), event.index), ("Chara1", 1));
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                world.resource::<Fixture>().legacy
            );
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("saved-npc-legacy");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    saved::smoke::verify_finished(world);
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 15);
    fixture.slot.finish(world);
    info!(
        "saved NPCs: exact rebuilt state, paused and resumed motion, route completion and legacy defaults verified"
    );
}
