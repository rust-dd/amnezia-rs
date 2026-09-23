use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Original scene counter: menus and battles count, asynchronous handoffs do not.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneFrames {
    pub frame: u32,
}

/// The frame began in an asynchronous wait, even if its handoff has since ended.
#[derive(Resource, Default)]
pub(crate) struct SceneWait(pub bool);

#[derive(SystemParam)]
pub(super) struct Waiting<'w> {
    transition: crate::transitions::TransitionPause<'w>,
    fade: Option<Res<'w, crate::teleport::Fade>>,
    teleport: Option<Res<'w, crate::teleport::PendingTeleport>>,
    new_game: Option<Res<'w, crate::session::NewGameRequest>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    title_state: Option<Res<'w, crate::title::TitleState>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    gameover_flow: Option<Res<'w, crate::gameover::GameOverFlow>>,
    files: Option<Res<'w, crate::menu::save_files::SaveFiles>>,
    equipment: Option<Res<'w, crate::menu::EquipmentSwitch>>,
    menu: Option<Res<'w, crate::menu::SceneFlow>>,
    shop: Option<Res<'w, crate::shop::SceneFlow>>,
    inn: Option<Res<'w, crate::shop::inn::State>>,
}

impl Waiting<'_> {
    pub(super) fn pending(&self) -> bool {
        self.transition.paused()
            || self.menu.as_ref().is_some_and(|menu| menu.active())
            || self.shop.as_ref().is_some_and(|shop| shop.active())
            || self.inn.as_ref().is_some_and(|inn| inn.resting())
            || self.fade.as_ref().is_some_and(|fade| fade.busy())
            || self
                .teleport
                .as_ref()
                .is_some_and(|pending| pending.0.is_some())
            || self
                .new_game
                .as_ref()
                .is_some_and(|request| request.requested)
            || (self.title.as_ref().is_some_and(|active| active.0)
                && self
                    .title_state
                    .as_ref()
                    .is_some_and(|state| state.waiting_for_scene()))
            || (self.gameover.as_ref().is_some_and(|active| active.0)
                && self
                    .gameover_flow
                    .as_ref()
                    .is_some_and(|flow| flow.waiting_for_scene()))
            || self
                .files
                .as_ref()
                .is_some_and(|files| files.waiting_for_scene())
            || self
                .equipment
                .as_ref()
                .is_some_and(|switch| switch.active())
    }
}

#[cfg(test)]
mod tests;
