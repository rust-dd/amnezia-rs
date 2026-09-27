use super::tests::{app_with_event, page};
use super::*;

#[test]
fn a_diagonal_sprite_facing_draws_no_neighboring_charset_rows_and_can_recover() {
    for (charset, index) in [("Chara1", 0), ("Chara1", 4), ("", 1)] {
        let mut app = app_with_event(Event {
            id: 1,
            x: 3,
            y: 4,
            name: String::new(),
            pages: vec![page(charset, index)],
        });
        let world = app.world_mut();
        let entity = world
            .query_filtered::<Entity, With<EventSprite>>()
            .single(world)
            .unwrap();
        for facing in [4, 5, 6, 7, 2] {
            app.world_mut().get_mut::<EventSprite>(entity).unwrap().dir = facing;
            app.update();
            assert_eq!(
                *app.world().get::<Visibility>(entity).unwrap(),
                if !charset.is_empty() && facing >= 4 {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                },
                "{charset}, index {index}, facing {facing}"
            );
        }
    }
}
