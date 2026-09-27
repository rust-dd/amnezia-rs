use super::*;
use bevy::ecs::system::SystemParam;
pub(crate) use movement::saved::MotionState;
use pages::{EventTileset, PageState};
use serde::{Deserialize, Serialize};

pub(crate) mod hero;
pub(crate) mod smoke;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EventState {
    character: EventSprite,
    motion: MotionState,
    route: RouteStepper,
    autonomy: AutoMove,
    page: PageState,
}

#[derive(SystemParam)]
pub(crate) struct Capture<'w, 's> {
    hero: Query<'w, 's, (&'static MoveQueue, &'static RouteStepper), With<crate::player::Player>>,
    events: Query<
        'w,
        's,
        (
            &'static EventSprite,
            &'static MoveQueue,
            &'static RouteStepper,
            &'static AutoMove,
            &'static PageState,
        ),
    >,
}

impl Capture<'_, '_> {
    pub(crate) fn hero(&self, player: &crate::player::Player) -> Option<hero::HeroState> {
        self.hero
            .single()
            .ok()
            .map(|(queue, route)| hero::HeroState::capture(player, queue, route))
    }

    pub(crate) fn snapshot(&self) -> Vec<EventState> {
        let mut events = self
            .events
            .iter()
            .map(EventState::capture)
            .collect::<Vec<_>>();
        events.sort_by_key(|event| event.character.id);
        events
    }
}

impl EventState {
    fn capture(
        (character, motion, route, autonomy, page): (
            &EventSprite,
            &MoveQueue,
            &RouteStepper,
            &AutoMove,
            &PageState,
        ),
    ) -> Self {
        Self {
            character: character.clone(),
            motion: motion.snapshot(),
            route: route.clone(),
            autonomy: autonomy.clone(),
            page: page.clone(),
        }
    }
}

pub(crate) fn snapshot(world: &mut World) -> Vec<EventState> {
    let mut events = world
        .query::<(
            &EventSprite,
            &MoveQueue,
            &RouteStepper,
            &AutoMove,
            &PageState,
        )>()
        .iter(world)
        .map(EventState::capture)
        .collect::<Vec<_>>();
    events.sort_by_key(|event| event.character.id);
    events
}

pub(crate) fn valid(events: &[EventState], map: &Map) -> bool {
    let mut ids = std::collections::HashSet::new();
    events.iter().all(|saved| {
        let ch = &saved.character;
        ids.insert(ch.id)
            && ch.dir < 8
            && ch.frame < 4
            && ch.layer <= 2
            && ch.index < if ch.charset.is_empty() { 144 } else { 8 }
            && saved.motion.valid_for_event()
            && saved.route.valid_for_event()
            && saved.autonomy.valid()
            && map
                .events
                .iter()
                .find(|event| event.id == ch.id)
                .is_some_and(|event| saved.page.valid(event))
    })
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    events: Vec<EventState>,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RestoreCharacters;

pub(crate) fn prepare(world: &mut World, map_id: u32, events: Vec<EventState>) {
    world.remove_resource::<Pending>();
    if !events.is_empty() {
        world.insert_resource(Pending { map_id, events });
    }
}

pub(crate) fn register(app: &mut App) {
    hero::register(app);
    app.add_message::<MapRebuilt>().add_systems(
        Update,
        restore
            .in_set(RestoreCharacters)
            .after(crate::teleport::MapTransfer)
            .before(crate::interpreter::InterpreterStep),
    );
}

#[allow(clippy::too_many_arguments)]
fn restore(
    mut commands: Commands,
    mut changes: MessageReader<MapRebuilt>,
    pending: Option<Res<Pending>>,
    data: Option<Res<MapData>>,
    mut events: Option<ResMut<MapEvents>>,
    server: Option<Res<AssetServer>>,
    tileset: Option<Res<EventTileset>>,
    characters: Query<(Entity, &EventSprite)>,
) {
    if changes.read().count() == 0 {
        return;
    }
    let (Some(pending), Some(data), Some(events), Some(server), Some(tileset)) =
        (pending, data, events.as_deref_mut(), server, tileset)
    else {
        return;
    };
    if data.map_id != pending.map_id {
        return;
    }
    for saved in &pending.events {
        let ch = &saved.character;
        let Some((entity, _)) = characters.iter().find(|(_, event)| event.id == ch.id) else {
            continue;
        };
        if let Some(event) = events.events.iter_mut().find(|event| event.id == ch.id) {
            event.x = ch.tile_x as u32;
            event.y = ch.tile_y as u32;
        }
        let queue = saved.motion.clone().into_queue();
        let mut route = saved.route.clone();
        let mut autonomy = saved.autonomy.clone();
        autonomy.restore_clock(&mut route);
        let position = queue.render_position(ch, &data);
        let (mut sprite, visible) = pages::graphic(ch, &tileset.0, &server);
        sprite.color = sprite.color.with_alpha(saved.route.alpha());
        commands.entity(entity).insert((
            ch.clone(),
            queue,
            route,
            autonomy,
            saved.page.clone(),
            sprite,
            visible,
            Transform::from_xyz(position.x, position.y + ch.y_offset(), ch.draw_z(ch.tile_y)),
        ));
    }
    commands.remove_resource::<Pending>();
}
