use super::*;

#[test]
fn a_page_less_mover_retains_its_overlap_rule_after_explicitly_disabling_through() {
    let mut moving = gated(page(vec![command(33, 7), command(37, 0), command(1, 0)]));
    moving.layer = 0;
    moving.overlap_forbidden = true;
    let mut other = page(vec![]);
    other.layer = 2;
    let mut app = app(
        vec![event(1, 1, vec![moving]), event(2, 2, vec![other])],
        true,
    );
    app.update();
    let npc = entity(&mut app, 1);
    let route = app.world().get::<RouteStepper>(npc).unwrap();
    assert!(!route.page_present());
    assert!(!route.through());
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
    let encoded = ron::to_string(route).unwrap();
    let restored = ron::from_str::<RouteStepper>(&encoded).unwrap();
    assert_eq!(restored.overlap_forbidden(), Some(true));
}

#[test]
fn an_inactive_page_with_explicit_through_off_can_still_block_another_character() {
    let obstacle = gated(page(vec![command(33, 7), command(37, 0)]));
    let mut app = app(
        vec![
            event(1, 2, vec![obstacle]),
            event(2, 1, vec![page(vec![command(1, 0)])]),
        ],
        true,
    );
    app.update();
    let npc = entity(&mut app, 2);
    assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
    let obstacle = entity(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<RouteStepper>(obstacle)
            .unwrap()
            .stop_count(),
        1
    );
}
