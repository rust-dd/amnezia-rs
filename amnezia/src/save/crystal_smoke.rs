use super::{SaveAccess, SaveLocation, read_save};
use crate::interpreter::RunningEvent;
use crate::menu::save_files::SaveFiles;
use crate::player::Player;
use crate::state::{Inventory, Switches};
use crate::world::{EventSprite, MapData, MoveQueue};
use bevy::prelude::*;
use std::path::PathBuf;

mod pixels;
mod source;
pub(crate) use pixels::snapshot;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Arrival,
    WalkIn,
    OpenCancel,
    Cancel,
    OpenSave,
    Save,
    WalkOut,
    Settling,
    Done,
    Load,
}

#[derive(Resource)]
struct Probe {
    map: u32,
    crystal: amnezia_data::Event,
    approach: (i32, i32),
    path: PathBuf,
    before: Vec<u8>,
    stage: Stage,
    stage_frame: u32,
    sounds: usize,
    resume: bool,
}

pub(crate) fn resuming() -> bool {
    std::env::args().any(|arg| arg == "--smoke-crystal-resume")
}

pub(crate) fn configure(app: &mut App) {
    let map = std::env::args()
        .find_map(|arg| arg.strip_prefix("--smoke-map=").map(str::to_owned))
        .expect("crystal probes require --smoke-map=ID")
        .parse::<u32>()
        .unwrap();
    let crystal = source::crystal(map);
    let approach = source::approach(map, &crystal);
    let directory = std::env::args()
        .find_map(|arg| arg.strip_prefix("--smoke-crystal-dir=").map(PathBuf::from))
        .expect("crystal probes require an isolated --smoke-crystal-dir=PATH");
    let directory = directory.canonicalize().unwrap();
    assert!(
        directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("amnezia-crystals-")
    );
    assert!(
        directory.starts_with(std::env::temp_dir().canonicalize().unwrap())
            || directory.starts_with("/private/tmp"),
        "only a temporary probe directory is allowed"
    );
    let location = directory.join(map.to_string());
    std::fs::create_dir_all(&location).unwrap();
    let path = location.join("slot1.ron");
    let resume = resuming();
    assert_eq!(
        path.exists(),
        resume,
        "save probes never overwrite an existing slot"
    );
    let before = if resume {
        std::fs::read(&path).unwrap()
    } else {
        Vec::new()
    };
    app.insert_resource(SaveLocation(path.clone()))
        .init_resource::<pixels::Checks>()
        .insert_resource(Probe {
            map,
            crystal,
            approach,
            path,
            before,
            stage: if resume { Stage::Load } else { Stage::Arrival },
            stage_frame: 0,
            sounds: 0,
            resume,
        })
        .add_systems(Update, observe_audio);
}

fn observe_audio(
    mut requests: MessageReader<crate::audio::AudioRequest>,
    mut probe: ResMut<Probe>,
) {
    for request in requests.read() {
        if matches!(request, crate::audio::AudioRequest::Sound { name, .. } if name.eq_ignore_ascii_case("save"))
        {
            probe.sounds += 1;
        }
    }
}

pub(crate) fn entry(world: &mut World) -> Vec<amnezia_data::EventCommand> {
    let probe = world.resource::<Probe>();
    let (map, (x, y)) = (probe.map, probe.approach);
    world
        .resource_mut::<Switches>()
        .load(source::checkpoint_switches(map));
    world
        .resource_mut::<Inventory>()
        .restore(vec![(1, 3)], 137 + map as i32);
    world.resource_mut::<crate::text::HeroName>().0 = format!("Crystal-{map}");
    vec![amnezia_data::EventCommand {
        code: 10810,
        indent: 0,
        string: String::new(),
        params: vec![map as i32, x, y],
    }]
}

