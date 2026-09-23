use super::*;
use crate::menu::save_files::smoke::Pixels;
use crate::windowskin::reference::{Canvas, rgba};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checks: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let (top, offset, selected) = match label {
        "save-slots-early" | "save-slots-fade" | "save-slots-cancel-fade" => (0, 0, 0),
        "save-slots-bottom"
        | "save-slots-updated"
        | "save-crystal-cancel"
        | "save-crystal-confirm"
        | "save-crystal-fade"
        | "save-crystal-erased-fade"
        | "load-slots-fade"
        | "load-slots-bottom"
        | "load-slots-reopened" => (12, 0, 14),
        "save-slots-corrupt" | "load-slots-corrupt" => (11, 0, 11),
        "load-slots-empty" => (10, 0, 10),
        "save-slots-moving" => (10, -55, 10),
        _ => return None,
    };
    let files = world.resource::<SaveFiles>();
    let loading = label.starts_with("load-");
    assert_eq!(files.mode == Mode::Load, loading);
    assert!(files.active());
    let nav = &files.navigation;
    assert_eq!((nav.top, nav.offset(), nav.index), (top, offset, selected));
    let entries = files.entries.as_ref().unwrap();
    assert_eq!(entries.len(), 15);
    if selected == 14 {
        let Contents::Party(party) = &entries[14].contents else {
            panic!("saved party preview missing")
        };
        let expected = if label != "save-slots-bottom" {
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
    let mut canvas = Canvas::new(Some(rgba(skin, 0, 32)));
    canvas.window(skin, (0, 0, 320, 32));
    add_text(
        &mut canvas,
        font,
        skin,
        (8, 8),
        (304, 16),
        vec![Run::new(
            if loading {
                "Honnan töltesz?"
            } else {
                "Hova mentesz?"
            },
            0,
            2,
            0,
        )],
    );
    canvas.clip = IRect::new(0, 40, 320, 232);
    for (index, entry) in entries.iter().enumerate() {
        let y = 40 + (index as i32 - top as i32) * 64 + offset;
        if y >= 232 || y + 64 <= 40 {
            continue;
        }
        canvas.window(skin, (0, y, 320, 64));
        if index == selected {
            canvas.cursor(
                skin,
                (4, y + 8, 47, 16),
                if nav.cursors[index] <= 10 { 64 } else { 96 },
            );
        }
        let color = if loading && !matches!(entry.contents, Contents::Party(_)) {
            crate::font::bitmap::DISABLED
        } else {
            0
        };
        let mut runs = vec![
            Run::new("File", 4, 2, color),
            Run::new(format!("{:>2}", index + 1), 31, 2, color),
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
                    canvas.blit(
                        face,
                        (96 + member as i32 * 56, y + 8),
                        (index % 4 * 48, index / 4 * 48, 48, 48),
                    );
                }
            }
        }
        add_text(&mut canvas, font, skin, (4, y + 8), (312, 48), runs);
    }
    canvas.clip = IRect::new(0, 0, 320, 240);
    let fading_in = matches!(
        label,
        "load-slots-fade" | "save-slots-fade" | "save-crystal-fade" | "save-crystal-erased-fade"
    );
    for up in [true, false] {
        if !fading_in && nav.arrow < 20 && if up { top > 0 } else { top < 12 } {
            canvas.blit(
                skin,
                (152, if up { 32 } else { 232 }),
                (40, if up { 8 } else { 16 }, 16, 8),
            );
        }
    }
    let mut pixels = canvas.pixels();
    assert_eq!(pixels.len(), 320 * 240);
    if fading_in || label == "save-slots-cancel-fade" {
        assert_eq!(world.resource::<crate::transitions::Transition>().age(), 1);
        let factor = if fading_in { 127 } else { 128 };
        for (_, _, color) in &mut pixels {
            for channel in &mut color[..3] {
                *channel = ((u32::from(*channel) * factor + 127) / 255) as u8;
            }
        }
    }
    Some(Snapshot {
        pixels,
        checks: world.resource::<Pixels>().0.clone(),
    })
}

fn add_text(
    canvas: &mut Canvas,
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
    canvas.blit(&text, position, (0, 0, size.0, size.1));
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
            "save selector: {} complete native reference pixels verified",
            self.pixels.len()
        );
    }
}
