use super::*;
use crate::menu::save_files::smoke::Pixels;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

type Sample = (u32, u32, [u8; 4]);

pub(crate) struct Snapshot {
    pixels: Vec<Sample>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (top, offset, selected) = match label {
        "save-slots-early" => (0, 0, 0),
        "save-slots-bottom" | "save-slots-updated" => (12, 0, 14),
        "save-slots-corrupt" => (11, 0, 11),
        "save-slots-moving" => (10, -55, 10),
        _ => return None,
    };
    let files = world.resource::<SaveFiles>();
    assert!(files.active());
    let nav = &files.navigation;
    assert_eq!((nav.top, nav.offset(), nav.index), (top, offset, selected));
    let entries = files.entries.as_ref().unwrap();
    assert_eq!(entries.len(), 15);
    if selected == 14 {
        let Contents::Party(party) = &entries[14].contents else {
            panic!("saved party preview missing")
        };
        let expected = if label == "save-slots-updated" {
            ("Áron", 23)
        } else {
            ("Álmos", 17)
        };
        assert_eq!((party.name.as_str(), party.hp), expected);
        assert_eq!(party.level, 2);
        assert_eq!(party.faces.len(), 4);
    }
    let server = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let skin = images
        .get(&server.load("graphics/System/System.png"))
        .unwrap();
    let font = world.resource::<BitmapFont>();
    let terms = world.resource::<Terms>();
    assert_eq!(
        (&*terms.0.file, &*terms.0.lvl_short, &*terms.0.hp_short),
        ("File", "Sz", "HP")
    );
    let mut pixels = Vec::new();
    border(&mut pixels, skin, 0, 320, 32, 32, false);
    add_text(
        &mut pixels,
        font,
        skin,
        (8, 8),
        (304, 16),
        vec![Run::new("Hova mentesz?", 0, 2, 0)],
    );
    for (index, entry) in entries.iter().enumerate() {
        let y = 40 + (index as i32 - top as i32) * 64 + offset;
        if y >= 232 || y + 64 <= 40 {
            continue;
        }
        border(&mut pixels, skin, y, 320, 64, 32, true);
        if index == selected {
            cursor(
                &mut pixels,
                skin,
                y + 8,
                if nav.cursors[index] <= 10 { 64 } else { 96 },
            );
        }
        let mut runs = vec![
            Run::new("File", 4, 2, 0),
            Run::new(format!("{:>2}", index + 1), 31, 2, 0),
        ];
        match &entry.contents {
            Contents::Empty => {}
            Contents::Corrupt => runs.push(Run::new("Savegame corrupted", 4, 18, 5)),
            Contents::Party(party) => {
                runs.extend([
                    Run::new(&party.name, 4, 18, 0),
                    Run::new("Sz", 4, 34, 1),
                    Run::new(format!("{:>2}", party.level), 16, 34, 0),
                    Run::new("HP", 46, 34, 1),
                    Run::new(format!("{:>3}", party.hp), 58, 34, 0),
                ]);
                for (member, (name, index)) in party.faces.iter().enumerate() {
                    let face = images
                        .get(&server.load(crate::assets::resolve_png("FaceSet", name)))
                        .unwrap();
                    for cy in 0..48 {
                        for cx in 0..48 {
                            sample(
                                &mut pixels,
                                face,
                                (96 + member as i32 * 56 + cx, y + 8 + cy),
                                (index % 4 * 48 + cx as u32, index / 4 * 48 + cy as u32),
                                true,
                            );
                        }
                    }
                }
            }
        }
        add_text(&mut pixels, font, skin, (4, y + 8), (312, 48), runs);
    }
    arrows(&mut pixels, skin, top, nav.arrow < 20);
    Some(Snapshot {
        pixels,
        checks: world.resource::<Pixels>().0.clone(),
    })
}

fn sample(
    pixels: &mut Vec<Sample>,
    image: &Image,
    target: (i32, i32),
    source: (u32, u32),
    clipped: bool,
) {
    if !(0..320).contains(&target.0)
        || !(0..240).contains(&target.1)
        || (clipped && !(40..232).contains(&target.1))
    {
        return;
    }
    let color = image
        .get_color_at(source.0, source.1)
        .unwrap()
        .to_srgba()
        .to_u8_array();
    if color[3] == 255 {
        pixels.push((target.0 as u32, target.1 as u32, color));
    }
}

fn border(
    pixels: &mut Vec<Sample>,
    skin: &Image,
    top: i32,
    width: u32,
    height: u32,
    origin: u32,
    clipped: bool,
) {
    let source = |position: u32, length: u32| {
        if position < 8 {
            position
        } else if position >= length - 8 {
            24 + position - (length - 8)
        } else {
            8 + (position - 8) % 16
        }
    };
    for y in 0..height {
        for x in 0..width {
            if x >= 8 && y >= 8 && x + 8 < width && y + 8 < height {
                continue;
            }
            sample(
                pixels,
                skin,
                (x as i32, top + y as i32),
                (origin + source(x, width), source(y, height)),
                clipped,
            );
        }
    }
}

fn cursor(pixels: &mut Vec<Sample>, skin: &Image, top: i32, origin: u32) {
    for y in 0..16 {
        for x in 0..47 {
            if x != 0 && x != 46 && y != 0 && y != 15 {
                continue;
            }
            let sx = if x < 8 {
                x
            } else if x >= 39 {
                24 + x - 39
            } else {
                8 + (x - 8) % 16
            };
            let sy = if y < 8 { y } else { y + 16 };
            sample(
                pixels,
                skin,
                (4 + x, top + y),
                (origin + sx as u32, sy as u32),
                true,
            );
        }
    }
}

fn add_text(
    pixels: &mut Vec<Sample>,
    font: &BitmapFont,
    skin: &Image,
    position: (i32, i32),
    size: (u32, u32),
    runs: Vec<Run>,
) {
    let text = font.render(
        &PixelText {
            size: UVec2::new(size.0, size.1),
            runs,
        },
        skin,
    );
    for y in 0..size.1 {
        for x in 0..size.0 {
            sample(
                pixels,
                &text,
                (position.0 + x as i32, position.1 + y as i32),
                (x, y),
                size.1 == 48,
            );
        }
    }
}

fn arrows(pixels: &mut Vec<Sample>, skin: &Image, top: usize, light: bool) {
    let background = skin.get_color_at(0, 32).unwrap().to_srgba().to_u8_array();
    for up in [true, false] {
        let visible = light && if up { top > 0 } else { top < 12 };
        for y in 0..8 {
            for x in 0..320 {
                let mut color = background;
                if visible && (152..168).contains(&x) {
                    let arrow = skin
                        .get_color_at(40 + x - 152, if up { 8 + y } else { 16 + y })
                        .unwrap()
                        .to_srgba()
                        .to_u8_array();
                    if arrow[3] == 255 {
                        color = arrow;
                    }
                }
                pixels.push((x, if up { 32 + y } else { 232 + y }, color));
            }
        }
    }
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "save selector ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checks.fetch_add(1, Ordering::Relaxed);
        info!(
            "save selector: {} original skin, text and portrait pixels verified",
            self.pixels.len()
        );
    }
}
