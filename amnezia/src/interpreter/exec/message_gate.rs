use super::super::opcodes::*;
use amnezia_data::EventCommand;

pub(super) fn needs_free_message(command: &EventCommand) -> bool {
    match command.code {
        SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE | MESSAGE_OPTIONS | SHOW_CHOICE
        | INPUT_NUMBER | TELEPORT | RECALL_TO_LOCATION | ENEMY_ENCOUNTER | OPEN_SHOP
        | ERASE_SCREEN | SHOW_SCREEN | GAME_OVER | OPEN_SAVE_MENU | RETURN_TO_TITLE => true,
        // The original non-English RPG2000 runtime also blocks picture commands.
        SHOW_PICTURE | MOVE_PICTURE | ERASE_PICTURE => true,
        // Paid inns in parallel events overwrite an existing message in RPG_RT.
        SHOW_INN => command.params.get(1).copied().unwrap_or(0) == 0,
        CHANGE_LEVEL => command.params.get(5).copied().unwrap_or(0) != 0,
        _ => false,
    }
}
