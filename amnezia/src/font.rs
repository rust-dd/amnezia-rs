//! The database-selected Western font, using EasyRPG's bitmap-compatible faces.
//! Outlines preserve the 6×12 source cells; UI coordinates are scaled threefold.

use bevy::prelude::*;
use bevy::text::FontSmoothing;

pub(crate) mod bitmap;

const RM2000: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/rm2000.ttf"));
const RMG2000: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/rmg2000.ttf"));

pub const UI_FONT_PX: f32 = 36.0;
pub const UI_LINE_PX: f32 = 48.0;

/// Handle to the loaded game font, shared by every text surface.
#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

pub struct FontPlugin;

impl Plugin for FontPlugin {
    fn build(&self, app: &mut App) {
        bitmap::register(app);
        app.add_systems(PreStartup, load_font)
            .add_systems(Update, (keep_text_crisp, add_text_shadows));
    }
}

/// RM2000's font is a bitmap/pixel face; Bevy antialiases text by default, which
/// blurs the pixels and makes it read as the wrong font entirely. Force every text
/// surface to render with no smoothing so the pixels stay sharp — the way RPG_RT
/// draws them. Runs each frame but only touches text not already set, so it never
/// needlessly re-rasterizes; this keeps the fix in one place instead of on every
/// `TextFont` across the UI.
fn keep_text_crisp(mut text_fonts: Query<&mut TextFont>) {
    for mut text_font in &mut text_fonts {
        if text_font.font_smoothing != FontSmoothing::None {
            text_font.font_smoothing = FontSmoothing::None;
        }
    }
}

fn add_text_shadows(
    mut commands: Commands,
    text: Query<Entity, (With<Text>, Without<TextShadow>)>,
) {
    for entity in &text {
        commands.entity(entity).insert(TextShadow {
            offset: Vec2::splat(3.0),
            color: Color::BLACK,
        });
    }
}

fn load_font(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let system = crate::assets::load_ron::<amnezia_data::SystemDef>(&format!(
        "{}/system.ron",
        crate::assets::asset_root(),
    ));
    let font = Font::from_bytes(font_bytes(system.font_id).to_vec());
    commands.insert_resource(GameFont(fonts.add(font)));
}

fn font_bytes(id: u32) -> &'static [u8] {
    if id == 1 { RMG2000 } else { RM2000 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_font_selection_uses_rm2000_not_rmg2000() {
        let system = crate::assets::load_ron::<amnezia_data::SystemDef>(&format!(
            "{}/system.ron",
            crate::assets::asset_root(),
        ));
        assert_eq!(system.font_id, 0);
        assert_eq!(font_bytes(system.font_id), RM2000);
        assert_ne!(font_bytes(0), font_bytes(1));
    }
}
