use super::{Dialogue, MessageOptions, MessagePosition, MessageTransparent};
use crate::events::MessageFace;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Persistent presentation settings, independent of a message's current text or reveal.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MessageState {
    pub position: MessagePosition,
    pub transparent: bool,
    pub options: MessageOptions,
    pub face: MessageFace,
}

impl MessageState {
    pub(crate) fn restore(self, world: &mut World) {
        world.resource_mut::<Dialogue>().face = self.face;
        world.insert_resource(self.position);
        world.insert_resource(MessageTransparent(self.transparent));
        world.insert_resource(self.options);
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Capture<'w> {
    position: Option<Res<'w, MessagePosition>>,
    transparent: Option<Res<'w, MessageTransparent>>,
    options: Option<Res<'w, MessageOptions>>,
}

impl Capture<'_> {
    pub(crate) fn snapshot(&self, dialogue: &Dialogue) -> MessageState {
        MessageState {
            position: self.position.as_deref().copied().unwrap_or_default(),
            transparent: self.transparent.as_ref().is_some_and(|value| value.0),
            options: self.options.as_deref().copied().unwrap_or_default(),
            face: dialogue.face.clone(),
        }
    }
}
