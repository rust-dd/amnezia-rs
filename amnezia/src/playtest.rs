use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::time::TimeUpdateStrategy;
use std::path::PathBuf;
use std::time::Duration;

mod clock;
mod snapshot;
mod storage;
#[cfg(test)]
mod tests;

pub(crate) struct PlaytestPlugin;

pub(crate) fn enabled() -> bool {
    cfg!(debug_assertions) && std::env::args().any(|arg| arg == "--playtest")
}

pub(crate) fn windowed() -> bool {
    enabled() && std::env::args().any(|arg| arg == "--playtest-window")
}

#[derive(Resource)]
struct Controller {
    directory: PathBuf,
    id: u64,
    remaining: u32,
    elapsed: u32,
    interval: u32,
    keys: Vec<KeyCode>,
    input: ButtonInput<KeyCode>,
    capture: bool,
    last_message: String,
    clock: clock::Clock,
}

impl Plugin for PlaytestPlugin {
    fn build(&self, app: &mut App) {
        if !enabled() {
            return;
        }
        assert!(
            !std::env::args().any(|arg| arg == "--smoke-test"),
            "playtest cannot run a smoke fixture"
        );
        let mut args = std::env::args();
        let directory = args
            .find(|arg| arg == "--playtest")
            .and_then(|_| args.next())
            .map(PathBuf::from)
            .expect("--playtest requires an isolated directory");
        std::fs::create_dir_all(directory.join("saves")).unwrap();
        let directory = directory.canonicalize().unwrap();
        app.insert_resource(crate::save::SaveLocation(directory.join("saves/slot1.ron")));
        crate::smoke::offscreen::configure(app);
        if windowed() {
            app.insert_resource(bevy::winit::WinitSettings::continuous());
        }
        let id = storage::next_id(&directory);
        app.insert_resource(Controller {
            directory,
            id,
            remaining: 120,
            elapsed: 0,
            interval: 0,
            keys: Vec::new(),
            input: default(),
            capture: true,
            last_message: String::new(),
            clock: clock::Clock::new(windowed()),
        })
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_systems(First, poll.before(bevy::time::TimeSystems))
        .add_systems(PreUpdate, input.after(bevy::input::InputSystems))
        .add_systems(Last, finish);
    }
}

fn parse(text: &str) -> Result<(u64, u32, u32, Vec<KeyCode>, bool), String> {
    let mut fields = text.split_whitespace();
    let id = fields
        .next()
        .ok_or("missing id")?
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let frames = fields
        .next()
        .ok_or("missing frames")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let interval = fields
        .next()
        .ok_or("missing interval")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    if !(1..=36000).contains(&frames) {
        return Err("frames must be between 1 and 36000".into());
    }
    let mut keys = Vec::new();
    let mut capture = false;
    for field in fields {
        let key = match field {
            "up" => KeyCode::ArrowUp,
            "right" => KeyCode::ArrowRight,
            "down" => KeyCode::ArrowDown,
            "left" => KeyCode::ArrowLeft,
            "enter" => KeyCode::Enter,
            "space" => KeyCode::Space,
            "escape" => KeyCode::Escape,
            "save" => KeyCode::KeyS,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "shift" => KeyCode::ShiftLeft,
            "capture" => {
                capture = true;
                continue;
            }
            _ => return Err(format!("unknown key {field}")),
        };
        keys.push(key);
    }
    Ok((id, frames, interval, keys, capture))
}

fn poll(mut controller: ResMut<Controller>, mut strategy: ResMut<TimeUpdateStrategy>) {
    let mut started = false;
    if controller.remaining == 0 {
        let path = controller.directory.join("command.txt");
        if let Ok(text) = std::fs::read_to_string(&path) {
            match parse(&text) {
                Ok((id, frames, interval, keys, capture)) if id > controller.id => {
                    controller.id = id;
                    controller.remaining = frames;
                    controller.elapsed = 0;
                    controller.interval = interval;
                    controller.keys = keys;
                    controller.capture = capture;
                    started = true;
                    storage::report(storage::append(
                        &controller.directory.join("inputs.log"),
                        text.trim(),
                    ));
                }
                Err(error) => {
                    storage::report(std::fs::write(
                        controller.directory.join("error.txt"),
                        error,
                    ));
                }
                _ => {}
            }
            storage::report(std::fs::remove_file(path));
        }
    }
    let remaining = controller.remaining;
    *strategy = TimeUpdateStrategy::ManualDuration(controller.clock.poll(remaining, started));
}

fn input(mut controller: ResMut<Controller>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    controller.input.clear();
    let desired = if controller.remaining > 0
        && pulse_due(
            controller.elapsed,
            controller.clock.ticks,
            controller.interval,
        ) {
        controller.keys.clone()
    } else {
        Vec::new()
    };
    let held = controller.input.get_pressed().copied().collect::<Vec<_>>();
    for key in held {
        if !desired.contains(&key) {
            controller.input.release(key);
        }
    }
    for key in desired {
        controller.input.press(key);
    }
    *keys = controller.input.clone();
}

fn pulse_due(elapsed: u32, ticks: u32, interval: u32) -> bool {
    interval == 0
        || elapsed.is_multiple_of(interval)
        || elapsed / interval != (elapsed + ticks.saturating_sub(1)) / interval
}

fn finish(world: &mut World) {
    let remaining = world.resource::<Controller>().remaining;
    if remaining == 0 {
        return;
    }
    snapshot::log_message(world);
    let (directory, id, capture) = {
        let mut controller = world.resource_mut::<Controller>();
        let ticks = controller.clock.ticks;
        controller.remaining -= ticks;
        controller.elapsed += ticks;
        if controller.remaining != 0 {
            return;
        }
        (
            controller.directory.clone(),
            controller.id,
            controller.capture,
        )
    };
    snapshot::write(world, &directory, id, capture);
    if capture {
        let screenshot = world
            .get_resource::<crate::smoke::offscreen::Target>()
            .map_or_else(Screenshot::primary_window, |target| {
                Screenshot::image(target.0.clone())
            });
        world
            .spawn(screenshot)
            .observe(save_to_disk(directory.join(format!("screen-{id:06}.png"))));
    }
}
