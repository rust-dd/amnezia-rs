//! The menu's input systems. Escape toggles the menu and backs out
//! of a sub-screen; Up/Down move the active cursor; Enter/Space confirm. The pure
//! navigation math lives in [`super::nav`]; these systems only apply it and touch
//! the world (inventory, vitals, save request, title).

use super::save_files::SaveFiles;
use crate::battle::BattleActive;
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::interpreter::RunningEvent;
use crate::progression::Progression;
use crate::save::SaveAccess;
use crate::shop::ShopOpen;
use crate::state::{Inventory, Party};
use crate::teleport::Fade;
use crate::title::TitleActive;
use crate::vitals::Vitals;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::nav::{
    confirm_pressed, end_game_transition, escape_transition, item_target, skill_target,
};
use super::{MenuAccess, MenuOpen, MenuScreen, MenuState, command, equip, items, skills, use_item};

mod sounds;
use sounds::MenuSfx;

/// The transient overlays and flows that must not be interrupted by *opening* the
/// menu: a message box, a running event, a choice or number prompt, a teleport
/// fade, or the game-over hand-off. Mirrors the open-guard lists in
/// [`crate::dialogue`]'s `interact` and [`crate::interpreter`]'s autorun reader;
/// bundled into one `SystemParam` so [`menu_input`] stays within Bevy's
/// 16-parameter cap. A menu already up ignores these — only opening is gated.
#[derive(SystemParam)]
pub(super) struct OpenBlockers<'w> {
    frame: Option<Res<'w, crate::timing::SceneWait>>,
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
/// one `SystemParam` so [`menu_input`] stays within Bevy's 16-parameter cap.
/// `title` stays separate because End Game mutates it.
#[derive(SystemParam)]
pub(super) struct MenuGates<'w> {
    scene: Option<Res<'w, super::SceneFlow>>,
    switch: Option<ResMut<'w, equip::Switch>>,
    shop: Res<'w, ShopOpen>,
    battle: Res<'w, BattleActive>,
    menu_access: Res<'w, MenuAccess>,
    save_access: Res<'w, SaveAccess>,
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
    mut gates: MenuGates,
    mut title: ResMut<TitleActive>,
    mut open: ResMut<MenuOpen>,
    mut state: ResMut<MenuState>,
    mut save_files: ResMut<SaveFiles>,
    mut item_list: ResMut<items::List>,
    blockers: OpenBlockers,
    mut sfx: MenuSfx,
    mut skill_rng: Local<crate::interpreter::EventRng>,
) {
    if save_files.active()
        || gates.switch.as_ref().is_some_and(|switch| switch.active())
        || gates.scene.as_ref().is_some_and(|scene| scene.active())
        || blockers.frame.as_ref().is_some_and(|frame| frame.0)
    {
        return;
    }
    if blockers.fade.busy()
        || blockers
            .transition
            .as_ref()
            .is_some_and(|transition| transition.busy())
    {
        return;
    }
    // These guards block opening; an existing menu must remain usable and closable.
    if !open.0
        && (gates.shop.0 || gates.battle.0 || title.0 || !gates.menu_access.0 || blockers.any())
    {
        return;
    }
    if keys.just_pressed(KeyCode::Escape)
        && !(open.0
            && matches!(
                state.screen,
                MenuScreen::ItemTarget { .. } | MenuScreen::SkillTarget { .. }
            ))
    {
        let was_open = open.0;
        let (next_open, next_screen) = if matches!(state.screen, MenuScreen::ItemTarget { .. }) {
            (open.0, item_list.return_to_list(&data, &inventory))
        } else {
            escape_transition(open.0, state.screen)
        };
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
    if gates.save_access.0 && keys.just_pressed(KeyCode::KeyS) {
        save_files.request();
        sfx.decision();
        return;
    }
    let confirm = confirm_pressed(&keys);
    let members = party.snapshot().len().saturating_sub(1);
    if confirm
        && !matches!(
            state.screen,
            MenuScreen::Command
                | MenuScreen::MemberSelect { .. }
                | MenuScreen::ItemList { .. }
                | MenuScreen::ItemTarget { .. }
                | MenuScreen::SkillList { .. }
                | MenuScreen::SkillTarget { .. }
                | MenuScreen::Equip { .. }
        )
    {
        sfx.decision();
    }
    match state.screen {
        MenuScreen::Command => {
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
                        save_files.request();
                    }
                }
            }
        }
        MenuScreen::ItemList { cursor } => {
            if confirm {
                if let Some(next) = item_target(cursor, &data, &inventory) {
                    sfx.decision();
                    state.screen = next;
                } else {
                    sfx.buzzer();
                }
            }
        }
        MenuScreen::ItemTarget { item_id, cursor } => {
            if confirm {
                if use_item::apply_field_item(
                    item_id,
                    cursor,
                    &data,
                    &party,
                    &progression,
                    &mut inventory,
                    &mut vitals,
                ) {
                    sfx.play(|sounds| &sounds.item);
                } else {
                    sfx.buzzer();
                }
            }
            if keys.just_pressed(KeyCode::Escape) {
                sfx.cancel();
                state.screen = item_list.return_to_list(&data, &inventory);
            }
        }
        MenuScreen::MemberSelect { action, cursor } => {
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
            if confirm {
                if let Some(next) = skill_target(
                    member,
                    cursor,
                    &data,
                    &party,
                    &progression,
                    &vitals,
                    &equipment,
                ) {
                    sfx.decision();
                    state.screen = next;
                } else {
                    sfx.buzzer();
                }
            }
        }
        MenuScreen::SkillTarget {
            member,
            skill_id,
            cursor,
        } => {
            if confirm {
                if skills::apply_field_skill(
                    member,
                    cursor,
                    skill_id,
                    &data,
                    &party,
                    &progression,
                    &mut vitals,
                    &equipment,
                    &mut skill_rng,
                ) {
                    sfx.skill(skill_id, &data);
                } else {
                    sfx.buzzer();
                }
            }
            if keys.just_pressed(KeyCode::Escape) {
                sfx.cancel();
                let cursor = skills::known_skills(member, &data, &party, &progression)
                    .iter()
                    .position(|skill| skill.id == skill_id)
                    .unwrap_or(0);
                state.screen = MenuScreen::SkillList { member, cursor };
            }
        }
        MenuScreen::Equip {
            member,
            slot,
            picking: None,
        } => {
            if confirm {
                if equip::can_change(member, &data, &party) {
                    sfx.decision();
                    state.screen = MenuScreen::Equip {
                        member,
                        slot,
                        picking: Some(0),
                    };
                } else {
                    sfx.buzzer();
                }
            } else if members > 0
                && (keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::ArrowLeft))
            {
                let member = if keys.just_pressed(KeyCode::ArrowRight) {
                    (member + 1) % (members + 1)
                } else {
                    (member + members) % (members + 1)
                };
                sfx.cursor();
                let next = MenuScreen::Equip {
                    member,
                    slot,
                    picking: None,
                };
                if let Some(switch) = &mut gates.switch {
                    switch.request(next);
                } else {
                    state.screen = next;
                }
            }
        }
        MenuScreen::Equip {
            member,
            slot,
            picking: Some(cursor),
        } => {
            if confirm {
                sfx.decision();
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
        MenuScreen::EndGame { cursor } => {
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
