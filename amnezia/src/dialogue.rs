//! Dialogue: the message-box UI, plus the action-key interaction that starts an
//! event's interpreter run when the player presses it facing that event. The
//! interpreter opens boxes via [`Dialogue::open`]; the [`typewriter`] reveals
//! each page letter by letter and [`view`] draws it. Decision or Cancel advances
//! a completed page or releases an explicit key-wait.

mod action;
pub(crate) mod async_smoke;
#[cfg(test)]
mod async_tests;
mod embedded;
mod input_prompts;
#[cfg(test)]
mod interaction_tests;
mod options;
mod pause;
pub(crate) mod saved;
#[cfg(test)]
pub(crate) mod testing;
#[cfg(test)]
mod tests;
mod typewriter;
mod view;

use crate::events::MessageBox;
use bevy::prelude::*;
pub(crate) use embedded::MessagePrompt;
pub(crate) use embedded::smoke as embedded_smoke;
pub(crate) use input_prompts::{InputPrompts, PromptFrame};
pub use options::MessageOptions;
pub(crate) use pause::MessagePause;
use typewriter::Typewriter;
pub(crate) use typewriter::smoke as timing_smoke;
pub(crate) use view::DialoguePanel;
pub(crate) use view::prompts::Clock as PromptClock;
pub(crate) use view::prompts::smoke as prompt_smoke;
pub(crate) use view::smoke as font_smoke;

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
    prompt: Option<embedded::Embedded>,
}

impl Dialogue {
    pub(crate) fn close(&mut self) {
        self.active = false;
        self.boxes.clear();
        self.index = 0;
        self.reveal = None;
        self.prompt = None;
    }

    /// Show `boxes` from the first one. Called by the event interpreter, which
    /// then pauses until the player dismisses the last box (`active` clears).
    pub fn open(&mut self, boxes: Vec<MessageBox>) {
        self.boxes = boxes;
        self.index = 0;
        self.active = true;
        self.reveal = None;
        self.prompt = None;
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
#[derive(
    Resource, Default, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PromptInput;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MessageUpdate;

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

pub(crate) fn verify_saved_presentation(
    world: &mut World,
    top: bool,
    transparent: bool,
    face: bool,
) {
    verify_placement(world, top);
    let expected = |shown| {
        if shown {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    };
    let frames = world
        .query_filtered::<&Visibility, With<view::DialogueFrame>>()
        .iter(world)
        .copied()
        .collect::<Vec<_>>();
    assert!(!frames.is_empty());
    assert!(
        frames
            .iter()
            .all(|visibility| *visibility == expected(!transparent))
    );
    assert_eq!(
        *world
            .query_filtered::<&Visibility, With<view::DialogueFace>>()
            .single(world)
            .unwrap(),
        expected(face)
    );
}

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        InputPrompts::register(app);
        action::register(app);
        view::prompts::register(app);
        app.init_resource::<Dialogue>()
            .init_resource::<crate::timing::GameFrames>()
            .init_resource::<MessageOptions>()
            .init_resource::<MessagePosition>()
            .init_resource::<MessageTransparent>()
            .add_systems(Startup, view::spawn_ui)
            .add_systems(
                Update,
                (
                    interact.in_set(DialogueInput).after(PromptInput),
                    typewriter::drive_reveal,
                    embedded::update,
                    view::render_box,
                    view::target_camera,
                    view::render_reveal,
                    view::prompts::render_cursor,
                    view::update_position,
                )
                    .chain()
                    .in_set(MessageUpdate),
            );
    }
}

/// Advance an open message box on Decision or Cancel.
/// Typing ignores Decision and Cancel. Both release a key-wait or advance a
/// completed page.
fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    prompts: InputPrompts,
    pause: MessagePause,
    mut dialogue: ResMut<Dialogue>,
) {
    let decision = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    if (!decision && !keys.just_pressed(KeyCode::Escape))
        || pause.paused()
        || prompts.nested_active()
    {
        return;
    }
    if dialogue.active {
        match dialogue.reveal.as_mut() {
            Some(reveal) if reveal.waiting_for_key() => reveal.resume(),
            Some(reveal) if reveal.is_complete() => dialogue.advance(),
            _ => {}
        }
    }
}
