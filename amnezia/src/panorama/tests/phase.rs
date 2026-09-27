use super::*;
use std::time::Duration;

fn position(app: &mut App) -> Vec2 {
    let world = app.world_mut();
    world
        .query::<(&PanoramaTile, &Transform)>()
        .iter(world)
        .find(|(tile, _)| tile.0 == 0 && tile.1 == 0)
        .unwrap()
        .1
        .translation
        .truncate()
}

fn change(app: &mut App, name: &str, params: &[i32]) {
    loading::change(app, name, params);
}

fn update(app: &mut App, seconds: f64) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(seconds),
    ));
    app.update();
}

#[test]
fn changing_the_same_sky_speed_does_not_restart_its_scroll_phase() {
    let mut app = loading::loaded_without_tiles();
    change(&mut app, "Sky", &[1, 1, 1, 8, 1, -6]);
    for _ in 0..3 {
        update(&mut app, 1.0 / 60.0);
    }
    let before = position(&mut app);
    change(&mut app, "Sky", &[1, 1, 1, 6, 1, -4]);
    update(&mut app, 0.0);
    assert_eq!(position(&mut app), before);
}

#[test]
fn changing_only_loop_flags_does_not_initialize_the_same_bitmap_again() {
    let mut app = loading::loaded_without_tiles();
    let before = position(&mut app);
    change(&mut app, "Sky", &[0; 6]);
    update(&mut app, 0.0);
    assert_eq!(position(&mut app), before);
}

#[test]
fn an_empty_override_returns_to_the_maps_original_panorama() {
    let mut app = loading::loaded_without_tiles();
    let definition = PanoramaDef::from_command("Sky".into(), &[1, 1, 0, 0, 0, 0]);
    app.world_mut().resource_mut::<MapData>().panorama = Some(definition.clone());
    change(&mut app, "", &[0; 6]);
    update(&mut app, 0.0);
    assert_eq!(
        app.world().resource::<Panorama>().definition,
        Some(definition)
    );
    assert_eq!(
        app.world_mut()
            .query::<&PanoramaTile>()
            .iter(app.world())
            .count(),
        9
    );
}

#[test]
fn fractional_auto_scroll_uses_the_original_integer_source_pixel() {
    let mut app = loading::loaded_without_tiles();
    change(&mut app, "Sky", &[1, 1, 1, 1, 0, 0]);
    update(&mut app, 1.0 / 60.0);
    assert_eq!(position(&mut app), Vec2::new(161.0, -600.0));
    update(&mut app, 0.0);
    assert_eq!(position(&mut app), Vec2::new(161.0, -600.0));
}
