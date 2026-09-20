use super::InputNumber;
use crate::audio::{AudioRequest, SystemSounds};
use crate::interpreter::RunningEvent;
use crate::state::Variables;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: usize,
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    if frame < 1120 {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.clear();
    if !(1131..1190).contains(&frame) {
        keys.reset_all();
    }
    let pressed = match frame {
        1130..=1189 | 1194 => &[KeyCode::ArrowUp][..],
        1192 | 1227 => &[KeyCode::ArrowLeft],
        1196 => &[KeyCode::ArrowDown, KeyCode::ArrowRight],
        1198 => &[KeyCode::Escape],
        1204 => &[
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::ArrowLeft,
        ],
        1208 => &[KeyCode::Enter, KeyCode::Space],
        1225 => &[KeyCode::ArrowRight],
        1229 => &[KeyCode::ArrowDown, KeyCode::ArrowUp],
        1231 => &[KeyCode::ArrowDown, KeyCode::Enter],
        _ => &[],
    };
    for key in pressed {
        keys.press(*key);
    }
    true
}

fn start(world: &mut World, map_id: u32, variable: i32) {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    let command = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|command| command.code == 10150 && command.params[1] == variable)
        .unwrap()
        .clone();
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<RunningEvent>().start(0, vec![command]);
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1120 {
        world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
        world.init_resource::<Checks>();
        start(world, 247, 76);
    }
    if frame == 1220 {
        start(world, 106, 24);
    }
    if (1131..=1212).contains(&frame) || (1223..=1242).contains(&frame) {
        verify_frame(world, frame);
    }
    match frame {
        1191 => Some("number-held-units"),
        1195 => Some("number-edited-tens"),
        _ => None,
    }
}

fn verify_frame(world: &mut World, frame: u32) {
    let number = world.resource::<InputNumber>();
    assert_eq!(
        number.active(),
        frame <= 1208 || (1223..=1231).contains(&frame),
        "number input at {frame}"
    );
    let value = match frame {
        1131..=1189 => (1 + frame.saturating_sub(1150) / 4) as i64 % 10,
        1195..=1196 => 11,
        1190..=1212 => 1,
        1232..=1242 => 9,
        _ => 0,
    };
    assert_eq!(number.value, value, "number value at {frame}");
    let index = if frame >= 1223 {
        0
    } else if (1193..=1196).contains(&frame) {
        2
    } else {
        3
    };
    assert_eq!(number.cursor, index, "number digit at {frame}");
    if (1210..=1212).contains(&frame) {
        assert_eq!(world.resource::<Variables>().get(76), 1);
    }
    if frame >= 1233 {
        assert_eq!(world.resource::<Variables>().get(24), 9);
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let actual = checks
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        let count = match frame - 1 {
            1130 | 1153 | 1157 | 1161 | 1165 | 1169 | 1173 | 1177 | 1181 | 1185 | 1189 | 1192
            | 1194 | 1227 | 1229 | 1231 => 1,
            1196 => 2,
            1204 => 3,
            _ => 0,
        };
        let sounds = world.resource::<SystemSounds>();
        let mut expected = vec![
            AudioRequest::se(
                &sounds.cursor.name,
                sounds.cursor.volume,
                sounds.cursor.tempo
            )
            .unwrap();
            count
        ];
        if matches!(frame - 1, 1208 | 1231) {
            expected.push(
                AudioRequest::se(
                    &sounds.decision.name,
                    sounds.decision.volume,
                    sounds.decision.tempo,
                )
                .unwrap(),
            );
        }
        assert_eq!(actual, expected, "number sounds at {frame}");
        checks.frames += 1;
        checks.sounds += actual.len();
    });
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (102, 23));
    assert_eq!(world.resource::<Variables>().get(76), 1);
    assert_eq!(world.resource::<Variables>().get(24), 9);
    assert!(!world.resource::<InputNumber>().active());
    assert!(!world.resource::<RunningEvent>().active());
    info!(
        "number input: 102 states, 23 exact sounds, held editing and original event variable handoffs verified"
    );
}
