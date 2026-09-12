use super::{camera, looping, message_options};
use amnezia_data::{EventCommand, Map};
use bevy::prelude::*;

pub(super) fn start(world: &mut World, scenario: &str) {
    crate::session::clear_transient(world);
    world.insert_resource(crate::teleport::Fade::default());
    world.insert_resource(crate::teleport::PendingTeleport::default());
    world.insert_resource(crate::player::HeroHidden::default());
    if scenario == "battle-menus" {
        crate::battle::smoke::prepare(world);
    }
    let commands = if matches!(
        scenario,
        "message-options"
            | "world-tones"
            | "ui-layers"
            | "actor-graphics"
            | "actor-names"
            | "pictures"
            | "colors"
            | "font-colors"
            | "display"
            | "animation-colors"
            | "water"
            | "transitions"
            | "return-title"
            | "save-music"
            | "save-screen"
    ) {
        message_options::entry()
    } else if scenario == "screen-events" {
        crate::transitions::event_smoke::entry()
    } else if scenario == "battle-transitions" {
        crate::battle::flow::smoke::entry()
    } else if matches!(scenario, "gameover" | "battle-defeat") {
        crate::gameover::smoke::entry(scenario == "battle-defeat")
    } else if matches!(
        scenario,
        "camera"
            | "save-camera"
            | "save-pictures"
            | "save-weather"
            | "map-animations"
            | "map-flashes"
            | "weather"
    ) {
        camera::entry()
    } else if scenario == "looping" {
        looping::entry()
    } else if scenario == "airship" {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_0125.ron",
            crate::assets::asset_root()
        ));
        let commands = &map
            .events
            .iter()
            .flat_map(|e| &e.pages)
            .find(|page| {
                page.commands
                    .iter()
                    .any(|c| c.code == 10850 && c.params == [2, 0, 13, 55, 100])
            })
            .expect("original fortress flight")
            .commands;
        let vehicle = commands.iter().position(|c| c.code == 10850).unwrap();
        let start = commands[..vehicle]
            .iter()
            .rposition(|c| c.code == 10810)
            .unwrap();
        let end = commands[vehicle..]
            .iter()
            .position(|c| c.code == 10810)
            .map_or(commands.len(), |i| vehicle + i);
        commands[start..end].to_vec()
    } else if matches!(
        scenario,
        "battle" | "timer" | "battle-menus" | "battle-events"
    ) {
        let mut commands = if scenario == "timer" {
            let mut commands = vec![EventCommand {
                code: 10810,
                indent: 0,
                string: String::new(),
                params: vec![3, 15, 6],
            }];
            commands.extend(mission_timer_start());
            commands
        } else {
            Vec::new()
        };
        commands.push(EventCommand {
            code: 10710,
            indent: 0,
            string: "Cave1".into(),
            params: vec![
                0,
                if scenario == "battle-events" { 15 } else { 2 },
                1,
                0,
                0,
                0,
            ],
        });
        if scenario == "battle-events" {
            commands.push(EventCommand {
                code: 10210,
                indent: 0,
                string: String::new(),
                params: vec![0, 9999, 9999, 0],
            });
        }
        commands
    } else if scenario == "font" {
        vec![
            EventCommand {
                code: 10810,
                indent: 0,
                string: String::new(),
                params: vec![3, 15, 6],
            },
            EventCommand {
                code: 10130,
                indent: 0,
                string: "Ron".into(),
                params: vec![0, 0, 0],
            },
            EventCommand {
                code: 10110,
                indent: 0,
                string: "Hát... hol vagyok?".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "Árvíztűrő tükörfúrógép.".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "Őrült éjszaka volt!".into(),
                params: vec![],
            },
            EventCommand {
                code: 20110,
                indent: 0,
                string: "0123456789 ÁÉÍÓÖŐÚÜŰ".into(),
                params: vec![],
            },
        ]
    } else if scenario == "menu" {
        vec![EventCommand {
            code: 10810,
            indent: 0,
            string: String::new(),
            params: vec![3, 15, 6],
        }]
    } else if matches!(scenario, "panorama" | "escape") {
        airship_interior_entry()
    } else {
        unreachable!("unknown smoke scenario: {scenario}")
    };
    world
        .resource_mut::<crate::interpreter::RunningEvent>()
        .start(1, commands);
}

pub(super) fn selected() -> &'static str {
    if std::env::args().any(|arg| arg == "--smoke-save-weather") {
        "save-weather"
    } else if std::env::args().any(|arg| arg == "--smoke-weather") {
        "weather"
    } else if std::env::args().any(|arg| arg == "--smoke-save-music") {
        "save-music"
    } else if std::env::args().any(|arg| arg == "--smoke-save-camera") {
        "save-camera"
    } else if std::env::args().any(|arg| arg == "--smoke-save-pictures") {
        "save-pictures"
    } else if std::env::args().any(|arg| arg == "--smoke-save-screen") {
        "save-screen"
    } else if std::env::args().any(|arg| arg == "--smoke-ui-layers") {
        "ui-layers"
    } else if std::env::args().any(|arg| arg == "--smoke-map-flashes") {
        "map-flashes"
    } else if std::env::args().any(|arg| arg == "--smoke-world-tones") {
        "world-tones"
    } else if std::env::args().any(|arg| arg == "--smoke-map-animations") {
        "map-animations"
    } else if std::env::args().any(|arg| arg == "--smoke-actor-graphics") {
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
