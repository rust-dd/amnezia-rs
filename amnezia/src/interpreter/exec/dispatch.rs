//! The RM2000 opcode dispatch: given one command and the shared [`Exec`] IO, it
//! mutates the game state and reports (via [`Flow`]) whether to step on, yield the
//! frame, or end the run. Lifted out of the per-frame driver so foreground and
//! parallel interpreters run the exact same command semantics against different
//! [`Frame`]s. The multi-line arms delegate to [`super::handlers`].

use super::super::commands::{
    apply_change_gold, apply_change_items, apply_change_level, apply_change_party,
    apply_control_switches, decode_key_accept, key_code,
};
use super::super::flow::{
    after_loop_end, call_event_page, choice_labels, find_label, loop_start, skip_else_body,
    skip_option_body, skip_to_terminator,
};
use super::super::frame::{CallFrame, Frame, MAX_CALL_DEPTH};
use super::super::opcodes::*;
use super::handlers;
use super::{Exec, Flow};
use crate::appearance::SpriteChange;
use crate::audio::AudioRequest;
use crate::battle::{BattleOutcome, BattleRequest};
use crate::dialogue::MessagePosition;
use crate::events::message_boxes;
use crate::screenfx::Weather;
use crate::shop::ShopRequest;
use crate::text;
use crate::world::{MoveQueue, RouteStepper};
use amnezia_data::EventCommand;
use bevy::prelude::*;

