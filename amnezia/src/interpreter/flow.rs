//! The pure control-flow navigation helpers: given a page's flat command list and
//! a cursor, they compute where execution jumps for branches, loops, labels,
//! choices, and the shop/battle subsystem blocks. All are indent-driven and Bevy-
//! free, so they unit-test directly against hand-built command lists.

use super::opcodes::*;
use amnezia_data::EventCommand;

/// The instruction pointer to jump to when a branch at `indent` is NOT taken:
/// skip the true body (every command deeper than `indent`), then enter the else
/// body if an `ELSE_BRANCH` marker follows, else land on the block terminator
/// (which the main loop skips).
pub(super) fn skip_true_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && commands[j].indent > indent {
        j += 1;
    }
    if j < commands.len() && commands[j].code == ELSE_BRANCH && commands[j].indent == indent {
        j + 1
    } else {
        j
    }
}

/// The instruction pointer to jump to when the true body falls through to an
/// `ELSE_BRANCH` at `indent`: skip the else body, landing on the block
/// terminator (which the main loop skips).
pub(super) fn skip_else_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && commands[j].indent > indent {
        j += 1;
    }
    j
}

/// Index of the `Label` (12110) whose first param equals `id`, anywhere in the
/// page (RM2000 jumps forward or back).
pub(super) fn find_label(commands: &[EventCommand], id: i32) -> Option<usize> {
    commands.iter().position(|c| c.code == LABEL && c.params.first().copied() == Some(id))
}

/// The option labels of a `ShowChoice` at `ip`/`indent`: each following
/// `ShowChoiceOption` (20140) `.string` at `indent`, until `ShowChoiceEnd`.
pub(super) fn choice_labels(commands: &[EventCommand], ip: usize, indent: u32) -> Vec<String> {
    let mut labels = Vec::new();
    for c in &commands[(ip + 1).min(commands.len())..] {
        if c.indent == indent && c.code == SHOW_CHOICE_END {
            break;
        }
        if c.indent == indent && c.code == SHOW_CHOICE_OPTION {
            labels.push(c.string.clone());
        }
    }
    labels
}

/// Index of the next `ShowChoiceOption` or `ShowChoiceEnd` at `indent` — where
/// execution resumes after skipping a non-selected option's body.
pub(super) fn skip_option_body(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len()
        && !(commands[j].indent == indent
            && matches!(commands[j].code, SHOW_CHOICE_OPTION | SHOW_CHOICE_END))
    {
        j += 1;
    }
    j
}

/// Index of the next `EnemyEncounter` outcome handler or `EndBattle` at `indent` —
/// where execution resumes after skipping a non-selected handler's body. Mirrors
/// [`skip_option_body`], tolerating a handler being absent (an escape/defeat
/// branch the map author omitted) by landing on whichever marker comes next.
pub(super) fn skip_battle_handler(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len()
        && !(commands[j].indent == indent
            && matches!(
                commands[j].code,
                VICTORY_HANDLER | ESCAPE_HANDLER | DEFEAT_HANDLER | END_BATTLE
            ))
    {
        j += 1;
    }
    j
}

/// Index of the `Loop` (12210) at `indent` that an `EndLoop` at `ip` closes
/// (scanning backward); falls back to `ip` if unmatched (a one-shot loop).
pub(super) fn loop_start(commands: &[EventCommand], ip: usize, indent: u32) -> usize {
    (0..ip)
        .rev()
        .find(|&j| commands[j].code == LOOP && commands[j].indent == indent)
        .unwrap_or(ip)
}

/// Index just past the `EndLoop` enclosing a `BreakLoop` at `ip` (indent
/// `break_indent`): the next `EndLoop` at a shallower indent.
pub(super) fn after_loop_end(commands: &[EventCommand], ip: usize, break_indent: u32) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && !(commands[j].code == END_LOOP && commands[j].indent < break_indent)
    {
        j += 1;
    }
    (j + 1).min(commands.len())
}

