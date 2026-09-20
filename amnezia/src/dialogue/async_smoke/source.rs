use amnezia_data::EventCommand;

fn command(code: u32, indent: u32, string: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: string.into(),
        params,
    }
}

pub(super) fn commands(kind: usize) -> Vec<EventCommand> {
    let mut commands = vec![command(10130, 0, "", vec![0])];
    match kind {
        0 => commands.push(command(10110, 0, &"abcdefghij".repeat(12), vec![])),
        1 => {
            commands.push(command(10140, 0, "", vec![2]));
            for (index, text) in ["Első", "Második"].into_iter().enumerate() {
                commands.push(command(20140, 0, text, vec![index as i32]));
                commands.push(command(
                    10220,
                    1,
                    "",
                    vec![0, 9019, 9019, 0, 0, index as i32, 0],
                ));
            }
            commands.push(command(20141, 0, "", vec![]));
        }
        _ => commands.push(command(10150, 0, "", vec![2, 9020])),
    }
    commands
}
