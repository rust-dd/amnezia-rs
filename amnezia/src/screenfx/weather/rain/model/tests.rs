use super::*;

fn rain() -> Rain {
    Rain::new(EventRng::seeded(0x1234_5678))
}

#[test]
fn startup_initializes_all_hundred_drops_in_the_original_ranges() {
    let rain = rain();
    assert_eq!(rain.drops.len(), 100);
    assert!(rain.drops.iter().any(|drop| drop.life > 12));
    for drop in rain.drops {
        assert!((0..40).contains(&drop.life));
        assert!((0..320).contains(&drop.x));
        assert!((0..160).contains(&drop.y));
    }
}

#[test]
fn a_live_drop_moves_one_left_and_four_down_without_wrapping() {
    let mut rain = rain();
    rain.drops[0] = Drop {
        x: 0,
        y: 159,
        life: 12,
    };
    for tick in 1..=12 {
        rain.advance(1.0 / 60.0);
        assert_eq!(
            rain.drops[0],
            Drop {
                x: -tick,
                y: 159 + tick * 4,
                life: 12 - tick as u8,
            }
        );
    }
}

#[test]
fn subframe_time_does_not_move_or_dim_a_drop() {
    let mut rain = rain();
    let initial = rain.drops.clone();
    rain.advance(1.0 / 144.0);
    rain.advance(1.0 / 144.0);
    assert_eq!(rain.drops, initial);
    rain.advance(1.0 / 144.0);
    assert_ne!(rain.drops, initial);
    assert!((rain.fraction - 0.25).abs() < 1e-10);
}

#[test]
fn logical_rain_matches_at_low_and_high_render_rates() {
    let mut expected = rain();
    for _ in 0..600 {
        expected.advance(1.0 / 60.0);
    }
    for fps in [15, 30, 60, 120, 144] {
        let mut actual = rain();
        for _ in 0..fps * 10 {
            actual.advance(1.0 / f64::from(fps));
        }
        assert_eq!(actual.drops, expected.drops, "{fps} FPS");
        assert!(actual.fraction < 1e-6);
    }
}

#[test]
fn dead_drops_wait_for_a_ten_percent_respawn_and_restart_at_twelve() {
    let mut rain = rain();
    let mut spawned = 0;
    for _ in 0..1000 {
        rain.drops.fill(Drop {
            x: -20,
            y: 250,
            life: 0,
        });
        rain.advance(1.0 / 60.0);
        for drop in &rain.drops {
            if drop.life == 0 {
                assert_eq!((drop.x, drop.y), (-20, 250));
            } else {
                spawned += 1;
                assert_eq!(drop.life, 12);
                assert!((0..320).contains(&drop.x));
                assert!((0..160).contains(&drop.y));
            }
        }
    }
    assert!((9500..10500).contains(&spawned), "{spawned} of 100000");
}
