use super::Choice;
use crate::audio::{AudioRequest, SystemSounds};
use crate::interpreter::RunningEvent;
use crate::state::Variables;
use amnezia_data::EventCommand;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: usize,
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    if frame < 1250 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    if !(1261..1320).contains(&frame) {
        keys.reset_all();
    }
    let pressed = match frame {
        1260..=1319 | 1340 => &[KeyCode::ArrowDown][..],
        1325 => &[KeyCode::PageUp],
        1330 | 1335 | 1400 => &[KeyCode::PageDown],
        1345 | 1375 => &[KeyCode::ArrowUp],
        1350 | 1380 => &[KeyCode::Escape, KeyCode::Enter],
        1355 => &[KeyCode::Space],
        1395 => &[KeyCode::ArrowDown, KeyCode::ArrowUp],
        1405 => &[KeyCode::Enter, KeyCode::Space],
        _ => &[],
    };
    for key in pressed {
        keys.press(*key);
    }
    true
}

fn command(code: u32, indent: u32, text: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: text.into(),
        params,
    }
}

fn start(world: &mut World, count: usize, cancel: i32) {
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<Variables>().set(9012, -1);
    let mut commands = vec![command(10140, 0, "", vec![cancel])];
    for (index, text) in ["Első", "Második", "Harmadik", "Negyedik"]
        .into_iter()
        .take(count)
        .enumerate()
    {
        commands.push(command(20140, 0, text, vec![index as i32]));
        commands.push(command(
            10220,
            1,
            "",
            vec![0, 9012, 9012, 0, 0, index as i32, 0],
        ));
    }
    commands.push(command(20141, 0, "", Vec::new()));
    world.resource_mut::<RunningEvent>().start(0, commands);
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        1236 => {
            world.init_resource::<Checks>();
            start(world, 4, 0);
        }
        1363 => start(world, 2, 2),
        1386 => start(world, 1, 0),
        _ => {}
    }
    if (1255..=1362).contains(&frame)
        || (1373..=1385).contains(&frame)
        || (1393..=1412).contains(&frame)
    {
        verify_frame(world, frame);
    }
    match frame {
        1321 => Some("choice-held-last"),
        1341 => Some("choice-wrapped-first"),
        _ => None,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let choice = world.resource::<Choice>();
    let index = match frame {
        1261..=1320 => (1 + frame.saturating_sub(1280) / 4) as usize % 4,
        1321..=1325 | 1331..=1340 | 1346..=1362 => 3,
        1376..=1385 => 1,
        _ => 0,
    };
    assert_eq!(choice.cursor, index, "choice cursor at {frame}");
    assert_eq!(
        choice.active(),
        frame <= 1355 || (1373..=1380).contains(&frame) || (1393..=1405).contains(&frame),
        "choice activity at {frame}"
    );
    if (1360..=1362).contains(&frame) {
        assert_eq!(world.resource::<Variables>().get(9012), 3);
    }
    if (1383..=1385).contains(&frame) {
        assert_eq!(world.resource::<Variables>().get(9012), 1);
    }
    if frame >= 1408 {
        assert_eq!(world.resource::<Variables>().get(9012), 0);
    }
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let sounds = world.resource::<SystemSounds>();
        let cursor = match frame - 1 {
            1260 | 1283 | 1287 | 1291 | 1295 | 1299 | 1303 | 1307 | 1311 | 1315 | 1319 | 1325
            | 1330 | 1340 | 1345 | 1375 => 1,
            1395 => 2,
            _ => 0,
        };
        let mut expected = vec![
            AudioRequest::se(
                &sounds.cursor.name,
                sounds.cursor.volume,
                sounds.cursor.tempo
            )
            .unwrap();
            cursor
        ];
        let extra = match frame - 1 {
            1355 | 1405 => Some(&sounds.decision),
            1380 => Some(&sounds.cancel),
            _ => None,
        };
        if let Some(sound) = extra {
            expected.push(AudioRequest::se(&sound.name, sound.volume, sound.tempo).unwrap());
        }
        assert_eq!(actual, expected, "choice sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len();
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (141, 21));
    assert_eq!(world.resource::<Variables>().get(9012), 0);
    assert!(!world.resource::<Choice>().active());
    assert!(!world.resource::<RunningEvent>().active());
    info!(
        "dialogue choices: 141 states, 21 exact sounds, held wrapping, pages, cancellation and three event branches verified"
    );
}