/// Index just past a subsystem block's terminator: the first command at
/// `indent` whose `code == terminator`, plus one (or the end of the list).
pub(super) fn skip_to_terminator(
    commands: &[EventCommand],
    ip: usize,
    indent: u32,
    terminator: u32,
) -> usize {
    let mut j = ip + 1;
    while j < commands.len() && !(commands[j].code == terminator && commands[j].indent == indent) {
        j += 1;
    }
    (j + 1).min(commands.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(code: u32, indent: u32) -> EventCommand {
        EventCommand { code, indent, string: String::new(), params: Vec::new() }
    }

    fn cmd_params(code: u32, indent: u32, params: Vec<i32>) -> EventCommand {
        EventCommand { code, indent, string: String::new(), params }
    }

    #[test]
    fn skips_whole_subsystem_block() {
        // 0: EnemyEncounter@0  1: VictoryHandler@0  2: body@1  3: EndBattle@0  4: after@0
        let commands =
            vec![cmd(10710, 0), cmd(20710, 0), cmd(10210, 1), cmd(20713, 0), cmd(10110, 0)];
        assert_eq!(skip_to_terminator(&commands, 0, 0, 20713), 4);
    }

    #[test]
    fn jump_finds_label_forward_and_back() {
        let commands = vec![
            cmd_params(12110, 0, vec![1]),
            cmd(10110, 0),
            cmd_params(12110, 0, vec![2]),
        ];
        assert_eq!(find_label(&commands, 2), Some(2));
        assert_eq!(find_label(&commands, 1), Some(0));
        assert_eq!(find_label(&commands, 9), None);
    }

    #[test]
    fn end_loop_jumps_back_to_loop() {
        // 0: Loop@0  1: Wait@1  2: EndLoop@0
        let commands = vec![cmd(12210, 0), cmd(11410, 1), cmd(22210, 0)];
        assert_eq!(loop_start(&commands, 2, 0), 0);
    }

    #[test]
    fn break_loop_skips_past_end() {
        // 0: Loop@0  1: Break@1  2: body@1  3: EndLoop@0  4: after@0
        let commands =
            vec![cmd(12210, 0), cmd(12220, 1), cmd(10110, 1), cmd(22210, 0), cmd(10110, 0)];
        assert_eq!(after_loop_end(&commands, 1, 1), 4);
    }

    #[test]
    fn choice_labels_collects_option_strings() {
        // ShowChoice@0; Option0 "Yes"@0 body@1; Option1 "No"@0 body@1; End@0
        let commands = vec![
            cmd_params(10140, 0, vec![0]),
            EventCommand { code: 20140, indent: 0, string: "Yes".into(), params: vec![0] },
            cmd(10210, 1),
            EventCommand { code: 20140, indent: 0, string: "No".into(), params: vec![1] },
            cmd(10210, 1),
            cmd(20141, 0),
        ];
        assert_eq!(choice_labels(&commands, 0, 0), vec!["Yes", "No"]);
    }

    #[test]
    fn skip_option_body_lands_on_next_option() {
        // 0: ShowChoice@0  1: Option0@0  2: body@1  3: Option1@0  4: End@0
        let commands =
            vec![cmd(10140, 0), cmd(20140, 0), cmd(10210, 1), cmd(20140, 0), cmd(20141, 0)];
        assert_eq!(skip_option_body(&commands, 1, 0), 3);
    }

    #[test]
    fn skip_battle_handler_lands_on_next_handler() {
        // 0: EnemyEncounter@0  1: Victory@0  2: body@1  3: Escape@0  4: EndBattle@0
        let commands =
            vec![cmd(10710, 0), cmd(20710, 0), cmd(10210, 1), cmd(20711, 0), cmd(20713, 0)];
        // From the Victory handler, skip its body → land on the Escape handler.
        assert_eq!(skip_battle_handler(&commands, 1, 0), 3);
    }

    #[test]
    fn skip_battle_handler_skips_absent_handler_to_end() {
        // 0: Victory@0  1: body@1  2: EndBattle@0 — no Escape/Defeat handlers.
        let commands = vec![cmd(20710, 0), cmd(10210, 1), cmd(20713, 0)];
        assert_eq!(skip_battle_handler(&commands, 0, 0), 2);
    }

    #[test]
    fn skip_true_body_with_else() {
        // 0: branch@0  1: body@1  2: else@0  3: elsebody@1  4: end@0
        let commands = vec![
            cmd(CONDITIONAL_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(ELSE_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(END_BRANCH, 0),
        ];
        // Not taken → jump into the else body at index 3.
        assert_eq!(skip_true_body(&commands, 0, 0), 3);
    }

    #[test]
    fn skip_true_body_without_else() {
        // 0: branch@0  1: body@1  2: end@0
        let commands =
            vec![cmd(CONDITIONAL_BRANCH, 0), cmd(SHOW_MESSAGE, 1), cmd(END_BRANCH, 0)];
        // Not taken, no else → land on the end marker at index 2.
        assert_eq!(skip_true_body(&commands, 0, 0), 2);
    }

    #[test]
    fn skip_else_body_lands_on_end() {
        let commands = vec![
            cmd(CONDITIONAL_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(ELSE_BRANCH, 0),
            cmd(SHOW_MESSAGE, 1),
            cmd(END_BRANCH, 0),
        ];
        // From the else marker at index 2, skip the else body → end at index 4.
        assert_eq!(skip_else_body(&commands, 2, 0), 4);
    }
}
