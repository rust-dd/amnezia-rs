use amnezia_data::{EventCommand, Map};
use bevy::prelude::*;

pub(super) fn verify_airship_staging(world: &mut World) {
    assert!(world.resource::<crate::dialogue::Dialogue>().active);
    let mut query = world.query::<(&crate::world::EventSprite, &Visibility)>();
    for (id, position, charset, index, direction) in [
        (2, (9, 9), "Chara1", 0, 2),
        (3, (8, 9), "Chara1", 1, 2),
        (4, (10, 10), "Chara4", 2, 0),
        (5, (10, 9), "Chara4", 0, 2),
        (6, (9, 10), "Chara2", 2, 0),
    ] {
        let (character, visibility) = query.iter(world).find(|(event, _)| event.id == id).unwrap();
        assert_eq!((character.tile_x, character.tile_y), position, "event {id}");
        assert_eq!(
            (&*character.charset, character.index),
            (charset, index),
            "event {id}"
        );
        assert_eq!(character.dir, direction, "event {id}");
        assert_ne!(*visibility, Visibility::Hidden, "event {id}");
    }
}

pub(super) fn airship_interior_entry() -> Vec<EventCommand> {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_0084.ron",
        crate::assets::asset_root()
    ));
    let commands = &map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .find(|page| {
            page.commands
                .iter()
                .any(|command| command.code == 10810 && command.params.first() == Some(&94))
        })
        .expect("original airship emergency exit")
        .commands;
    let transfer = commands
        .iter()
        .position(|command| command.code == 10810 && command.params.first() == Some(&94))
        .unwrap();
    let start = commands[..transfer]
        .iter()
        .rposition(|command| command.code == 11010)
        .expect("erase screen before entering the cutscene");
    let end = transfer
        + commands[transfer..]
            .iter()
            .position(|command| command.code == 11020)
            .expect("show screen after entering the cutscene");
    commands[start..=end].to_vec()
}

pub(super) fn mission_timer_start() -> Vec<EventCommand> {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_0098.ron",
        crate::assets::asset_root()
    ));
    let commands = &map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .find(|page| {
            page.commands
                .iter()
                .any(|command| command.code == 10230 && command.params.first() == Some(&0))
        })
        .expect("original timed house mission")
        .commands;
    let start = commands
        .iter()
        .position(|command| command.code == 10230 && command.params.first() == Some(&0))
        .unwrap();
    let end = start
        + commands[start..]
            .iter()
            .position(|command| command.code == 10230 && command.params.first() == Some(&1))
            .unwrap();
    commands[start..=end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn airship_entry_hides_the_player_before_the_original_transfer() {
        let commands = airship_interior_entry();
        assert_eq!(
            commands.iter().map(|c| c.code).collect::<Vec<_>>(),
            [11010, 11310, 11410, 10810, 11020]
        );
        assert_eq!(commands[1].params, [0]);
        assert_eq!(commands[3].params, [94, 9, 4]);
    }
}
