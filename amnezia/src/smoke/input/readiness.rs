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
        ("items" | "skills", 319)
        | ("items", 961 | 1181 | 1197)
        | ("skills", 397 | 513 | 999)
        | ("equipment", 1094 | 1140 | 1250) => true,
        ("items", 979) | ("equipment", 719 | 1109 | 1169) => false,
        _ => {
            if let Some(mut clock) = world.get_resource_mut::<MenuClock>() {
                clock.consecutive = 0;
            }
            return false;
        }
    };
    if world.resource::<crate::menu::MenuOpen>().0 == open
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
        assert!(hold(&mut world, "items", 961));
        world.resource_mut::<crate::timing::SceneWait>().0 = false;
        assert!(!hold(&mut world, "items", 961));
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
    fn equipment_script_waits_before_cancel_and_fixture_reentry() {
        for (frame, open) in [
            (1094, true),
            (1140, true),
            (1250, true),
            (719, false),
            (1109, false),
            (1169, false),
        ] {
            let mut world = World::new();
            world.insert_resource(crate::menu::MenuOpen(!open));
            world.init_resource::<crate::menu::SceneFlow>();
            world.init_resource::<crate::transitions::Transition>();
            world.init_resource::<crate::timing::SceneWait>();
            world.init_resource::<ButtonInput<KeyCode>>();
            assert!(hold(&mut world, "equipment", frame));
            world.resource_mut::<crate::menu::MenuOpen>().0 = open;
            world.resource_mut::<crate::timing::SceneWait>().0 = true;
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            assert!(hold(&mut world, "equipment", frame));
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
            assert!(!hold(&mut world, "equipment", frame));
            assert!(!waiting(&world));
            assert_eq!(scripted_frame(&world, frame + 3), frame + 1);
        }
    }
}
