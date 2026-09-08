use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

pub struct SmokePlugin;

#[derive(Resource, Default)]
struct SmokeRun {
    frame: u32,
}

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        if !cfg!(debug_assertions) || !std::env::args().any(|arg| arg == "--smoke-test") {
            return;
        }
        app.init_resource::<SmokeRun>()
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
    let advance =
        frame > 90 && frame % 15 == 0 && world.resource::<crate::dialogue::Dialogue>().active;
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
    if frame % 300 == 0 {
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
        capture(world, "intro");
    }
    if frame >= 1260 {
        world.write_message(AppExit::Success);
    }
}
