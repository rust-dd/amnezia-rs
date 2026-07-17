//! Message assembly: turns a run of RM2000 message commands into the boxes the
//! game shows. The interpreter calls [`message_boxes`] on the message run it
//! reaches; teleports, switches, and the rest are executed by the interpreter.

use amnezia_data::EventCommand;

const SHOW_MESSAGE: u32 = 10110;
const SHOW_MESSAGE_2: u32 = 20110;
const CHANGE_FACE: u32 = 10130;

/// One message box: up to four lines of text and an optional face graphic (the
/// FaceSet name plus which of its 16 faces, from `ChangeFaceGraphic`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBox {
    pub face: Option<String>,
    pub face_index: u32,
    pub lines: Vec<String>,
}

/// Collect the sequence of message boxes a command run displays, in order.
/// `ShowMessage` starts a new box; `ShowMessage_2` appends a line to the
/// current box; `ChangeFaceGraphic` sets the face (name + `params[0]` index)
/// for boxes that follow.
pub fn message_boxes(commands: &[EventCommand]) -> Vec<MessageBox> {
    let mut boxes = Vec::new();
    let mut face: Option<String> = None;
    let mut face_index = 0;
    for command in commands {
        match command.code {
            CHANGE_FACE => {
                face = if command.string.is_empty() {
                    None
                } else {
                    Some(command.string.clone())
                };
                face_index = command.params.first().copied().unwrap_or(0).max(0) as u32;
            }
            SHOW_MESSAGE => {
                boxes.push(MessageBox {
                    face: face.clone(),
                    face_index,
                    lines: vec![command.string.clone()],
                });
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
        EventCommand {
            code,
            indent: 0,
            string: string.to_string(),
            params: Vec::new(),
        }
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

    fn cmd_params(code: u32, string: &str, params: Vec<i32>) -> EventCommand {
        EventCommand {
            code,
            indent: 0,
            string: string.to_string(),
            params,
        }
    }

    #[test]
    fn face_applies_to_following_boxes() {
        let commands = vec![cmd_params(10130, "Ron", vec![6, 0, 0]), cmd(10110, "Hi")];
        let boxes = message_boxes(&commands);
        assert_eq!(boxes[0].face, Some("Ron".to_string()));
        assert_eq!(boxes[0].face_index, 6);
    }

    #[test]
    fn ignores_unrelated_commands() {
        let commands = vec![cmd(10210, "switch"), cmd(11330, "move")];
        assert!(message_boxes(&commands).is_empty());
    }
}
