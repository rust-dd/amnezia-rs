use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct InputPrompts<'w> {
    choice: Option<Res<'w, crate::choice::Choice>>,
    number: Option<Res<'w, crate::inputnumber::InputNumber>>,
}

impl InputPrompts<'_> {
    pub(crate) fn active(&self) -> bool {
        self.choice.as_ref().is_some_and(|v| v.active())
            || self.number.as_ref().is_some_and(|v| v.active())
    }
}
