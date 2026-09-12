use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct ScenePause<'w> {
    transition: Option<Res<'w, crate::transitions::Transition>>,
    fade: Option<Res<'w, crate::teleport::Fade>>,
    menu: Option<Res<'w, crate::menu::MenuOpen>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    pub(crate) vehicles: Option<Res<'w, crate::vehicles::Vehicles>>,
}

impl ScenePause<'_> {
    pub(crate) fn paused(&self) -> bool {
        self.screen_effects_paused() || self.battle.as_ref().is_some_and(|v| v.0)
    }

    pub(crate) fn screen_effects_paused(&self) -> bool {
        self.fade.as_ref().is_some_and(|v| v.busy())
            || self.transition.as_ref().is_some_and(|v| v.busy())
            || self.menu.as_ref().is_some_and(|v| v.0)
            || self.shop.as_ref().is_some_and(|v| v.0)
            || self.title.as_ref().is_some_and(|v| v.0)
            || self.gameover.as_ref().is_some_and(|v| v.0)
    }

    pub(crate) fn riding(&self) -> bool {
        self.vehicles.as_ref().is_some_and(|v| v.riding())
    }
}
