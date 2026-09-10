use amnezia_data::{EventCommand, Map};
use bevy::prelude::*;

pub(super) fn selected() -> &'static str {
    if std::env::args().any(|arg| arg == "--smoke-actor-graphics") {
        "actor-graphics"
    } else if std::env::args().any(|arg| arg == "--smoke-actor-names") {
        "actor-names"
    } else if std::env::args().any(|arg| arg == "--smoke-return-title") {
        "return-title"
    } else if std::env::args().any(|arg| arg == "--smoke-gameover") {
        "gameover"
    } else if std::env::args().any(|arg| arg == "--smoke-battle-defeat") {
        "battle-defeat"
    } else if std::env::args().any(|arg| arg == "--smoke-battle-transitions") {
        "battle-transitions"
    } else if std::env::args().any(|arg| arg == "--smoke-screen-events") {
        "screen-events"
    } else if std::env::args().any(|arg| arg == "--smoke-transitions") {
        "transitions"
    } else if std::env::args().any(|arg| arg == "--smoke-font-colors") {
        "font-colors"
    } else if std::env::args().any(|arg| arg == "--smoke-water") {
        "water"
    } else if std::env::args().any(|arg| arg == "--smoke-animation-colors") {
        "animation-colors"
    } else if std::env::args().any(|arg| arg == "--smoke-display") {
        "display"
    } else if std::env::args().any(|arg| arg == "--smoke-colors") {
        "colors"
    } else if std::env::args().any(|arg| arg == "--smoke-pictures") {
        "pictures"
    } else if std::env::args().any(|arg| arg == "--smoke-message-options") {
        "message-options"
    } else if std::env::args().any(|arg| arg == "--smoke-camera") {
        "camera"
    } else if std::env::args().any(|arg| arg == "--smoke-looping") {
        "looping"
    } else if std::env::args().any(|arg| arg == "--smoke-airship") {
        "airship"
    } else if std::env::args().any(|arg| arg == "--smoke-battle") {
        "battle"
    } else if std::env::args().any(|arg| arg == "--smoke-battle-menus") {
        "battle-menus"
    } else if std::env::args().any(|arg| arg == "--smoke-battle-events") {
        "battle-events"
    } else if std::env::args().any(|arg| arg == "--smoke-timer") {
        "timer"
    } else if std::env::args().any(|arg| arg == "--smoke-panorama") {
        "panorama"
    } else if std::env::args().any(|arg| arg == "--smoke-airship-escape") {
        "escape"
    } else if std::env::args().any(|arg| arg == "--smoke-font") {
        "font"
    } else if std::env::args().any(|arg| arg == "--smoke-menu") {
        "menu"
    } else {
        "intro"
    }
}

pub(super) fn escaped_airship_cast(world: &mut World) -> u8 {
    if world.resource::<crate::world::MapData>().map_id != 94 {
        return 0;
    }
    let mut mask = 0;
    for event in world.query::<&crate::world::EventSprite>().iter(world) {
        if !(2..=6).contains(&event.id) || (event.charset.as_str(), event.index) != ("Torch", 1) {
            continue;
        }
        let destination = [(13, 15), (12, 15), (14, 15), (15, 15), (13, 15)][event.id as usize - 2];
        assert_eq!(
            (event.tile_x, event.tile_y),
            destination,
            "escaped event {}",
            event.id
        );
        mask |= 1 << (event.id - 2);
    }
    mask
}

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
