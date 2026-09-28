use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct ScenePause<'w> {
    destination: Option<Res<'w, crate::interpreter::destination::Visit>>,
    transition: Option<Res<'w, crate::transitions::Transition>>,
    frame: Option<Res<'w, crate::timing::SceneWait>>,
    continuation: Option<Res<'w, crate::interpreter::continuation::Continuation>>,
    menu_flow: Option<Res<'w, crate::menu::SceneFlow>>,
    shop_flow: Option<Res<'w, crate::shop::SceneFlow>>,
    fade: Option<Res<'w, crate::teleport::Fade>>,
    menu: Option<Res<'w, crate::menu::MenuOpen>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
    inn: Option<Res<'w, crate::shop::inn::State>>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    save: Option<Res<'w, crate::save::EventSaveRequest>>,
    files: Option<Res<'w, crate::menu::save_files::SaveFiles>>,
    pub(crate) vehicles: Option<Res<'w, crate::vehicles::Vehicles>>,
}

impl ScenePause<'_> {
    pub(crate) fn paused(&self) -> bool {
        self.blocked()
            || self.inn.as_ref().is_some_and(|inn| inn.resting())
            || self.battle.as_ref().is_some_and(|v| v.0)
            || self.characters_paused()
    }

    fn characters_paused(&self) -> bool {
        let waiting = self.destination.is_none() && self.frame.as_ref().is_some_and(|v| v.0);
        self.continuation
            .as_ref()
            .map_or(waiting, |state| state.characters_paused(waiting))
    }

    pub(crate) fn message_paused(&self) -> bool {
        self.blocked_except_transfer()
            || (self.transfer_blocked() && !self.inn.as_ref().is_some_and(|inn| inn.closing()))
            || (!self.inn.as_ref().is_some_and(|inn| inn.closing())
                && (self.inn.as_ref().is_some_and(|inn| inn.resting()) || self.characters_paused()))
    }

    pub(crate) fn map_effects_paused(&self) -> bool {
        self.screen_effects_paused() || self.battle.as_ref().is_some_and(|v| v.0)
    }

    pub(crate) fn screen_effects_paused(&self) -> bool {
        let waiting = self.frame.as_ref().is_some_and(|v| v.0);
        self.blocked()
            || self.inn.as_ref().is_some_and(|inn| inn.resting())
            || self
                .continuation
                .as_ref()
                .map_or(waiting, |state| state.effects_paused(waiting))
    }

    pub(crate) fn tail_paused(&self) -> bool {
        let waiting = self.frame.as_ref().is_some_and(|v| v.0);
        self.blocked()
            || self.inn.as_ref().is_some_and(|inn| inn.resting())
            || self.battle.as_ref().is_some_and(|v| v.0)
            || self
                .continuation
                .as_ref()
                .map_or(waiting, |state| state.tail_paused(waiting))
    }

    fn blocked(&self) -> bool {
        self.transfer_blocked() || self.blocked_except_transfer()
    }

    fn transfer_blocked(&self) -> bool {
        self.destination.is_none() && self.fade.as_ref().is_some_and(|v| v.busy())
    }

    fn blocked_except_transfer(&self) -> bool {
        self.menu_flow.as_ref().is_some_and(|v| v.blocks_map())
            || self.shop_flow.as_ref().is_some_and(|v| v.active())
            || self.transition.as_ref().is_some_and(|v| v.busy())
            || self.menu.as_ref().is_some_and(|v| v.0)
            || self.shop.as_ref().is_some_and(|v| v.0)
            || self.title.as_ref().is_some_and(|v| v.0)
            || self.gameover.as_ref().is_some_and(|v| v.0)
            || self.save.as_ref().is_some_and(|v| v.0)
            || self.files.as_ref().is_some_and(|v| v.active())
    }

    pub(crate) fn airship(&self) -> bool {
        self.vehicles
            .as_ref()
            .is_some_and(|v| v.save.riding == Some(2))
    }
}
