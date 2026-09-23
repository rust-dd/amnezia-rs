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
    if frame < 1262 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    if !(1273..1332).contains(&frame) {
        keys.reset_all();
    }
    let pressed = match frame {
        1272..=1331 | 1352 => &[KeyCode::ArrowDown][..],
        1337 => &[KeyCode::PageUp],
        1342 | 1347 | 1436 => &[KeyCode::PageDown],
        1357 | 1399 => &[KeyCode::ArrowUp],
        1362 | 1404 => &[KeyCode::Escape, KeyCode::Enter],
        1367 => &[KeyCode::Space],
        1431 => &[KeyCode::ArrowDown, KeyCode::ArrowUp],
        1441 => &[KeyCode::Enter, KeyCode::Space],
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
        1375 => start(world, 2, 2),
        1410 => start(world, 1, 0),
        _ => {}
    }
    if (1267..=1374).contains(&frame)
        || (1397..=1409).contains(&frame)
        || (1429..=1448).contains(&frame)
    {
        verify_frame(world, frame);
    }
    match frame {
        1333 => Some("choice-held-last"),
        1353 => Some("choice-wrapped-first"),
        _ => None,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let choice = world.resource::<Choice>();
    let index = match frame {
        1273..=1332 => (1 + frame.saturating_sub(1292) / 4) as usize % 4,
        1333..=1337 | 1343..=1352 | 1358..=1374 => 3,
        1400..=1409 => 1,
        _ => 0,
    };
    assert_eq!(choice.cursor, index, "choice cursor at {frame}");
    assert_eq!(
        choice.active(),
        frame <= 1367 || (1397..=1404).contains(&frame) || (1429..=1441).contains(&frame),
        "choice activity at {frame}"
    );
    if (1372..=1374).contains(&frame) {
        assert_eq!(world.resource::<Variables>().get(9012), 3);
    }
    if (1407..=1409).contains(&frame) {
        assert_eq!(world.resource::<Variables>().get(9012), 1);
    }
    if frame >= 1444 {
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
            1272 | 1295 | 1299 | 1303 | 1307 | 1311 | 1315 | 1319 | 1323 | 1327 | 1331 | 1337
            | 1342 | 1352 | 1357 | 1399 => 1,
            1431 => 2,
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
            1367 | 1441 => Some(&sounds.decision),
            1404 => Some(&sounds.cancel),
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
