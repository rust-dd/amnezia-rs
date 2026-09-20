use amnezia_data::{EventCommand, Map};

pub(super) fn commands(flag: i32) -> Vec<EventCommand> {
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_0050.ron",
        crate::assets::asset_root()
    ));
    let shop = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .flat_map(|page| &page.commands)
        .find(|command| command.code == 10720)
        .expect("original Map0050 merchant")
        .clone();
    assert_eq!(&shop.params[..6], [0, 0, 1, 0, 2, 7]);
    let command = |code, indent, params| EventCommand {
        code,
        indent,
        string: String::new(),
        params,
    };
    vec![
        shop,
        command(20720, 0, vec![]),
        command(10210, 1, vec![0, flag, flag, 0]),
        command(20721, 0, vec![]),
        command(10210, 1, vec![0, flag, flag, 1]),
        command(20722, 0, vec![]),
    ]
}
