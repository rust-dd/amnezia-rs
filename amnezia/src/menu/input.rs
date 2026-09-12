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
    prompt_frame: Option<Res<'w, crate::dialogue::PromptFrame>>,
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
        self.prompt_frame
            .as_ref()
            .is_some_and(|frame| frame.active())
            || self.dialogue.active
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
    // These guards block opening; an existing menu must remain usable and closable.
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
    // The shortcut respects the same cutscene save restriction as the command.
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
mod tests;
