use super::*;

#[test]
fn foreground_rain_start_waits_for_the_following_screen_update() {
    let mut app = app(60);
    app.insert_resource(Weather::None);
    let before = app.world().resource::<Rain>().drops.clone();
    app.add_systems(
        Update,
        (|mut weather: ResMut<Weather>| *weather = Weather::Rain)
            .in_set(crate::interpreter::InterpreterStep),
    );
    app.update();
    assert_eq!(app.world().resource::<Rain>().drops, before);
    app.update();
    assert_ne!(app.world().resource::<Rain>().drops, before);
}

#[test]
fn foreground_rain_stop_keeps_the_preceding_screen_update() {
    let mut app = app(60);
    let mut reference = Rain::new(crate::interpreter::EventRng::seeded(31415));
    reference.advance(1.0 / 60.0);
    app.add_systems(
        Update,
        (|mut weather: ResMut<Weather>| *weather = Weather::None)
            .in_set(crate::interpreter::InterpreterStep),
    );
    app.update();
    assert_eq!(app.world().resource::<Rain>().drops, reference.drops);
    app.update();
    assert_eq!(app.world().resource::<Rain>().drops, reference.drops);
}
