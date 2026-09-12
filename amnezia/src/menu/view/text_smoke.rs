use super::*;
use crate::font::bitmap::BitmapFont;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Checks(Arc<AtomicUsize>);

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 800 {
        world.init_resource::<Checks>();
        world.resource_mut::<crate::save::SaveAccess>().0 = false;
        let mut vitals = world.resource_mut::<Vitals>();
        vitals.set(1, 10, 0);
        vitals.set(2, 0, 75);
        vitals.set_states(3, vec![2, 3]);
    }
    if frame == 880 {
        world.resource_mut::<crate::save::SaveAccess>().0 = true;
    }
    if frame == 980 {
        world.resource_mut::<Party>().restore(Vec::new());
    }
    if frame == 1060 {
        world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
        world.resource_mut::<Vitals>().heal_all();
        world.resource_mut::<crate::save::SaveAccess>().0 = false;
    }
    match frame {
        820 => Some("menu-font-status"),
        920 => Some("menu-font-save-enabled"),
        1000 => Some("menu-font-empty"),
        _ => None,
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "menu-font-status" | "menu-font-save-enabled" | "menu-font-empty"
    ) {
        return None;
    }
    assert!(world.resource::<MenuOpen>().0);
    let party = world.resource::<Party>();
    let empty = label == "menu-font-empty";
    assert_eq!(party.snapshot().is_empty(), empty);
    let save = world.resource::<crate::save::SaveAccess>().0;
    assert_eq!(save, label != "menu-font-status");
    if !empty {
        let vitals = world.resource::<Vitals>();
        assert_eq!(vitals.get_stored(1), Some((10, 0)));
        assert_eq!(vitals.states(2), [1]);
        assert_eq!(vitals.states(3), [2, 3]);
    }
    let members = render::members(
        world.resource::<crate::text::HeroName>(),
        world.resource::<GameData>(),
        party,
        world.resource::<Progression>(),
        world.resource::<Vitals>(),
    );
    let font = world.resource::<BitmapFont>();
    let terms = world.resource::<Terms>();
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = Vec::new();
    let mut add = |left, top, width, runs| {
        let text = PixelText {
            size: UVec2::new(width, 16),
            runs,
        };
        let image = font.render(&text, skin);
        for y in 0..16 {
            for x in 0..width {
                let color = image.get_color_at(x, y).unwrap().to_srgba().to_u8_array();
                if color[3] == 255 {
                    pixels.push((left + x, top + y, color));
                }
            }
        }
    };
    for (index, &command) in command::COMMANDS.iter().enumerate() {
        add(
            8,
            10 + index as u32 * 16,
            72,
            vec![Run::new(
                command::label(command, terms),
                0,
                0,
                main_text::command_color(index, members.len(), save),
            )],
        );
    }
    add(
        8,
        218,
        72,
        main_text::gold(world.resource::<Inventory>().gold(), terms, font),
    );
    for (slot, member) in members.iter().enumerate() {
        for (field, left, top, width) in [
            (MemberField::Name, 152, 10, 160),
            (MemberField::Title, 240, 10, 72),
            (MemberField::Level, 152, 26, 160),
            (MemberField::Condition, 194, 26, 118),
            (MemberField::Hp, 258, 26, 54),
            (MemberField::Exp, 152, 42, 160),
            (MemberField::Sp, 258, 42, 54),
        ] {
            add(
                left,
                top + slot as u32 * 58,
                width,
                main_text::member(member, field, terms, font),
            );
        }
    }
    if empty {
        for (slot, visible) in world
            .query::<(&MenuText, &InheritedVisibility)>()
            .iter(world)
        {
            if matches!(slot.0, TextSlot::Member { .. }) {
                assert!(!visible.get());
            }
        }
    }
    assert!(pixels.len() > 200);
    Some(Snapshot {
        pixels,
        checks: world.resource::<Checks>().0.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "field text ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "field text: {} original glyph and shadow pixels verified",
            self.pixels.len()
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 3);
}
