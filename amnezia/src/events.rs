//! Event interpreter: turns an event page's RM2000 command list into the
//! message boxes the game shows. This first pass handles the dialogue commands
//! (`ShowMessage` 10110, `ShowMessage_2` 20110, `ChangeFaceGraphic` 10130);
//! other commands are ignored for now.

use amnezia_data::EventCommand;

const SHOW_MESSAGE: u32 = 10110;
const SHOW_MESSAGE_2: u32 = 20110;
const CHANGE_FACE: u32 = 10130;

/// One message box: up to four lines of text and an optional face graphic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBox {
    pub face: Option<String>,
    pub lines: Vec<String>,
}

/// Collect the sequence of message boxes a page's commands display, in order.
/// `ShowMessage` starts a new box; `ShowMessage_2` appends a line to the
/// current box; `ChangeFaceGraphic` sets the face for boxes that follow.
pub fn message_boxes(commands: &[EventCommand]) -> Vec<MessageBox> {
    let mut boxes = Vec::new();
    let mut face: Option<String> = None;
    for command in commands {
        match command.code {
            CHANGE_FACE => {
                face = if command.string.is_empty() {
                    None
                } else {
                    Some(command.string.clone())
                };
            }
            SHOW_MESSAGE => {
                boxes.push(MessageBox { face: face.clone(), lines: vec![command.string.clone()] });
            }
            SHOW_MESSAGE_2 => {
                if let Some(current) = boxes.last_mut() {
                    current.lines.push(command.string.clone());
                }
            }
            _ => {}
        }
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(code: u32, string: &str) -> EventCommand {
        EventCommand { code, indent: 0, string: string.to_string(), params: Vec::new() }
    }

    #[test]
    fn groups_lines_into_boxes() {
        let commands = vec![
            cmd(10110, "Line 1"),
            cmd(20110, "Line 2"),
            cmd(10110, "Second box"),
        ];
        let boxes = message_boxes(&commands);
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].lines, vec!["Line 1", "Line 2"]);
        assert_eq!(boxes[1].lines, vec!["Second box"]);
    }

    #[test]
    fn face_applies_to_following_boxes() {
        let commands = vec![cmd(10130, "Ron"), cmd(10110, "Hi")];
        let boxes = message_boxes(&commands);
        assert_eq!(boxes[0].face, Some("Ron".to_string()));
    }

    #[test]
    fn ignores_unrelated_commands() {
        let commands = vec![cmd(10210, "switch"), cmd(11330, "move")];
        assert!(message_boxes(&commands).is_empty());
    }
}
