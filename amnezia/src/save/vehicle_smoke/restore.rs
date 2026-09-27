use super::*;
use crate::world::MainCamera;

pub(super) fn check(world: &mut World, frame: u32) -> Option<&'static str> {
    if !(311..400).contains(&frame) && !(491..610).contains(&frame) {
        return None;
    }
    if world.contains_resource::<saved::Pending>() {
        return None;
    }
    assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
    let phase = world.resource::<Fixture>().phase;
    let resumed = 2 << (phase * 2);
    if world.resource::<Fade>().busy() {
        let expected = world.resource::<Fixture>().saved.as_ref().unwrap().clone();
        let vehicles = world.resource::<Vehicles>();
        assert_eq!(vehicles.save, expected.vehicles);
        assert_eq!(vehicles.motion_snapshot(), expected.motion);
        assert_eq!(world.resource::<CameraPan>().snapshot(), expected.camera);
        let ground = vehicles.pixel(10004, world.resource::<MapData>()).unwrap()
            - Vec2::Y * vehicles.airship_altitude();
        let (hero, transform) = world
            .query::<(&Player, &Transform)>()
            .single(world)
            .unwrap();
        assert_eq!(
            transform.translation.truncate(),
            ground + Vec2::Y * hero.y_offset()
        );
        let camera = world
            .query_filtered::<&Transform, With<MainCamera>>()
            .single(world)
            .unwrap()
            .translation
            .truncate();
        assert_eq!(camera, Vec2::from_array(expected.camera.position.unwrap()));
        black_stage(world);
        if phase == 1 {
            world.resource_mut::<Switches>().set(7, false);
        } else {
            ascent::arm(world);
        }
        let mut fixture = world.resource_mut::<Fixture>();
        fixture.held += 1;
        fixture.checks |= 1 << (phase * 2);
    } else if world.resource::<Fixture>().checks & resumed == 0 {
        assert!(world.resource::<Fixture>().held > 30);
        let vehicles = world.resource::<Vehicles>();
        if phase == 0 {
            assert!(vehicles.airship_transitioning());
        } else {
            assert!(vehicles.routes_pending() && vehicles.jumping(1));
        }
        world.resource_mut::<Fixture>().checks |= resumed;
        return Some(if phase == 0 {
            "saved-vehicles-ascent-resumed"
        } else {
            "saved-vehicles-restored"
        });
    }
    None
}
