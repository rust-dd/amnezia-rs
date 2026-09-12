//! The menu's input systems. Escape toggles the menu and backs out
//! of a sub-screen; Up/Down move the active cursor; Enter/Space confirm. The pure
//! navigation math lives in [`super::nav`]; these systems only apply it and touch
//! the world (inventory, vitals, save request, title).

use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::battle::BattleActive;
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::interpreter::RunningEvent;
use crate::progression::Progression;
use crate::save::{SaveAccess, SaveRequest};
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party};
use crate::teleport::Fade;
use crate::title::TitleActive;
use crate::vitals::Vitals;
use amnezia_data::SoundDef;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::nav::{
    confirm_pressed, end_game_transition, escape_transition, item_target, skill_target, step,
};
use super::{MenuAccess, MenuOpen, MenuScreen, MenuState, command, equip, items, skills, use_item};

/// The transient overlays and flows that must not be interrupted by *opening* the
/// menu: a message box, a running event, a choice or number prompt, a teleport
/// fade, or the game-over hand-off. Mirrors the open-guard lists in
/// [`crate::dialogue`]'s `interact` and [`crate::interpreter`]'s autorun reader;
/// bundled into one `SystemParam` so [`menu_input`] stays within Bevy's
/// 16-parameter cap. A menu already up ignores these — only opening is gated.
#[derive(SystemParam)]
pub(super) struct OpenBlockers<'w> {
    transition: Option<Res<'w, crate::transitions::Transition>>,
    dialogue: Res<'w, Dialogue>,
    running: Res<'w, RunningEvent>,
    choice: Res<'w, Choice>,
    input_number: Res<'w, InputNumber>,
    fade: Res<'w, Fade>,
    gameover: Res<'w, GameOverActive>,
}

impl OpenBlockers<'_> {
    /// Whether any transient flow is live, so the menu must refuse to open.
    fn any(&self) -> bool {
        self.dialogue.active
            || self.transition.as_ref().is_some_and(|v| v.busy())
            || self.running.active()
            || self.choice.active()
            || self.input_number.active()
            || self.fade.busy()
            || self.gameover.0
    }
}

/// The resources that gate *opening* the menu and its save entry: a live shop or
/// battle, the title screen, and the cutscene menu/save access flags. Bundled into
/// one `SystemParam` so [`menu_input`] stays within Bevy's 16-parameter cap (the
/// interactive equip screen added the runtime [`Equipment`] store). `title` stays a
/// separate parameter because End Game mutates it.
#[derive(SystemParam)]
pub(super) struct MenuGates<'w> {
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    menu_access: Res<'w, MenuAccess>,
    save_access: Res<'w, SaveAccess>,
}

/// The menu's navigation sound-effect channel: the audio writer and the loaded
/// [`SystemSounds`]. Bundled into one `SystemParam` so [`menu_input`] stays within
/// Bevy's 16-parameter cap. Each helper plays a system SE (a no-op until the
/// sounds load), mirroring the choice/number boxes' cursor/decision/cancel cues.
#[derive(SystemParam)]
pub(super) struct MenuSfx<'w> {
    audio: MessageWriter<'w, AudioRequest>,
    sounds: Option<Res<'w, SystemSounds>>,
}

impl MenuSfx<'_> {
    fn play(&mut self, pick: impl FnOnce(&SystemSounds) -> &SoundDef) {
        if let Some(sounds) = &self.sounds {
            play_system_se(&mut self.audio, pick(sounds));
        }
    }

    /// The cursor-move cue for list navigation.
    fn cursor(&mut self) {
        self.play(|s| &s.cursor);
    }

    /// The confirm cue, played when a selection is entered.
    fn decision(&mut self) {
        self.play(|s| &s.decision);
    }

    /// The back/close cue, played when Escape backs out of or shuts the menu.
    fn cancel(&mut self) {
        self.play(|s| &s.cancel);
    }

    fn buzzer(&mut self) {
        self.play(|s| &s.buzzer);
    }
}

