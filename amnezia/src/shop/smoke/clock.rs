use bevy::prelude::*;

#[derive(Resource, Default)]
struct Clock {
    delay: u32,
}

pub(super) fn initialize(world: &mut World) {
    world.init_resource::<Clock>();
}

pub(super) fn scripted_frame(world: &World, frame: u32) -> u32 {
    frame.saturating_sub(world.get_resource::<Clock>().map_or(0, |clock| clock.delay))
}

pub(crate) fn finish_frame(world: &World) -> u32 {
    1260 + world.get_resource::<Clock>().map_or(0, |clock| clock.delay)
}

pub(super) fn input_frame(world: &mut World, frame: u32) -> Option<u32> {
    let frame = scripted_frame(world, frame);
    if matches!(frame, 315 | 719 | 779 | 977 | 1027 | 1067 | 1107)
        && (!world.resource::<crate::shop::ShopOpen>().0
            || world.resource::<crate::shop::fades::Flow>().active()
            || world.resource::<crate::transitions::Transition>().busy()
            || world.resource::<crate::timing::SceneWait>().0)
    {
        // Native snapshot readiness can outlive the scripted six-frame fades.
        let mut clock = world.resource_mut::<Clock>();
        clock.delay += 1;
        assert!(clock.delay < 120, "shop did not become ready for input");
        return None;
    }
    Some(frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_shop_key_waits_for_the_terminal_barrier_without_skipping_the_capture() {
        let mut world = World::new();
        initialize(&mut world);
        world.insert_resource(crate::shop::ShopOpen(true));
        world.init_resource::<crate::shop::fades::Flow>();
        world.init_resource::<crate::transitions::Transition>();
        world.insert_resource(crate::timing::SceneWait(true));
        world.init_resource::<ButtonInput<KeyCode>>();
        for frame in 315..319 {
            assert!(super::super::input(&mut world, frame));
            assert_eq!(scripted_frame(&world, frame + 1), 315);
            assert!(
                world
                    .resource::<ButtonInput<KeyCode>>()
                    .get_pressed()
                    .next()
                    .is_none()
            );
        }
        world.resource_mut::<crate::timing::SceneWait>().0 = false;
        assert!(super::super::input(&mut world, 319));
        assert_eq!(scripted_frame(&world, 320), 316);
        assert!(
            !world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::Enter)
        );
        assert!(super::super::input(&mut world, 320));
        assert!(
            world
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Enter)
        );
        assert_eq!(scripted_frame(&world, 321), 317);
        assert!(super::super::input(&mut world, 321));
        assert!(
            !world
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::Enter)
        );
        assert_eq!(finish_frame(&world), 1264);
    }

    #[test]
    fn every_shop_entry_waits_for_the_open_scene() {
        for checkpoint in [315, 719, 779, 977, 1027, 1067, 1107] {
            let mut world = World::new();
            initialize(&mut world);
            world.init_resource::<crate::shop::ShopOpen>();
            world.init_resource::<crate::shop::fades::Flow>();
            world.init_resource::<crate::transitions::Transition>();
            world.init_resource::<crate::timing::SceneWait>();
            assert_eq!(input_frame(&mut world, checkpoint), None);
            world.resource_mut::<crate::shop::ShopOpen>().0 = true;
            assert_eq!(input_frame(&mut world, checkpoint + 1), Some(checkpoint));
        }
    }
}
