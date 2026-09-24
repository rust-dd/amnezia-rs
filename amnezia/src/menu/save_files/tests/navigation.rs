use super::super::navigation::Navigation;

fn navigate(nav: &mut Navigation, action: usize, fresh: bool) -> u32 {
    let mut repeated = [false; 6];
    repeated[action] = true;
    nav.tick(repeated, [fresh && action == 0, fresh && action == 1], true)
}

#[test]
fn scrolling_uses_original_integer_interpolation_and_blocks_through_the_last_tick() {
    for (index, action, top, distance) in [(2, 0, 1, 64), (0, 1, 12, 768), (14, 0, 0, -768)] {
        let mut nav = Navigation::new(index);
        assert_eq!(navigate(&mut nav, action, true), 1);
        assert_eq!(nav.top, top);
        for frame in 1..=7 {
            assert!(nav.moving());
            assert_eq!(
                nav.offset(),
                distance - distance * frame / 7,
                "frame {frame}"
            );
            assert_eq!(nav.tick([false; 6], [false; 2], true), 0);
        }
        assert!(!nav.moving());
        assert_eq!(nav.offset(), 0);
    }
}

#[test]
fn repeated_vertical_arrows_stop_at_the_boundary_but_fresh_presses_wrap() {
    for (index, action, wrapped) in [(14, 0, 0), (0, 1, 14)] {
        let mut nav = Navigation::new(index);
        for _ in 0..300 {
            assert_eq!(navigate(&mut nav, action, false), 0);
            assert_eq!(nav.index, index);
        }
        assert!(!nav.moving());
        assert_eq!(navigate(&mut nav, action, true), 1);
        assert_eq!(nav.index, wrapped);
    }
}

#[test]
fn page_keys_move_three_slots_and_initial_view_contains_the_latest_slot() {
    for index in 0..15 {
        let nav = Navigation::new(index);
        assert_eq!(nav.top, index.saturating_sub(2));
    }
    let mut nav = Navigation::new(0);
    navigate(&mut nav, 4, true);
    assert_eq!((nav.index, nav.top), (3, 1));
    for _ in 0..8 {
        nav.tick([false; 6], [false; 2], true);
    }
    navigate(&mut nav, 5, true);
    assert_eq!((nav.index, nav.top), (0, 0));
}

#[test]
fn clocks_count_logical_frames_without_advancing_on_extra_render_frames() {
    for fps in [30, 60, 120, 144] {
        let mut clock = crate::timing::GameFrames::default();
        let mut nav = Navigation::new(0);
        for _ in 0..fps {
            let before = clock.frame;
            clock.advance(1.0 / fps as f64);
            let elapsed = clock.frame - before;
            for _ in 0..elapsed.max(1) {
                nav.tick([false; 6], [false; 2], elapsed > 0);
            }
        }
        assert_eq!((nav.arrow, nav.cursors[0]), (20, 19), "{fps} FPS");
    }
}
