use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct MessagePause<'w> {
    frame: Option<Res<'w, crate::timing::SceneWait>>,
    scene: crate::world::ScenePause<'w>,
    transition: crate::transitions::TransitionPause<'w>,
}

impl MessagePause<'_> {
    pub(crate) fn paused(&self) -> bool {
        self.frame.as_ref().is_some_and(|frame| frame.0)
            || self.scene.screen_effects_paused()
            || self.transition.paused()
    }
}
