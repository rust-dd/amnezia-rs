//! Merchant requests, keyboard input and phase dispatch to [`super::steps`],
//! including RM2000 system sounds and purchased/sold confirmation timing.

use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use crate::state::Inventory;
use amnezia_data::SoundDef;
use bevy::prelude::*;

use super::{Phase, Screen, ShopOpen, ShopOutcome, ShopRequest, ShopState};
use super::{clock, quantity, steps};

/// The RM2000 system sound a step asks the dispatcher to play.
#[derive(Clone, Copy)]
pub(super) enum Se {
    None,
    Decision,
    Cancel,
    Buzzer,
}

/// A step's result: whether the screen stays open and the sound to play.
struct StepResult {
    keep: bool,
    se: Se,
}

impl StepResult {
    fn stay(se: Se) -> Self {
        Self { keep: true, se }
    }
    fn leave(se: Se) -> Self {
        Self { keep: false, se }
    }
}

/// A shop phase step's outcome: hold the phase, switch to another, or leave.
pub(super) enum Transition {
    Stay(Se),
    To(Phase, Se),
    Leave(Se),
}

/// The shop phase a given RM2000 mode opens on: the Buy/Sell/Leave menu for a
/// full shop, or straight into the sole list for a buy-only or sell-only shop.
fn initial_phase(allow_buy: bool, allow_sell: bool) -> Phase {
    match (allow_buy, allow_sell) {
        (true, true) => Phase::Command {
            cursor: 0,
            regreet: false,
        },
        (false, true) => Phase::Sell { cursor: 0 },
        _ => Phase::Buy { cursor: 0 },
    }
}

/// Open (or replace) the merchant screen when a [`ShopRequest`] arrives.
pub fn open_requests(
    mut requests: MessageReader<ShopRequest>,
    mut screen: ResMut<Screen>,
    mut open: ResMut<ShopOpen>,
    mut outcome: ResMut<ShopOutcome>,
    mut fades: Option<ResMut<super::SceneFlow>>,
) {
    for request in requests.read() {
        let state = match request {
            ShopRequest::OpenShop {
                items,
                allow_buy,
                allow_sell,
                shop_type,
            } => Box::new(ShopState {
                items: items.clone(),
                allow_buy: *allow_buy,
                allow_sell: *allow_sell,
                shop_type: *shop_type,
                phase: initial_phase(*allow_buy, *allow_sell),
                scene: default(),
            }),
            ShopRequest::ShowInn { .. } => continue,
        };
        if let Some(fades) = fades.as_deref_mut() {
            if fades.active() {
                continue;
            }
            fades.enter(state);
        } else {
            *screen = Screen::Shop(state);
            open.0 = true;
        }
        outcome.transacted = false;
    }
}

/// Drive the open screen from the keyboard, then play the step's system sound.
#[allow(clippy::too_many_arguments)]
pub fn shop_input(
    keys: Res<ButtonInput<KeyCode>>,
    directions: Res<crate::menu::DirectionInput>,
    frames: Res<crate::timing::SceneFrames>,
    pause: clock::Pause,
    mut clock: Local<clock::Clock>,
    data: Res<GameData>,
    mut inventory: ResMut<Inventory>,
    mut screen: ResMut<Screen>,
    mut open: ResMut<ShopOpen>,
    mut outcome: ResMut<ShopOutcome>,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
    mut fades: Option<ResMut<super::SceneFlow>>,
) {
    let ticks = clock.advance(frames.frame);
    if pause.paused()
        || fades.as_ref().is_some_and(|fades| fades.active())
        || matches!(*screen, Screen::Closed)
    {
        return;
    }
    if let Screen::Shop(state) = &mut *screen {
        let moves = super::scene::update(state, &directions, &keys, ticks, &data, &inventory);
        if let Some(sounds) = sounds.as_deref() {
            for _ in 0..moves {
                play_system_se(&mut audio, &sounds.cursor);
            }
            if matches!(state.phase, Phase::Command { .. })
                && confirm(&keys)
                && keys.just_pressed(KeyCode::Escape)
            {
                play_system_se(&mut audio, &sounds.decision);
            }
        }
        if clock::confirmation(state, ticks, &data, &inventory) {
            return;
        }
        if let Phase::Number(number) = &mut state.phase {
            for step in directions.steps() {
                if quantity::navigate(number, step)
                    && let Some(sounds) = sounds.as_deref()
                {
                    play_system_se(&mut audio, &sounds.cursor);
                }
            }
        }
    }
    if !any_menu_key(&keys) {
        return;
    }
    let mut current = std::mem::take(&mut *screen);
    let result = match &mut current {
        Screen::Closed => StepResult::leave(Se::None),
        Screen::Shop(state) => shop_step(&keys, &data, &mut inventory, &mut outcome, state),
    };
    if !result.keep
        && let Some(fades) = fades.as_deref_mut()
    {
        fades.leave();
        *screen = current;
    } else {
        *screen = if result.keep { current } else { Screen::Closed };
        open.0 = result.keep;
    }
    if let Some(sounds) = sounds.as_deref()
        && let Some(sound) = pick_se(sounds, result.se)
    {
        play_system_se(&mut audio, sound);
    }
}

