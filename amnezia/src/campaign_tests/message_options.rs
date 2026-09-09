use super::maps;
use crate::dialogue::MessageOptions;

#[test]
fn all_original_message_options_keep_their_fixed_and_continue_flags() {
    let maps = maps();
    let commands = maps
        .values()
        .flat_map(|m| &m.events)
        .flat_map(|e| &e.pages)
        .flat_map(|p| &p.commands)
        .filter(|c| c.code == 10120)
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 46);
    let mut fixed = 0;
    let mut continuing = 0;
    for command in commands {
        let mut options = MessageOptions::default();
        options.apply(&command.params);
        fixed += usize::from(options.fixed);
        continuing += usize::from(options.continue_events);
    }
    assert_eq!((fixed, continuing), (15, 38));
}
