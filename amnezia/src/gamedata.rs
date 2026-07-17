//! Shared read-only game database: the actor, item, and skill definitions the
//! in-game menu and the shop screens look up by id. Loaded once from the
//! converted RON at plugin-build time (like the hero name in `main`), so every
//! consumer sees a ready [`GameData`] resource without an `Option` guard.

use crate::assets::{ASSET_ROOT, load_ron};
use amnezia_data::{ActorDef, ItemDef, SkillDef};
use bevy::prelude::*;

/// The static database read by the menu and shop: playable actors, the item
/// catalogue, and the skill list, each indexed by its 1-based id.
#[derive(Resource)]
pub struct GameData {
    pub actors: Vec<ActorDef>,
    pub items: Vec<ItemDef>,
    pub skills: Vec<SkillDef>,
}

impl GameData {
    /// The actor with `id`, if defined.
    pub fn actor(&self, id: u32) -> Option<&ActorDef> {
        self.actors.iter().find(|a| a.id == id)
    }

    /// The item with `id`, if defined.
    pub fn item(&self, id: u32) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.id == id)
    }
}

pub struct GameDataPlugin;

impl Plugin for GameDataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameData {
            actors: load_ron(&format!("{ASSET_ROOT}/actors.ron")),
            items: load_ron(&format!("{ASSET_ROOT}/items.ron")),
            skills: load_ron(&format!("{ASSET_ROOT}/skills.ron")),
        });
    }
}
