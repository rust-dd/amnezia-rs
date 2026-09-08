use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

pub struct SmokePlugin;

#[derive(Resource)]
struct SmokeRun {
    frame: u32,
    scenario: &'static str,
}

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        if !cfg!(debug_assertions) || !std::env::args().any(|arg| arg == "--smoke-test") {
            return;
        }
        let scenario = if std::env::args().any(|arg| arg == "--smoke-airship") {
            "airship"
        } else if std::env::args().any(|arg| arg == "--smoke-battle") {
            "battle"
        } else if std::env::args().any(|arg| arg == "--smoke-panorama") {
            "panorama"
        } else {
            "intro"
        };
        app.insert_resource(SmokeRun { frame: 0, scenario })
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ))
            .add_systems(
                PreUpdate,
                input
                    .after(bevy::input::InputSystems)
                    .before(crate::vehicles::VehicleInput),
            )
            .add_systems(PostUpdate, drive);
    }
}

fn capture(world: &mut World, label: &str) {
    let path = std::env::temp_dir().join(format!("amnezia-smoke-{label}.png"));
    info!("smoke screenshot: {}", path.display());
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn input(world: &mut World) {
    let frame = world.resource::<SmokeRun>().frame;
    let advance = frame > 90
        && frame.is_multiple_of(15)
        && (world.resource::<crate::dialogue::Dialogue>().active
            || world.resource::<crate::battle::BattleActive>().0);
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::Enter);
    if advance {
        keys.press(KeyCode::Enter);
    }
}

fn drive(world: &mut World) {
    let frame = {
        let mut smoke = world.resource_mut::<SmokeRun>();
        smoke.frame += 1;
        smoke.frame
    };
    if frame == 60 {
        capture(world, "title");
    }
    if frame == 90 {
        world.resource_mut::<crate::title::TitleActive>().0 = false;
        world.resource_mut::<crate::session::NewGameRequest>().0 = true;
    }
    let scenario = world.resource::<SmokeRun>().scenario;
    if frame == 150 && scenario != "intro" {
        start_scenario(world, scenario);
    }
    if frame.is_multiple_of(300) {
        let map = world.resource::<crate::world::MapData>().map_id;
        let running = world
            .resource::<crate::interpreter::RunningEvent>()
            .debug_id();
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .map(|p| (p.tile_x, p.tile_y));
        info!("smoke frame={frame} map={map} hero={hero:?} event={running:?}");
    }
    if frame == 1200 {
        capture(world, scenario);
    }
    if frame == 360 && scenario != "intro" {
        capture(world, &format!("{scenario}-early"));
    }
    if frame >= 1260 {
        if scenario == "intro" {
            assert_eq!(world.resource::<crate::world::MapData>().map_id, 3);
            assert!(
                !world
                    .resource::<crate::interpreter::RunningEvent>()
                    .active()
            );
        } else if scenario == "airship" {
            let (x, y, _) = world
                .resource::<crate::vehicles::Vehicles>()
                .character(10004)
                .unwrap();
            assert_eq!((x, y), (28, 101));
        }
        world.write_message(AppExit::Success);
    }
}

fn start_scenario(world: &mut World, scenario: &str) {
    use amnezia_data::EventCommand;
    crate::session::clear_transient(world);
    world.insert_resource(crate::teleport::Fade::default());
    world.insert_resource(crate::teleport::PendingTeleport::default());
    let commands = if scenario == "airship" {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_0125.ron",
            crate::assets::asset_root()
        ));
        let commands = &map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .find(|page| {
                page.commands
                    .iter()
                    .any(|c| c.code == 10850 && c.params == [2, 0, 13, 55, 100])
            })
            .expect("original fortress flight")
            .commands;
        let vehicle = commands.iter().position(|c| c.code == 10850).unwrap();
        let start = commands[..vehicle]
            .iter()
            .rposition(|c| c.code == 10810)
            .unwrap();
        let end = commands[vehicle..]
            .iter()
            .position(|c| c.code == 10810)
            .map_or(commands.len(), |i| vehicle + i);
        commands[start..end].to_vec()
    } else if scenario == "battle" {
        vec![EventCommand {
            code: 10710,
            indent: 0,
            string: "Cave1".into(),
            params: vec![0, 2, 1, 0, 0, 0],
        }]
    } else {
        vec![EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![94, 10, 7],
        }]
    };
    world
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(1, commands);
}
