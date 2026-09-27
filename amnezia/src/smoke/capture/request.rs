use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot};
use bevy::window::PrimaryWindow;

pub(super) fn entity(world: &mut World, screenshot: Screenshot) -> Entity {
    let primary = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok();
    let target = screenshot.0.normalize(primary);
    if target.is_some()
        && let Some(entity) = world
            .query_filtered::<(Entity, &Screenshot), Without<Capturing>>()
            .iter(world)
            .find_map(|(entity, pending)| {
                (pending.0.normalize(primary) == target).then_some(entity)
            })
    {
        return entity;
    }
    world.spawn(screenshot).id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::view::screenshot::ScreenshotCaptured;

    #[derive(Resource, Default)]
    struct Delivered(Vec<&'static str>);

    fn world() -> World {
        let mut world = World::new();
        world.spawn(PrimaryWindow);
        world
    }

    #[test]
    fn same_frame_window_captures_share_one_request_and_deliver_to_every_label() {
        let mut world = world();
        world.init_resource::<Delivered>();
        let first = entity(&mut world, Screenshot::primary_window());
        world.entity_mut(first).observe(
            |_: On<ScreenshotCaptured>, mut labels: ResMut<Delivered>| {
                labels.0.push("transferred");
            },
        );
        let second = entity(&mut world, Screenshot::primary_window());
        world.entity_mut(second).observe(
            |_: On<ScreenshotCaptured>, mut labels: ResMut<Delivered>| {
                labels.0.push("final");
            },
        );
        world.trigger(ScreenshotCaptured {
            entity: first,
            image: Image::default(),
        });
        let mut labels = world.resource::<Delivered>().0.clone();
        labels.sort_unstable();
        assert_eq!(labels, ["final", "transferred"]);
        assert_eq!(first, second);
        assert_eq!(world.query::<&Screenshot>().iter(&world).count(), 1);
    }

    #[test]
    fn offscreen_requests_share_their_target_but_never_a_different_target() {
        let mut world = world();
        let mut images = Assets::<Image>::default();
        let a = images.add(Image::default());
        let b = images.add(Image::default());
        let first = entity(&mut world, Screenshot::image(a.clone()));
        assert_eq!(entity(&mut world, Screenshot::image(a)), first);
        assert_ne!(entity(&mut world, Screenshot::image(b)), first);
        assert_ne!(entity(&mut world, Screenshot::primary_window()), first);
    }

    #[test]
    fn an_in_flight_capture_cannot_receive_a_later_frames_observer() {
        let mut world = world();
        let first = entity(&mut world, Screenshot::primary_window());
        world.entity_mut(first).insert(Capturing);
        let second = entity(&mut world, Screenshot::primary_window());
        assert_ne!(first, second);
        assert_eq!(entity(&mut world, Screenshot::primary_window()), second);
    }

    #[test]
    fn primary_and_explicit_window_references_resolve_to_the_same_capture() {
        let mut world = world();
        let window = world
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(&world)
            .unwrap();
        let first = entity(&mut world, Screenshot::primary_window());
        assert_eq!(entity(&mut world, Screenshot::window(window)), first);
    }
}
