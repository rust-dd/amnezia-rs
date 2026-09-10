use super::*;
use crate::legacy_colors::flash::SpriteFlash;

struct CharacterView {
    charset: String,
    source: (f32, f32),
    tile: (i32, i32),
    position: Vec3,
    opacity: u8,
    flash: [u8; 4],
}

fn character(
    actor: &impl Character,
    sprite: &Sprite,
    transform: &GlobalTransform,
    flash: Option<&SpriteFlash>,
) -> CharacterView {
    CharacterView {
        charset: actor.charset().into(),
        source: crate::tiles::charset_source(actor.index(), actor.dir(), actor.frame()),
        tile: actor.tile(),
        position: transform.translation(),
        opacity: (sprite.color.alpha() * 255.0).round() as u8,
        flash: flash.map_or([0; 4], |flash| flash.0),
    }
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 3])>,
    complete: Arc<AtomicUsize>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    let index = LABELS.iter().position(|&value| value == label)?;
    let case = index / 3;
    let active = index % 3 < 2;
    assert!(world.resource::<crate::state::Switches>().get(4501));
    assert!(!world.resource::<RunningEvent>().active());
    let camera = world
        .query_filtered::<&GlobalTransform, With<MainCamera>>()
        .single(world)
        .unwrap()
        .translation();
    let actor = match target(case) {
        AnimTarget::Hero => {
            let (actor, sprite, transform, flash) = world
                .query::<(&Player, &Sprite, &GlobalTransform, Option<&SpriteFlash>)>()
                .single(world)
                .unwrap();
            character(actor, sprite, transform, flash)
        }
        AnimTarget::Event(id) => {
            let (actor, sprite, transform, flash) = world
                .query::<(
                    &EventSprite,
                    &Sprite,
                    &GlobalTransform,
                    Option<&SpriteFlash>,
                )>()
                .iter(world)
                .find(|(event, _, _, _)| event.id == id)
                .unwrap();
            character(actor, sprite, transform, flash)
        }
    };
    let expected_flash = if active { [248, 168, 0, 200] } else { [0; 4] };
    assert_eq!(actor.flash, expected_flash, "{label}");
    assert_eq!(actor.opacity, opacity(case));
    if active {
        let animation = world
            .query::<&playback::LiveAnimation>()
            .single(world)
            .unwrap();
        assert_eq!(animation.frame, 20);
        assert_eq!(animation.map_target, Some(target(case)));
    } else {
        assert_eq!(world.resource::<ActiveAnimations>().total, 0);
    }
    let data = world.resource::<MapData>();
    let bush = data
        .terrain_at(actor.tile.0, actor.tile.1)
        .unwrap()
        .bush_depth;
    assert_eq!(bush, u32::from(case >= 4));
    let (ground_x, ground_y) = data.tile_center(actor.tile.0, actor.tile.1);
    let effect_left = 160 + ground_x as i32 - camera.x as i32 + 32 - 24;
    let effect_top = 120 + camera.y as i32 - ground_y as i32 - 4 - 56 - 24;
    let animation = world
        .resource::<AnimationLibrary>()
        .0
        .iter()
        .find(|def| def.id == 62)
        .unwrap();
    let cell = animation.frames[20]
        .cells
        .iter()
        .find(|cell| cell.valid)
        .unwrap();
    assert_eq!(
        (cell.cell_id, cell.scale, cell.transparency, cell.x, cell.y),
        (0, 50, 50, 32, -56)
    );
    let server = world.resource::<AssetServer>();
    let charset = server.load::<Image>(crate::assets::resolve_png("CharSet", &actor.charset));
    let images = world.resource::<Assets<Image>>();
    let charset = images.get(&charset).unwrap();
    let left = (160.0 + actor.position.x - camera.x - 12.0).round() as i32;
    let top = (120.0 - actor.position.y + camera.y - 16.0).round() as i32;
    assert!(
        !active || top >= effect_top + 48 || left + 24 <= effect_left || left >= effect_left + 48
    );
    let mut pixels = Vec::new();
    let mut lower_pixels = 0;
    for y in 0..32 {
        for x in 0..24 {
            let source = charset
                .get_color_at(actor.source.0 as u32 + x, actor.source.1 as u32 + y)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            let (screen_x, screen_y) = (left + x as i32, top + y as i32);
            if source[3] != 255 || !(0..320).contains(&screen_x) || !(0..240).contains(&screen_y) {
                continue;
            }
            let rgb =
                crate::legacy_colors::tone::apply([source[0], source[1], source[2]], tone(case));
            let rgb = blend_flash(rgb, expected_flash);
            let alpha = if bush == 1 && y >= 22 {
                lower_pixels += 1;
                u32::from(actor.opacity).div_ceil(2)
            } else {
                u32::from(actor.opacity)
            };
            pixels.push((
                screen_x as u32,
                screen_y as u32,
                rgb.map(|c| ((u32::from(c) * alpha + 127) / 255) as u8),
            ));
        }
    }
    assert!(
        pixels.len() > 50,
        "{label}: enough unobscured original character pixels"
    );
    if bush == 1 {
        assert!(
            lower_pixels > 10,
            "{label}: the real bush child must also be sampled"
        );
    }
    Some(Snapshot {
        pixels,
        complete: world.resource::<Fixture>().complete.clone(),
    })
}

fn blend_flash(rgb: [u8; 3], flash: [u8; 4]) -> [u8; 3] {
    std::array::from_fn(|i| {
        let product = u32::from(rgb[i]) * u32::from(255 - flash[3]) + 128;
        let original = (product + (product >> 8)) >> 8;
        let overlay = (u32::from(flash[i]) * u32::from(flash[3])) >> 8;
        (original + overlay).min(255) as u8
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(&a, b)| a.abs_diff(b) <= 1),
                "map target flash ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
        self.complete.fetch_add(1, Ordering::Relaxed);
        info!(
            "map target flash: {} original character pixels, including tone/opacity/bush, verified",
            self.pixels.len()
        );
    }
}
