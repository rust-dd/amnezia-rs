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
#[cfg(test)]
mod campaign_tests;
mod choice;
mod conditions;
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
mod panorama;
mod picture;
mod player;
mod progression;
mod save;
mod screenfx;
mod session;
mod shop;
mod smoke;
mod state;
mod system_bgm;
mod teleport;
mod terms;
mod text;
mod tiles;
mod timer;
mod title;
mod vehicles;
mod vitals;
mod world;

use assets::{asset_root, load_ron};
use bevy::prelude::*;

fn main() -> AppExit {
    let hero: amnezia_data::Hero = load_ron(&format!("{}/hero.ron", asset_root()));
    let offscreen = smoke::offscreen::enabled();
    let mut plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(AssetPlugin {
            file_path: asset_root().to_string(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                // WindowResolution uses physical pixels, including on Retina.
                resolution: if offscreen {
                    bevy::window::WindowResolution::new(960, 720)
                } else {
                    bevy::window::WindowResolution::new(1440, 1080)
                },
                resizable: true,
                title: "Amnézia".to_string(),
                ..default()
            }),
            exit_condition: if offscreen {
                bevy::window::ExitCondition::DontExit
            } else {
                bevy::window::ExitCondition::OnAllClosed
            },
            ..default()
        });
    if offscreen {
        plugins = plugins.disable::<bevy::winit::WinitPlugin>();
    }
    App::new()
        .add_plugins(plugins)
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
        .add_plugins(session::SessionPlugin)
        .add_plugins(vehicles::VehiclePlugin)
        .add_plugins(conditions::ConditionsPlugin)
        .add_plugins(panorama::PanoramaPlugin)
        .add_plugins(smoke::SmokePlugin)
        .run()
}
