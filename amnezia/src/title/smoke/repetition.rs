use super::*;
use bevy::ecs::message::MessageCursor;

pub(crate) const LABELS: [&str; 4] = [
    "title-repeat-down",
    "title-repeat-simultaneous",
    "title-repeat-up",
    "title-repeat-page",
];

#[derive(Resource, Default)]
struct Checks {
    audio: MessageCursor<AudioRequest>,
    frames: u32,
    sounds: u32,
}

pub(super) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        610 | 611 | 690 => Some(KeyCode::PageDown),
        612 | 613 | 695 => Some(KeyCode::PageUp),
        626 => Some(KeyCode::ArrowUp),
        630 => Some(KeyCode::ArrowDown),
        _ => None,
    }
}

pub(crate) fn held_input(world: &mut World, frame: u32) -> bool {
    let (start, pressed) = if (570..602).contains(&frame) {
        (570, vec![KeyCode::ArrowDown])
    } else if (650..682).contains(&frame) {
        (650, vec![KeyCode::ArrowUp])
    } else if frame == 620 {
        (
            620,
            vec![
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
                KeyCode::PageDown,
                KeyCode::PageUp,
            ],
        )
    } else {
        return false;
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    if frame == start {
        keys.reset_all();
    }
    for key in pressed {
        keys.press(key);
    }
    true
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 560 {
        assert!(ready(world));
        world.resource_mut::<TitleState>().cursor = NEW_GAME;
        world.insert_resource(Checks::default());
    }
    if (561..=702).contains(&frame) {
        let expected = match frame {
            571..=593 | 602..=610 | 674..=677 => 1,
            594..=597 | 611..=612 | 627..=630 | 651..=673 | 682..=695 => 2,
            _ => 0,
        };
        assert!(ready(world));
        assert_eq!(
            world.resource::<TitleState>().cursor,
            expected,
            "title input at {frame}"
        );
        world.resource_scope(|world, mut checks: Mut<Checks>| {
            let actual = checks
                .audio
                .read(world.resource::<Messages<AudioRequest>>())
                .filter(|request| matches!(request, AudioRequest::Sound { .. }))
                .cloned()
                .collect::<Vec<_>>();
            let count = match frame - 1 {
                620 => 4,
                570 | 593 | 597 | 601 | 610 | 612 | 626 | 630 | 650 | 673 | 677 | 681 | 695 => 1,
                _ => 0,
            };
            let sound = &world.resource::<SystemSounds>().cursor;
            let expected = AudioRequest::se(&sound.name, sound.volume, sound.tempo)
                .map(|request| vec![request; count])
                .unwrap_or_default();
            assert_eq!(actual, expected, "title sounds at {frame}");
            checks.frames += 1;
            checks.sounds += count as u32;
        });
    }
    match frame {
        604 => Some(LABELS[0]),
        621 => Some(LABELS[1]),
        684 => Some(LABELS[2]),
        697 => Some(LABELS[3]),
        _ => None,
    }
}

pub(super) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert_eq!((checks.frames, checks.sounds), (142, 17));
    info!("title repetition: 142 states and 17 exact cursor sounds verified");
}