/// Map a step's requested sound onto the loaded system effect.
fn pick_se(sounds: &SystemSounds, se: Se) -> Option<&SoundDef> {
    match se {
        Se::None => None,
        Se::Decision => Some(&sounds.decision),
        Se::Cancel => Some(&sounds.cancel),
        Se::Buzzer => Some(&sounds.buzzer),
    }
}

/// Step the shop's phase machine one keypress and apply the transition.
fn shop_step(
    keys: &ButtonInput<KeyCode>,
    data: &GameData,
    inventory: &mut Inventory,
    outcome: &mut ShopOutcome,
    state: &mut ShopState,
) -> StepResult {
    let transition = match &mut state.phase {
        Phase::Command { cursor, .. } => {
            steps::command_step(keys, cursor, state.scene.buy.index, state.scene.sell.index)
        }
        Phase::Buy { cursor } => steps::buy_step(
            keys,
            data,
            inventory,
            &state.items,
            state.allow_sell,
            cursor,
        ),
        Phase::Sell { cursor } => steps::sell_step(keys, data, inventory, state.allow_buy, cursor),
        Phase::Number(num) => steps::number_step(keys, data, inventory, outcome, num),
        Phase::Bought { .. } | Phase::Sold { .. } => Transition::Stay(Se::None),
    };
    match transition {
        Transition::Stay(se) => StepResult::stay(se),
        Transition::To(phase, se) => {
            state.set_phase(phase, data, inventory);
            StepResult::stay(se)
        }
        Transition::Leave(se) => StepResult::leave(se),
    }
}

/// Debug-only triggers so the shop/inn UI can be exercised without the
/// interpreter: F7 opens a full buy+sell shop, F8 an inn.
pub fn debug_triggers(
    keys: Res<ButtonInput<KeyCode>>,
    scene: crate::world::ScenePause,
    running: Option<Res<crate::interpreter::RunningEvent>>,
    dialogue: Option<Res<crate::dialogue::Dialogue>>,
    mut requests: MessageWriter<ShopRequest>,
) {
    if !crate::debug::tools_enabled()
        || scene.paused()
        || running.as_ref().is_some_and(|r| r.active())
        || dialogue.as_ref().is_some_and(|d| d.active)
    {
        return;
    }
    if keys.just_pressed(KeyCode::F7) {
        requests.write(ShopRequest::OpenShop {
            items: vec![1, 2, 3, 4],
            allow_buy: true,
            allow_sell: true,
            shop_type: 0,
        });
    }
    if keys.just_pressed(KeyCode::F8) {
        requests.write(ShopRequest::ShowInn {
            cost: 10,
            inn_type: 1,
        });
    }
}

/// Whether any key the merchant screens react to was just pressed, so the input
/// system only wakes (and marks the screen changed) on real input.
fn any_menu_key(keys: &ButtonInput<KeyCode>) -> bool {
    const RELEVANT: [KeyCode; 7] = [
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::Space,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ];
    RELEVANT.iter().any(|k| keys.just_pressed(*k))
}

/// The action key: Space or Enter, as used by the dialogue and choice boxes.
pub(super) fn confirm(keys: &ButtonInput<KeyCode>) -> bool {
    keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_mode_selects_the_opening_phase() {
        assert!(matches!(initial_phase(true, true), Phase::Command { .. }));
        assert!(matches!(initial_phase(true, false), Phase::Buy { .. }));
        assert!(matches!(initial_phase(false, true), Phase::Sell { .. }));
    }
}
