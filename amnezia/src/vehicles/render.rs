use super::Vehicles;
use crate::assets::resolve_png;
use crate::player::Player;
use crate::tiles::{self, CHAR_Y_OFFSET};
use crate::world::{Character, MapData, MoveQueue};
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct VehicleSprite(usize);

pub(super) fn sync_hero(
    data: Res<MapData>,
    mut vehicles: ResMut<Vehicles>,
    mut players: Query<(&mut Player, &mut Transform, &mut MoveQueue, &mut Visibility)>,
) {
    let Ok((mut hero, mut transform, mut queue, mut visibility)) = players.single_mut() else {
        return;
    };
    let transferred = vehicles.last_map.is_some_and(|id| id != data.map_id);
    vehicles.last_map = Some(data.map_id);
    if let Some(index) = vehicles.save.riding {
        if transferred {
            vehicles.set_location(
                index,
                data.map_id,
                hero.tile_x.max(0) as u32,
                hero.tile_y.max(0) as u32,
            );
        }
        let vehicle = &vehicles.save.vehicles[index];
        if vehicle.definition.map_id != data.map_id {
            return;
        }
        let (x, y) = vehicle.tile();
        hero.tile_x = x;
        hero.tile_y = y;
        hero.dir = vehicle.dir;
        let (wx, wy) = data.tile_center(x, y);
        let pixel = vehicles.motion[index].pixel.unwrap_or(Vec2::new(wx, wy));
        transform.translation.x = pixel.x;
        transform.translation.y = pixel.y + CHAR_Y_OFFSET;
        *queue = default();
        *visibility = Visibility::Hidden;
    } else {
        *visibility = Visibility::Inherited;
        if let Some((x, y, dir)) = vehicles.disembark.take() {
            hero.tile_x = x;
            hero.tile_y = y;
            hero.dir = dir;
            let (wx, wy) = data.tile_center(x, y);
            transform.translation.x = wx;
            transform.translation.y = wy + CHAR_Y_OFFSET;
            *queue = default();
        }
    }
}

pub(super) fn draw(
    mut commands: Commands,
    data: Res<MapData>,
    vehicles: Res<Vehicles>,
    assets: Res<AssetServer>,
    mut sprites: Query<(&VehicleSprite, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    if sprites.is_empty() {
        for index in 0..3 {
            commands.spawn((
                VehicleSprite(index),
                Sprite::default(),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
        return;
    }
    for (id, mut sprite, mut transform, mut visible) in &mut sprites {
        let vehicle = &vehicles.save.vehicles[id.0];
        if vehicle.definition.map_id != data.map_id || vehicle.charset().is_empty() {
            *visible = Visibility::Hidden;
            continue;
        }
        *visible = Visibility::Inherited;
        sprite.image = assets.load(resolve_png("CharSet", vehicle.charset()));
        let (sx, sy) = tiles::charset_source(vehicle.index(), vehicle.dir, vehicle.frame);
        sprite.rect = Some(Rect::new(sx, sy, sx + tiles::CHAR_W, sy + tiles::CHAR_H));
        sprite.custom_size = Some(Vec2::new(tiles::CHAR_W, tiles::CHAR_H));
        sprite.color = sprite.color.with_alpha(vehicles.motion[id.0].alpha);
        let (x, y) = vehicle.tile();
        let (wx, wy) = data.tile_center(x, y);
        let pixel = vehicles.motion[id.0].pixel.unwrap_or(Vec2::new(wx, wy));
        let flying = id.0 == 2 && vehicles.save.riding == Some(2);
        transform.translation = Vec3::new(
            pixel.x,
            pixel.y + CHAR_Y_OFFSET + if flying { 12.0 } else { 0.0 },
            if flying { 250.0 } else { tiles::character_z(y) },
        );
    }
}
