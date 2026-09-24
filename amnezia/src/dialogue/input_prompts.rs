use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// An overlay owns its closing frame even after Update clears its live state.
#[derive(Resource, Default)]
pub(crate) struct PromptFrame {
    message: bool,
    nested: bool,
}

impl PromptFrame {
    pub(crate) fn active(&self) -> bool {
        self.message || self.nested
    }
}

fn capture(
    dialogue: Option<Res<super::Dialogue>>,
    choice: Option<Res<crate::choice::Choice>>,
    number: Option<Res<crate::inputnumber::InputNumber>>,
    mut frame: ResMut<PromptFrame>,
) {
    frame.message = dialogue.is_some_and(|dialogue| dialogue.busy());
    frame.nested = choice.is_some_and(|choice| choice.active())
        || number.is_some_and(|number| number.active());
}

#[derive(SystemParam)]
pub(crate) struct InputPrompts<'w> {
    choice: Option<Res<'w, crate::choice::Choice>>,
    number: Option<Res<'w, crate::inputnumber::InputNumber>>,
    frame: Option<Res<'w, PromptFrame>>,
}

impl InputPrompts<'_> {
    pub(crate) fn register(app: &mut App) {
        app.init_resource::<PromptFrame>();
        crate::timing::logical::pre(app, || capture);
    }

    pub(crate) fn active(&self) -> bool {
        self.nested_active() || self.frame.as_ref().is_some_and(|frame| frame.active())
    }

    pub(crate) fn nested_active(&self) -> bool {
        self.choice.as_ref().is_some_and(|v| v.active())
            || self.number.as_ref().is_some_and(|v| v.active())
            || self.frame.as_ref().is_some_and(|frame| frame.nested)
    }
}
