use super::*;
use crate::world::{AutoMove, MapEvents, MapScene};
use amnezia_data::{Event, EventPage};

pub(super) fn spawn_target(world: &mut World) {
    let page = EventPage {
        trigger: 0,
        graphic_name: "Chara1".into(),
        graphic_index: 1,
        direction: 1,
        pattern: 1,
        animation_type: 0,
        translucent: false,
        overlap_forbidden: false,
        move_type: 0,
        move_frequency: 8,
        move_speed: 4,
        move_route: default(),
        layer: 1,
        condition: default(),
        commands: vec![],
    };
    world.spawn((
        EventSprite {
            id: 9000,
            tile_x: 60,
            tile_y: 60,
            dir: page.direction,
            frame: page.pattern,
            charset: page.graphic_name.clone(),
            index: page.graphic_index,
            layer: page.layer,
        },
        RouteStepper::from_event_page(Some(&page)),
        AutoMove::new(page.move_type, page.move_frequency, page.move_speed, 9000),
        MoveQueue::default(),
        Sprite::default(),
        Transform::default(),
        MapScene,
    ));
    world.resource_mut::<MapEvents>().events.push(Event {
        id: 9000,
        x: 60,
        y: 60,
        name: "Animation target".into(),
        pages: vec![page],
    });
}
