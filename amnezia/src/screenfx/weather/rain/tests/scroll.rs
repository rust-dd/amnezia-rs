use super::*;
use crate::player::{CameraFollow, CameraPan};
use crate::world::RouteStepper;

#[test]
fn jump_landing_rounding_does_not_scroll_the_rain_surface() {
    let (mut app, hero) = crate::world::test_support::camera_app((20, 15));
    app.init_resource::<Scroll>()
        .add_message::<MapChanged>()
        .add_systems(Update, view::track_scroll.after(CameraFollow));
    app.update();
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 1, 0]);
    for _ in 0..33 {
        app.update();
    }
    let before = app.world().resource::<Scroll>().pan;
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(6);
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 25]));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(app.world().resource::<Scroll>().pan, before - Vec2::X);
}
