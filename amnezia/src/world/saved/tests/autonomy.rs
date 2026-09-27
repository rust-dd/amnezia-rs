use super::*;
use std::time::Duration;

fn advance(app: &mut App) -> EventState {
    let world = app.world_mut();
    crate::world::autonomy::autonomous_movement(world);
    world
        .run_system_cached_with(crate::world::movement::walk_selected::<EventSprite>, None)
        .unwrap();
    snapshot(world).remove(0)
}

#[test]
fn random_movement_resumes_the_exact_saved_stream_and_stop_clock_at_four_frame_rates() {
    let mut reference = None;
    for fps in [15, 30, 60, 144] {
        let (mut app, path) = app(&format!("random-autonomy-{fps}"));
        app.init_resource::<crate::menu::MenuOpen>()
            .init_resource::<crate::shop::ShopOpen>()
            .init_resource::<crate::battle::BattleActive>()
            .init_resource::<crate::gameover::GameOverActive>()
            .insert_resource(crate::title::TitleActive(false));
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(1.0 / 60.0),
        ));
        let entity = npc(app.world_mut());
        let mut page = app.world().resource::<MapEvents>().events[0].pages[0].clone();
        page.move_type = 1;
        page.move_frequency = 7;
        page.move_speed = 4;
        let mut route = RouteStepper::from_event_page(Some(&page));
        let mut auto = AutoMove::new(1, 7, 4, 1);
        auto.refresh(Some(&page), &mut route);
        app.world_mut().entity_mut(entity).insert((route, auto));
        app.update();
        for _ in 0..35 {
            advance(&mut app);
        }
        save_and_load(&mut app);
        let saved = snapshot(app.world_mut()).remove(0);
        let mut expected = Vec::new();
        let mut clock = crate::timing::GameFrames::default();
        for _ in 0..fps * 2 {
            let before = clock.frame;
            clock.advance(1.0 / f64::from(fps));
            for _ in before..clock.frame {
                expected.push(advance(&mut app));
            }
        }
        assert_eq!(expected.len(), 120);
        assert_ne!(expected.last().unwrap().autonomy, saved.autonomy);
        assert!(
            expected
                .iter()
                .any(|state| state.motion.clone().into_queue().busy())
        );
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(snapshot(app.world_mut()).remove(0), saved);
        for state in &expected {
            assert_eq!(&advance(&mut app), state, "{fps} FPS");
        }
        if let Some(reference) = &reference {
            assert_eq!(&expected, reference, "{fps} FPS");
        } else {
            reference = Some(expected);
        }
        std::fs::remove_file(path).unwrap();
    }
}
