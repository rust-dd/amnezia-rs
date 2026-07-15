//! The Amnézia game: a faithful Bevy/Rust remake driven entirely by the clean
//! intermediate assets produced by `amnezia-convert`. This binary depends only
//! on `amnezia-data` and Bevy; it never touches the legacy RPG Maker formats.

use bevy::prelude::*;

fn main() -> AppExit {
    App::new().add_plugins(DefaultPlugins).run()
}
