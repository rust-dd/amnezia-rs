//! Offline converter from the original RPG Maker 2000 project to the clean
//! intermediate assets the game consumes.

pub mod midi;

mod animations;
mod audio;
mod database_battle;
mod database_party;
mod graphics;
mod maps;
mod system;

pub use animations::convert_animations;
pub use audio::convert_audio;
pub use database_battle::{convert_attributes, convert_monsters, convert_states, convert_troops};
pub use database_party::{convert_actors, convert_hero, convert_items, convert_skills};
pub use graphics::convert_graphics;
pub use maps::{
    convert_chipsets, convert_common_events, convert_map_info, convert_maps, convert_start,
};
pub use system::convert_system;
