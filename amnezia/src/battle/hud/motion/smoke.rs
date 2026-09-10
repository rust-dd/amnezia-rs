use super::*;

pub(crate) struct Snapshot(Vec<(u32, u32, [u8; 4])>);

pub(in crate::battle) fn label(frame: u32) -> Option<&'static str> {
    match frame {
        401 => Some("battle-window-left-start"),
        405 => Some("battle-window-left-half"),
        409 => Some("battle-window-left-end"),
        410 => Some("battle-window-left-ready"),
        441 => Some("battle-window-right-start"),
        445 => Some("battle-window-right-half"),
        449 => Some("battle-window-right-end"),
        450 => Some("battle-window-right-ready"),
        _ => None,
    }
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let offset = match label {
        "battle-window-left-start" | "battle-window-right-end" | "battle-window-right-ready" => 0,
        "battle-window-left-half" | "battle-window-right-half" => -38,
        "battle-window-left-end" | "battle-window-left-ready" | "battle-window-right-start" => -76,
        _ => return None,
    };
    let windows = world.resource::<CommandWindows>();
    assert_eq!(windows.x, offset, "{label}");
    assert_eq!(windows.moving(), !label.ends_with("ready"), "{label}");
    assert_eq!(
        world.resource::<Battle>().cursor,
        0,
        "movement consumed a cursor key"
    );
    let battle = world.resource::<Battle>();
    if label.contains("left") {
        assert!(battle.phase == Phase::Command && battle.menu == MenuLevel::Command);
    } else {
        assert!(battle.phase == Phase::PartyCommand);
    }
    for (panel, node) in world.query::<(&Panel, &Node)>().iter(world) {
        let base = match panel {
            Panel::Option => 0,
            Panel::Status => 76,
            Panel::Command => 320,
            _ => continue,
        };
        let Val::Percent(actual) = node.left else {
            panic!("{label}: non-percent window")
        };
        assert!((actual - (base + offset) as f32 / 320.0 * 100.0).abs() < 0.001);
    }
    let background = world
        .query::<(&Panel, &Children)>()
        .iter(world)
        .find(|(panel, _)| **panel == Panel::Option)
        .unwrap()
        .1[0];
    let skin = world.get::<ImageNode>(background).unwrap().image.clone();
    let skin = world.resource::<Assets<Image>>().get(&skin).unwrap();
    let mut pixels = Vec::new();
    for (base, width) in [(0, 76), (76, 244), (320, 76)] {
        for (x, source_x) in [(base + offset, 32), (base + offset + width - 8, 56)] {
            for dy in 0..8 {
                for dx in 0..8 {
                    let actual_x = x + dx;
                    let expected = skin
                        .get_color_at((source_x + dx) as u32, dy)
                        .unwrap()
                        .to_srgba()
                        .to_u8_array();
                    if (0..320).contains(&actual_x) && expected[3] == 255 {
                        pixels.push((actual_x as u32, 160 + dy, expected));
                    }
                }
            }
        }
    }
    assert!(!pixels.is_empty());
    Some(Snapshot(pixels))
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.0 {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "moving window ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        info!(
            "moving windows: {} source-asset GPU pixels verified",
            self.0.len()
        );
    }
}
