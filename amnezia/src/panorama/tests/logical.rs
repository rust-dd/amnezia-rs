use super::*;
use std::time::Duration;

fn update(app: &mut App, seconds: f64) {
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(seconds),
    ));
    app.update();
}

fn phase(app: &App) -> [i64; 2] {
    app.world()
        .resource::<Panorama>()
        .motion
        .as_ref()
        .unwrap()
        .phase
}

#[test]
fn original_subpixel_clock_and_pauses_match_at_low_and_high_render_rates() {
    for fps in [15, 30, 60, 144] {
        let mut app = loading::loaded_without_tiles();
        loading::change(&mut app, "Sky", &[1, 1, 1, -1, 1, 1]);
        for frame in 1..=fps {
            update(&mut app, 1.0 / f64::from(fps));
            let ticks = i64::from(frame * 60 / fps);
            assert_eq!(
                phase(&app),
                [ticks * 2, 256 - ticks * 2],
                "{fps} FPS, render {frame}"
            );
        }
        let before = app.world().resource::<Panorama>().clone();
        app.insert_resource(crate::menu::MenuOpen(true));
        for _ in 0..fps {
            update(&mut app, 1.0 / f64::from(fps));
        }
        assert_eq!(*app.world().resource::<Panorama>(), before);
        app.world_mut().resource_mut::<crate::menu::MenuOpen>().0 = false;
        update(&mut app, 0.0);
        assert_eq!(*app.world().resource::<Panorama>(), before);
        update(&mut app, 1.0 / 60.0);
        assert_eq!(phase(&app), [122, 134]);
    }
}

#[test]
fn serialized_phase_fraction_and_changed_speed_continue_without_restarting() {
    for fps in [15, 30, 60, 144] {
        let mut app = loading::loaded_without_tiles();
        loading::change(&mut app, "Sky", &[1, 1, 1, -1, 1, 1]);
        update(&mut app, 1.0 / 144.0);
        let text = ron::to_string(app.world().resource::<Panorama>()).unwrap();
        let saved = ron::from_str::<Panorama>(&text).unwrap();
        assert!(saved.valid());
        let mut first = Vec::new();
        for round in 0..2 {
            if round == 1 {
                app.insert_resource(saved.clone());
            }
            loading::change(&mut app, "Sky", &[1, 1, 1, -3, 1, 2]);
            let mut trace = Vec::new();
            for _ in 0..fps {
                update(&mut app, 1.0 / f64::from(fps));
                trace.push(phase(&app));
            }
            assert_eq!(phase(&app), [480, 16]);
            if round == 0 {
                first = trace;
            } else {
                assert_eq!(trace, first);
            }
        }
    }
}

#[test]
fn a_new_map_reinitializes_the_same_image_without_inheriting_its_previous_auto_phase() {
    let mut app = loading::loaded_without_tiles();
    loading::change(&mut app, "Sky", &[1, 1, 1, -1, 1, 1]);
    update(&mut app, 0.1);
    assert_eq!(phase(&app), [12, 244]);
    app.world_mut().resource_mut::<MapData>().map_id = 118;
    update(&mut app, 0.0);
    assert_eq!(phase(&app), [0, 256]);
}

#[test]
fn legacy_panorama_offsets_migrate_once_without_restarting_the_background() {
    let mut app = loading::loaded_without_tiles();
    let legacy = "(map_id:Some(94),definition:Some((name:\"Sky\",loop_x:true,loop_y:true,auto_x:false,auto_y:false,speed_x:0,speed_y:0)),scroll:(12.25,8.5))";
    let old = ron::from_str::<Panorama>(legacy).unwrap();
    assert!(old.motion.is_none());
    assert!(old.valid());
    app.insert_resource(old);
    update(&mut app, 0.0);
    assert_eq!(phase(&app), [20088, 15344]);
    assert_eq!(
        app.world()
            .resource::<Panorama>()
            .motion
            .as_ref()
            .unwrap()
            .offset(),
        Vec2::new(13.0, 1.0)
    );
    let current = app.world().resource::<Panorama>().clone();
    update(&mut app, 0.0);
    assert_eq!(*app.world().resource::<Panorama>(), current);
    let text = ron::to_string(&current).unwrap();
    assert_eq!(ron::from_str::<Panorama>(&text).unwrap(), current);
}

#[test]
fn invalid_legacy_background_offsets_are_rejected_before_they_can_reach_the_renderer() {
    let original = Panorama::default();
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for axis in 0..2 {
            let mut saved = original.clone();
            if axis == 0 {
                saved.scroll.0 = invalid;
            } else {
                saved.scroll.1 = invalid;
            }
            assert!(!saved.valid());
        }
    }
}
