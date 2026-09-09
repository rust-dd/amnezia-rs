use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

mod scenarios;

pub struct SmokePlugin;

#[derive(Resource)]
struct SmokeRun {
    frame: u32,
    scenario: &'static str,
    escaped_cast: u8,
    finish_at: Option<u32>,
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
        } else if std::env::args().any(|arg| arg == "--smoke-timer") {
            "timer"
        } else if std::env::args().any(|arg| arg == "--smoke-panorama") {
            "panorama"
        } else if std::env::args().any(|arg| arg == "--smoke-airship-escape") {
            "escape"
        } else if std::env::args().any(|arg| arg == "--smoke-font") {
            "font"
        } else if std::env::args().any(|arg| arg == "--smoke-menu") {
            "menu"
        } else {
            "intro"
        };
        app.insert_resource(SmokeRun {
            frame: 0,
            scenario,
            escaped_cast: 0,
            finish_at: None,
        })
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
        && world.resource::<SmokeRun>().finish_at.is_none()
        && !matches!(
            world.resource::<SmokeRun>().scenario,
            "font" | "panorama" | "timer"
        )
        && frame.is_multiple_of(15)
        && (world.resource::<crate::dialogue::Dialogue>().active
            || world.resource::<crate::battle::BattleActive>().0);
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    *keys = ButtonInput::default();
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
    if frame == 300 && scenario == "menu" {
        world.resource_mut::<crate::menu::MenuOpen>().0 = true;
        world
            .resource_mut::<crate::state::Party>()
            .restore(vec![1, 2, 3, 4]);
    }
    if frame == 900 && scenario == "timer" {
        assert!(world.resource::<crate::battle::BattleActive>().0);
        world.resource_mut::<crate::timer::GameClock>().remaining = 1.0;
    }
    if scenario == "escape" {
        let escaped = scenarios::escaped_airship_cast(world);
        world.resource_mut::<SmokeRun>().escaped_cast |= escaped;
        if world.resource::<crate::world::MapData>().map_id == 86
            && world.resource::<SmokeRun>().finish_at.is_none()
        {
            assert_eq!(world.resource::<SmokeRun>().escaped_cast, 0b11111);
            assert!(!world.resource::<crate::player::HeroHidden>().0);
            world.resource_mut::<SmokeRun>().finish_at = Some(frame + 90);
        }
        if world.resource::<SmokeRun>().finish_at == Some(frame + 10) {
            assert!(!world.resource::<crate::teleport::Fade>().busy());
            assert_eq!(
                world.resource::<crate::screenfx::TintState>().tone(),
                [100.0; 4]
            );
            capture(world, "escape");
        }
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
    if frame == 1200 && scenario != "escape" {
        capture(world, scenario);
    }
    if frame == 360 && scenario != "intro" {
        capture(world, &format!("{scenario}-early"));
    }
    let finish = world
        .resource::<SmokeRun>()
        .finish_at
        .unwrap_or(if scenario == "escape" { 2400 } else { 1260 });
    if frame >= finish {
        if scenario == "escape" {
            assert!(
                world.resource::<SmokeRun>().finish_at.is_some(),
                "airship escape never reached the forest scene"
            );
        }
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
        } else if scenario == "panorama" {
            assert_eq!(world.resource::<crate::world::MapData>().map_id, 94);
            assert!(world.resource::<crate::player::HeroHidden>().0);
            let (hero, visibility) = world
                .query::<(&crate::player::Player, &Visibility)>()
                .single(world)
                .unwrap();
            assert_eq!((hero.tile_x, hero.tile_y), (9, 4));
            assert_eq!(*visibility, Visibility::Hidden);
            scenarios::verify_airship_staging(world);
        } else if scenario == "timer" {
            let clock = world.resource::<crate::timer::GameClock>();
            assert_eq!(clock.seconds(), 0);
            assert!(!clock.running && !clock.visible);
            assert!(!world.resource::<crate::battle::BattleActive>().0);
            assert!(!world.resource::<crate::gameover::GameOverActive>().0);
        }
        world.write_message(AppExit::Success);
    }
}

fn start_scenario(world: &mut World, scenario: &str) {
    use amnezia_data::EventCommand;
    crate::session::clear_transient(world);
    world.insert_resource(crate::teleport::Fade::default());
    world.insert_resource(crate::teleport::PendingTeleport::default());
    world.insert_resource(crate::player::HeroHidden::default());
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
    } else if matches!(scenario, "battle" | "timer") {
        let mut commands = if scenario == "timer" {
            let mut commands = vec![EventCommand {
                code: 10810,
                indent: 0,
                string: String::new(),
                params: vec![3, 15, 6],
            }];
            commands.extend(scenarios::mission_timer_start());
            commands
        } else {
            Vec::new()
        };
        commands.push(EventCommand {
            code: 10710,
            indent: 0,
            string: "Cave1".into(),
            params: vec![0, 2, 1, 0, 0, 0],
        });
        commands
    } else if scenario == "font" {
        vec![
            EventCommand {
                code: 10810,
                indent: 0,
                string: String::new(),
                params: vec![3, 15, 6],
            },
            EventCommand {
                code: 10130,
                indent: 0,
                string: "Ron".into(),
                params: vec![0, 0, 0],
            },
            EventCommand {
                code: 10110,
                indent: 0,
                string: "Hát... hol vagyok?".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "Árvíztűrő tükörfúrógép.".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "Őrült éjszaka volt!".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "0123456789 ÁÉÍÓÖŐÚÜŰ".into(),
                params: vec![],
            },
        ]
    } else if scenario == "menu" {
        vec![EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![3, 15, 6],
        }]
    } else if matches!(scenario, "panorama" | "escape") {
        scenarios::airship_interior_entry()
    } else {
        unreachable!("unknown smoke scenario: {scenario}")
    };
    world
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(1, commands);
}
