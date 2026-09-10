//! Persistent actor graphics and the party leader's map representation.

use crate::player::Player;
use crate::state::Party;
use crate::world::{Character, MapChanged, RouteStepper};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) mod smoke;

/// Every actor's current CharSet graphic as `(charset, index)`, keyed by actor
/// id. Actors that were never reskinned simply aren't present.
#[derive(Resource, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appearance(BTreeMap<u32, (String, u32)>);

impl Appearance {
    /// Record `actor_id`'s graphic, replacing any previous one.
    pub fn set(&mut self, actor_id: u32, charset: String, index: u32) {
        self.0.insert(actor_id, (charset, index));
    }

    /// The actor's graphic override, if one was set.
    pub fn get(&self, actor_id: u32) -> Option<(&str, u32)> {
        self.0
            .get(&actor_id)
            .map(|(charset, index)| (charset.as_str(), *index))
    }
}

/// A request to reskin an actor's CharSet graphic (RM2000 opcode 10630). The
/// interpreter writes one per `ChangeActorGraphic`: `actor_id` is the database
/// actor, `charset` the CharSet file name, `index` the 0-based cell within it.
#[derive(Message)]
pub struct SpriteChange {
    pub actor_id: u32,
    pub charset: String,
    pub index: u32,
}

pub struct AppearancePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ActorGraphics;

impl Plugin for AppearancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Appearance>()
            .add_message::<SpriteChange>()
            .add_message::<MapChanged>()
            .add_systems(
                Update,
                apply_sprite_change
                    .in_set(ActorGraphics)
                    .after(crate::interpreter::InterpreterStep)
                    .before(crate::world::walk::<Player>)
                    .before(crate::player::update_player_sprite),
            );
    }
}

fn apply_sprite_change(
    mut reader: MessageReader<SpriteChange>,
    mut transfers: MessageReader<MapChanged>,
    data: Res<crate::gamedata::GameData>,
    party: Res<Party>,
    mut appearance: ResMut<Appearance>,
    mut players: Query<(&mut Player, Option<&mut RouteStepper>, Option<&mut Sprite>)>,
    mut last_revision: Local<Option<u64>>,
) {
    let mut refresh = transfers.read().count() > 0
        || *last_revision != Some(party.graphics_revision())
        || appearance.is_changed();
    *last_revision = Some(party.graphics_revision());
    for msg in reader.read() {
        if data.actor(msg.actor_id).is_none() {
            continue;
        }
        appearance.set(msg.actor_id, msg.charset.clone(), msg.index);
        refresh = true;
    }
    let roster = party.snapshot();
    let graphic = roster
        .first()
        .and_then(|&id| data.actor(id))
        .map_or(("", 0), |actor| {
            appearance
                .get(actor.id)
                .unwrap_or((&actor.character_name, actor.character_index))
        });
    for (mut player, route, sprite) in &mut players {
        if !refresh && !player.is_added() {
            continue;
        }
        player.set_graphic(graphic.0.to_owned(), graphic.1);
        if let Some(mut route) = route {
            route.reset_transparency();
        }
        if let Some(mut sprite) = sprite {
            sprite.color = sprite.color.with_alpha(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod refresh;

    #[test]
    fn lead_change_retextures_hero_and_is_recorded() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::gamedata::GameDataPlugin,
            AppearancePlugin,
        ));
        app.insert_resource(Party::default());
        let hero = app
            .world_mut()
            .spawn(Player {
                tile_x: 0,
                tile_y: 0,
                dir: 0,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            })
            .id();

        app.world_mut().write_message(SpriteChange {
            actor_id: 1,
            charset: "Poses2".into(),
            index: 4,
        });
        app.update();

        let player = app.world().entity(hero).get::<Player>().unwrap();
        assert_eq!(player.charset, "Poses2");
        assert_eq!(player.index, 4);
        assert_eq!(
            app.world().resource::<Appearance>().get(1),
            Some(("Poses2", 4))
        );
    }
}
