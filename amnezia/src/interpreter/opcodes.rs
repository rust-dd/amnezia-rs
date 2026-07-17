//! The RM2000 event-command opcodes the interpreter dispatches on, verified
//! empirically against the converted map assets. Shared by the main loop in
//! [`super`] and the flow helpers in the sibling modules.

pub(super) const SHOW_MESSAGE: u32 = 10110;
pub(super) const SHOW_MESSAGE_2: u32 = 20110;
pub(super) const CHANGE_FACE: u32 = 10130;
pub(super) const CONTROL_SWITCHES: u32 = 10210;
pub(super) const CONTROL_VARIABLES: u32 = 10220;
pub(super) const TELEPORT: u32 = 10810;
pub(super) const WAIT: u32 = 11410;
pub(super) const CONDITIONAL_BRANCH: u32 = 12010;
pub(super) const ELSE_BRANCH: u32 = 22010;
pub(super) const END_BRANCH: u32 = 22011;
pub(super) const ENEMY_ENCOUNTER: u32 = 10710;
pub(super) const OPEN_SHOP: u32 = 10720;
pub(super) const SHOW_INN: u32 = 10730;
pub(super) const CHANGE_GOLD: u32 = 10310;
pub(super) const CHANGE_ITEMS: u32 = 10320;
pub(super) const CHANGE_PARTY: u32 = 10330;

/// The `EnemyEncounter` block's outcome handlers and terminator: the interpreter
/// runs the body under the handler matching the finished battle outcome, skips
/// the rest, and `EndBattle` closes the block.
pub(super) const VICTORY_HANDLER: u32 = 20710;
pub(super) const ESCAPE_HANDLER: u32 = 20711;
pub(super) const DEFEAT_HANDLER: u32 = 20712;
pub(super) const END_BATTLE: u32 = 20713;
/// The merchant blocks' terminators, past which the interpreter resumes once the
/// shop or inn screen closes.
pub(super) const END_SHOP: u32 = 20722;
pub(super) const END_INN: u32 = 20732;
pub(super) const LABEL: u32 = 12110;
pub(super) const JUMP_TO_LABEL: u32 = 12120;
pub(super) const LOOP: u32 = 12210;
pub(super) const END_LOOP: u32 = 22210;
pub(super) const BREAK_LOOP: u32 = 12220;
pub(super) const SHOW_CHOICE: u32 = 10140;
pub(super) const SHOW_CHOICE_OPTION: u32 = 20140;
pub(super) const SHOW_CHOICE_END: u32 = 20141;
pub(super) const MOVE_EVENT: u32 = 11330;
pub(super) const PLAY_BGM: u32 = 11510;
pub(super) const FADE_OUT_BGM: u32 = 11520;
pub(super) const PLAY_SOUND: u32 = 11550;
