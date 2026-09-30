use bevy::prelude::*;

#[derive(Resource, Default)]
struct MenuClock {
    delay: u32,
    waiting: bool,
    consecutive: u32,
}

pub(super) fn waiting(world: &World) -> bool {
    world
        .get_resource::<MenuClock>()
        .is_some_and(|clock| clock.waiting)
}

pub(super) fn scripted_frame(world: &World, frame: u32) -> u32 {
    frame.saturating_sub(
        world
            .get_resource::<MenuClock>()
            .map_or(0, |clock| clock.delay),
    )
}

pub(super) fn hold(world: &mut World, scenario: &str, frame: u32) -> bool {
    if let Some(mut clock) = world.get_resource_mut::<MenuClock>() {
        clock.waiting = false;
    }
    let open = match (scenario, frame) {
        ("items" | "skills", 319) | ("equipment", 1094 | 1140 | 1250) => Some(true),
        ("items", 979) | ("equipment", 719 | 1109 | 1169) => Some(false),
        ("items", _) if crate::menu::item_smoke::input(frame).is_some() => None,
        ("skills", _) if crate::menu::skill_smoke::input(frame).is_some() => None,
        _ => {
            if let Some(mut clock) = world.get_resource_mut::<MenuClock>() {
                clock.consecutive = 0;
            }
            return false;
        }
    };
    if open.is_none_or(|open| world.resource::<crate::menu::MenuOpen>().0 == open)
        && !world.resource::<crate::menu::SceneFlow>().active()
        && !world.resource::<crate::transitions::Transition>().busy()
        && !world.resource::<crate::timing::SceneWait>().0
    {
        if let Some(mut clock) = world.get_resource_mut::<MenuClock>() {
            clock.consecutive = 0;
        }
        return false;
    }
    // Scripted scene changes must finish their terminal pause before the next action.
    world.init_resource::<MenuClock>();
    let mut clock = world.resource_mut::<MenuClock>();
    clock.delay += 1;
    clock.waiting = true;
    clock.consecutive += 1;
    assert!(
        clock.consecutive < 60,
        "{scenario} scene did not become ready at {frame}"
    );
    world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_and_skill_scripts_wait_before_the_first_menu_action() {
        for scenario in ["items", "skills"] {
            let mut world = World::new();
            world.insert_resource(crate::menu::MenuOpen(true));
            world.init_resource::<crate::menu::SceneFlow>();
            world.init_resource::<crate::transitions::Transition>();
            world.insert_resource(crate::timing::SceneWait(true));
            world.init_resource::<ButtonInput<KeyCode>>();
            for raw in 319..323 {
                assert_eq!(scripted_frame(&world, raw), 319);
                assert!(hold(&mut world, scenario, 319));
                assert!(waiting(&world));
                assert_eq!(scripted_frame(&world, raw + 1), 319);
                assert!(
                    world
                        .resource::<ButtonInput<KeyCode>>()
                        .get_pressed()
                        .next()
                        .is_none()
                );
            }
            world.resource_mut::<crate::timing::SceneWait>().0 = false;
            assert!(!hold(&mut world, scenario, 319));
            assert!(!waiting(&world));
            assert_eq!(scripted_frame(&world, 324), 320);
            assert_eq!(scripted_frame(&world, 1484), 1480);
        }
    }

    #[test]
    fn returning_from_items_waits_for_both_the_menu_and_map_fades() {
        let mut world = World::new();
        world.insert_resource(crate::menu::MenuOpen(true));
        world.init_resource::<crate::menu::SceneFlow>();
        world.init_resource::<crate::transitions::Transition>();
        world.insert_resource(crate::timing::SceneWait(true));
        world.init_resource::<ButtonInput<KeyCode>>();
        assert!(hold(&mut world, "items", 962));
        world.resource_mut::<crate::timing::SceneWait>().0 = false;
        assert!(!hold(&mut world, "items", 962));
        assert!(hold(&mut world, "items", 979));
        world.resource_mut::<crate::menu::MenuOpen>().0 = false;
        world.resource_mut::<crate::timing::SceneWait>().0 = true;
        assert!(hold(&mut world, "items", 979));
        world.resource_mut::<crate::timing::SceneWait>().0 = false;
        assert!(!hold(&mut world, "items", 979));
        assert_eq!(scripted_frame(&world, 983), 980);
        assert!(!waiting(&world));
    }

    #[test]
    fn scripted_cancellation_and_reentry_wait_for_the_terminal_frame() {
        for (scenario, frame, open) in [
            ("equipment", 1094, true),
            ("equipment", 1140, true),
            ("equipment", 1250, true),
            ("equipment", 719, false),
            ("equipment", 1109, false),
            ("equipment", 1169, false),
            ("items", 1426, true),
            ("items", 1442, true),
        ] {
            let mut world = World::new();
            world.insert_resource(crate::menu::MenuOpen(open));
            world.init_resource::<crate::menu::SceneFlow>();
            world.init_resource::<crate::transitions::Transition>();
            world.insert_resource(crate::timing::SceneWait(true));
            world.init_resource::<ButtonInput<KeyCode>>();
            assert!(hold(&mut world, scenario, frame));
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            assert!(hold(&mut world, scenario, frame));
            assert_eq!(scripted_frame(&world, frame + 2), frame);
            assert!(waiting(&world));
            assert!(
                world
                    .resource::<ButtonInput<KeyCode>>()
                    .get_pressed()
                    .next()
                    .is_none()
            );
            world.resource_mut::<crate::timing::SceneWait>().0 = false;
            assert!(!hold(&mut world, scenario, frame));
            assert!(!waiting(&world));
            assert_eq!(scripted_frame(&world, frame + 3), frame + 1);
        }
    }

    #[test]
    fn every_discrete_item_and_skill_action_waits_for_scene_readiness() {
        for scenario in ["items", "skills"] {
            for frame in 300..1480 {
                let key = if scenario == "items" {
                    crate::menu::item_smoke::input(frame)
                } else {
                    crate::menu::skill_smoke::input(frame)
                };
                let Some(key) = key else {
                    continue;
                };
                let mut world = World::new();
                world.insert_resource(crate::menu::MenuOpen(true));
                world.init_resource::<crate::menu::SceneFlow>();
                world.init_resource::<crate::transitions::Transition>();
                world.insert_resource(crate::timing::SceneWait(true));
                world.init_resource::<ButtonInput<KeyCode>>();
                world.resource_mut::<ButtonInput<KeyCode>>().press(key);
                assert!(hold(&mut world, scenario, frame), "{scenario} at {frame}");
                assert_eq!(scripted_frame(&world, frame + 1), frame);
                assert!(!world.resource::<ButtonInput<KeyCode>>().pressed(key));
                world.resource_mut::<crate::timing::SceneWait>().0 = false;
                assert!(!hold(&mut world, scenario, frame));
                assert_eq!(scripted_frame(&world, frame + 2), frame + 1);
            }
        }
    }

    #[test]
    fn intentional_transition_input_checks_do_not_pause_the_script() {
        for (scenario, frames) in [
            ("skills", &[1010, 1035, 1036, 1042, 1090][..]),
            ("equipment", &[900, 901][..]),
            ("menu", &[1140][..]),
        ] {
            let mut world = World::new();
            world.insert_resource(crate::timing::SceneWait(true));
            world.init_resource::<ButtonInput<KeyCode>>();
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowDown);
            for &frame in frames {
                assert!(!hold(&mut world, scenario, frame));
                assert!(!waiting(&world));
                assert_eq!(scripted_frame(&world, frame), frame);
                assert!(
                    world
                        .resource::<ButtonInput<KeyCode>>()
                        .pressed(KeyCode::ArrowDown)
                );
            }
        }
    }
}
