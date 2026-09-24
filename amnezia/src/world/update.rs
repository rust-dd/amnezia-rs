use super::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EventStep;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HeroRouteStep;

pub(crate) fn register(app: &mut App) {
    app.init_resource::<TouchEvents>()
        .add_message::<RelocateEvent>()
        .add_systems(
            Update,
            (
                pages::refresh_pages
                    .after(saved::RestoreCharacters)
                    .before(crate::interpreter::ParallelStep),
                (
                    route::route_events,
                    autonomy::autonomous_movement,
                    walk::<EventSprite>,
                    touch::trigger,
                )
                    .chain()
                    .in_set(EventStep)
                    .after(crate::interpreter::ParallelStep)
                    .after(crate::menu::MenuInput)
                    .before(HeroRouteStep),
                route::route_hero
                    .in_set(HeroRouteStep)
                    .after(crate::appearance::PlayerGraphics)
                    .before(crate::player::PlayerStep),
                (apply_relocate, update_event_sprites)
                    .chain()
                    .after(EventStep)
                    .after(crate::interpreter::InterpreterStep),
            ),
        );
}
