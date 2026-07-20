//! The game font: RM2000's built-in pixel font, embedded in the binary and
//! registered as a Bevy asset before any UI is built. RPG Maker 2000 ships no font
//! of its own — RPG_RT draws text with a built-in bitmap face — so the faithful font
//! is EasyRPG's free reproduction of it, "RMG2000". `fonts/rmg2000.ttf` is that
//! bitmap font converted to a TrueType outline by `fonts/rmg2000_to_ttf.py` (the
//! Hungarian ő/ű come from EasyRPG's ttyp0 fallback, and the menu cursor / arrows /
//! em dash are drawn in since RM2000 renders those graphically, not as characters).

use bevy::prelude::*;

const FONT_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/rmg2000.ttf"));

/// Handle to the loaded game font, shared by every text surface.
#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

pub struct FontPlugin;

impl Plugin for FontPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_font);
    }
}

fn load_font(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let font = Font::from_bytes(FONT_BYTES.to_vec());
    commands.insert_resource(GameFont(fonts.add(font)));
}
