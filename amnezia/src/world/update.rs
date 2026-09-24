use super::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EventStep;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HeroRouteStep;

#[derive(Resource, Default)]
struct CharacterUpdates;

pub(crate) fn register(app: &mut App) {
    app.init_resource::<TouchEvents>()
        .init_resource::<CharacterUpdates>()
        .add_message::<RelocateEvent>()
        .configure_sets(Update, EventStep.before(HeroRouteStep))
        .add_systems(
            Update,
            (
                pages::refresh_pages
                    .after(saved::RestoreCharacters)
                    .before(crate::interpreter::ParallelStep),
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

pub(crate) fn event(world: &mut World, id: u32) {
    if !world.contains_resource::<CharacterUpdates>() {
        return;
    }
    refresh(world);
    let Some(entity) = world
        .query::<(Entity, &EventSprite)>()
        .iter(world)
        .find(|(_, event)| event.id == id)
        .map(|(entity, _)| entity)
    else {
        return;
    };
    world
        .run_system_cached_with(route::route_event, Some(id))
        .unwrap();
    world
        .run_system_cached_with(autonomy::advance_event, Some(id))
        .unwrap();
    world
        .run_system_cached_with(movement::walk_selected::<EventSprite>, Some(entity))
        .unwrap();
    world
        .run_system_cached_with(touch::trigger_event, Some(id))
        .unwrap();
}

pub(crate) fn refresh(world: &mut World) {
    if world.contains_resource::<CharacterUpdates>() {
        world.run_system_cached(pages::refresh_pages).unwrap();
    }
}
