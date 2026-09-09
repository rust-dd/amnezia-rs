use super::{Character, EventSprite, MainCamera, MapData};
use crate::player::Player;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

impl MapData {
    pub(crate) fn screen_position(&self, point: Vec2, camera: Vec2) -> (i32, i32) {
        let tile_x = (point.x + self.offset_x - 8.0) as i32;
        let tile_y = (self.offset_y - point.y - 8.0) as i32;
        let display_x = (camera.x + self.offset_x - 160.0) as i32;
        let display_y = (self.offset_y - camera.y - 120.0) as i32;
        let mut x = tile_x - display_x + 16;
        let mut y = tile_y - display_y + 16;
        if self.loops_x() {
            x = x.rem_euclid(self.width * 16);
        }
        if self.loops_y() {
            y = y.rem_euclid(self.height * 16);
        }
        (x - 8, y)
    }
}

#[derive(SystemParam)]
pub(crate) struct MapScreen<'w, 's> {
    cameras: Query<'w, 's, &'static Transform, With<MainCamera>>,
    characters: Query<
        'w,
        's,
        (
            &'static Transform,
            Option<&'static Player>,
            Option<&'static EventSprite>,
        ),
        Or<(With<Player>, With<EventSprite>)>,
    >,
}

impl MapScreen<'_, '_> {
    pub(crate) fn character(
        &self,
        reference: i32,
        this_event: u32,
        data: &MapData,
        vehicles: &crate::vehicles::Vehicles,
        tile: (i32, i32),
    ) -> (i32, i32) {
        let event_id = if reference == 10005 {
            this_event as i32
        } else {
            reference
        };
        let pixel = if (10002..=10004).contains(&reference) {
            vehicles.pixel(reference, data)
        } else {
            self.characters
                .iter()
                .find_map(|(transform, player, event)| {
                    let y_offset = if reference == 10001 && player.is_some() {
                        player?.y_offset()
                    } else if event.is_some_and(|e| e.id as i32 == event_id) {
                        event?.y_offset()
                    } else {
                        return None;
                    };
                    Some(transform.translation.truncate() - Vec2::Y * y_offset)
                })
        }
        .unwrap_or_else(|| Vec2::from(data.tile_center(tile.0, tile.1)));
        self.project(pixel, data)
    }

    pub(crate) fn project(&self, point: Vec2, data: &MapData) -> (i32, i32) {
        let camera = self
            .cameras
            .single()
            .map_or(Vec2::ZERO, |t| t.translation.truncate());
        data.screen_position(point, camera)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_coordinates_use_native_pixels_character_feet_and_camera_scroll() {
        let data = MapData::for_test(40, 30);
        let point = Vec2::from(data.tile_center(20, 15));
        assert_eq!(data.screen_position(point, Vec2::ZERO), (168, 136));
        assert_eq!(
            data.screen_position(point, Vec2::new(40.0, -16.0)),
            (128, 120)
        );
        assert_eq!(
            data.screen_position(point + Vec2::new(2.0, 7.0), Vec2::ZERO),
            (170, 129)
        );
        assert_eq!(
            data.screen_position(point, point + Vec2::X * 8.0),
            (152, 128)
        );
    }

    #[test]
    fn loop_coordinates_match_the_original_positive_modulo_before_anchor_offsets() {
        let mut data = MapData::for_test(140, 140);
        data.scroll_type = 3;
        let point = Vec2::from(data.tile_center(0, 0));
        assert_eq!(
            data.screen_position(point, Vec2::new(1120.0, -1120.0)),
            (168, 136)
        );
        assert_eq!(
            data.screen_position(point, Vec2::new(3360.0, -3360.0)),
            (168, 136)
        );
    }
}
