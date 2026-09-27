use super::*;
use crate::player::Player;

fn fixture(tag: &str) -> (App, std::path::PathBuf, Entity) {
    let (mut app, path) = app(tag);
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world.entity_mut(hero).insert((
        MoveQueue::default(),
        RouteStepper::default(),
        Transform::default(),
        Sprite::default(),
    ));
    let npc = npc(world);
    world.entity_mut(npc).insert(AutoMove::new(3, 3, 4, 1));
    for entity in [hero, npc] {
        let mut route = world.get_mut::<RouteStepper>(entity).unwrap();
        route.set_stop_maximum(64);
        route.set_stop_count(37);
    }
    (app, path, hero)
}

#[test]
fn hero_and_npc_share_their_exact_saved_stop_threshold_and_elapsed_count() {
    let (mut app, path, hero) = fixture("shared-stops");
    let before = snapshot(app.world_mut());
    let before_hero = crate::world::saved::hero::snapshot(app.world_mut()).unwrap();
    save_and_load(&mut app);
    assert_eq!(snapshot(app.world_mut()), before);
    assert_eq!(
        crate::world::saved::hero::snapshot(app.world_mut()).unwrap(),
        before_hero
    );
    let encoded = std::fs::read_to_string(&path).unwrap();
    assert!(encoded.contains("format_version: 20"));
    let npc = npc(app.world_mut());
    for entity in [hero, npc] {
        let mut route = app.world_mut().get_mut::<RouteStepper>(entity).unwrap();
        assert_eq!((route.stop_count(), route.stop_maximum()), (37, 64));
        for _ in 0..27 {
            assert!(route.stop_active());
            route.advance_stop_clock(false, true);
        }
        assert!(!route.stop_active());
        assert_eq!(route.stop_count(), 64);
        assert!(!ron::to_string(&*route).unwrap().contains("timer:"));
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn old_route_and_autonomous_countdowns_migrate_without_rewriting_the_slot() {
    let (mut app, path, hero) = fixture("legacy-stops");
    let event = ron::to_string(&snapshot(app.world_mut()).remove(0)).unwrap();
    let hero_state =
        ron::to_string(&crate::world::saved::hero::snapshot(app.world_mut()).unwrap()).unwrap();
    let compact = format!(
        "(format_version:19,map_id:3,x:10,y:10,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,map_events:[{event}],hero_motion:Some({hero_state}))"
    );
    let old_stop = "stop:Some((count:37,maximum:64)),";
    assert_eq!(compact.matches(old_stop).count(), 2);
    let old_auto = "autonomy:(move_type:3,frequency:3,speed:4,rng:";
    assert!(compact.contains(old_auto));
    let original = compact.replace(old_stop, "timer:0.25,").replace(
        old_auto,
        "autonomy:(move_type:3,frequency:3,speed:4,timer:0.1,rng:",
    );
    std::fs::write(&path, &original).unwrap();
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    let npc = npc(app.world_mut());
    for (entity, expected) in [(hero, 49), (npc, 58)] {
        let route = app.world().get::<RouteStepper>(entity).unwrap();
        assert_eq!((route.stop_count(), route.stop_maximum()), (expected, 64));
        assert!(route.stop_active());
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    std::fs::remove_file(path).unwrap();
}