pub(crate) fn input(world: &mut World, frame: u32) {
    let probe = world.resource::<Probe>();
    let key = match probe.stage {
        Stage::WalkIn => Some(source::key(
            probe.approach,
            (probe.crystal.x as i32, probe.crystal.y as i32),
        )),
        Stage::WalkOut => Some(source::key(
            (probe.crystal.x as i32, probe.crystal.y as i32),
            probe.approach,
        )),
        Stage::OpenCancel | Stage::OpenSave if frame.is_multiple_of(12) => Some(KeyCode::Enter),
        Stage::Cancel if frame.is_multiple_of(12) => Some(KeyCode::Escape),
        Stage::Save if frame > 90 && frame.is_multiple_of(12) => Some(KeyCode::Enter),
        Stage::Load
            if frame > 90
                && frame.is_multiple_of(12)
                && world.resource::<crate::title::TitleActive>().0 =>
        {
            Some(KeyCode::Enter)
        }
        _ => None,
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
}

fn stage(world: &mut World, value: Stage, frame: u32) {
    let mut probe = world.resource_mut::<Probe>();
    info!("crystal {}: {:?} -> {value:?}", probe.map, probe.stage);
    probe.stage = value;
    probe.stage_frame = frame;
}

fn idle(world: &mut World) -> bool {
    !world.resource::<crate::teleport::Fade>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
        && !world.resource::<RunningEvent>().active()
        && !world.resource::<crate::dialogue::Dialogue>().busy()
        && !world.resource::<SaveFiles>().active()
        && world
            .query_filtered::<&MoveQueue, With<Player>>()
            .single(world)
            .is_ok_and(|queue| !queue.busy())
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<String> {
    if frame < 200 {
        return None;
    }
    let probe = world.resource::<Probe>();
    let (current, map, x, y, since) = (
        probe.stage,
        probe.map,
        probe.crystal.x as i32,
        probe.crystal.y as i32,
        probe.stage_frame,
    );
    let approach = probe.approach;
    let hero = world.query::<&Player>().single(world).unwrap();
    let position = (hero.tile_x, hero.tile_y);
    let ready = idle(world);
    match current {
        Stage::Arrival if ready && world.resource::<MapData>().map_id == map => {
            assert_eq!(position, approach);
            assert!(!world.resource::<SaveAccess>().0);
            stage(world, Stage::WalkIn, frame);
        }
        Stage::WalkIn if ready && position == (x, y) => {
            verify_crystal(world);
            assert!(world.resource::<SaveAccess>().0);
            assert!(world.resource::<Switches>().get(70));
            stage(world, Stage::OpenCancel, frame);
            return Some(format!("crystal-{map}-active"));
        }
        Stage::OpenCancel if world.resource::<SaveFiles>().active() => {
            stage(world, Stage::Cancel, frame)
        }
        Stage::Cancel if ready => {
            assert!(!world.resource::<Probe>().path.exists());
            assert!(world.resource::<SaveAccess>().0);
            stage(world, Stage::OpenSave, frame);
        }
        Stage::OpenSave if world.resource::<SaveFiles>().active() => {
            stage(world, Stage::Save, frame)
        }
        Stage::Save if ready && world.resource::<Probe>().path.exists() => {
            let path = &world.resource::<Probe>().path;
            let game = read_save(path).unwrap();
            assert_eq!((game.map_id, game.x, game.y), (map, x as u32, y as u32));
            assert!(game.foreground.is_some());
            assert!(game.save_access);
            assert_eq!(game.hero_name, format!("Crystal-{map}"));
            assert_eq!(game.gold, 137 + map as i32);
            assert_eq!(game.items, [(1, 3)]);
            let before = std::fs::read(path).unwrap();
            world.resource_mut::<Probe>().before = before;
            stage(world, Stage::WalkOut, frame);
        }
        Stage::Load if ready && !world.resource::<crate::title::TitleActive>().0 => {
            assert_eq!(world.resource::<MapData>().map_id, map);
            assert_eq!(position, (x, y));
            assert_eq!(
                world.resource::<crate::text::HeroName>().0,
                format!("Crystal-{map}")
            );
            assert_eq!(world.resource::<Inventory>().gold(), 137 + map as i32);
            assert_eq!(world.resource::<Inventory>().count(1), 3);
            assert!(world.resource::<SaveAccess>().0);
            assert_eq!(
                world.resource::<Probe>().sounds,
                0,
                "restoring must not replay the crystal sound"
            );
            verify_crystal(world);
            stage(world, Stage::WalkOut, frame);
            return Some(format!("crystal-{map}-restored"));
        }
        Stage::WalkOut if ready && position == approach => {
            assert!(!world.resource::<SaveAccess>().0);
            assert!(!world.resource::<Switches>().get(70));
            stage(world, Stage::Settling, frame);
        }
        Stage::Settling if ready && frame >= since + 40 => {
            stage(world, Stage::Done, frame);
            return Some(format!("crystal-{map}-continued"));
        }
        _ => {}
    }
    assert!(
        frame - since < 600,
        "crystal {map} stalled in {current:?}, hero={position:?}"
    );
    None
}

fn verify_crystal(world: &mut World) {
    let id = world.resource::<Probe>().crystal.id;
    let crystals = world
        .query::<(&EventSprite, &Sprite, &InheritedVisibility)>()
        .iter(world)
        .filter(|(event, _, _)| event.id == id)
        .collect::<Vec<_>>();
    assert_eq!(crystals.len(), 1);
    let (event, sprite, visible) = crystals[0];
    assert_eq!(
        (event.charset.as_str(), event.index, event.layer),
        ("Object3", 0, 2)
    );
    assert!(visible.get());
    assert!((sprite.color.alpha() - 159.0 / 255.0).abs() < 1e-6);
    let (sx, sy) = crate::tiles::charset_source(0, event.dir, event.frame);
    assert_eq!(sprite.rect, Some(Rect::new(sx, sy, sx + 24.0, sy + 32.0)));
}

pub(crate) fn ready(world: &World, frame: u32) -> bool {
    let probe = world.resource::<Probe>();
    probe.stage == Stage::Done && frame > probe.stage_frame + 30
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.stage, Stage::Done);
    assert_eq!(probe.sounds, if probe.resume { 0 } else { 3 });
    assert_eq!(std::fs::read(&probe.path).unwrap(), probe.before);
    pixels::verify_finished(world);
    info!(
        "crystal {}: {} and continuation verified with an unchanged isolated slot",
        probe.map,
        if probe.resume {
            "fresh-process title load"
        } else {
            "walk-in, cancellation and original event save"
        }
    );
}
