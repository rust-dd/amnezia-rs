//! Read-only converted database, loaded at plugin build so consumers need no readiness guard.

use crate::assets::{asset_root, load_ron};
use amnezia_data::{ActorDef, AttributeDef, ItemDef, SkillDef};
use bevy::prelude::*;
use std::sync::OnceLock;

pub(crate) fn attribute_definitions() -> &'static [AttributeDef] {
    static ATTRIBUTES: OnceLock<Vec<AttributeDef>> = OnceLock::new();
    ATTRIBUTES.get_or_init(|| load_ron(&format!("{}/attributes.ron", asset_root())))
}

/// The static database read by the menu and shop: playable actors, the item
/// catalogue, and the skill list, each indexed by its 1-based id.
#[derive(Resource)]
pub struct GameData {
    pub actors: Vec<ActorDef>,
    pub items: Vec<ItemDef>,
    pub skills: Vec<SkillDef>,
}

impl GameData {
    pub fn actor(&self, id: u32) -> Option<&ActorDef> {
        self.actors.iter().find(|a| a.id == id)
    }

    pub fn item(&self, id: u32) -> Option<&ItemDef> {
        self.items.iter().find(|i| i.id == id)
    }
}

pub struct GameDataPlugin;

impl Plugin for GameDataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameData {
            actors: load_ron(&format!("{}/actors.ron", asset_root())),
            items: load_ron(&format!("{}/items.ron", asset_root())),
            skills: load_ron(&format!("{}/skills.ron", asset_root())),
        });
    }
}
