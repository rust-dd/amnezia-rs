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
/// Fully restore the party's HP/SP (`params[0] == 0` targets the whole party).
pub(super) const FULL_HEAL: u32 = 10490;
/// Prompt for a number written into a variable: `params = [digits, var_id]`.
pub(super) const INPUT_NUMBER: u32 = 10150;
/// Reskin an actor (ChangeActorGraphic): `string` = charset, `params = [actor_id, index, transparent]`.
pub(super) const CHANGE_SPRITE: u32 = 10630;
/// Move an event to a tile: `params = [event_ref, mode, x, y]` (mode 1 = coords from variables).
pub(super) const CHANGE_EVENT_LOCATION: u32 = 10860;
/// Message-box options: `params = [transparent, position(0 top/1 mid/2 bottom), …]`.
pub(super) const MESSAGE_OPTIONS: u32 = 10120;
/// Timer operation: `params = [op(0 set/1 start/2 stop), _, seconds, …]`.
pub(super) const TIMER: u32 = 10230;
/// Pan the map view: `params = [op(2 pan/3 return/0 lock/1 unlock), dir, dist, speed, _]`.
pub(super) const PAN_SCREEN: u32 = 11060;
/// Weather effect: `params = [type(0 none/1 rain/2 snow/3 fog), strength]`.
pub(super) const WEATHER: u32 = 11070;
/// Hero transparency: `params = [flag]` (1 = transparent, 0 = opaque).
pub(super) const PLAYER_TRANSPARENCY: u32 = 11310;
/// Play a battle animation on a character (ShowBattleAnimation): `params =
/// [anim_id, target_char_ref, wait, global]`. `target_char_ref` decodes like
/// 10860/11330 — 10001 = hero, 10005 = this event, else event id — and the
/// wait/global flags are ignored (the animation is fire-and-forget).
pub(super) const SHOW_BATTLE_ANIMATION: u32 = 11210;
/// Shop/inn outcome handlers, self-selected like the battle handlers (param-less).
pub(super) const TRANSACTION: u32 = 20720;
pub(super) const NO_TRANSACTION: u32 = 20721;
pub(super) const INN_STAY: u32 = 20730;
pub(super) const INN_CANCEL: u32 = 20731;

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
pub(super) const MEMORIZE_BGM: u32 = 11530;
pub(super) const PLAY_MEMORIZED_BGM: u32 = 11540;
pub(super) const PLAY_SOUND: u32 = 11550;

/// The presentation commands: screen effects and on-screen pictures.
pub(super) const ERASE_SCREEN: u32 = 11010;
pub(super) const SHOW_SCREEN: u32 = 11020;
pub(super) const TINT_SCREEN: u32 = 11030;
pub(super) const FLASH_SCREEN: u32 = 11040;
pub(super) const SHAKE_SCREEN: u32 = 11050;
pub(super) const SHOW_PICTURE: u32 = 11110;
pub(super) const MOVE_PICTURE: u32 = 11120;
pub(super) const ERASE_PICTURE: u32 = 11130;

/// End the game to the Game Over screen.
pub(super) const GAME_OVER: u32 = 12420;

/// Open the save menu (RM2000 `SaveMenu`). The remake performs a single-slot save
/// directly, so this needs no interactive screen; the save crystal's action page
/// runs it.
pub(super) const OPEN_SAVE_MENU: u32 = 11910;

/// Change an actor's level: `params = [mode, id, operation, operand_type, operand,
/// show_msg]`. Applied through the experience curve (see `apply_change_level`).
pub(super) const CHANGE_LEVEL: u32 = 10420;
/// Set actor `params[0]`'s name to `string`. The remake tracks only the hero
/// (actor 1) name, which `\N[k]` inserts.
pub(super) const CHANGE_HERO_NAME: u32 = 10610;
/// Memorize the party's position: write the current map id and the hero's tile
/// into the three variables `params = [var_map, var_x, var_y]`.
pub(super) const MEMORIZE_LOCATION: u32 = 10820;
/// Teleport the hero to a memorized position read back from `params = [var_map,
/// var_x, var_y]`; pauses through the fade like [`TELEPORT`].
pub(super) const RECALL_TO_LOCATION: u32 = 10830;
/// Cancel every character's pending forced movement (`params` empty).
pub(super) const HALT_ALL_MOVEMENT: u32 = 11350;
/// Wait for (or sample) a key press and store its RM2000 code in a variable:
/// `params = [var_id, wait, dirs/decision/cancel flags…]`.
pub(super) const KEY_INPUT_PROC: u32 = 11610;
/// Enable or disable the in-menu Save command: `params = [allow]` (0 off, 1 on).
pub(super) const CHANGE_SAVE_ACCESS: u32 = 11930;
/// Enable or disable opening the main menu: `params = [allow]` (0 off, 1 on).
pub(super) const CHANGE_MENU_ACCESS: u32 = 11960;
/// Call another event's page (or a common event) as a sub-frame: `params = [mode,
/// event_ref, page]`; only map-event mode (1) occurs in the data.
pub(super) const CALL_EVENT: u32 = 12330;
/// Return to the title screen (`params` empty); ends the running event.
pub(super) const RETURN_TO_TITLE: u32 = 12510;

/// Learn or forget an actor skill. Decoded but a no-op: the game has no per-actor
/// skill state (the menu casts from the whole skill database).
pub(super) const CHANGE_SKILLS: u32 = 10440;
/// Equip or unequip an actor. Decoded but a no-op: equipment is view-only here.
pub(super) const CHANGE_EQUIPMENT: u32 = 10450;
/// Add or remove an actor state/condition. A no-op: the remake carries no
/// persistent field conditions (every observed use is a state removal).
pub(super) const CHANGE_CONDITION: u32 = 10480;
/// Set a system BGM slot (battle, victory, …). A no-op: music is driven by
/// explicit `PlayBgm` events, and the system slots have no consumer.
pub(super) const CHANGE_SYSTEM_BGM: u32 = 10660;
/// Choose the map/battle transition style. A cosmetic no-op: the fade is fixed.
pub(super) const CHANGE_SCREEN_TRANSITIONS: u32 = 10690;
/// Board or leave a vehicle. A no-op: the game has no vehicles.
pub(super) const ENTER_EXIT_VEHICLE: u32 = 10840;
/// Reposition a vehicle. A no-op: the game has no vehicles.
pub(super) const SET_VEHICLE_LOCATION: u32 = 10850;
/// Flash a character sprite. A cosmetic no-op: no per-sprite flash effect exists.
pub(super) const FLASH_SPRITE: u32 = 11320;
/// Change the panorama background. A no-op: no parallax/panorama renderer exists.
pub(super) const CHANGE_PBG: u32 = 11720;
/// An editor comment; carries no runtime effect.
pub(super) const COMMENT: u32 = 12410;
/// A comment continuation line; carries no runtime effect.
pub(super) const COMMENT_2: u32 = 22410;
/// The block/event terminator marker; flow continues past it.
pub(super) const END_MARKER: u32 = 10;
