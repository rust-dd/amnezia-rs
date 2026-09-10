use super::*;
use crate::animation::render::{FlashStamp, spawn_screen_flash};
use crate::animation::{ActiveAnimations, AnimAnchor, PlayAnimation};

pub(super) const COUNT: usize = 8;

const SHOTS: [(u32, &str); COUNT] = [
    (1181, "animation-sound-start"),
    (1199, "animation-sound-middle"),
    (1220, "animation-sound-last"),
    (1221, "animation-sound-finished"),
    (1231, "animation-flash-peak"),
    (1233, "animation-flash-step"),
    (1240, "animation-flash-last"),
    (1241, "animation-flash-finished"),
];

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1180 {
        if let Some(previous) = world.remove_resource::<Fixture>() {
            for entity in previous.entities {
                world.despawn(entity);
            }
        }
        let def = world
            .resource::<AnimationLibrary>()
            .0
            .iter()
            .find(|a| a.id == 1)
            .unwrap();
        assert_eq!(def.frames.len(), 58, "original Ron unarmed animation");
        world.write_message(PlayAnimation {
            anim_id: 1,
            targets: vec![AnimAnchor {
                pos: Vec2::ZERO,
                height: 24.0,
            }],
            screen_center: Vec2::ZERO,
            global: false,
            sound_only: true,
        });
    }
    if frame == 1230 {
        let now = world.resource::<crate::timing::GameFrames>().frame;
        world
            .run_system_once(move |mut commands: Commands| {
                spawn_screen_flash(
                    &mut commands,
                    [248, 160, 80],
                    31,
                    FlashStamp { age: 0, frame: now },
                );
            })
            .unwrap();
    }
    SHOTS
        .iter()
        .find(|(at, _)| *at == frame)
        .map(|(_, label)| *label)
}

pub(super) fn pixels(world: &World, label: &str) -> Option<Vec<[u8; 3]>> {
    let &(frame, _) = SHOTS.iter().find(|(_, name)| *name == label)?;
    if frame <= 1221 {
        assert_eq!(
            world.resource::<ActiveAnimations>().0,
            usize::from(frame < 1221)
        );
        return Some(vec![[0; 3]; 320 * 240]);
    }
    let alpha = match frame {
        1231 => 248,
        1233 => 200,
        1240 => 80,
        _ => 0,
    };
    let color = [248u32, 160, 80].map(|v| ((v * alpha + 127) / 255) as u8);
    Some(vec![color; 320 * 240])
}