/// Handle one event command against `frame`, returning how the run continues.
pub(super) fn dispatch(frame: &mut Frame, command: EventCommand, x: &mut Exec) -> Flow {
    match command.code {
        SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE => {
            let run_len = frame.commands[frame.ip..]
                .iter()
                .take_while(|c| is_message(c.code))
                .count();
            let mut boxes = message_boxes(
                &frame.commands[frame.ip..frame.ip + run_len],
                &mut x.dialogue.face,
            );
            frame.ip += run_len;
            if boxes.is_empty() {
                Flow::Advance
            } else {
                // Translate each line but keep its RM2000 control codes intact: the
                // dialogue typewriter expands `\N`/`\V` and acts on the reveal-timing
                // codes (`\s`, `\|`, `\^`, …) as it types the page.
                for message in &mut boxes {
                    for line in &mut message.lines {
                        *line = crate::i18n::tr(line);
                    }
                }
                x.dialogue.open(boxes);
                Flow::Yield
            }
        }
        CONTROL_SWITCHES => {
            apply_control_switches(&mut x.switches, &command.params);
            frame.ip += 1;
            Flow::Advance
        }
        CONTROL_VARIABLES => handlers::control_variables(frame, &command, x),
        CHANGE_GOLD => {
            apply_change_gold(&mut x.inventory, &command.params, &x.variables);
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_ITEMS => {
            apply_change_items(&mut x.inventory, &command.params);
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_PARTY => {
            apply_change_party(
                &mut x.party,
                &x.subsystems.actor_edits.game_data,
                &command.params,
            );
            frame.ip += 1;
            Flow::Advance
        }
        FULL_HEAL | CHANGE_SKILLS | CHANGE_EQUIPMENT | CHANGE_CONDITION => {
            super::actors::execute(frame, &command, x)
        }
        INPUT_NUMBER => {
            let digits = command.params.first().copied().unwrap_or(0).max(0) as u32;
            let var_id = command.params.get(1).copied().unwrap_or(0) as u32;
            x.subsystems.input_number.open(digits, var_id);
            frame.input_pending = true;
            Flow::Yield
        }
        CHANGE_SPRITE => {
            let actor_id = command.params.first().copied().unwrap_or(0).max(0) as u32;
            let index = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
            x.subsystems.visuals.sprite_writer.write(SpriteChange {
                actor_id,
                charset: command.string.clone(),
                index,
            });
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_EVENT_LOCATION => handlers::change_event_location(frame, &command, x),
        MESSAGE_OPTIONS => {
            x.subsystems.mapfx.message_transparent.0 =
                command.params.first().copied().unwrap_or(0) != 0;
            *x.subsystems.mapfx.message_position = match command.params.get(1).copied().unwrap_or(2)
            {
                0 => MessagePosition::Top,
                1 => MessagePosition::Middle,
                _ => MessagePosition::Bottom,
            };
            frame.ip += 1;
            Flow::Advance
        }
        TIMER => {
            x.subsystems
                .mapfx
                .game_clock
                .apply(&command.params, &x.variables);
            frame.ip += 1;
            Flow::Advance
        }
        PAN_SCREEN => {
            frame.wait = x.subsystems.mapfx.camera_pan.command(&command.params);
            frame.ip += 1;
            if frame.wait > 0.0 {
                Flow::Yield
            } else {
                Flow::Advance
            }
        }
        WEATHER => {
            *x.subsystems.mapfx.weather =
                Weather::from_code(command.params.first().copied().unwrap_or(0));
            x.subsystems.mapfx.weather_strength.0 = command.params.get(1).copied().unwrap_or(0);
            frame.ip += 1;
            Flow::Advance
        }
        PLAYER_TRANSPARENCY => {
            x.subsystems.mapfx.hero_hidden.0 = command.params.first().copied().unwrap_or(0) == 0;
            frame.ip += 1;
            Flow::Advance
        }
        SHOW_BATTLE_ANIMATION => handlers::show_battle_animation(frame, &command, x),
        TRANSACTION | INN_STAY => {
            frame.select_shop_handler(command.indent, true);
            Flow::Advance
        }
        NO_TRANSACTION | INN_CANCEL => {
            frame.select_shop_handler(command.indent, false);
            Flow::Advance
        }
        END_SHOP | END_INN => {
            frame.shop_transacted = None;
            frame.ip += 1;
            Flow::Advance
        }
        WAIT => {
            frame.wait = command.params.first().copied().unwrap_or(0) as f32 / 10.0;
            frame.ip += 1;
            Flow::Yield
        }
        TELEPORT => {
            if let [map, tx, ty, ..] = command.params.as_slice() {
                x.pending.0 = Some((*map as u32, *tx as u32, *ty as u32));
            }
            frame.ip += 1;
            // RM2000 parallel pages execute their trailing commands before the map unloads.
            if frame.parallel {
                Flow::Advance
            } else {
                Flow::Yield
            }
        }
        CONDITIONAL_BRANCH => handlers::conditional_branch(frame, &command, x),
        ELSE_BRANCH => {
            frame.ip = skip_else_body(&frame.commands, frame.ip, command.indent);
            Flow::Advance
        }
        END_BRANCH => {
            frame.ip += 1;
            Flow::Advance
        }
        LABEL => {
            frame.ip += 1;
            Flow::Advance
        }
        JUMP_TO_LABEL => {
            let id = command.params.first().copied().unwrap_or(0);
            frame.ip = find_label(&frame.commands, id).unwrap_or(frame.ip + 1);
            Flow::Advance
        }
        LOOP => {
            frame.ip += 1;
            Flow::Advance
        }
        END_LOOP => {
            frame.ip = loop_start(&frame.commands, frame.ip, command.indent);
            Flow::Advance
        }
        BREAK_LOOP => {
            frame.ip = after_loop_end(&frame.commands, frame.ip, command.indent);
            Flow::Advance
        }
        SHOW_CHOICE => {
            if frame.choices.contains_key(&command.indent) {
                frame.ip += 1;
                Flow::Advance
            } else {
                let labels: Vec<String> = choice_labels(&frame.commands, frame.ip, command.indent)
                    .iter()
                    .map(|l| {
                        text::substitute(
                            &crate::i18n::tr(l),
                            &x.subsystems.actor_edits.hero_name.0,
                            &x.variables,
                        )
                    })
                    .collect();
                if labels.is_empty() {
                    frame.ip = skip_to_terminator(
                        &frame.commands,
                        frame.ip,
                        command.indent,
                        SHOW_CHOICE_END,
                    );
                    Flow::Advance
                } else {
                    // RM2000 `ShowChoices` cancel type is `parameters[0]`.
                    let cancel_type = command.params.first().copied().unwrap_or(0);
                    x.choice.open(labels, command.indent, cancel_type);
                    Flow::Yield
                }
            }
        }
        SHOW_CHOICE_OPTION => {
            let want = frame.choices.get(&command.indent).copied().unwrap_or(-1);
            if command.params.first().copied() == Some(want) {
                frame.ip += 1;
            } else {
                frame.ip = skip_option_body(&frame.commands, frame.ip, command.indent);
            }
            Flow::Advance
        }
        SHOW_CHOICE_END => {
            frame.choices.remove(&command.indent);
            frame.ip += 1;
            Flow::Advance
        }
        MOVE_EVENT => handlers::move_event(frame, &command, x),
        PROCEED_WITH_MOVEMENT => {
            // The blocking half of a "wait until movement complete" Move Event: hold
            // here until every forced route has drained (see `run_frame`).
            frame.wait_movement = true;
            frame.ip += 1;
            Flow::Yield
        }
        ENEMY_ENCOUNTER => {
            let troop_id = super::super::commands::operate_value(
                0,
                command.params.first().copied().unwrap_or(0),
                command.params.get(1).copied().unwrap_or(0),
                &x.variables,
            )
            .max(0) as u32;
            x.subsystems.battle_writer.write(BattleRequest {
                troop_id,
                background: command.string.clone(),
                allow_escape: command.params.get(3).copied().unwrap_or(0) != 0,
                first_strike: command.params.get(5).copied().unwrap_or(0) != 0,
            });
            frame.battle_outcome = None;
            frame.battle_pending = true;
            frame.ip += 1;
            Flow::Yield
        }
        VICTORY_HANDLER => {
            frame.select_battle_handler(command.indent, BattleOutcome::Victory);
            Flow::Advance
        }
        ESCAPE_HANDLER => {
            frame.select_battle_handler(command.indent, BattleOutcome::Escape);
            Flow::Advance
        }
        DEFEAT_HANDLER => {
            frame.select_battle_handler(command.indent, BattleOutcome::Defeat);
            Flow::Advance
        }
        END_BATTLE => {
            frame.battle_outcome = None;
            frame.ip += 1;
            Flow::Advance
        }
        OPEN_SHOP => {
            // `params[0]` is the mode (0 buy+sell, 1 buy-only, 2 sell-only),
            // `params[1]` the shop type, and `params[4..]` the offered item ids.
            let (allow_buy, allow_sell) = match command.params.first().copied().unwrap_or(0) {
                1 => (true, false),
                2 => (false, true),
                _ => (true, true),
            };
            let shop_type = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
            let items = command
                .params
                .iter()
                .skip(4)
                .filter(|&&p| p >= 0)
                .map(|&p| p as u32)
                .collect();
            x.subsystems.merchant.writer.write(ShopRequest::OpenShop {
                items,
                allow_buy,
                allow_sell,
                shop_type,
            });
            frame.shop_pending = true;
            Flow::Yield
        }
        SHOW_INN => {
            let cost = command.params.get(1).copied().unwrap_or(0);
            x.subsystems
                .merchant
                .writer
                .write(ShopRequest::ShowInn { cost });
            frame.shop_pending = true;
            Flow::Yield
        }
        PLAY_SOUND => {
            x.audio
                .write(AudioRequest::play_sound(&command.string, &command.params));
            frame.ip += 1;
            Flow::Advance
        }
        PLAY_BGM => {
            x.audio
                .write(AudioRequest::play_bgm(&command.string, &command.params));
            frame.ip += 1;
            Flow::Advance
        }
        FADE_OUT_BGM => {
            x.audio.write(AudioRequest::fade_out(&command.params));
            frame.ip += 1;
            Flow::Advance
        }
        MEMORIZE_BGM => {
            x.audio.write(AudioRequest::MemorizeBgm);
            frame.ip += 1;
            Flow::Advance
        }
        PLAY_MEMORIZED_BGM => {
            x.audio.write(AudioRequest::PlayMemorizedBgm);
            frame.ip += 1;
            Flow::Advance
        }
        ERASE_SCREEN | SHOW_SCREEN | TINT_SCREEN | FLASH_SCREEN | SHAKE_SCREEN | SHOW_PICTURE
        | MOVE_PICTURE | ERASE_PICTURE | GAME_OVER => handlers::present(frame, &command, x),
        OPEN_SAVE_MENU => {
            // Request a single-slot save; `save_or_load` performs it even while this
            // event still runs — only a fade defers it — so the crystal saves.
            x.subsystems.event_save.0 = true;
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_LEVEL => {
            apply_change_level(
                &mut x.subsystems.actor_edits.progression,
                &x.subsystems.actor_edits.game_data,
                &command.params,
                &x.variables,
                &x.party,
            );
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_HERO_NAME => {
            // Set the hero's (actor 1's) display name; the remake tracks no live
            // name for other actors, so those are left as-is.
            if command.params.first().copied() == Some(1) {
                x.subsystems.actor_edits.hero_name.0 = command.string.clone();
            }
            frame.ip += 1;
            Flow::Advance
        }
        MEMORIZE_LOCATION => {
            if let [vm, vx, vy, ..] = command.params.as_slice()
                && let Some(map) = x.subsystems.flow.map_data.as_deref()
                && let Ok(player) = x.subsystems.flow.players.single()
            {
                x.variables.set(*vm as u32, map.map_id as i32);
                let (px, py, _) = x.subsystems.mapfx.vehicles.hero_position((
                    player.tile_x,
                    player.tile_y,
                    player.dir,
                ));
                x.variables.set(*vx as u32, px);
                x.variables.set(*vy as u32, py);
            }
            frame.ip += 1;
            Flow::Advance
        }
        RECALL_TO_LOCATION => {
            if let [vm, vx, vy, ..] = command.params.as_slice() {
                let (map, mx, my) = (
                    x.variables.get(*vm as u32),
                    x.variables.get(*vx as u32),
                    x.variables.get(*vy as u32),
                );
                if map > 0 && mx >= 0 && my >= 0 {
                    x.pending.0 = Some((map as u32, mx as u32, my as u32));
                }
            }
            frame.ip += 1;
            Flow::Yield
        }
        HALT_ALL_MOVEMENT => {
            x.subsystems.mapfx.vehicles.clear_motion();
            if let Ok((mut queue, mut stepper)) = x.hero_queue.single_mut() {
                *queue = MoveQueue::default();
                *stepper = RouteStepper::default();
            }
            for (_, mut queue, mut stepper) in &mut x.event_movers {
                *queue = MoveQueue::default();
                *stepper = RouteStepper::default();
            }
            frame.wait_movement = false;
            frame.ip += 1;
            Flow::Advance
        }
        KEY_INPUT_PROC => {
            let var_id = command.params.first().copied().unwrap_or(0).max(0) as u32;
            let wait = command.params.get(1).copied().unwrap_or(0) != 0;
            let accept = decode_key_accept(&command.params);
            if wait {
                // Reset the target and pause; the resume block stores the pressed
                // key's code and advances once a key comes in.
                x.variables.set(var_id, 0);
                frame.key_var = var_id;
                frame.key_accept = accept;
                frame.key_pending = true;
                Flow::Yield
            } else {
                // No-wait: sample the accepted keys once (held counts) and store the
                // code (0 when none is down), then continue.
                let keys = &x.subsystems.flow.keys;
                let code = key_code(
                    &accept,
                    keys.pressed(KeyCode::ArrowUp),
                    keys.pressed(KeyCode::ArrowDown),
                    keys.pressed(KeyCode::ArrowLeft),
                    keys.pressed(KeyCode::ArrowRight),
                    keys.pressed(KeyCode::Enter) || keys.pressed(KeyCode::Space),
                    keys.pressed(KeyCode::Escape),
                    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
                );
                x.variables.set(var_id, code);
                frame.ip += 1;
                Flow::Advance
            }
        }
        CHANGE_SAVE_ACCESS => {
            x.subsystems.access.save_access.0 = command.params.first().copied().unwrap_or(0) != 0;
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_MENU_ACCESS => {
            x.subsystems.access.menu_access.0 = command.params.first().copied().unwrap_or(0) != 0;
            frame.ip += 1;
            Flow::Advance
        }
        CALL_EVENT => {
            // Run the called page as a sub-frame; the caller resumes at ip+1 when
            // the callee ends. Depth-guarded against a self-calling cycle.
            match x
                .subsystems
                .flow
                .map_events
                .as_ref()
                .and_then(|ev| call_event_page(&ev.events, &command.params, frame.event_id))
            {
                Some((commands, target)) if frame.call_stack.len() < MAX_CALL_DEPTH => {
                    let caller = CallFrame {
                        commands: std::mem::take(&mut frame.commands),
                        ip: frame.ip + 1,
                        event_id: frame.event_id,
                    };
                    frame.call_stack.push(caller);
                    frame.commands = commands;
                    frame.ip = 0;
                    frame.event_id = target;
                }
                _ => frame.ip += 1,
            }
            Flow::Advance
        }
        RETURN_TO_TITLE => {
            // Hand the screen back to the title, mirroring the menu's End Game path.
            x.subsystems.flow.title.0 = true;
            Flow::Stop
        }
        ENTER_EXIT_VEHICLE => super::vehicles::toggle(frame, x),
        SET_VEHICLE_LOCATION => super::vehicles::locate(frame, &command, x),
        CHANGE_PBG => {
            if let Some(data) = x.subsystems.flow.map_data.as_ref() {
                x.subsystems.mapfx.panorama.change(
                    data.map_id,
                    amnezia_data::PanoramaDef::from_command(
                        command.string.clone(),
                        &command.params,
                    ),
                );
            }
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_SYSTEM_BGM => {
            x.subsystems
                .mapfx
                .system_bgm
                .change(&command.string, &command.params);
            frame.ip += 1;
            Flow::Advance
        }
        CHANGE_SCREEN_TRANSITIONS | FLASH_SPRITE | COMMENT | COMMENT_2 | END_MARKER => {
            // Faithfully decoded but deliberately inert in this remake (each
            // rationale is on its constant in `opcodes`).
            frame.ip += 1;
            Flow::Advance
        }
        _ => {
            // Any remaining unmapped command (including the empty `code: 0`) advances.
            frame.ip += 1;
            Flow::Advance
        }
    }
}

fn is_message(code: u32) -> bool {
    matches!(code, SHOW_MESSAGE | SHOW_MESSAGE_2 | CHANGE_FACE)
}
