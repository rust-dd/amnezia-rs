use super::*;

#[test]
fn weather_code_round_trips() {
    for (code, kind) in [
        (0, Weather::None),
        (1, Weather::Rain),
        (2, Weather::Snow),
        (3, Weather::Fog),
    ] {
        assert_eq!(Weather::from_code(code), kind);
        assert_eq!(kind.code(), code);
    }
    // Sandstorm (type 4) is not producible by the Weather command.
    assert_eq!(Weather::from_code(4), Weather::None);
}

#[test]
fn particle_count_scales_with_strength_and_clamps() {
    assert_eq!(particle_count(0), 20);
    assert_eq!(particle_count(1), 60);
    assert_eq!(particle_count(2), 100);
    assert_eq!(particle_count(9), 100, "strength clamps into 0..2");
}

#[test]
fn particle_midscreen_just_falls() {
    let mut p = WeatherParticle {
        x: 100.0,
        y: 10.0,
        fall: 240.0,
        drift: 0.0,
        wobble_amp: 0.0,
        wobble_freq: 0.0,
        phase: 0.0,
        rng: 1,
    };
    let recycled = p.advance(0.05);
    assert!(!recycled, "a mid-screen drop does not recycle");
    assert!((p.y - 22.0).abs() < 1e-4, "it falls fall*dt = 12 px");
}

#[test]
fn particle_recycles_off_the_bottom() {
    let mut p = WeatherParticle {
        x: 100.0,
        y: SCREEN_H - 2.0,
        fall: 240.0,
        drift: 0.0,
        wobble_amp: 0.0,
        wobble_freq: 0.0,
        phase: 0.0,
        rng: 12_345,
    };
    let recycled = p.advance(0.05);
    assert!(recycled, "passing the bottom edge recycles the drop");
    assert!(p.y >= 0.0 && p.y < SCREEN_H, "it wrapped back to the top");
    assert!(
        p.x >= 0.0 && p.x <= SCREEN_W,
        "it respawned within the width"
    );
}

#[test]
fn rain_spawns_scaled_particles_and_none_clears_them() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<MapChanged>();
    app.init_resource::<Weather>();
    app.init_resource::<WeatherStrength>();
    app.init_resource::<AppliedWeather>();
    app.insert_resource(FogAssets {
        texture: Handle::default(),
    });
    app.add_systems(Update, rebuild_weather);
    app.world_mut().spawn((MainCamera, Transform::default()));

    *app.world_mut().resource_mut::<Weather>() = Weather::Rain;
    app.world_mut().resource_mut::<WeatherStrength>().0 = 1;
    app.update();
    assert_eq!(count_particles(&mut app), 60, "medium rain spawns 60 drops");

    *app.world_mut().resource_mut::<Weather>() = Weather::None;
    app.update();
    assert_eq!(
        count_particles(&mut app),
        0,
        "clearing weather removes them"
    );
}

#[test]
fn snow_strength_two_spawns_the_full_count() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<MapChanged>();
    app.init_resource::<Weather>();
    app.init_resource::<WeatherStrength>();
    app.init_resource::<AppliedWeather>();
    app.insert_resource(FogAssets {
        texture: Handle::default(),
    });
    app.add_systems(Update, rebuild_weather);
    app.world_mut().spawn((MainCamera, Transform::default()));

    *app.world_mut().resource_mut::<Weather>() = Weather::Snow;
    app.world_mut().resource_mut::<WeatherStrength>().0 = 2;
    app.update();
    assert_eq!(
        count_particles(&mut app),
        100,
        "strong snow spawns 100 flakes"
    );
}

fn count_particles(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<(), With<WeatherParticle>>();
    query.iter(app.world()).count()
}
