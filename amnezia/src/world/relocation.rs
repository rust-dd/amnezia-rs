use super::{Character, EventSprite, MapData, MapEvents, MoveQueue, RelocateEvent};
use crate::tiles;
use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct Inbox(MessageCursor<RelocateEvent>);

/// The shared inbox prevents the scheduled pass from replaying command-time moves.
pub(super) fn apply_relocate(
    messages: Res<Messages<RelocateEvent>>,
    mut inbox: ResMut<Inbox>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut sprites: Query<(&mut EventSprite, &mut Transform, &mut MoveQueue)>,
) {
    for msg in inbox.0.read(&messages) {
        if let Some(event) = map_events.events.iter_mut().find(|e| e.id == msg.event_id) {
            event.x = msg.x;
            event.y = msg.y;
        }
        if let Some((mut sprite, mut transform, mut queue)) =
            sprites.iter_mut().find(|(s, _, _)| s.id == msg.event_id)
        {
            queue.relocate(sprite.tile());
            sprite.tile_x = msg.x as i32;
            sprite.tile_y = msg.y as i32;
            let (wx, wy) = data.tile_center(msg.x as i32, msg.y as i32);
            transform.translation = Vec3::new(
                wx,
                wy + sprite.y_offset(),
                tiles::character_z_layer(msg.y as i32, sprite.layer),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::DIR_DOWN;
    use amnezia_data::Event;

    #[test]
    fn relocate_moves_event_logical_and_visual() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<RelocateEvent>();
        app.init_resource::<Inbox>();
        app.add_systems(Update, apply_relocate);
        app.insert_resource(MapData::for_test(10, 10));
        app.insert_resource(MapEvents {
            events: vec![Event {
                id: 5,
                x: 0,
                y: 0,
                name: String::new(),
                pages: Vec::new(),
            }],
        });
        let entity = app
            .world_mut()
            .spawn((
                EventSprite {
                    id: 5,
                    tile_x: 0,
                    tile_y: 0,
                    dir: DIR_DOWN,
                    frame: 1,
                    charset: "C".into(),
                    index: 0,
                    layer: 1,
                },
                Transform::default(),
                MoveQueue::default(),
            ))
            .id();

        app.world_mut().write_message(RelocateEvent {
            event_id: 5,
            x: 3,
            y: 4,
        });
        app.update();

        let sprite = app.world().entity(entity).get::<EventSprite>().unwrap();
        assert_eq!(sprite.tile_x, 3);
        assert_eq!(sprite.tile_y, 4);
        let events = app.world().resource::<MapEvents>();
        let ev = events.events.iter().find(|e| e.id == 5).unwrap();
        assert_eq!(ev.x, 3);
        assert_eq!(ev.y, 4);
        let point = Vec2::from(app.world().resource::<MapData>().tile_center(3, 4));
        assert_eq!(
            app.world()
                .get::<Transform>(entity)
                .unwrap()
                .translation
                .truncate(),
            point + Vec2::Y * sprite.y_offset()
        );

        app.world_mut()
            .get_mut::<EventSprite>(entity)
            .unwrap()
            .charset
            .clear();
        app.world_mut().write_message(RelocateEvent {
            event_id: 5,
            x: 3,
            y: 4,
        });
        app.update();
        assert_eq!(
            app.world()
                .get::<Transform>(entity)
                .unwrap()
                .translation
                .truncate(),
            point
        );
    }
}
