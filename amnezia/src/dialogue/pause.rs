use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub(crate) struct MessagePause<'w> {
    scene: crate::world::ScenePause<'w>,
    transition: crate::transitions::TransitionPause<'w>,
}

impl MessagePause<'_> {
    pub(crate) fn paused(&self) -> bool {
        self.scene.message_paused() || self.transition.paused()
    }
}
