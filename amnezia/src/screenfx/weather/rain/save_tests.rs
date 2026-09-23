use super::*;
use crate::save::{EventSaveRequest, LoadRequest, SavePlugin};
use crate::screenfx::Fx;
use crate::world::{MapData, MapRebuilt};

fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path =
        std::env::temp_dir().join(format!("amnezia_weather_{tag}_{}.ron", std::process::id()));
    let mut app = crate::save::tests::save_resources(path.clone());
    app.add_plugins(SavePlugin)
        .init_resource::<Fx>()
        .init_resource::<crate::screenfx::flash::channel::Inbox>()
        .init_resource::<Rain>()
        .init_resource::<crate::interpreter::RunningEvent>()
        .add_message::<MapRebuilt>()
        .insert_resource(Scroll {
            pan: Vec2::new(12.25, 149.5),
            previous: Some(Vec2::new(100.0, 200.0)),
        })
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(crate::player::Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    (app, path)
}

fn save(app: &mut App) {
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
}

fn reload(app: &mut App) {
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    app.world_mut().write_message(MapRebuilt);
    app.update();
}

#[test]
fn a_saved_screen_restores_the_weather_scroll_but_initializes_fresh_drops() {
    let (mut app, path) = app("restore");
    *app.world_mut().resource_mut::<Weather>() = Weather::Rain;
    app.world_mut().resource_mut::<WeatherStrength>().0 = 2;
    app.world_mut()
        .resource_mut::<Rain>()
        .drops
        .fill(model::Drop {
            x: -999,
            y: 999,
            life: 0,
        });
    save(&mut app);
    reload(&mut app);
    std::fs::remove_file(path).unwrap();
    assert_eq!(*app.world().resource::<Weather>(), Weather::Rain);
    assert_eq!(app.world().resource::<WeatherStrength>().0, 2);
    assert_eq!(
        app.world().resource::<Scroll>().pan,
        Vec2::new(12.25, 149.5)
    );
    assert_eq!(app.world().resource::<Scroll>().previous, None);
    assert!(
        app.world()
            .resource::<Rain>()
            .drops
            .iter()
            .all(|drop| (0..320).contains(&drop.x) && (0..160).contains(&drop.y))
    );
}

#[test]
fn the_save_contains_scroll_even_when_weather_is_disabled() {
    let (mut app, path) = app("inactive");
    save(&mut app);
    let contents = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(contents.contains("weather_pan"));
    assert!(contents.contains("12.25") && contents.contains("149.5"));
}

#[test]
fn old_save_versions_default_scroll_without_rewriting_the_file() {
    for version in 0..=6 {
        let (mut app, path) = app(&format!("legacy_{version}"));
        let state = ron::to_string(&crate::screenfx::saved::snapshot(app.world())).unwrap();
        let old = state.replace(",weather_pan:(12.25,149.5)", "");
        assert!(!old.contains("weather_pan"));
        let screen = if version == 6 {
            format!(",screen:Some({old})")
        } else {
            String::new()
        };
        let contents = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,weather:1,weather_strength:1{screen})"
        );
        std::fs::write(&path, &contents).unwrap();
        reload(&mut app);
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(true)
        );
        assert_eq!(app.world().resource::<Scroll>().pan, Vec2::ZERO);
        assert_eq!(*app.world().resource::<Weather>(), Weather::Rain);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn invalid_weather_scroll_cannot_change_the_session_or_file() {
    for (tag, replace) in [
        ("nan", "NaN"),
        ("infinite", "inf"),
        ("negative", "-0.25"),
        ("past_edge", "320.0"),
    ] {
        let (mut app, path) = app(tag);
        save(&mut app);
        let original = std::fs::read_to_string(&path).unwrap();
        let invalid = original.replace("12.25", replace);
        assert_ne!(invalid, original);
        std::fs::write(&path, &invalid).unwrap();
        let before = app.world().resource::<Rain>().drops.clone();
        app.world_mut()
            .resource_mut::<crate::state::Switches>()
            .set(99, true);
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(false)
        );
        assert_eq!(
            app.world().resource::<Scroll>().pan,
            Vec2::new(12.25, 149.5)
        );
        assert_eq!(app.world().resource::<Rain>().drops, before);
        assert!(app.world().resource::<crate::state::Switches>().get(99));
        assert!(
            !app.world()
                .contains_resource::<crate::screenfx::saved::Pending>()
        );
        assert!(
            app.world()
                .resource::<crate::teleport::PendingTeleport>()
                .0
                .is_none()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn weather_scroll_validation_checks_both_axes_and_the_fractional_edges() {
    let (app, _) = app("bounds");
    let mut screen = crate::screenfx::saved::snapshot(app.world());
    for value in [[0.0, 0.0], [319.9375, 159.9375]] {
        screen.weather_pan = value;
        assert!(screen.valid());
    }
    for value in [
        [320.0, 0.0],
        [0.0, 160.0],
        [-0.0625, 0.0],
        [0.0, -0.0625],
        [f32::NAN, 0.0],
        [0.0, f32::INFINITY],
    ] {
        screen.weather_pan = value;
        assert!(!screen.valid());
    }
}
