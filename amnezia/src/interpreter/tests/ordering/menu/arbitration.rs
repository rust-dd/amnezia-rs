use super::super::scenes::{battle_requests, scene, shop_requests};
use super::*;

#[test]
fn a_queued_foreground_scene_replaces_the_players_menu_request() {
    for code in [10710, 10720, 11910, 12420] {
        let mut app = fixture();
        tick(&mut app, &[KeyCode::Escape]);
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 0, vec![scene(code)])],
        });
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .queue_event(0, 1, 0, false);
        tick(&mut app, &[]);
        assert!(!calling(&app), "{code}");
        assert!(!app.world().resource::<Transition>().busy(), "{code}");
        assert_eq!(battle_requests(&mut app).len(), usize::from(code == 10710));
        assert_eq!(shop_requests(&mut app).len(), usize::from(code == 10720));
        assert_eq!(app.world().resource::<EventSaveRequest>().0, code == 11910);
        assert_eq!(app.world().resource::<GameOverActive>().0, code == 12420);
    }
}

#[test]
fn the_players_menu_request_replaces_an_earlier_parallel_scene() {
    for code in [10710, 10720, 11910, 12420] {
        let mut app = fixture();
        tick(&mut app, &[KeyCode::Escape]);
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, vec![scene(code), cmd(11410, 0, vec![100])])],
        });
        tick(&mut app, &[]);
        assert!(calling(&app), "{code}");
        assert!(battle_requests(&mut app).is_empty());
        assert!(shop_requests(&mut app).is_empty());
        assert!(!app.world().resource::<EventSaveRequest>().0);
        assert!(!app.world().resource::<GameOverActive>().0);
        for _ in 0..6 {
            tick(&mut app, &[]);
        }
        assert!(app.world().resource::<MenuOpen>().0);
    }
}

#[test]
fn an_inn_prompt_is_not_a_replacement_scene() {
    let mut app = fixture();
    tick(&mut app, &[KeyCode::Escape]);
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 0, vec![cmd(10730, 0, vec![0, 50, 1])])],
    });
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .queue_event(0, 1, 0, false);
    tick(&mut app, &[]);
    assert!(calling(&app));
    assert!(matches!(
        shop_requests(&mut app).as_slice(),
        [ShopRequest::ShowInn { cost: 50, .. }]
    ));
}
