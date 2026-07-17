//! Actor appearance: the RM2000 `ChangeActorGraphic` opcode (10630) reskins an
//! actor's CharSet graphic, which the game leans on constantly (Ron alone
//! changes clothes 444×). Every change is recorded per actor id in the
//! [`Appearance`] store, and when it targets the party lead — the actor the
//! on-screen hero portrays — the walking [`Player`] sprite is re-textured too.

use crate::player::Player;
use crate::state::Party;
use crate::world::Character;
use bevy::prelude::*;
use std::collections::HashMap;

/// Every actor's current CharSet graphic as `(charset, index)`, keyed by actor
/// id. Actors that were never reskinned simply aren't present.
#[derive(Resource, Default)]
pub struct Appearance(HashMap<u32, (String, u32)>);

impl Appearance {
    /// Record `actor_id`'s graphic, replacing any previous one.
    pub fn set(&mut self, actor_id: u32, charset: String, index: u32) {
        self.0.insert(actor_id, (charset, index));
    }

    /// The recorded graphic for `actor_id`, if one was ever set. The read side
    /// of the store, for consumers that re-render an actor from its saved skin.
    #[allow(dead_code)]
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

impl Plugin for AppearancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Appearance>()
            .add_message::<SpriteChange>()
            .add_systems(Update, apply_sprite_change);
    }
}

/// Apply each [`SpriteChange`]: record it in [`Appearance`], and when it targets
/// the party lead (the first roster member, whom the on-screen hero portrays)
/// re-texture the walking [`Player`] via [`Character::set_graphic`] —
/// `update_player_sprite` reflects the change on the next frame.
fn apply_sprite_change(
    mut reader: MessageReader<SpriteChange>,
    party: Res<Party>,
    mut appearance: ResMut<Appearance>,
    mut players: Query<&mut Player>,
) {
    if reader.is_empty() {
        return;
    }
    let lead = party.snapshot().first().copied();
    for msg in reader.read() {
        appearance.set(msg.actor_id, msg.charset.clone(), msg.index);
        if Some(msg.actor_id) == lead {
            for mut player in &mut players {
                player.set_graphic(msg.charset.clone(), msg.index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_change_retextures_hero_and_is_recorded() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Default roster is `[1]`, so the party lead is actor 1.
        app.insert_resource(Party::default());
        app.init_resource::<Appearance>();
        app.add_message::<SpriteChange>();
        app.add_systems(Update, apply_sprite_change);
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
