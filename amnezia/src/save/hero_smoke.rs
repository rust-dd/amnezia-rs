use super::*;
use crate::player::{CameraPan, saved_camera::CameraState};
use crate::world::{Character, MapScene, MoveQueue, RouteStepper, saved::hero};

pub(crate) mod pixels;

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<hero::HeroState>,
    camera: Option<CameraState>,
    legacy: String,
    held: u32,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-hero");
    app.insert_resource(Fixture {
        slot,
        saved: None,
        camera: None,
        legacy: String::new(),
        held: 0,
        checks: 0,
    });
}

fn start(world: &mut World, params: Vec<i32>) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![amnezia_data::EventCommand {
            code: 11330,
            params,
            indent: 0,
            string: String::new(),
        }],
    );
}

fn black_stage(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, (With<MapScene>, Without<Player>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world
            .entity_mut(entity)
            .insert(bevy::camera::visibility::RenderLayers::none());
    }
}

fn verify_fixture_obstacle(world: &mut World) {
    use crate::world::{
        EventSprite, MapEvents,
        collision::{CollisionBodies, MapCollision, Mover},
    };
    let mut events = world.query::<(&EventSprite, Option<&RouteStepper>)>();
    let mut bodies = CollisionBodies::from_events(events.iter(world));
    bodies.include_vehicles(world.get_resource::<crate::vehicles::Vehicles>(), 13);
    let collision = MapCollision::new(
        world.resource::<MapData>(),
        world.resource::<MapEvents>(),
        (
            world.resource::<Switches>(),
            world.resource::<Variables>(),
            world.resource::<Party>(),
            world.resource::<Inventory>(),
        ),
        &bodies,
    );
    assert!(!collision.can_move((62, 61), (63, 61), Mover::hero(false), None, false));
    assert!(collision.can_move((62, 61), (63, 61), Mover::hero(true), None, false));
}

fn restored(world: &mut World, frame: u32) -> Option<&'static str> {
    if !(331..450).contains(&frame) || world.contains_resource::<hero::Pending>() {
        return None;
    }
    assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
    if world.resource::<Fade>().busy() {
        let mut expected = world.resource::<Fixture>().saved.clone().unwrap();
        expected.route.reset_transparency();
        assert_eq!(hero::snapshot(world).unwrap(), expected);
        assert_eq!(
            world.resource::<CameraPan>().snapshot(),
            *world.resource::<Fixture>().camera.as_ref().unwrap()
        );
        let (hero, queue, transform, sprite) = world
            .query::<(&Player, &MoveQueue, &Transform, &Sprite)>()
            .single(world)
            .unwrap();
        assert_eq!((hero.charset.as_str(), hero.index), ("Chara4", 3));
        let point = queue.render_position(hero, world.resource::<MapData>());
        assert_eq!(
            transform.translation.truncate(),
            point + Vec2::Y * hero.y_offset()
        );
        assert_eq!(
            sprite.image,
            world
                .resource::<AssetServer>()
                .load::<Image>(crate::assets::resolve_png("CharSet", "Chara4"))
        );
        assert_eq!(sprite.color.alpha(), 1.0);
        black_stage(world);
        world.resource_mut::<Switches>().set(7, false);
        let mut fixture = world.resource_mut::<Fixture>();
        fixture.held += 1;
        fixture.checks |= 1;
    } else if world.resource::<Fixture>().checks & 2 == 0 {
        assert!(world.resource::<Fixture>().held > 30);
        let (_, queue, route) = world
            .query::<(&Player, &MoveQueue, &RouteStepper)>()
            .single(world)
            .unwrap();
        assert!(queue.jumping() && route.forced() && route.through());
        world.resource_mut::<Fixture>().checks |= 2;
        return Some("saved-hero-restored");
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
            verify_fixture_obstacle(world);
            world.resource_mut::<TintState>().set_tone([100.0; 4]);
            world
                .resource_mut::<crate::appearance::Appearance>()
                .set(1, "Chara4".into(), 3);
            world.resource_mut::<Switches>().set(7, false);
            world.resource_mut::<Switches>().set(8, false);
            black_stage(world);
        }
        270 => start(
            world,
            vec![
                10001, 8, 0, 0, 34, 6, 67, 104, 97, 114, 97, 49, 1, 29, 29, 29, 36, 26, 32, 7, 24,
                1, 1, 2, 25, 23, 27, 32, 8, 1, 37,
            ],
        ),
        290 => {
            let saved = hero::snapshot(world).unwrap();
            assert!(saved.motion.clone().into_queue().jumping());
            let camera = world.resource::<CameraPan>().snapshot();
            world.resource_mut::<Fixture>().saved = Some(saved);
            world.resource_mut::<Fixture>().camera = Some(camera);
            world.resource_mut::<EventSaveRequest>().0 = true;
            return Some("saved-hero-before");
        }
        294 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(game.hero_motion, world.resource::<Fixture>().saved);
            assert_eq!(game.camera, world.resource::<Fixture>().camera);
        }
        305 => start(
            world,
            vec![10001, 8, 0, 0, 34, 6, 80, 111, 115, 101, 115, 50, 4, 40, 23],
        ),
        330 | 640 => world.resource_mut::<LoadRequest>().0 = true,
        570 => {
            let (hero, queue, route) = world
                .query::<(&Player, &MoveQueue, &RouteStepper)>()
                .single(world)
                .unwrap();
            assert_eq!(hero.tile(), (63, 61));
            assert!(!queue.busy() && !route.pending() && !route.forced() && !route.through());
            assert!(!world.resource::<Switches>().get(7));
            assert!(world.resource::<Switches>().get(8));
            let ground = queue.ground_position(hero, world.resource::<MapData>());
            assert!(
                world
                    .resource::<CameraPan>()
                    .position
                    .unwrap()
                    .abs_diff_eq(ground + Vec2::X * 8.0, 0.001)
            );
            world.resource_mut::<Fixture>().checks |= 4;
            return Some("saved-hero-landed");
        }
        630 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 9;
            let state = ron::to_string(&game.hero_motion).unwrap();
            let original = ron::to_string(&game)
                .unwrap()
                .replace(&format!(",hero_motion:{state}"), "");
            assert!(!original.contains("hero_motion"));
            std::fs::write(&path, &original).unwrap();
            world.resource_mut::<Fixture>().legacy = original;
        }
        750 => {
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(!world.resource::<Fade>().busy());
            let (hero, queue, route) = world
                .query::<(&Player, &MoveQueue, &RouteStepper)>()
                .single(world)
                .unwrap();
            assert_eq!(hero.tile(), (62, 61));
            assert!(!queue.busy() && !route.pending());
            assert_eq!((hero.charset.as_str(), hero.index), ("Chara4", 3));
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                world.resource::<Fixture>().legacy
            );
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("saved-hero-legacy");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    pixels::verify_finished(world);
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 15);
    fixture.slot.finish(world);
    info!(
        "saved hero: exact jump and camera state, restored actor graphics, route completion without repeated commands and legacy defaults verified"
    );
}
