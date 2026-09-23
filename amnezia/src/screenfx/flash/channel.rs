use crate::screenfx::{FlashOverlay, Fx, ScreenEffect};
use bevy::ecs::message::MessageCursor;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Advance;

#[derive(Resource, Default)]
pub(crate) struct Inbox {
    effects: MessageCursor<ScreenEffect>,
    battle: Option<u64>,
}

impl Inbox {
    fn scene_entry(
        &mut self,
        battle: Option<&crate::battle::Battle>,
        fx: &mut Fx,
        commands: &mut Commands,
    ) {
        let generation = battle
            .filter(|battle| battle.phase != crate::battle::Phase::Inactive)
            .map(|battle| battle.generation);
        if generation.is_some() && generation != self.battle {
            fx.flash = None;
            commands.queue(crate::animation::clear_screen_flash);
        }
        self.battle = generation;
    }
}

#[derive(SystemParam)]
pub(crate) struct PendingEffects<'w> {
    messages: Res<'w, Messages<ScreenEffect>>,
    inbox: ResMut<'w, Inbox>,
}

impl PendingEffects<'_> {
    pub(in crate::screenfx) fn apply(&mut self, fx: &mut Fx, commands: &mut Commands) {
        for effect in self.inbox.effects.read(&self.messages) {
            super::super::apply_effect(fx, effect);
            if matches!(effect, ScreenEffect::Flash { .. }) {
                commands.queue(crate::animation::clear_screen_flash);
            }
        }
    }

    pub(in crate::screenfx) fn scene_entry(
        &mut self,
        battle: Option<&crate::battle::Battle>,
        fx: &mut Fx,
        commands: &mut Commands,
    ) {
        self.inbox.scene_entry(battle, fx, commands);
    }
}

pub(in crate::screenfx) fn scene_entry(
    mut inbox: ResMut<Inbox>,
    battle: Option<Res<crate::battle::Battle>>,
    mut fx: ResMut<Fx>,
    mut commands: Commands,
) {
    inbox.scene_entry(battle.as_deref(), &mut fx, &mut commands);
}

pub(crate) fn receive(mut pending: PendingEffects, mut fx: ResMut<Fx>, mut commands: Commands) {
    pending.apply(&mut fx, &mut commands);
}

pub(crate) fn cancel_event_flash(world: &mut World) {
    if let Some(mut fx) = world.get_resource_mut::<Fx>() {
        fx.flash = None;
    }
}

pub(crate) fn event_color(fx: Option<&Fx>) -> Color {
    fx.and_then(|fx| fx.flash.as_ref())
        .map_or(Color::NONE, super::Flashing::color)
}

pub(crate) fn overlay(world: &mut World) -> Entity {
    if let Some(entity) = world
        .query_filtered::<Entity, With<FlashOverlay>>()
        .iter(world)
        .next()
    {
        return entity;
    }
    world
        .spawn((
            FlashOverlay,
            Sprite::from_color(Color::NONE, Vec2::new(320.0, 240.0)),
            Transform::from_xyz(0.0, 0.0, 300.0),
            crate::animation::overlay_layer(),
        ))
        .id()
}

pub(in crate::screenfx) fn spawn_overlay(world: &mut World) {
    overlay(world);
}

#[allow(clippy::type_complexity)]
pub(crate) fn paint(
    fx: Option<Res<Fx>>,
    scene: crate::animation::scene::Scenes,
    mut overlays: Query<
        (&mut Sprite, &mut Visibility),
        (With<FlashOverlay>, Without<crate::animation::ScreenFlash>),
    >,
) {
    for (mut sprite, mut visibility) in &mut overlays {
        sprite.color = event_color(fx.as_deref());
        *visibility = if scene.paused() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
