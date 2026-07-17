//! The Amnézia game: a faithful Bevy/Rust remake driven entirely by the clean
//! assets produced by `amnezia-convert`. It renders a map with its chipset,
//! lets the player walk the hero around it (blocked by passability), and shows
//! event dialogue. Depends only on `amnezia-data` + Bevy; it never touches the
//! legacy RPG Maker formats.

mod assets;
mod audio;
mod choice;
mod debug;
mod dialogue;
mod events;
mod font;
mod interpreter;
mod player;
mod save;
mod state;
mod teleport;
mod text;
mod tiles;
mod world;

use assets::{load_ron, ASSET_ROOT};
use bevy::prelude::*;

fn main() -> AppExit {
    let hero: amnezia_data::Hero = load_ron(&format!("{ASSET_ROOT}/hero.ron"));
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin { file_path: ASSET_ROOT.to_string(), ..default() }),
        )
        .insert_resource(text::HeroName(hero.name))
        .init_resource::<state::Switches>()
        .init_resource::<state::Variables>()
        .init_resource::<state::Party>()
        .init_resource::<state::Inventory>()
        .add_plugins((
            font::FontPlugin,
            world::WorldPlugin,
            player::PlayerPlugin,
            dialogue::DialoguePlugin,
            teleport::TeleportPlugin,
            interpreter::InterpreterPlugin,
            audio::AudioPlugin,
            choice::ChoicePlugin,
            save::SavePlugin,
            debug::DebugPlugin,
        ))
        .run()
}
