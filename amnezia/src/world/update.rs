use super::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct EventStep;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HeroRouteStep;

#[derive(Resource, Default)]
struct CharacterUpdates {
    processed: std::collections::HashSet<Entity>,
}

pub(crate) fn begin(world: &mut World) {
    if let Some(mut updates) = world.get_resource_mut::<CharacterUpdates>() {
        updates.processed.clear();
    }
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<TouchEvents>()
        .init_resource::<CharacterUpdates>()
        .init_resource::<relocation::Inbox>()
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
        crate::interpreter::foreground::queue_autorun(world, id);
        return;
    }
    refresh(world);
    let Some(entity) = world
        .query::<(Entity, &EventSprite)>()
        .iter(world)
        .find(|(_, event)| event.id == id)
        .map(|(entity, _)| entity)
    else {
        crate::interpreter::foreground::queue_autorun(world, id);
        return;
    };
    if !world
        .resource_mut::<CharacterUpdates>()
        .processed
        .insert(entity)
    {
        return;
    }
    let stopped = world
        .get::<MoveQueue>(entity)
        .is_none_or(|queue| !queue.busy());
    route::route_event(world, Some(id), Some(true));
    world
        .run_system_cached_with(touch::trigger_event, (Some(id), false))
        .unwrap();
    if stopped {
        refresh(world);
        crate::interpreter::foreground::queue_autorun(world, id);
        world
            .run_system_cached_with(touch::trigger_event, (Some(id), true))
            .unwrap();
    }
    route::route_event(world, Some(id), Some(false));
    world
        .run_system_cached_with(touch::trigger_event, (Some(id), false))
        .unwrap();
    autonomy::advance_event(world, Some(id));
    world
        .run_system_cached_with(touch::trigger_event, (Some(id), false))
        .unwrap();
    world
        .run_system_cached_with(movement::walk_selected::<EventSprite>, Some(entity))
        .unwrap();
}

pub(crate) fn refresh(world: &mut World) {
    if world.contains_resource::<CharacterUpdates>() {
        world.run_system_cached(pages::refresh_pages).unwrap();
    }
    crate::interpreter::refresh_map_pages(world);
}

pub(crate) fn refresh_route_switch(world: &mut World) {
    if world.contains_resource::<CharacterUpdates>() {
        pages::refresh_switch(world);
    }
    crate::interpreter::refresh_map_pages(world);
}

pub(crate) fn flush(world: &mut World) {
    if world.contains_resource::<CharacterUpdates>() {
        world.run_system_cached(apply_relocate).unwrap();
        refresh(world);
    }
}
