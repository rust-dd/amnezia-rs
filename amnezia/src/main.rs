//! The Amnézia game: a faithful Bevy/Rust remake driven entirely by the clean
//! assets produced by `amnezia-convert`. It renders a map with its chipset,
//! lets the player walk the hero around it (blocked by passability), and shows
//! event dialogue. Depends only on `amnezia-data` + Bevy; it never touches the
//! legacy RPG Maker formats.

mod appearance;
mod assets;
mod audio;
mod battle;
mod choice;
mod debug;
mod dialogue;
mod events;
mod font;
mod gamedata;
mod gameover;
mod inputnumber;
mod interpreter;
mod menu;
mod picture;
mod player;
mod save;
mod screenfx;
mod shop;
mod state;
mod teleport;
mod text;
mod tiles;
mod title;
mod vitals;
mod world;

use assets::{ASSET_ROOT, load_ron};
use bevy::prelude::*;

fn main() -> AppExit {
    let hero: amnezia_data::Hero = load_ron(&format!("{ASSET_ROOT}/hero.ron"));
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: ASSET_ROOT.to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    // 3× the 320×240 RM2000 viewport, 4:3, so the fixed camera
                    // scales pixel-perfect with no distortion or gray margin.
                    primary_window: Some(Window {
                        resolution: bevy::window::WindowResolution::new(960, 720),
                        resizable: false,
                        title: "Amnézia".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
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
            gamedata::GameDataPlugin,
            menu::MenuPlugin,
            shop::ShopPlugin,
            battle::BattlePlugin,
            title::TitlePlugin,
        ))
        .add_plugins((
            screenfx::ScreenFxPlugin,
            picture::PicturePlugin,
            gameover::GameOverPlugin,
        ))
        .add_plugins(inputnumber::InputNumberPlugin)
        .add_plugins(appearance::AppearancePlugin)
        .run()
}
