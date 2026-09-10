use super::*;
use crate::animation::render::{FlashStamp, spawn_screen_flash};
use crate::animation::{ActiveAnimations, AnimAnchor, PlayAnimation};

pub(super) const COUNT: usize = 12;

const SHOTS: [(u32, &str); COUNT] = [
    (1181, "animation-sound-start"),
    (1199, "animation-sound-middle"),
    (1220, "animation-sound-last"),
    (1221, "animation-sound-finished"),
    (1231, "animation-flash-peak"),
    (1233, "animation-flash-step"),
    (1240, "animation-flash-last"),
    (1241, "animation-flash-finished"),
    (1243, "animation-cell-flash-peak"),
    (1245, "animation-cell-flash-step"),
    (1252, "animation-cell-flash-last"),
    (1253, "animation-cell-flash-finished"),
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
    if frame == 1242 {
        spawn_flash_cell(world);
    }
    if matches!(frame, 1230 | 1242) {
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
        world
            .run_system_once(super::super::cells::sync_flash)
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
        1231 | 1243 => 248,
        1233 | 1245 => 200,
        1240 | 1252 => 80,
        _ => 0,
    };
    if frame >= 1243 {
        return Some(reference_pixels(
            world.resource::<Fixture>(),
            world.resource::<Assets<Image>>(),
            [248, 160, 80, alpha as u8],
        ));
    }
    let color = [248u32, 160, 80].map(|v| ((v * alpha + 127) / 255) as u8);
    Some(vec![color; 320 * 240])
}

fn spawn_flash_cell(world: &mut World) {
    let mut def = world
        .resource::<AnimationLibrary>()
        .0
        .iter()
        .find(|a| a.id == 23)
        .unwrap()
        .clone();
    let mut cell = def
        .frames
        .iter()
        .flat_map(|frame| &frame.cells)
        .find(|cell| cell.valid)
        .unwrap()
        .clone();
    cell.x = 0;
    cell.y = 0;
    cell.scale = 99;
    cell.transparency = 40;
    cell.tone_red = 100;
    cell.tone_green = 100;
    cell.tone_blue = 100;
    cell.tone_gray = 100;
    let image = world
        .resource::<AssetServer>()
        .load(crate::assets::resolve_png("Battle", &def.animation_name));
    def.frames = vec![AnimationFrameDef {
        cells: vec![cell.clone()],
    }];
    let entities = world
        .run_system_once(move |mut renderer: CellRenderer, mut commands: Commands| {
            renderer.spawn_frame(&mut commands, &def, 0, Vec2::ZERO)
        })
        .unwrap();
    world.insert_resource(Fixture {
        cell,
        image,
        entities,
    });
}
