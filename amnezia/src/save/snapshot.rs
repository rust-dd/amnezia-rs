use serde::{Deserialize, Serialize};

/// A versioned snapshot of the supported persistent state.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub(super) struct SaveGame {
    #[serde(default)]
    pub(super) format_version: u32,
    #[serde(default)]
    pub(super) game_frames: crate::timing::GameFrames,
    #[serde(default)]
    pub(super) transitions: crate::transitions::Settings,
    pub(super) map_id: u32,
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) dir: u32,
    pub(super) switches: Vec<(u32, bool)>,
    pub(super) variables: Vec<(u32, i32)>,
    pub(super) party: Vec<u32>,
    pub(super) items: Vec<(u32, u32)>,
    pub(super) gold: i32,
    #[serde(default)]
    pub(super) progression: Vec<(u32, u32)>,
    #[serde(default)]
    pub(super) learned_skills: Vec<(u32, Vec<u32>)>,
    #[serde(default)]
    pub(super) vitals: Vec<(u32, (i32, i32))>,
    #[serde(default)]
    pub(super) conditions: Vec<(u32, Vec<u32>)>,
    #[serde(default)]
    pub(super) field_steps: u64,
    #[serde(default)]
    pub(super) hero_name: String,
    #[serde(default)]
    pub(super) charset: String,
    #[serde(default)]
    pub(super) charset_index: u32,
    #[serde(default)]
    pub(super) hero_hidden: bool,
    #[serde(default = "super::neutral_tone")]
    pub(super) tone: (i32, i32, i32, i32),
    #[serde(default)]
    pub(super) weather: i32,
    #[serde(default)]
    pub(super) weather_strength: i32,
    #[serde(default)]
    pub(super) equipment: Vec<(u32, [u32; 5])>,
    #[serde(default)]
    pub(super) playtime: u64,
    #[serde(default)]
    pub(super) timer_remaining: f32,
    #[serde(default)]
    pub(super) timer_running: bool,
    #[serde(default)]
    pub(super) timer_visible: bool,
    #[serde(default)]
    pub(super) timer_in_battle: bool,
    #[serde(default)]
    pub(super) vehicles: crate::vehicles::VehicleSave,
    #[serde(default)]
    pub(super) system_bgm: crate::system_bgm::SystemBgm,
    #[serde(default)]
    pub(super) panorama: Option<crate::panorama::Panorama>,
    #[serde(default)]
    pub(super) appearance: crate::appearance::Appearance,
    #[serde(default)]
    pub(super) menu_access: Option<bool>,
    #[serde(default)]
    pub(super) save_access: bool,
    /// Missing legacy data differs from an explicitly saved silent scene.
    #[serde(default)]
    pub(super) music: Option<crate::audio::saved::MusicState>,
    #[serde(default)]
    pub(super) message: crate::dialogue::saved::MessageState,
    #[serde(default)]
    pub(super) camera: Option<crate::player::saved_camera::CameraState>,
    #[serde(default)]
    pub(super) pictures: Vec<crate::picture::saved::PictureState>,
    #[serde(default)]
    pub(super) screen: Option<crate::screenfx::saved::ScreenState>,
    #[serde(default)]
    pub(super) map_animation: crate::animation::saved::MapState,
    #[serde(default)]
    pub(super) map_events: Vec<crate::world::saved::EventState>,
    #[serde(default)]
    pub(super) hero_motion: Option<crate::world::saved::hero::HeroState>,
}
