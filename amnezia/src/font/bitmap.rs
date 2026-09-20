use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::BTreeMap;

pub(crate) mod smoke;

#[cfg(test)]
mod tests;

pub(crate) const DEFAULT: u32 = 0;
pub(crate) const DISABLED: u32 = 3;
pub(crate) const CRITICAL: u32 = 4;
pub(crate) const KNOCKOUT: u32 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Run {
    pub text: String,
    pub position: IVec2,
    pub color: u32,
    clear: Option<UVec2>,
}

impl Run {
    pub fn new(text: impl Into<String>, x: i32, y: i32, color: u32) -> Self {
        Self {
            text: text.into(),
            position: IVec2::new(x, y),
            color,
            clear: None,
        }
    }

    pub fn clear(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            clear: Some(UVec2::new(width, height)),
            ..Self::new("", x, y, DEFAULT)
        }
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
#[require(ImageNode, Rasterized)]
pub(crate) struct PixelText {
    pub size: UVec2,
    pub runs: Vec<Run>,
}

#[derive(Component, Default)]
struct Rasterized {
    text: Option<PixelText>,
    palette_revision: u64,
}

#[derive(Resource)]
pub(crate) struct BitmapFont {
    glyphs: BTreeMap<u32, (bool, [u16; 12])>,
}

impl BitmapFont {
    pub fn from_id(id: u32) -> Self {
        let source = if id == 1 {
            include_str!("../../fonts/rmg2000.ron")
        } else {
            include_str!("../../fonts/rm2000.ron")
        };
        Self {
            glyphs: ron::from_str(source).expect("embedded bitmap font"),
        }
    }

    fn glyph(&self, ch: char) -> (bool, [u16; 12]) {
        self.glyphs
            .get(&(ch as u32))
            .or_else(|| self.glyphs.get(&('?' as u32)))
            .copied()
            .unwrap_or((false, [0; 12]))
    }

    pub fn width(&self, text: &str) -> i32 {
        text.chars()
            .filter(|ch| !ch.is_control())
            .map(|ch| if self.glyph(ch).0 { 12 } else { 6 })
            .sum()
    }

    pub fn render(&self, text: &PixelText, system: &Image) -> Image {
        let size = text.size.max(UVec2::ONE);
        let mut image = Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0; 4],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.sampler = ImageSampler::nearest();
        for run in &text.runs {
            if let Some(size) = run.clear {
                clear_rect(&mut image, run.position, size);
            }
            let mut pen = run.position;
            for ch in run.text.chars() {
                if ch == '\n' {
                    pen = IVec2::new(run.position.x, pen.y + 16);
                    continue;
                }
                if ch.is_control() {
                    continue;
                }
                let (full, rows) = self.glyph(ch);
                let width = if full { 12 } else { 6 };
                let color = if run.color < 20 { run.color } else { DEFAULT };
                for shadow in [true, false] {
                    for (y, &bits) in rows.iter().enumerate() {
                        for x in 0..width {
                            if bits & (1 << x) == 0 {
                                continue;
                            }
                            let (sx, sy) = if shadow {
                                (16 + x, 32 + y as u32)
                            } else {
                                (color % 10 * 16 + 2 + x, color / 10 * 16 + 52 + y as u32)
                            };
                            let rgba = system
                                .get_color_at(sx, sy)
                                .map_or([255; 4], |c| c.to_srgba().to_u8_array());
                            let offset = i32::from(shadow);
                            pixel(
                                &mut image,
                                pen + IVec2::new(x as i32 + offset, y as i32 + offset),
                                rgba,
                            );
                        }
                    }
                }
                pen.x += width as i32;
            }
        }
        image
    }
}

fn clear_rect(image: &mut Image, position: IVec2, size: UVec2) {
    let bounds = |position: i32, size: u32, limit: u32| {
        let start = i64::from(position);
        let end = start + i64::from(size);
        start.clamp(0, i64::from(limit)) as usize..end.clamp(0, i64::from(limit)) as usize
    };
    let xs = bounds(position.x, size.x, image.width());
    let ys = bounds(position.y, size.y, image.height());
    let width = image.width() as usize;
    let pixels = image.data.as_mut().unwrap();
    for y in ys {
        pixels[(y * width + xs.start) * 4..(y * width + xs.end) * 4].fill(0);
    }
}

fn pixel(image: &mut Image, position: IVec2, rgba: [u8; 4]) {
    if position.x < 0
        || position.y < 0
        || position.x >= image.width() as i32
        || position.y >= image.height() as i32
    {
        return;
    }
    let index = (position.y as usize * image.width() as usize + position.x as usize) * 4;
    image.data.as_mut().unwrap()[index..index + 4].copy_from_slice(&rgba);
}

#[derive(Resource)]
struct Palette {
    source: Handle<Image>,
    revision: u64,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreStartup, setup)
        .add_systems(PostUpdate, rasterize.before(bevy::ui::UiSystems::Content));
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let system = crate::assets::load_ron::<amnezia_data::SystemDef>(&format!(
        "{}/system.ron",
        crate::assets::asset_root()
    ));
    commands.insert_resource(BitmapFont::from_id(system.font_id));
    commands.insert_resource(Palette {
        source: assets.load("graphics/System/System.png"),
        revision: 0,
    });
}

fn rasterize(
    font: Res<BitmapFont>,
    mut palette: ResMut<Palette>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut texts: Query<(&PixelText, &mut ImageNode, &mut Rasterized)>,
) {
    if events.read().any(|event| matches!(event, AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } if *id == palette.source.id())) {
        palette.revision = palette.revision.wrapping_add(1);
    }
    for (text, mut node, mut cache) in &mut texts {
        if cache.text.as_ref() == Some(text)
            && cache.palette_revision == palette.revision
            && !font.is_changed()
        {
            continue;
        }
        let Some(system) = images.get(&palette.source) else {
            continue;
        };
        let rendered = font.render(text, system);
        if cache.text.is_some() {
            if let Some(mut current) = images.get_mut(&node.image) {
                *current = rendered;
            } else {
                node.image = images.add(rendered);
            }
        } else {
            node.image = images.add(rendered);
        }
        node.image_mode = NodeImageMode::Stretch;
        cache.text = Some(text.clone());
        cache.palette_revision = palette.revision;
    }
}
