use super::super::flow::{choice_labels, skip_to_terminator};
use super::super::frame::Frame;
use super::super::opcodes::*;
use super::{Exec, Flow};
use crate::dialogue::MessagePrompt;
use amnezia_data::EventCommand;

pub(super) fn show(frame: &mut Frame, x: &mut Exec) -> Flow {
    let run_len = frame.commands[frame.ip..]
        .iter()
        .take_while(|command| matches!(command.code, SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE))
        .count();
    let mut boxes = crate::events::message_boxes(
        &frame.commands[frame.ip..frame.ip + run_len],
        &mut x.dialogue.face,
    );
    frame.ip += run_len;
    if boxes.is_empty() {
        return Flow::Advance;
    }
    for message in &mut boxes {
        for line in &mut message.lines {
            *line = crate::i18n::tr(line);
        }
    }
    x.dialogue.open(boxes);
    frame.message_pending = true;
    if frame.commands[frame.ip - 1].code != CHANGE_FACE {
        append_prompt(frame, x);
    }
    Flow::Yield
}

fn append_prompt(frame: &mut Frame, x: &mut Exec) {
    let Some(command) = frame.commands.get(frame.ip) else {
        return;
    };
    match command.code {
        SHOW_CHOICE => {
            let labels = choice_labels(&frame.commands, frame.ip, command.indent)
                .iter()
                .take(4)
                .map(|label| crate::i18n::tr(label))
                .collect();
            frame.choice_pending = x.dialogue.append_prompt(MessagePrompt::Choice {
                labels,
                indent: command.indent,
                cancel: command.params.first().copied().unwrap_or(0),
            });
        }
        INPUT_NUMBER => {
            frame.input_pending = x.dialogue.append_prompt(MessagePrompt::Number {
                digits: command.params.first().copied().unwrap_or(0).max(0) as u32,
                variable: command.params.get(1).copied().unwrap_or(0) as u32,
            });
        }
        _ => {}
    }
}

pub(super) fn choice(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    if frame.choices.contains_key(&command.indent) {
        frame.ip += 1;
        return Flow::Advance;
    }
    let labels = choice_labels(&frame.commands, frame.ip, command.indent)
        .iter()
        .take(4)
        .map(|label| crate::i18n::tr(label))
        .collect();
    if x.dialogue.open_prompt(MessagePrompt::Choice {
        labels,
        indent: command.indent,
        cancel: command.params.first().copied().unwrap_or(0),
    }) {
        frame.message_pending = true;
        frame.choice_pending = true;
        Flow::Yield
    } else {
        frame.ip = skip_to_terminator(&frame.commands, frame.ip, command.indent, SHOW_CHOICE_END);
        Flow::Advance
    }
}
