use amnezia_data::EventCommand;

pub(super) fn command(code: u32, indent: u32, text: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: text.into(),
        params,
    }
}

fn original(map_id: u32, code: u32, variable: i32) -> Vec<EventCommand> {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    for page in map.events.iter().flat_map(|event| &event.pages) {
        let Some(index) = page.commands.iter().position(|command| {
            command.code == code && (code == 10140 || command.params.get(1) == Some(&variable))
        }) else {
            continue;
        };
        let mut first = index - 1;
        while page.commands[first].code == 20110 {
            first -= 1;
        }
        assert_eq!(page.commands[first].code, 10110);
        return page.commands[first..=index].to_vec();
    }
    panic!("original embedded prompt on map {map_id}");
}

pub(super) fn commands(case: usize) -> Vec<EventCommand> {
    let mut commands = if case == 0 || case == 3 {
        let mut commands = original(1, 10140, 0);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].string, "\\N[1]");
        assert_eq!(commands[1].string, "(Segítek neki!)/(...)");
        assert_eq!(commands[1].params, [2]);
        if case == 3 {
            commands.remove(0);
        }
        for (index, label) in ["(Segítek neki!)", "(...)"].into_iter().enumerate() {
            commands.push(command(20140, 0, label, vec![index as i32]));
            commands.push(command(
                10220,
                1,
                "",
                vec![0, 9013, 9013, 0, 0, index as i32, 0],
            ));
        }
        commands.push(command(20141, 0, "", vec![]));
        commands
    } else if case == 1 {
        let mut commands = original(106, 10150, 24);
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].string, "Daren");
        assert_eq!(commands[1].string, "\"Hány csipet Terra-só legyen?\"");
        assert_eq!(commands[2].params, [1, 24]);
        for command in &mut commands {
            command.indent = 0;
        }
        commands
    } else {
        vec![
            command(10110, 0, "Első sor", vec![]),
            command(20110, 0, "Második sor", vec![]),
            command(20110, 0, "Harmadik sor", vec![]),
            command(10150, 0, "", vec![4, 9014]),
        ]
    };
    let (face, index) = match case {
        0 | 3 => ("Ron", 6),
        1 => ("Daren", 0),
        _ => ("", 0),
    };
    commands.insert(0, command(10130, 0, face, vec![index, 0, 0]));
    commands.push(command(
        10210,
        0,
        "",
        vec![0, 9015 + case as i32, 9015 + case as i32, 2],
    ));
    commands
}
