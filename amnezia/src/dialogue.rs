//! Dialogue: the message-box UI, plus the action-key interaction that starts an
//! event's interpreter run when the player presses it facing that event. The
//! interpreter opens boxes via [`Dialogue::open`]; the [`typewriter`] reveals
//! each page letter by letter, [`view`] draws it, and the confirm key here
//! fast-forwards the reveal or advances/closes the box.

mod input_prompts;
mod options;
#[cfg(test)]
mod tests;
mod typewriter;
mod view;

use crate::events::MessageBox;
use crate::interpreter::RunningEvent;
use crate::player::{Player, facing_tile};
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::world::{MapData, MapEvents};
use bevy::prelude::*;
pub(crate) use input_prompts::InputPrompts;
pub use options::MessageOptions;
use typewriter::Typewriter;

/// The active dialogue: the sequence of boxes, which one is showing, and the
/// current page's letter-by-letter reveal (rebuilt when the box changes).
#[derive(Resource, Default)]
pub struct Dialogue {
    pub(crate) face: crate::events::MessageFace,
    pub boxes: Vec<MessageBox>,
    pub index: usize,
    pub active: bool,
    generation: u64,
    reveal: Option<Typewriter>,
}

impl Dialogue {
    pub(crate) fn close(&mut self) {
        self.active = false;
        self.boxes.clear();
        self.index = 0;
        self.reveal = None;
    }

    /// Show `boxes` from the first one. Called by the event interpreter, which
    /// then pauses until the player dismisses the last box (`active` clears).
    pub fn open(&mut self, boxes: Vec<MessageBox>) {
        self.boxes = boxes;
        self.index = 0;
        self.active = true;
        self.reveal = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Advance past the current box to the next one, closing the dialogue when
    /// the last box is dismissed. Clears the reveal so the next box types afresh.
    fn advance(&mut self) {
        self.index += 1;
        self.reveal = None;
        if self.index >= self.boxes.len() {
            self.close();
        }
    }
}

/// Where the message box sits vertically (`MessageOptions` 10120). The default
/// [`MessagePosition::Bottom`] is RM2000's usual placement; [`view`] moves the
/// box when the interpreter changes this. `Top`/`Middle` are only produced by
/// the interpreter's MessageOptions arm, which lands separately.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessagePosition {
    Top,
    Middle,
    #[default]
    Bottom,
}

/// The `MessageOptions` (10120) transparent-box flag: when set, the message text
/// draws without the windowskin fill/frame behind it.
#[derive(Resource, Default)]
pub struct MessageTransparent(pub bool);

pub struct DialoguePlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DialogueInput;

pub(crate) fn verify_placement(world: &mut World, top: bool) {
    let node = world
        .query_filtered::<&Node, With<view::DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(node.top, if top { Val::Px(0.0) } else { Val::Auto });
    assert_eq!(node.bottom, if top { Val::Auto } else { Val::Px(0.0) });
}

pub(crate) fn verify_battle_layer(world: &mut World) {
    assert!(world.resource::<Dialogue>().active);
    let camera = world
        .query_filtered::<Entity, With<crate::battle::HudCamera>>()
        .single(world)
        .unwrap();
    let (target, visibility) = world
        .query_filtered::<(&UiTargetCamera, &Visibility), With<view::DialoguePanel>>()
        .single(world)
        .unwrap();
    assert_eq!(target.0, camera);
    assert_eq!(*visibility, Visibility::Visible);
}

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dialogue>()
            .init_resource::<MessageOptions>()
            .init_resource::<MessagePosition>()
            .init_resource::<MessageTransparent>()
            .add_systems(Startup, view::spawn_ui)
            .add_systems(
                Update,
                (
                    interact.in_set(DialogueInput),
                    typewriter::drive_reveal,
                    view::render_box,
                    view::target_camera,
                    view::render_reveal,
                    view::update_position,
                )
                    .chain(),
            );
    }
}

/// Advance an open message box on the action key, or — when idle — start the
/// interpreter for an action-key (trigger 0) event on the tile the player faces.
///
/// While a page is still revealing, the action key fast-forwards it to the full
/// page; on a `\!` mid-text pause it resumes the reveal; only once the page is
/// fully shown does the key advance to the next box (or close the dialogue).
#[allow(clippy::too_many_arguments)]
fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    prompts: InputPrompts,
    scene: crate::world::ScenePause,
    data: Res<MapData>,
    map_events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    mut dialogue: ResMut<Dialogue>,
    mut running: ResMut<RunningEvent>,
    players: Query<(&Player, Option<&crate::world::MoveQueue>)>,
) {
    if !keys.just_pressed(KeyCode::Space) && !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    if dialogue.active {
        match dialogue.reveal.as_mut() {
            // A `\!` mid-text pause: resume typing the rest of the page.
            Some(reveal) if reveal.waiting_for_key() => reveal.resume(),
            // Still typing: fast-forward to the fully-revealed page.
            Some(reveal) if !reveal.is_complete() => reveal.fast_forward(),
            // Page fully shown: advance to the next box (or close the dialogue).
            Some(_) => dialogue.advance(),
            // The reveal has not been built yet this frame; ignore the press.
            None => {}
        }
        return;
    }
    if scene.paused() || prompts.active() {
        return;
    }
    if running.active() {
        return;
    }
    if scene.vehicles.as_ref().is_some_and(|v| v.blocks_action()) {
        return;
    }
    let Ok((player, queue)) = players.single() else {
        return;
    };
    if queue.is_some_and(|q| q.busy()) {
        return;
    }
    let (fx, fy) = facing_tile(player);
    let (dx, dy) = (fx - player.tile_x, fy - player.tile_y);
    // RM2000 `CheckActionEvent`: a trigger-0 event on the tile the hero faces
    // fires only when its active page shares the hero's layer (layer 1). If that
    // tile is a counter, the scan reaches across it to the next tile — up to
    // RPG_RT's maximum of three counter tiles — so the hero can talk to an event
    // standing behind a shop counter.
    let (mut tx, mut ty) = data.normalize_tile(fx, fy);
    for hop in 0..=3 {
        for event in &map_events.events {
            if event.x as i32 != tx || event.y as i32 != ty {
                continue;
            }
            if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
                && page.trigger == 0
                && page.layer == 1
            {
                running.start(event.id, page.commands.clone());
                return;
            }
        }
        if hop == 3 || !data.is_counter(tx, ty) {
            break;
        }
        (tx, ty) = data.normalize_tile(tx + dx, ty + dy);
    }
    // RM2000 `CheckEventTriggerHere`: a trigger-0 event on the hero's own tile
    // fires when its active page is below or above the hero (layer != 1). The save
    // crystal's action page sits on the above-hero layer, so it activates while the
    // hero stands on the crystal rather than by facing it.
    for event in &map_events.events {
        if event.x as i32 != player.tile_x || event.y as i32 != player.tile_y {
            continue;
        }
        if let Some(page) = active_page(event, &switches, &variables, &party, &inventory)
            && page.trigger == 0
            && page.layer != 1
        {
            running.start(event.id, page.commands.clone());
            return;
        }
    }
}
