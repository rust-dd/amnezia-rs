use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn none_waits_its_requested_updates_without_waiting_for_a_scene_snapshot() {
    for erased in [false, true] {
        for erase in [false, true] {
            let mut world = World::new();
            world.init_resource::<crate::timing::GameFrames>();
            let mut state = Transition {
                erased,
                ..default()
            };
            assert!(state.start_for(Kind::None, erase, 50, IVec2::ZERO, 3));
            world.insert_resource(state);
            world.insert_resource(snapshots::Capture::new(
                Handle::default(),
                Handle::default(),
                Handle::default(),
            ));
            for raw in 50..=53 {
                world.resource_mut::<crate::timing::GameFrames>().frame = raw;
                world.run_system_once(tick).unwrap();
                let state = world.resource::<Transition>();
                assert!(state.busy());
                assert_eq!(state.erased, erased);
                assert_eq!(state.updated, raw > 50);
                assert_eq!(state.frame, (raw - 50).saturating_sub(1));
            }
            world.resource_mut::<crate::timing::GameFrames>().frame = 54;
            world.run_system_once(tick).unwrap();
            let state = world.resource::<Transition>();
            assert!(!state.busy());
            assert_eq!(state.erased(), erased);
        }
    }
}

#[test]
fn a_missing_snapshot_keeps_the_effect_invisible_until_its_first_real_update() {
    let mut world = World::new();
    world.init_resource::<crate::timing::GameFrames>();
    let mut state = Transition::default();
    state.start(Kind::Fade, false, 5, IVec2::ZERO);
    world.insert_resource(state);
    world.insert_resource(snapshots::Capture::new(
        Handle::default(),
        Handle::default(),
        Handle::default(),
    ));
    for raw in 6..100 {
        world.resource_mut::<crate::timing::GameFrames>().frame = raw;
        world.run_system_once(tick).unwrap();
        let state = world.resource::<Transition>();
        assert!(!state.updated);
        assert!(state.busy());
    }
    world.remove_resource::<snapshots::Capture>();
    world.resource_mut::<crate::timing::GameFrames>().frame = 100;
    world.run_system_once(tick).unwrap();
    let state = world.resource::<Transition>();
    assert!(state.updated);
    assert_eq!(state.frame, 0);
}
