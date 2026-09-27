use super::*;
use crate::player::{CameraPan, saved_camera::CameraState};
use crate::vehicles::{VehicleSave, Vehicles, saved};
use crate::world::{Character, MapScene};

mod ascent;
pub(crate) mod pixels;
mod restore;

#[derive(Clone)]
struct State {
    vehicles: VehicleSave,
    motion: saved::State,
    camera: CameraState,
    hero: crate::world::saved::hero::HeroState,
}

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    saved: Option<State>,
    phase: u8,
    held: u32,
    checks: u8,
    legacy: String,
}

pub(crate) fn configure(app: &mut App) {
    ascent::configure(app);
    let slot = super::smoke_slot::Slot::new(app, "save-vehicles");
    app.insert_resource(Fixture {
        slot,
        saved: None,
        phase: 0,
        held: 0,
        checks: 0,
        legacy: String::new(),
    });
}

fn command(code: u32, params: Vec<i32>) -> amnezia_data::EventCommand {
    amnezia_data::EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

fn start(world: &mut World, commands: Vec<amnezia_data::EventCommand>) {
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(0, commands);
}

fn black_stage(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, With<MapScene>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world
            .entity_mut(entity)
            .insert(bevy::camera::visibility::RenderLayers::none());
    }
}

fn remember(world: &mut World, phase: u8) {
    let hero = crate::world::saved::hero::snapshot(world).unwrap();
    let vehicles = world.resource::<Vehicles>();
    let saved = State {
        vehicles: vehicles.save.clone(),
        motion: vehicles.motion_snapshot(),
        camera: world.resource::<CameraPan>().snapshot(),
        hero,
    };
    let mut fixture = world.resource_mut::<Fixture>();
    fixture.saved = Some(saved);
    fixture.phase = phase;
    fixture.held = 0;
    world.resource_mut::<EventSaveRequest>().0 = true;
}

fn routes() -> Vec<amnezia_data::EventCommand> {
    (0..3)
        .map(|index| {
            let mut params = vec![10002 + index, 8, 0, 0];
            if index == 0 {
                params.extend([34, 6, 67, 104, 97, 114, 97, 49, 2]);
            }
            params.extend(std::iter::repeat_n(29, if index == 2 { 4 } else { 3 }));
            params.extend([36, 40, 26, 32, 7]);
            params.extend(if index == 1 {
                vec![24, 1, 1, 2, 25]
            } else {
                vec![1, 1]
            });
            params.extend([23, 27, 41, 37, 32, 8]);
            command(11330, params)
        })
        .collect()
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(label) = restore::check(world, frame) {
        return Some(label);
    }
    let path = world.resource::<Fixture>().slot.path(world);
    match frame {
        260 => {
            world.resource_mut::<TintState>().set_tone([100.0; 4]);
            world.resource_mut::<Switches>().set(7, false);
            world.resource_mut::<Switches>().set(8, false);
            black_stage(world);
            start(
                world,
                vec![
                    command(10850, vec![0, 0, 13, 54, 62]),
                    command(10850, vec![1, 0, 13, 64, 62]),
                    command(10850, vec![2, 0, 13, 60, 60]),
                    command(10840, vec![]),
                ],
            );
        }
        275 => {
            assert!(world.resource::<Vehicles>().airship_transitioning());
            remember(world, 0);
        }
        279 | 454 => {
            let game = read_save(&path).unwrap();
            let expected = world.resource::<Fixture>().saved.as_ref().unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(game.vehicle_motion.as_ref(), Some(&expected.motion));
            assert_eq!(game.vehicles, expected.vehicles);
            assert_eq!(game.camera.as_ref(), Some(&expected.camera));
            assert_eq!(game.hero_motion.as_ref(), Some(&expected.hero));
        }
        310 | 490 | 840 => world.resource_mut::<LoadRequest>().0 = true,
        430 => {
            ascent::finish(world);
            world.resource_mut::<Fixture>().checks |= 16;
            start(world, routes());
        }
        450 => {
            assert!(world.resource::<Vehicles>().jumping(1));
            remember(world, 1);
            return Some("saved-vehicles-before");
        }
        470 => start(
            world,
            vec![
                command(10850, vec![0, 0, 13, 50, 65]),
                command(10850, vec![1, 0, 13, 66, 65]),
                command(10850, vec![2, 0, 13, 60, 64]),
            ],
        ),
        800 => {
            assert!(
                !crate::world::saved::hero::snapshot(world)
                    .unwrap()
                    .route
                    .pending()
            );
            let vehicles = world.resource::<Vehicles>();
            assert!(!vehicles.routes_pending());
            for (index, tile) in [(56, 62), (66, 63), (62, 60)].into_iter().enumerate() {
                assert_eq!(vehicles.save.vehicles[index].tile(), tile);
            }
            assert!(!world.resource::<Switches>().get(7));
            assert!(world.resource::<Switches>().get(8));
            world.resource_mut::<Fixture>().checks |= 32;
            return Some("saved-vehicles-landed");
        }
        820 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 10;
            game.hero_motion = None;
            let state = ron::to_string(&game.vehicle_motion).unwrap();
            let original = ron::to_string(&game)
                .unwrap()
                .replace(&format!(",vehicle_motion:{state}"), "");
            assert!(!original.contains("vehicle_motion"));
            std::fs::write(&path, &original).unwrap();
            world.resource_mut::<Fixture>().legacy = original;
        }
        960 => {
            black_stage(world);
            assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
            assert!(!world.resource::<Fade>().busy());
            let hero = crate::world::saved::hero::snapshot(world).unwrap();
            assert!(!hero.route.pending() && !hero.motion.into_queue().busy());
            let vehicles = world.resource::<Vehicles>();
            assert!(!vehicles.routes_pending());
            assert_eq!(vehicles.save.riding, Some(2));
            for (index, tile) in [(55, 62), (66, 63), (61, 60)].into_iter().enumerate() {
                assert_eq!(vehicles.save.vehicles[index].tile(), tile);
            }
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                world.resource::<Fixture>().legacy
            );
            world.resource_mut::<Fixture>().checks |= 64;
            return Some("saved-vehicles-legacy");
        }
        _ => {}
    }
    None
}

pub(crate) fn verify_finished(world: &mut World) {
    pixels::verify_finished(world);
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 127);
    fixture.slot.finish(world);
    info!(
        "saved vehicles: exact ascent and motion state, rider and camera sync, retained graphics and opacity, resumed routes and legacy defaults verified"
    );
}
