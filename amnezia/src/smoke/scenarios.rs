use amnezia_data::{EventCommand, Map};

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