/// Toggle the menu on Escape (backing out of a sub-screen first) and drive the
/// active screen: move the cursor and confirm into the next screen, apply a field
/// item or skill, request a save, or return to the title on End Game.
#[allow(clippy::too_many_arguments)]
pub(super) fn menu_input(
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    mut inventory: ResMut<Inventory>,
    mut vitals: ResMut<Vitals>,
    mut equipment: ResMut<Equipment>,
    gates: MenuGates,
    mut title: ResMut<TitleActive>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
    mut save_request: ResMut<SaveRequest>,
    blockers: OpenBlockers,
    mut sfx: MenuSfx,
    mut skill_rng: Local<crate::interpreter::EventRng>,
) {
    // A shop, battle, the title screen, a cutscene that locked menu access (opcode
    // 11960), or any live overlay/flow (message box, event, choice, number prompt,
    // fade, game over) owns the input, so the menu can't open over it. Only opening
    // is gated — a menu already up stays usable and closable.
    if !open.0
        && (gates.shop.0 || gates.battle.0 || title.0 || !gates.menu_access.0 || blockers.any())
    {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        let was_open = open.0;
        let (next_open, next_screen) = escape_transition(open.0, state.screen);
        // Opening the menu is a decision cue; backing out of a sub-screen or
        // closing it is a cancel cue.
        if next_open && !was_open {
            sfx.decision();
        } else {
            sfx.cancel();
        }
        open.0 = next_open;
        state.screen = next_screen;
        if next_open && !was_open {
            state.cursor = 0;
        }
        return;
    }
    if !open.0 {
        return;
    }
    // The F5 / Esc-S quick save from anywhere in the menu (silent, no prompt),
    // blocked while a cutscene has disabled save access (opcode 11930).
    if gates.save_access.0 && keys.just_pressed(KeyCode::KeyS) {
        save_request.0 = true;
    }
    let confirm = confirm_pressed(&keys);
    let up = keys.just_pressed(KeyCode::ArrowUp);
    let down = keys.just_pressed(KeyCode::ArrowDown);
    let members = party.snapshot().len().saturating_sub(1);
    if up || down {
        sfx.cursor();
    }
    if confirm
        && !matches!(
            state.screen,
            MenuScreen::Command | MenuScreen::MemberSelect { .. }
        )
    {
        sfx.decision();
    }
    match state.screen {
        MenuScreen::Command => {
            state.cursor = step(state.cursor, up, down, command::COMMANDS.len() - 1);
            if confirm {
                let command = command::COMMANDS[state.cursor];
                if !command::enabled(command, party.snapshot().len(), gates.save_access.0) {
                    sfx.buzzer();
                    return;
                }
                sfx.decision();
                match command::dispatch(command) {
                    command::CommandAction::Open(screen) => state.screen = screen,
                    command::CommandAction::Save => {
                        save_request.0 = true;
                        state.screen = MenuScreen::Saved;
                    }
                }
            }
        }
        MenuScreen::ItemList { cursor } => {
            let max = items::selectable(&data, &inventory).saturating_sub(1);
            let cursor = step(cursor, up, down, max);
            state.screen = MenuScreen::ItemList { cursor };
            if confirm && let Some(next) = item_target(cursor, &data, &inventory) {
                state.screen = next;
            }
        }
        MenuScreen::ItemTarget { item_id, cursor } => {
            let cursor = step(cursor, up, down, members);
            state.screen = MenuScreen::ItemTarget { item_id, cursor };
            if confirm
                && use_item::apply_field_item(
                    item_id,
                    cursor,
                    &data,
                    &party,
                    &progression,
                    &mut inventory,
                    &mut vitals,
                )
            {
                state.screen = MenuScreen::ItemList { cursor: 0 };
            }
        }
        MenuScreen::MemberSelect { action, cursor } => {
            let cursor = step(cursor, up, down, members);
            state.screen = MenuScreen::MemberSelect { action, cursor };
            if confirm {
                if command::member_enabled(action, cursor, &party, &vitals) {
                    sfx.decision();
                    state.screen = command::member_screen(action, cursor);
                } else {
                    sfx.buzzer();
                }
            }
        }
        MenuScreen::SkillList { member, cursor } => {
            let count = skills::known_skills(member, &data, &party, &progression).len();
            let cursor = step(cursor, up, down, count.saturating_sub(1));
            state.screen = MenuScreen::SkillList { member, cursor };
            if confirm && let Some(next) = skill_target(member, cursor, &data, &party, &progression)
            {
                state.screen = next;
            }
        }
        MenuScreen::SkillTarget {
            member,
            skill_id,
            cursor,
        } => {
            let cursor = step(cursor, up, down, members);
            state.screen = MenuScreen::SkillTarget {
                member,
                skill_id,
                cursor,
            };
            if confirm
                && skills::apply_field_skill(
                    member,
                    cursor,
                    skill_id,
                    &data,
                    &party,
                    &progression,
                    &mut vitals,
                    &equipment,
                    &mut skill_rng,
                )
            {
                state.screen = MenuScreen::SkillList { member, cursor: 0 };
            }
        }
        MenuScreen::Equip {
            member,
            slot,
            picking: None,
        } => {
            // Slot selection: move over the five gear rows; a confirm opens the
            // item picker for the slot, unless the actor's gear is fixed.
            let slot = step(slot, up, down, 4);
            state.screen = MenuScreen::Equip {
                member,
                slot,
                picking: None,
            };
            if confirm && equip::can_change(member, &data, &party) {
                state.screen = MenuScreen::Equip {
                    member,
                    slot,
                    picking: Some(0),
                };
            }
        }
        MenuScreen::Equip {
            member,
            slot,
            picking: Some(cursor),
        } => {
            // Item selection: move over the slot's candidate items (unequip plus
            // the held gear of that type); a confirm swaps and returns to the slot
            // list.
            let count = equip::candidates(member, slot, &data, &party, &inventory).len();
            let cursor = step(cursor, up, down, count.saturating_sub(1));
            state.screen = MenuScreen::Equip {
                member,
                slot,
                picking: Some(cursor),
            };
            if confirm {
                equip::apply(
                    member,
                    slot,
                    cursor,
                    &data,
                    &party,
                    &mut inventory,
                    &mut equipment,
                );
                state.screen = MenuScreen::Equip {
                    member,
                    slot,
                    picking: None,
                };
            }
        }
        MenuScreen::Status { .. } => {}
        MenuScreen::Saved => {
            if confirm {
                state.screen = MenuScreen::Command;
            }
        }
        MenuScreen::EndGame { cursor } => {
            let cursor = step(cursor, up, down, 1);
            state.screen = MenuScreen::EndGame { cursor };
            if confirm {
                let (next_open, next_title) = end_game_transition(cursor, title.0);
                open.0 = next_open;
                title.0 = next_title;
                state.screen = MenuScreen::Command;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::testkit;

    mod guards;

    /// A headless app with just the menu input system and the resources it reads,
    /// opened on `screen` with `cursor` as the command-list cursor.
    fn app_on(cursor: usize, screen: MenuScreen) -> App {
        let mut app = App::new();
        app.insert_resource(testkit::data())
            .init_resource::<Party>()
            .init_resource::<Progression>()
            .init_resource::<Inventory>()
            .init_resource::<Vitals>()
            .init_resource::<Equipment>()
            .insert_resource(ShopOpen(false))
            .insert_resource(BattleActive(false))
            .insert_resource(TitleActive(false))
            .init_resource::<MenuAccess>()
            .init_resource::<SaveAccess>()
            .insert_resource(MenuOpen(true))
            .insert_resource(MenuState { cursor, screen })
            .init_resource::<SaveRequest>()
            .init_resource::<Dialogue>()
            .init_resource::<RunningEvent>()
            .init_resource::<Choice>()
            .init_resource::<InputNumber>()
            .init_resource::<Fade>()
            .init_resource::<GameOverActive>()
            .init_resource::<ButtonInput<KeyCode>>()
            // `menu_input` writes navigation SE; register the channel (SystemSounds
            // is optional, so the tests run without loading the effects).
            .add_message::<AudioRequest>()
            .add_systems(Update, menu_input);
        app
    }

    /// Press a key and run one input frame.
    fn confirm(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
    }

    /// Release every held key, press `key` afresh, and run one frame — so a
    /// multi-step interaction sees a genuine just-pressed each step (no input plugin
    /// runs to reset it in these headless apps, and `press` only re-arms
    /// `just_pressed` for a newly held key).
    fn press_frame(app: &mut App, key: KeyCode) {
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.reset_all();
            input.press(key);
        }
        app.update();
    }

    #[test]
    fn save_command_saves_and_confirms_when_save_access_is_enabled() {
        // Cursor 3 is "Mentés" (Save) in the RM2000 command order. Manual save is
        // disabled by default (a crystal-save game); enable it as a save crystal
        // transiently does, and the Save command then writes and confirms.
        let mut app = app_on(3, MenuScreen::Command);
        app.world_mut().insert_resource(SaveAccess(true));
        confirm(&mut app, KeyCode::Enter);
        assert!(
            app.world().resource::<SaveRequest>().0,
            "with save access on, the Save command must raise SaveRequest"
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Saved,
            "and show the save confirmation screen"
        );
    }

    #[test]
    fn menu_stays_closed_when_menu_access_is_disabled() {
        // A cutscene disabled menu access (opcode 11960): Escape must not open it.
        let mut app = app_on(0, MenuScreen::Command);
        app.world_mut().insert_resource(MenuOpen(false));
        app.world_mut().insert_resource(MenuAccess(false));
        confirm(&mut app, KeyCode::Escape);
        assert!(
            !app.world().resource::<MenuOpen>().0,
            "the menu must stay closed while access is disabled"
        );
    }

    #[test]
    fn menu_will_not_open_while_a_dialogue_is_active() {
        // A message box owns the input: Escape must not pop the menu open over it.
        let mut app = app_on(0, MenuScreen::Command);
        app.world_mut().insert_resource(MenuOpen(false));
        app.world_mut().resource_mut::<Dialogue>().active = true;
        confirm(&mut app, KeyCode::Escape);
        assert!(
            !app.world().resource::<MenuOpen>().0,
            "the menu must refuse to open while a dialogue is showing"
        );
    }

    #[test]
    fn open_menu_still_closes_while_a_blocker_would_forbid_opening() {
        // The guard gates opening only: an already-open menu closes on Escape even
        // if a transient flow (here a running event) is flagged active.
        let mut app = app_on(0, MenuScreen::Command);
        app.world_mut().resource_mut::<Dialogue>().active = true;
        confirm(&mut app, KeyCode::Escape);
        assert!(
            !app.world().resource::<MenuOpen>().0,
            "an open menu must still close despite an active blocker"
        );
    }

    #[test]
    fn save_command_is_inert_when_save_access_is_disabled() {
        // Save access disabled (opcode 11930): the Save entry raises no request.
        let mut app = app_on(3, MenuScreen::Command);
        app.world_mut().insert_resource(SaveAccess(false));
        confirm(&mut app, KeyCode::Enter);
        assert!(
            !app.world().resource::<SaveRequest>().0,
            "Save must be inert while save access is disabled"
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command,
            "and the menu stays on the command list"
        );
    }

    #[test]
    fn the_equip_screen_swaps_gear_through_the_runtime_store() {
        let mut app = app_on(
            0,
            MenuScreen::Equip {
                member: 0,
                slot: 0,
                picking: None,
            },
        );
        {
            let mut data = app.world_mut().resource_mut::<GameData>();
            data.actors[0].weapon = 10; // starting short-sword
            data.items.push(testkit::weapon(10, "Rövidkard", 4));
            data.items.push(testkit::weapon(12, "Hosszúkard", 12));
        }
        app.world_mut().resource_mut::<Inventory>().add_item(12, 1);

        // Confirm the weapon slot -> the item picker opens.
        press_frame(&mut app, KeyCode::Enter);
        assert!(
            matches!(
                app.world().resource::<MenuState>().screen,
                MenuScreen::Equip {
                    picking: Some(_),
                    ..
                }
            ),
            "confirming a slot opens the item picker"
        );
        // Move to the long-sword candidate (row 1; row 0 is unequip), then confirm.
        press_frame(&mut app, KeyCode::ArrowDown);
        press_frame(&mut app, KeyCode::Enter);

        // Back on the slot list, with the swap done through the store + inventory.
        assert!(matches!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Equip { picking: None, .. }
        ));
        let worn = {
            let data = app.world().resource::<GameData>();
            let def = data.actor(1).unwrap();
            app.world().resource::<Equipment>().slots(def)[0]
        };
        assert_eq!(worn, 12, "the long-sword is now worn");
        let inv = app.world().resource::<Inventory>();
        assert_eq!(inv.count(12), 0, "it left the bag");
        assert_eq!(inv.count(10), 1, "the short-sword returned to the bag");
    }

    #[test]
    fn end_game_igen_closes_the_menu_and_raises_the_title() {
        // Confirming Igen (cursor 0) on the End Game prompt returns to the title.
        let mut app = app_on(4, MenuScreen::EndGame { cursor: 0 });
        confirm(&mut app, KeyCode::Enter);
        assert!(!app.world().resource::<MenuOpen>().0, "menu closed");
        assert!(
            app.world().resource::<TitleActive>().0,
            "title raised (return-to-title, opcode 12510)"
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            MenuScreen::Command,
            "and the menu resets to the command list for next time"
        );
    }
}
