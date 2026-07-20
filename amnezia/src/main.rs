//! The Amnézia game: a faithful Bevy/Rust remake driven entirely by the clean
//! assets produced by `amnezia-convert`. It renders a map with its chipset,
//! lets the player walk the hero around it (blocked by passability), and shows
//! event dialogue. Depends only on `amnezia-data` + Bevy; it never touches the
//! legacy RPG Maker formats.

mod animation;
mod appearance;
mod assets;
mod audio;
mod battle;
mod choice;
mod debug;
mod dialogue;
mod equipment;
mod events;
mod font;
mod gamedata;
mod gameover;
mod i18n;
mod inputnumber;
mod interpreter;
mod map_bgm;
mod menu;
mod picture;
mod player;
mod progression;
mod save;
mod screenfx;
mod shop;
mod state;
mod teleport;
mod terms;
mod text;
mod tiles;
mod timer;
mod title;
mod vitals;
mod world;

use assets::{asset_root, load_ron};
use bevy::prelude::*;

fn main() -> AppExit {
    let hero: amnezia_data::Hero = load_ron(&format!("{}/hero.ron", asset_root()));
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: asset_root().to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // A moderate 4:3 window (`new` is physical pixels, so on a 2×
                        // Retina display this is a ~720×540-point window). The camera's
                        // Fixed 320×240 scaling fills the window with the world at any
                        // size, and `world::fit_ui_scale` scales the 960×720 UI to
                        // match — so the game stays consistent across window sizes and
                        // display densities. Resizable, so the player can fine-tune it.
                        resolution: bevy::window::WindowResolution::new(1440, 1080),
                        resizable: true,
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
        .add_plugins(map_bgm::MapBgmPlugin)
        .add_plugins((
            screenfx::ScreenFxPlugin,
            picture::PicturePlugin,
            gameover::GameOverPlugin,
            animation::AnimationPlugin,
        ))
        .add_plugins(inputnumber::InputNumberPlugin)
        .add_plugins(appearance::AppearancePlugin)
        .add_plugins(progression::ProgressionPlugin)
        .add_plugins(equipment::EquipmentPlugin)
        .add_plugins(i18n::I18nPlugin)
        .add_plugins(terms::TermsPlugin)
        .add_plugins(timer::GameClockPlugin)
        .run()
}
