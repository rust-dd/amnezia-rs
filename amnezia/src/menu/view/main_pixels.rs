use super::*;
use crate::font::bitmap::BitmapFont;

pub(super) fn compose(world: &World) -> Vec<(u32, u32, [u8; 4])> {
    let server = world.resource::<AssetServer>();
    let images = world.resource::<Assets<Image>>();
    let handle = server.load("graphics/System/System.png");
    let skin = images.get(&handle).unwrap();
    let font = world.resource::<BitmapFont>();
    let terms = world.resource::<Terms>();
    let state = world.resource::<MenuState>();
    let clock = world.resource::<clocks::Clock>();
    let members = render::members(
        world.resource::<crate::text::HeroName>(),
        world.resource::<GameData>(),
        world.resource::<Party>(),
        world.resource::<Progression>(),
        world.resource::<Vitals>(),
    );
    let mut canvas = Canvas(vec![rgba(skin, 0, 32); 320 * 240]);
    for rect in [(0, 0, 88, 96), (0, 208, 88, 32), (88, 0, 232, 240)] {
        canvas.window(skin, rect);
    }
    canvas.cursor(
        skin,
        (4, 8 + state.cursor as u32 * 16, 80, 16),
        clock.source_x(CursorId::Command) as u32,
    );
    if let MenuScreen::MemberSelect { cursor, .. } = state.screen {
        canvas.cursor(
            skin,
            (148, 8 + cursor as u32 * 58, 168, 48),
            clock.source_x(CursorId::Status) as u32,
        );
    } else {
        assert_eq!(state.screen, MenuScreen::Command);
    }
    let save = world.resource::<crate::save::SaveAccess>().0;
    for (index, &command) in command::COMMANDS.iter().enumerate() {
        canvas.text(
            font,
            skin,
            (8, 10 + index as u32 * 16, 72),
            vec![Run::new(
                command::label(command, terms),
                0,
                0,
                main_text::command_color(index, members.len(), save),
            )],
        );
    }
    canvas.text(
        font,
        skin,
        (8, 218, 72),
        main_text::gold(world.resource::<Inventory>().gold(), terms, font),
    );
    for (slot, member) in members.iter().enumerate() {
        let top = slot as u32 * 58;
        if !member.face_name.is_empty() {
            let handle = server.load(resolve_png("FaceSet", &member.face_name));
            let face = images.get(&handle).unwrap();
            canvas.blit(
                face,
                (96, 8 + top),
                (
                    member.face_index % 4 * 48,
                    member.face_index / 4 * 48,
                    48,
                    48,
                ),
            );
        }
        let vital = main_text::vital_width(member);
        for (field, left, y, width) in [
            (MemberField::Name, 152, 10, 160),
            (MemberField::Title, 240, 10, 72),
            (MemberField::Level, 152, 26, 160),
            (MemberField::Condition, 194, 26, 118),
            (MemberField::Exp, 152, 42, 160),
            (MemberField::Hp, 312 - vital, 26, vital),
            (MemberField::Sp, 312 - vital, 42, vital),
        ] {
            canvas.text(
                font,
                skin,
                (left, top + y, width),
                main_text::member(member, field, terms, font),
            );
        }
    }
    canvas
        .0
        .into_iter()
        .enumerate()
        .map(|(index, pixel)| (index as u32 % 320, index as u32 / 320, pixel))
        .collect()
}

struct Canvas(Vec<[u8; 4]>);

impl Canvas {
    fn window(&mut self, skin: &Image, (left, top, width, height): (u32, u32, u32, u32)) {
        for y in 0..height {
            for x in 0..width {
                let mut pixel = rgba(skin, background(x, width), background(y, height));
                if x < 8 || y < 8 || x >= width - 8 || y >= height - 8 {
                    pixel = over(rgba(skin, 32 + tile(x, width), tile(y, height)), pixel);
                }
                self.0[((top + y) * 320 + left + x) as usize] = pixel;
            }
        }
    }

    fn cursor(&mut self, skin: &Image, rect: (u32, u32, u32, u32), source: u32) {
        let (left, top, width, height) = rect;
        for y in 0..height {
            for x in 0..width {
                self.put(
                    left + x,
                    top + y,
                    rgba(skin, source + tile(x, width), tile(y, height)),
                );
            }
        }
    }

    fn text(&mut self, font: &BitmapFont, skin: &Image, rect: (u32, u32, u32), runs: Vec<Run>) {
        let (left, top, width) = rect;
        let text = font.render(
            &PixelText {
                size: UVec2::new(width, 16),
                runs,
            },
            skin,
        );
        self.blit(&text, (left, top), (0, 0, width, 16));
    }

    fn blit(&mut self, image: &Image, target: (u32, u32), source: (u32, u32, u32, u32)) {
        let (sx, sy, width, height) = source;
        for y in 0..height {
            for x in 0..width {
                self.put(target.0 + x, target.1 + y, rgba(image, sx + x, sy + y));
            }
        }
    }

    fn put(&mut self, x: u32, y: u32, foreground: [u8; 4]) {
        assert!(x < 320 && y < 240);
        let pixel = &mut self.0[(y * 320 + x) as usize];
        *pixel = over(foreground, *pixel);
    }
}

fn background(position: u32, size: u32) -> u32 {
    let scale = (32 << 16) / size;
    ((2 * position + 1) * scale / 2).saturating_sub(1) >> 16
}

fn tile(position: u32, size: u32) -> u32 {
    if position < 8 {
        position
    } else if position >= size - 8 {
        24 + position - (size - 8)
    } else {
        8 + (position - 8) % 16
    }
}

fn rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.get_color_at(x, y).unwrap().to_srgba().to_u8_array()
}

fn over(foreground: [u8; 4], background: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(foreground[3]);
    let mut pixel = [0, 0, 0, 255];
    for i in 0..3 {
        pixel[i] = ((u32::from(foreground[i]) * alpha + u32::from(background[i]) * (255 - alpha))
            / 255) as u8;
    }
    pixel
}
