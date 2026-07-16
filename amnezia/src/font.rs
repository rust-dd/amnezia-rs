//! The game font: a full-Latin (Hungarian-covering) TrueType face embedded in
//! the binary and registered as a Bevy asset before any UI is built.

use bevy::prelude::*;

const FONT_BYTES: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/DejaVuSans.ttf"));

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
