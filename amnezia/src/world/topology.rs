use super::{Character, EventSprite, MainCamera, MapData, MoveQueue, render::MapTile};
use crate::player::Player;
use bevy::prelude::*;

impl MapData {
    pub(crate) fn loops_x(&self) -> bool {
        self.scroll_type & 2 != 0 && self.width > 0
    }

    pub(crate) fn loops_y(&self) -> bool {
        self.scroll_type & 1 != 0 && self.height > 0
    }

    pub(crate) fn normalize_tile(&self, x: i32, y: i32) -> (i32, i32) {
        (
            if self.loops_x() {
                x.rem_euclid(self.width)
            } else {
                x
            },
            if self.loops_y() {
                y.rem_euclid(self.height)
            } else {
                y
            },
        )
    }

    pub(crate) fn contains_tile(&self, x: i32, y: i32) -> bool {
        let (x, y) = self.normalize_tile(x, y);
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub(crate) fn tile_delta(&self, from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
        (
            shortest_delta(to.0 - from.0, self.width, self.loops_x()),
            shortest_delta(to.1 - from.1, self.height, self.loops_y()),
        )
    }

    pub(crate) fn world_near(&self, point: Vec2, reference: Vec2) -> Vec2 {
        let period = Vec2::new(self.width as f32, self.height as f32) * crate::tiles::TILE;
        let mut out = point;
        if self.loops_x() {
            out.x += ((reference.x - point.x) / period.x).round() * period.x;
        }
        if self.loops_y() {
            out.y += ((reference.y - point.y) / period.y).round() * period.y;
        }
        out
    }

    pub(crate) fn draw_row(&self, tile: (i32, i32), camera: Vec2) -> i32 {
        if self.loops_y() {
            let (x, y) = self.tile_center(tile.0, tile.1);
            let foot = self.world_near(Vec2::new(x, y), camera);
            100 + ((camera.y - foot.y) / crate::tiles::TILE).round() as i32
        } else {
            tile.1
        }
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn wrap_scene(
    data: Res<MapData>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    cameras: Query<&Transform, With<MainCamera>>,
    mut tiles: Query<(&MapTile, &mut Transform), Without<MainCamera>>,
    mut characters: Query<
        (
            Option<&Player>,
            Option<&EventSprite>,
            &MoveQueue,
            &mut Transform,
        ),
        (Without<MainCamera>, Without<MapTile>),
    >,
) {
    if !data.loops_x() && !data.loops_y() {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    let camera = camera.translation.truncate();
    for (tile, mut transform) in &mut tiles {
        let point = data.world_near(tile.0, camera);
        if transform.translation.truncate() != point {
            transform.translation.x = point.x;
            transform.translation.y = point.y;
        }
    }
    for (player, event, queue, mut transform) in &mut characters {
        if let Some(player) = player {
            let point = if vehicles.as_ref().is_some_and(|v| v.riding()) {
                transform.translation.truncate() - Vec2::Y * player.y_offset()
            } else {
                queue.render_position(player, &data)
            };
            position_character(player, point, &data, camera, &mut transform);
        } else if let Some(event) = event {
            position_character(
                event,
                queue.render_position(event, &data),
                &data,
                camera,
                &mut transform,
            );
        }
    }
}

fn position_character(
    ch: &impl Character,
    point: Vec2,
    data: &MapData,
    camera: Vec2,
    transform: &mut Transform,
) {
    let point = data.world_near(point, camera);
    transform.translation = Vec3::new(
        point.x,
        point.y + ch.y_offset(),
        ch.draw_z(data.draw_row(ch.tile(), camera)),
    );
}

fn shortest_delta(delta: i32, period: i32, looping: bool) -> i32 {
    if !looping {
        return delta;
    }
    let delta = delta % period;
    if delta > period / 2 {
        delta - period
    } else if delta < -period / 2 {
        delta + period
    } else {
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periodic_rendering_keeps_one_entity_and_sorts_characters_across_the_seam() {
        let mut data = MapData::for_test(140, 140);
        data.scroll_type = 3;
        let mut app = App::new();
        app.insert_resource(data).add_systems(Update, wrap_scene);
        let camera = app
            .world_mut()
            .spawn((MainCamera, Transform::from_xyz(1120.0, -1120.0, 0.0)))
            .id();
        let tile = app
            .world_mut()
            .spawn((MapTile(Vec2::new(-1112.0, 1112.0)), Transform::default()))
            .id();
        let mut characters = Vec::new();
        for row in [139, 0] {
            characters.push(
                app.world_mut()
                    .spawn((
                        EventSprite {
                            id: row as u32 + 1,
                            tile_x: 0,
                            tile_y: row,
                            dir: 0,
                            frame: 1,
                            charset: String::new(),
                            index: 0,
                            layer: 1,
                        },
                        MoveQueue::default(),
                        Transform::default(),
                    ))
                    .id(),
            );
        }
        app.update();
        let first = app
            .world()
            .get::<Transform>(characters[0])
            .unwrap()
            .translation;
        let next = app
            .world()
            .get::<Transform>(characters[1])
            .unwrap()
            .translation;
        assert_eq!(first.y - next.y, 16.0);
        assert!(first.z < next.z);
        assert_eq!(
            app.world()
                .get::<Transform>(tile)
                .unwrap()
                .translation
                .truncate(),
            Vec2::new(1128.0, -1128.0)
        );
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation
            .x += 2240.0;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(tile).unwrap().translation.x,
            3368.0
        );
        assert_eq!(
            app.world_mut()
                .query::<&MapTile>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn each_axis_wraps_independently_and_bounded_axes_stay_outside() {
        for mode in 0..=3 {
            let mut map = MapData::for_test(20, 30);
            map.scroll_type = mode;
            let expected = (
                if mode & 2 != 0 { 19 } else { -1 },
                if mode & 1 != 0 { 0 } else { 30 },
            );
            assert_eq!(map.normalize_tile(-1, 30), expected);
            assert_eq!(map.contains_tile(-1, 30), mode == 3);
            assert_eq!(
                map.tile_delta((0, 0), (19, 29)),
                (
                    if mode & 2 != 0 { -1 } else { 19 },
                    if mode & 1 != 0 { -1 } else { 29 }
                )
            );
        }
    }

    #[test]
    fn wrapped_collision_checks_both_tile_edges_in_the_original_direction() {
        let mut map = MapData::for_test(3, 3);
        map.scroll_type = 3;
        assert!(map.can_move(0, 1, -1, 1));
        map.upper[5] = 10001;
        map.passages_up[1] = crate::tiles::PASS_ALL & !crate::tiles::PASS_RIGHT;
        assert!(!map.can_move(0, 1, -1, 1));
        assert!(map.can_move(2, 1, 1, 1));
        map.scroll_type = 0;
        assert!(!map.can_move(0, 1, -1, 1));
    }

    #[test]
    fn world_positions_use_the_nearest_period_and_leave_other_axes_unchanged() {
        let mut map = MapData::for_test(140, 140);
        map.scroll_type = 3;
        assert_eq!(
            map.world_near(Vec2::new(-1112.0, 1112.0), Vec2::new(1120.0, -1120.0)),
            Vec2::new(1128.0, -1128.0)
        );
        map.scroll_type = 1;
        assert_eq!(
            map.world_near(Vec2::new(-1112.0, 1112.0), Vec2::new(1120.0, -1120.0)),
            Vec2::new(-1112.0, -1128.0)
        );
    }
}
