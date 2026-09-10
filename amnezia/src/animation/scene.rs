use super::{AnimationSlot, playback::LiveAnimation, render::FlashQuad};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(super) struct Scenes<'w> {
    menu: Option<Res<'w, crate::menu::MenuOpen>>,
    shop: Option<Res<'w, crate::shop::ShopOpen>>,
    title: Option<Res<'w, crate::title::TitleActive>>,
    gameover: Option<Res<'w, crate::gameover::GameOverActive>>,
    battle: Option<Res<'w, crate::battle::Battle>>,
    battle_active: Option<Res<'w, crate::battle::BattleActive>>,
}

impl Scenes<'_> {
    pub(super) fn paused(&self) -> bool {
        self.menu.as_ref().is_some_and(|v| v.0)
            || self.shop.as_ref().is_some_and(|v| v.0)
            || self.title.as_ref().is_some_and(|v| v.0)
            || self.gameover.as_ref().is_some_and(|v| v.0)
    }

    fn in_battle(&self) -> bool {
        // The encounter flag also covers the still-visible map entry/return effects.
        self.battle.as_ref().map_or_else(
            || self.battle_active.as_ref().is_some_and(|v| v.0),
            |battle| battle.phase != crate::battle::Phase::Inactive,
        )
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        PostUpdate,
        visibility.before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
    );
}

fn visibility(
    scenes: Scenes,
    animations: Query<&LiveAnimation>,
    mut cells: Query<&mut Visibility, Without<FlashQuad>>,
    mut flashes: Query<&mut Visibility, With<FlashQuad>>,
) {
    for animation in &animations {
        let visible =
            !scenes.paused() && (animation.slot != AnimationSlot::Map || !scenes.in_battle());
        let next = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for &cell in &animation.cells {
            if let Ok(mut visibility) = cells.get_mut(cell) {
                visibility.set_if_neq(next);
            }
        }
    }
    let next = if scenes.paused() {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut flash in &mut flashes {
        flash.set_if_neq(next);
    }
}
