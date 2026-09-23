use super::*;
use crate::animation::{AnimationSlot, PlayAnimation};
use amnezia_data::{AnimationTimingDef, EventCommand};

pub(super) const COUNT: usize = 6;

const SHOTS: [(u32, &str, [u8; 4]); COUNT] = [
    (
        1265,
        "flash-channel-event-over-battle-plane",
        [248, 0, 0, 248],
    ),
    (1270, "flash-channel-event-decay", [248, 0, 0, 227]),
    (
        1283,
        "flash-channel-animation-replaces-event",
        [0, 0, 248, 248],
    ),
    (
        1285,
        "flash-channel-foreground-replaces-animation",
        [0, 248, 0, 160],
    ),
    (
        1286,
        "flash-channel-next-animation-update",
        [0, 0, 248, 200],
    ),
    (1305, "flash-channel-no-old-event-resume", [0; 4]),
];

fn event(world: &mut World, channels: [i32; 4]) {
    let mut running = world.resource_mut::<crate::interpreter::RunningEvent>();
    assert!(!running.active());
    running.start(
        0,
        vec![EventCommand {
            code: 11040,
            indent: 0,
            string: String::new(),
            params: channels.into_iter().chain([10, 0]).collect(),
        }],
    );
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        1264 => {
            if let Some(previous) = world.remove_resource::<Fixture>() {
                for entity in previous.entities {
                    world.despawn(entity);
                }
            }
            event(world, [31, 0, 0, 31]);
        }
        1280 => {
            let mut library = world.resource_mut::<AnimationLibrary>();
            let mut definition = library.0[0].clone();
            definition.id = 9000;
            definition.frames = vec![AnimationFrameDef { cells: Vec::new() }; 10];
            definition.timings = vec![AnimationTimingDef {
                frame: 1,
                se_name: String::new(),
                se_volume: 100,
                se_tempo: 100,
                flash_scope: 2,
                flash_red: 0,
                flash_green: 0,
                flash_blue: 31,
                flash_power: 31,
            }];
            library.0.push(definition);
            world.write_message(PlayAnimation {
                slot: AnimationSlot::Enemies,
                anim_id: 9000,
                map_target: None,
                targets: Vec::new(),
                screen_center: Vec2::ZERO,
                global: false,
                sound_only: false,
            });
        }
        1284 => event(world, [0, 31, 0, 20]),
        _ => {}
    }
    SHOTS
        .iter()
        .find(|(at, _, _)| *at == frame)
        .map(|(_, label, _)| *label)
}

pub(super) fn pixels(world: &mut World, label: &str) -> Option<Vec<[u8; 3]>> {
    let &(_, _, expected) = SHOTS.iter().find(|(_, name, _)| *name == label)?;
    let sprite = world
        .query_filtered::<&Sprite, With<crate::screenfx::FlashOverlay>>()
        .single(world)
        .unwrap();
    assert_eq!(sprite.color.to_srgba().to_u8_array(), expected, "{label}");
    let color = [expected[0], expected[1], expected[2]]
        .map(|channel| ((u32::from(channel) * u32::from(expected[3]) + 127) / 255) as u8);
    Some(vec![color; 320 * 240])
}
