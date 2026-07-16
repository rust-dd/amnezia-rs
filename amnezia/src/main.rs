//! The Amnézia game: a faithful Bevy/Rust remake driven entirely by the clean
//! assets produced by `amnezia-convert`. It renders a map with its chipset,
//! lets the player walk the hero around it (blocked by passability), and shows
//! event dialogue. Depends only on `amnezia-data` + Bevy; it never touches the
//! legacy RPG Maker formats.

mod assets;
mod dialogue;
mod events;
mod player;
mod tiles;
mod world;

use assets::ASSET_ROOT;
use bevy::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin { file_path: ASSET_ROOT.to_string(), ..default() }),
        )
        .add_plugins((world::WorldPlugin, player::PlayerPlugin, dialogue::DialoguePlugin))
        .run()
}
