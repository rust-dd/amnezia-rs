use super::Vehicles;
use crate::assets::resolve_png;
use crate::tiles::{self, CHAR_Y_OFFSET};
use crate::world::{Character, MainCamera, MapData};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct VehicleSprite(pub(crate) usize);

pub(super) fn draw(
    mut commands: Commands,
    data: Res<MapData>,
    vehicles: Res<Vehicles>,
    assets: Res<AssetServer>,
    cameras: Query<&Transform, With<MainCamera>>,
    mut sprites: Query<
        (&VehicleSprite, &mut Sprite, &mut Transform, &mut Visibility),
        Without<MainCamera>,
    >,
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
        let camera = cameras
            .single()
            .map_or(Vec2::ZERO, |t| t.translation.truncate());
        let pixel = data.world_near(
            vehicles.motion[id.0].pixel.unwrap_or(Vec2::new(wx, wy)),
            camera,
        );
        let flying = id.0 == 2 && vehicles.save.riding == Some(2);
        transform.translation = Vec3::new(
            pixel.x,
            pixel.y
                + CHAR_Y_OFFSET
                + if flying {
                    vehicles.airship_altitude()
                } else {
                    0.0
                },
            if flying {
                250.0
            } else {
                tiles::character_z(data.draw_row((x, y), camera))
            },
        );
    }
}
