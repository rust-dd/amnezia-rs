use super::{Character, EventSprite, MapData, MoveQueue};
use crate::player::Player;
use crate::tiles;
use crate::vehicles::{VehicleSprite, Vehicles};
use bevy::prelude::*;
use bevy::sprite::Anchor;

#[derive(Component)]
pub(super) struct BushChild(Entity);

#[derive(Component)]
pub(super) struct BushBottom;

struct Visual {
    source: Rect,
    tile: (i32, i32),
    layer: u32,
    jumping: bool,
    flying: bool,
}

fn source(character: &impl Character, tile_graphic: bool) -> Option<Rect> {
    if character.charset().is_empty() {
        let (x, y) =
            tile_graphic.then(|| tiles::upper_source(10000 + character.index() as u16))??;
        Some(Rect::new(x, y, x + tiles::TILE, y + tiles::TILE))
    } else {
        let (x, y) = tiles::charset_source(character.index(), character.dir(), character.frame());
        Some(Rect::new(x, y, x + tiles::CHAR_W, y + tiles::CHAR_H))
    }
}

fn visual(
    player: Option<&Player>,
    event: Option<&EventSprite>,
    vehicle: Option<&VehicleSprite>,
    queue: Option<&MoveQueue>,
    vehicles: Option<&Vehicles>,
) -> Option<Visual> {
    let jumping = queue.is_some_and(MoveQueue::jumping);
    if let Some(player) = player {
        return Some(Visual {
            source: source(player, false)?,
            tile: player.tile(),
            layer: 1,
            jumping,
            flying: vehicles.is_some_and(|v| v.save.riding == Some(2)),
        });
    }
    if let Some(event) = event {
        return Some(Visual {
            source: source(event, true)?,
            tile: event.tile(),
            layer: event.layer,
            jumping,
            flying: false,
        });
    }
    let id = vehicle?.0;
    let vehicles = vehicles?;
    let vehicle = vehicles.save.vehicles.get(id)?;
    Some(Visual {
        source: source(vehicle, false)?,
        tile: vehicle.tile(),
        layer: 1,
        jumping: vehicles.jumping(id),
        flying: id == 2 && vehicles.save.riding == Some(2),
    })
}

fn depth(data: &MapData, visual: &Visual) -> u32 {
    if visual.layer != 1
        || visual.jumping
        || visual.flying
        || !data.contains_tile(visual.tile.0, visual.tile.1)
    {
        return 0;
    }
    let mode = data
        .terrain_at(visual.tile.0, visual.tile.1)
        .map_or(0, |t| t.bush_depth);
    if (1..=3).contains(&mode) {
        visual.source.height() as u32 / (4 - mode)
    } else {
        0
    }
}

fn split(
    sprite: &mut Sprite,
    anchor: &mut Anchor,
    full: Rect,
    depth: u32,
) -> Option<(Sprite, Transform)> {
    let size = full.size();
    sprite.rect = Some(full);
    sprite.custom_size = Some(size);
    *anchor = Anchor::CENTER;
    if depth == 0 {
        return None;
    }
    let lower_height = (depth as f32).min(size.y);
    let upper_height = size.y - lower_height;
    let split_y = full.max.y - lower_height;
    let mut bottom = sprite.clone();
    bottom.rect = Some(Rect::new(full.min.x, split_y, full.max.x, full.max.y));
    bottom.custom_size = Some(Vec2::new(size.x, lower_height));
    let opacity = (sprite.color.alpha() * 255.0).round().clamp(0.0, 255.0) as u32;
    bottom.color = sprite.color.with_alpha(opacity.div_ceil(2) as f32 / 255.0);
    sprite.rect = Some(Rect::new(full.min.x, full.min.y, full.max.x, split_y));
    sprite.custom_size = Some(Vec2::new(size.x, upper_height));
    if upper_height > 0.0 {
        *anchor = Anchor(Vec2::new(0.0, -lower_height / (2.0 * upper_height)));
    }
    Some((
        bottom,
        Transform::from_xyz(0.0, (lower_height - size.y) / 2.0, 0.0),
    ))
}

#[allow(clippy::type_complexity)]
pub(super) fn update(
    mut commands: Commands,
    data: Res<MapData>,
    vehicles: Option<Res<Vehicles>>,
    mut characters: Query<
        (
            Entity,
            Option<&Player>,
            Option<&EventSprite>,
            Option<&VehicleSprite>,
            Option<&MoveQueue>,
            &mut Sprite,
            &mut Anchor,
            Option<&BushChild>,
        ),
        (
            Without<BushBottom>,
            Or<(With<Player>, With<EventSprite>, With<VehicleSprite>)>,
        ),
    >,
    mut children: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<BushBottom>>,
) {
    for (entity, player, event, vehicle, queue, mut sprite, mut anchor, child) in &mut characters {
        let Some(visual) = visual(player, event, vehicle, queue, vehicles.as_deref()) else {
            continue;
        };
        let depth = depth(&data, &visual);
        if depth == 0 && child.is_none() {
            continue;
        }
        let parts = split(&mut sprite, &mut anchor, visual.source, depth);
        match (
            child.and_then(|child| children.get_mut(child.0).ok()),
            parts,
        ) {
            (Some((mut sprite, mut transform, mut visibility)), Some((bottom, position))) => {
                *sprite = bottom;
                *transform = position;
                *visibility = Visibility::Inherited;
            }
            (Some((_, _, mut visibility)), None) => *visibility = Visibility::Hidden,
            (None, Some((bottom, position))) => {
                let child = commands
                    .spawn((bottom, position, BushBottom, ChildOf(entity)))
                    .id();
                commands.entity(entity).insert(BushChild(child));
            }
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests;
