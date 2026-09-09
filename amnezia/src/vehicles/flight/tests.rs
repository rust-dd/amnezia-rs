use super::*;
use crate::tiles::DIR_RIGHT;
use crate::vehicles::VehicleSave;
use bevy::prelude::Vec2;

fn airborne() -> Vehicles {
    let mut vehicles = Vehicles::default();
    vehicles.set_location(2, 0, 4, 4);
    vehicles.save.riding = Some(2);
    vehicles
}

#[test]
fn boarding_and_landing_take_32_ticks_with_original_integer_altitude() {
    let data = MapData::for_test(10, 10);
    let mut vehicles = airborne();
    vehicles.save.riding = None;
    assert!(vehicles.toggle(&data, (4, 4, DIR_RIGHT), |_, _| false));
    for tick in 0..32 {
        assert_eq!(vehicles.airship_altitude(), (tick / 2) as f32);
        assert!(!vehicles.toggle(&data, (4, 4, DIR_RIGHT), |_, _| false));
        assert!(!vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false));
    }
    assert_eq!(vehicles.airship_altitude(), 16.0);
    assert!(!vehicles.airship_transitioning());
    let ground = Vec2::from(data.tile_center(4, 4));
    assert_eq!(vehicles.pixel(10004, &data), Some(ground + Vec2::Y * 16.0));
    assert!(vehicles.toggle(&data, (4, 4, DIR_RIGHT), |_, _| false));
    for tick in 0..32 {
        assert_eq!(vehicles.airship_altitude(), ((32 - tick) / 2) as f32);
        assert_eq!(
            vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false),
            tick == 31
        );
    }
    assert!(!vehicles.riding());
    assert_eq!(vehicles.airship_altitude(), 0.0);
    assert_eq!(vehicles.disembark, Some((4, 4, DIR_DOWN)));
    assert_eq!(vehicles.save.vehicles[2].dir, DIR_LEFT);
}

#[test]
fn blocked_landings_descend_then_reascend_and_recheck_only_at_the_end() {
    let data = MapData::for_test(10, 10);
    for blocked_at_end in [false, true] {
        let mut vehicles = airborne();
        assert!(vehicles.toggle(&data, (4, 4, DIR_RIGHT), |_, _| true));
        for _ in 0..31 {
            assert!(!vehicles.advance_flight(1.0 / 60.0, &data, |_, _| panic!("too early")));
        }
        assert_eq!(
            vehicles.advance_flight(1.0 / 60.0, &data, |_, _| blocked_at_end),
            !blocked_at_end
        );
        assert_eq!(vehicles.riding(), blocked_at_end);
        if blocked_at_end {
            assert_eq!(vehicles.airship_altitude(), 0.0);
            assert!(vehicles.airship_transitioning());
            for _ in 0..32 {
                vehicles.advance_flight(1.0 / 60.0, &data, |_, _| true);
            }
            assert_eq!(vehicles.airship_altitude(), 16.0);
            assert!(!vehicles.airship_transitioning());
        }
    }
}

#[test]
fn ascent_duration_is_independent_of_render_frame_rate() {
    let data = MapData::for_test(10, 10);
    for fps in [15_u32, 30, 60, 120, 144] {
        let mut vehicles = airborne();
        vehicles.save.airship_flight.ascend();
        let frames = (32 * fps).div_ceil(60);
        for frame in 0..frames {
            assert!(vehicles.airship_transitioning(), "fps {fps}, frame {frame}");
            vehicles.advance_flight(1.0 / fps as f32, &data, |_, _| false);
        }
        assert!(!vehicles.airship_transitioning(), "fps {fps}");
        assert_eq!(vehicles.airship_altitude(), 16.0);
    }
}

#[test]
fn flight_progress_survives_save_and_old_mounted_saves_resume_at_cruising_height() {
    let data = MapData::for_test(10, 10);
    let mut vehicles = airborne();
    vehicles.save.airship_flight.ascend();
    for _ in 0..15 {
        vehicles.advance_flight(1.0 / 60.0, &data, |_, _| false);
    }
    let mut restored = Vehicles::default();
    restored.restore(ron::from_str(&ron::to_string(&vehicles.save).unwrap()).unwrap());
    assert_eq!(restored.save, vehicles.save);
    assert_eq!(restored.airship_altitude(), 7.0);
    for _ in 0..17 {
        restored.advance_flight(1.0 / 60.0, &data, |_, _| false);
    }
    assert!(!restored.airship_transitioning());
    let old = format!(
        "(vehicles:{},riding:Some(2),before_music:None)",
        ron::to_string(&vehicles.save.vehicles).unwrap()
    );
    restored.restore(ron::from_str::<VehicleSave>(&old).unwrap());
    assert_eq!(restored.airship_altitude(), 16.0);
    assert!(!restored.airship_transitioning());
}
