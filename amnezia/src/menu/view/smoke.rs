use super::*;
use crate::menu::equip::smoke as equip_selection;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod navigation;

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        435 | 480 | 510 | 540 | 1010 | 1030 => Some(KeyCode::ArrowDown),
        450 | 995 | 1020 | 1040 => Some(KeyCode::Enter),
        620 | 760 | 780 => Some(KeyCode::Escape),
        640 => Some(KeyCode::ArrowUp),
        1050 => Some(KeyCode::ArrowRight),
        _ => equip_selection::input(frame).or_else(|| crate::menu::navigation_smoke::input(frame)),
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let fade = crate::menu::scene::smoke::drive(world, frame);
    equip_selection::drive(world, frame);
    let navigation = crate::menu::navigation_smoke::drive(world, frame);
    if frame == 300 {
        world.init_resource::<Checks>();
        world.resource_mut::<MenuOpen>().0 = true;
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
    }
    if frame == 700 {
        world
            .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
            .single_mut(world)
            .unwrap()
            .resolution
            .set(1001.0, 751.0);
    }
    match frame {
        373 => Some("menu-cursor-light"),
        570 => Some("menu-member-blink"),
        580 => Some("menu-layout-member"),
        740 => Some("menu-layout-resized"),
        798 => Some("menu-cursor-reopen"),
        _ => super::text_smoke::drive(world, frame)
            .or(navigation)
            .or(fade),
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if matches!(label, "menu-scene-fade-in" | "menu-scene-fade-out") {
        assert_eq!(world.resource::<crate::transitions::Transition>().age(), 1);
        let factor = if label == "menu-scene-fade-in" {
            127
        } else {
            128
        };
        let mut pixels = super::main_pixels::compose(world);
        for (_, _, color) in &mut pixels {
            for channel in &mut color[..3] {
                *channel = ((u32::from(*channel) * factor + 127) / 255) as u8;
            }
        }
        return Some(Snapshot {
            pixels,
            checks: world.resource::<Checks>().0.clone(),
        });
    }
    if let Some(snapshot) = navigation::snapshot(world, label) {
        return Some(snapshot);
    }
    if !matches!(
        label,
        "menu-early"
            | "menu-cursor-light"
            | "menu-member-blink"
            | "menu-layout-member"
            | "menu-layout-resized"
            | "menu-cursor-reopen"
    ) {
        return None;
    }
    let member = matches!(label, "menu-layout-member" | "menu-member-blink");
    let expected = if member {
        MenuScreen::MemberSelect {
            action: crate::menu::MemberAction::Skill,
            cursor: 3,
        }
    } else {
        MenuScreen::Command
    };
    assert_eq!(world.resource::<MenuState>().screen, expected);
    assert_eq!(world.resource::<Party>().snapshot(), [1, 2, 3, 4]);
    let command_source = if matches!(label, "menu-early" | "menu-layout-resized") {
        96
    } else {
        64
    };
    let clock = world.resource::<clocks::Clock>();
    assert_eq!(
        clock.source_x(CursorId::Command) as u32,
        command_source,
        "{label}"
    );
    if member {
        let source = if label == "menu-member-blink" { 96 } else { 64 };
        assert_eq!(clock.source_x(CursorId::Status) as u32, source, "{label}");
    }
    let faces = world
        .query::<(&MenuFace, &ImageNode, &InheritedVisibility)>()
        .iter(world)
        .map(|(face, image, visible)| {
            (
                face.0,
                image.image.clone(),
                image.rect.unwrap(),
                visible.get(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(faces.len(), 4);
    let actors = world.resource::<GameData>();
    for (slot, image, rect, visible) in faces {
        assert!(visible);
        let actor = actors.actor(slot as u32 + 1).unwrap();
        assert_eq!(
            image,
            world
                .resource::<AssetServer>()
                .load(resolve_png("FaceSet", &actor.face_name))
        );
        assert_eq!(
            rect.min,
            Vec2::new(
                (actor.face_index % 4 * 48) as f32,
                (actor.face_index / 4 * 48) as f32
            )
        );
    }
    let pixels = super::main_pixels::compose(world);
    assert_eq!(pixels.len(), 320 * 240);
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().0.clone(),
    })
}

pub(super) fn border(
    pixels: &mut Vec<(u32, u32, [u8; 4])>,
    skin: &Image,
    rect: (u32, u32, u32, u32),
    source_x: u32,
    outer_only: bool,
) {
    let (left, top, width, height) = rect;
    for y in 0..height {
        for x in 0..width {
            if outer_only {
                if x != 0 && y != 0 && x + 1 != width && y + 1 != height {
                    continue;
                }
            } else if x >= 8 && y >= 8 && x + 8 < width && y + 8 < height {
                continue;
            }
            let coordinate = |position, length| {
                if position < 8 {
                    position
                } else if position >= length - 8 {
                    24 + position - (length - 8)
                } else {
                    8 + (position - 8) % 16
                }
            };
            sample(
                pixels,
                skin,
                (left + x, top + y),
                (source_x + coordinate(x, width), coordinate(y, height)),
            );
        }
    }
}

pub(super) fn sample(
    pixels: &mut Vec<(u32, u32, [u8; 4])>,
    image: &Image,
    target: (u32, u32),
    source: (u32, u32),
) {
    let color = image
        .get_color_at(source.0, source.1)
        .unwrap()
        .to_srgba()
        .to_u8_array();
    if color[3] == 255 {
        pixels.push((target.0, target.1, color));
    }
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "field menu ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "field menu: {} original reference pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    crate::menu::scene::smoke::verify_finished(world);
    equip_selection::verify_finished(world);
    crate::menu::navigation_smoke::verify_finished(world);
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 8);
}
