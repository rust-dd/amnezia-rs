use super::{AutoMove, EventSprite, MapData, MapEvents, MapScene, MoveQueue, RouteStepper};
use crate::assets::resolve_png;
use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use crate::tiles::{self, CHAR_Y_OFFSET};
use amnezia_data::{Event, EventPage};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct PageState(Option<usize>);

#[derive(Resource)]
pub(super) struct EventTileset(pub Handle<Image>);

fn auto_for(page: Option<&EventPage>, id: u32) -> AutoMove {
    page.map_or_else(
        || AutoMove::new(0, 3, 3, id),
        |p| AutoMove::new(p.move_type, p.move_frequency, p.move_speed, id),
    )
}

pub(super) fn graphic(
    character: &EventSprite,
    tileset: &Handle<Image>,
    server: &AssetServer,
) -> (Sprite, Visibility) {
    let (image, source, size) = if character.charset.is_empty() {
        let Some((x, y)) = tiles::upper_source(10000 + character.index as u16) else {
            return (Sprite::default(), Visibility::Hidden);
        };
        (tileset.clone(), Vec2::new(x, y), Vec2::splat(tiles::TILE))
    } else {
        let (x, y) = tiles::charset_source(character.index, character.dir, character.frame);
        (
            server.load(resolve_png("CharSet", &character.charset)),
            Vec2::new(x, y),
            Vec2::new(tiles::CHAR_W, tiles::CHAR_H),
        )
    };
    (
        Sprite {
            image,
            rect: Some(Rect::from_corners(source, source + size)),
            custom_size: Some(size),
            ..default()
        },
        Visibility::Visible,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_event(
    commands: &mut Commands,
    server: &AssetServer,
    state: (&Switches, &Variables, &Party, &Inventory),
    event: &Event,
    offset: (f32, f32),
    tileset: &Handle<Image>,
) {
    let index = active_page_index(event, state.0, state.1, state.2, state.3);
    let page = index.map(|i| &event.pages[i]);
    let character = EventSprite {
        id: event.id,
        tile_x: event.x as i32,
        tile_y: event.y as i32,
        dir: page.map_or(2, |p| p.direction),
        frame: page.map_or(1, |p| p.pattern),
        charset: page.map_or_else(String::new, |p| p.graphic_name.clone()),
        index: page.map_or(0, |p| p.graphic_index),
        layer: page.map_or(0, |p| p.layer),
    };
    let (sprite, visibility) = graphic(&character, tileset, server);
    let y_offset = if character.charset.is_empty() {
        0.0
    } else {
        CHAR_Y_OFFSET
    };
    commands.spawn((
        Transform::from_xyz(
            event.x as f32 * tiles::TILE - offset.0 + tiles::TILE / 2.0,
            offset.1 - event.y as f32 * tiles::TILE - tiles::TILE / 2.0 + y_offset,
            tiles::character_z_layer(event.y as i32, character.layer),
        ),
        character,
        sprite,
        visibility,
        PageState(index),
        MoveQueue::default(),
        auto_for(page, event.id),
        RouteStepper::from_event_page(page),
        MapScene,
    ));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_pages(
    server: Res<AssetServer>,
    data: Res<MapData>,
    events: Res<MapEvents>,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    tileset: Option<Res<EventTileset>>,
    mut characters: Query<(
        &mut EventSprite,
        &mut PageState,
        &mut AutoMove,
        &mut RouteStepper,
        &MoveQueue,
        &mut Sprite,
        &mut Visibility,
        &mut Transform,
    )>,
) {
    let Some(tileset) = tileset else {
        return;
    };
    for (
        mut ch,
        mut selected,
        mut auto,
        mut route,
        queue,
        mut sprite,
        mut visibility,
        mut transform,
    ) in &mut characters
    {
        let Some(event) = events.events.iter().find(|event| event.id == ch.id) else {
            continue;
        };
        let index = active_page_index(event, &switches, &variables, &party, &inventory);
        if selected.0 == index {
            continue;
        }
        let old = selected.0.and_then(|i| event.pages.get(i));
        let page = index.map(|i| &event.pages[i]);
        selected.0 = index;
        *auto = auto_for(page, ch.id);
        route.refresh_page(page);
        ch.charset = page.map_or_else(String::new, |p| p.graphic_name.clone());
        ch.index = page.map_or(0, |p| p.graphic_index);
        ch.layer = page.map_or(0, |p| p.layer);
        if !queue.busy()
            && let Some(page) = page
            && old.is_none_or(|p| (p.direction, p.pattern) != (page.direction, page.pattern))
        {
            ch.dir = page.direction;
            ch.frame = page.pattern;
        }
        if let Some(page) = page {
            if matches!(page.animation_type, 2..=4) {
                ch.dir = page.direction;
            }
            if matches!(page.animation_type, 4 | 5) {
                ch.frame = page.pattern;
            }
        }
        (*sprite, *visibility) = graphic(&ch, &tileset.0, &server);
        if !queue.busy() {
            let (x, y) = data.tile_center(ch.tile_x, ch.tile_y);
            let offset = if ch.charset.is_empty() {
                0.0
            } else {
                CHAR_Y_OFFSET
            };
            transform.translation =
                Vec3::new(x, y + offset, tiles::character_z_layer(ch.tile_y, ch.layer));
        }
    }
}

#[cfg(test)]
mod tests;
