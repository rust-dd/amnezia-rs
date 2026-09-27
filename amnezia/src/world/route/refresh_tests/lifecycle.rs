use super::*;

#[test]
fn losing_every_page_keeps_the_current_route_burst_and_finishes_that_character_update() {
    let only = gated(page(vec![command(33, 7), command(1, 0)]));
    let mut app = app(vec![event(1, 1, vec![only])], true);
    let npc = entity(&mut app, 1);
    {
        let mut data = app.world_mut().resource_mut::<MapData>();
        let index = data.width as usize + 2;
        data.upper[index] = 10001;
        data.passages_up[1] = 0;
    }
    app.update();
    let character = app.world().get::<EventSprite>(npc).unwrap();
    assert_eq!(character.tile_x, 2);
    assert_eq!((character.charset.as_str(), character.layer), ("Chara1", 1));
    assert_eq!(
        *app.world().get::<Visibility>(npc).unwrap(),
        Visibility::Hidden
    );
    let route = app.world().get::<RouteStepper>(npc).unwrap();
    assert!(!route.page_present());
    assert!(route.through());
    assert_eq!(route.stop_count(), 0);
    let before = app.world().get::<MoveQueue>(npc).unwrap().snapshot();
    let transform = *app.world().get::<Transform>(npc).unwrap();
    assert!(app.world().get::<MoveQueue>(npc).unwrap().busy());
    app.update();
    assert_eq!(
        app.world().get::<MoveQueue>(npc).unwrap().snapshot(),
        before
    );
    assert_eq!(*app.world().get::<Transform>(npc).unwrap(), transform);
}

#[test]
fn an_all_instant_repeating_route_crosses_refresh_boundaries_only_once_per_update() {
    let mut repeated = page(vec![command(32, 7), command(40, 0), command(33, 7)]);
    repeated.move_route.repeat = true;
    let mut app = app(vec![event(1, 1, vec![repeated])], false);
    let npc = entity(&mut app, 1);
    for level in 1..=3 {
        app.update();
        assert!(!app.world().resource::<Switches>().get(7));
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().alpha(),
            crate::tiles::character_alpha(level)
        );
    }
}

#[test]
fn a_hero_route_refreshes_a_new_obstacle_before_attempting_its_next_move() {
    let mut original = page(vec![]);
    original.layer = 0;
    let mut next = gated(original.clone());
    next.layer = 1;
    let mut obstacle = event(1, 6, vec![original, next]);
    obstacle.y = 5;
    let mut app = app(vec![obstacle], false);
    let world = app.world_mut();
    world
        .query_filtered::<&mut RouteStepper, With<Player>>()
        .single_mut(world)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 32, 7, 1]));
    app.update();
    let world = app.world_mut();
    assert_eq!(world.query::<&Player>().single(world).unwrap().tile_x, 5);
}

#[test]
fn a_page_disappearing_mid_update_preserves_graphics_but_hides_them_and_counts_once() {
    let mut app = app(
        vec![event(1, 1, vec![gated(page(vec![command(33, 7)]))])],
        true,
    );
    let npc = entity(&mut app, 1);
    app.update();
    let character = app.world().get::<EventSprite>(npc).unwrap();
    assert_eq!((character.charset.as_str(), character.layer), ("Chara1", 1));
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
    app.update();
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
    assert_eq!(
        *app.world().get::<Visibility>(npc).unwrap(),
        Visibility::Hidden
    );
}

#[test]
fn page_reactivation_restores_explicit_route_through_and_legacy_routes_keep_it() {
    for through in [false, true] {
        let mut route = RouteStepper::from_event_page(Some(&page(vec![])));
        let encoded = ron::to_string(&route).unwrap();
        let encoded = encoded
            .replace("route_through:Some(false),", "")
            .replace("through:false", &format!("through:{through}"));
        route = ron::from_str::<RouteStepper>(&encoded).unwrap();
        route.refresh_page(None);
        assert!(route.through());
        let encoded = ron::to_string(&route).unwrap();
        route = ron::from_str::<RouteStepper>(&encoded).unwrap();
        route.refresh_page(Some(&page(vec![])));
        assert_eq!(route.through(), through);
    }
}

#[test]
fn a_nonzero_repeat_cursor_is_not_replayed_across_switch_refreshes() {
    let mut repeated = page(vec![
        command(40, 0),
        command(32, 7),
        command(41, 0),
        command(33, 7),
    ]);
    repeated.move_route.repeat = true;
    let mut app = app(vec![event(1, 1, vec![repeated])], false);
    let npc = entity(&mut app, 1);
    let original = ron::to_string(app.world().get::<RouteStepper>(npc).unwrap()).unwrap();
    assert_eq!(original.matches("index:0").count(), 1);
    let route = ron::from_str::<RouteStepper>(&original.replace("index:0", "index:2")).unwrap();
    app.world_mut().entity_mut(npc).insert(route);
    for _ in 0..3 {
        app.update();
        assert!(app.world().resource::<Switches>().get(7));
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().alpha(),
            223.0 / 255.0
        );
    }
}

#[test]
fn only_a_real_refresh_restores_implicit_through_on_an_inactive_page() {
    for refresh in [false, true] {
        let mut commands = vec![command(33, 7), command(37, 0)];
        if refresh {
            commands.push(command(32, 9));
        }
        commands.push(command(1, 0));
        let mut app = app(vec![event(1, 1, vec![gated(page(commands))])], true);
        let npc = entity(&mut app, 1);
        {
            let mut data = app.world_mut().resource_mut::<MapData>();
            let index = data.width as usize + 2;
            data.upper[index] = 10001;
            data.passages_up[1] = 0;
        }
        app.update();
        assert_eq!(
            app.world().get::<EventSprite>(npc).unwrap().tile_x,
            if refresh { 2 } else { 1 }
        );
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().through(),
            refresh
        );
    }
}
