use super::*;

#[test]
fn random_directions_rotate_the_logical_heading() {
    for direction in 0..8 {
        for (seed, expected) in [
            (8, direction),
            (7, (direction + 3) % 4),
            (5, (direction + 1) % 4),
            (3, (direction + 2) % 4),
        ] {
            let mut auto = AutoMove::new(1, 3, 4, 1);
            auto.rng = seed;
            assert_eq!(
                auto.decide(direction, (0, 0), true, 64),
                Decision::Move(expected)
            );
        }
    }
}

#[test]
fn cycles_choose_the_page_axis_but_retain_its_reverse_heading() {
    for (kind, primary) in [(2, DIR_DOWN), (3, DIR_RIGHT)] {
        for direction in 0..8 {
            let mut auto = AutoMove::new(kind, 3, 4, 1);
            let rng = auto.rng;
            let expected = if direction == reverse(primary) {
                direction
            } else {
                primary
            };
            assert_eq!(
                auto.decide(direction, (0, 0), false, 64),
                Decision::Cycle(expected)
            );
            assert_eq!(auto.rng, rng);
        }
    }
}

#[test]
fn targeted_seek_uses_the_dominant_axis_with_vertical_ties() {
    for (delta, direction) in [
        ((3, 2), DIR_RIGHT),
        ((-3, 2), DIR_LEFT),
        ((2, 3), DIR_DOWN),
        ((2, -3), DIR_UP),
        ((2, 2), DIR_DOWN),
        ((-2, -2), DIR_UP),
        ((0, 0), DIR_DOWN),
    ] {
        for kind in [4, 5] {
            let mut auto = AutoMove::new(kind, 3, 4, 1);
            auto.rng = 8;
            let expected = if kind == 4 {
                direction
            } else {
                reverse(direction)
            };
            assert_eq!(
                auto.decide(DIR_LEFT, delta, true, 64),
                Decision::Move(expected)
            );
        }
    }
}

#[test]
fn random_delay_uses_integer_rounding_and_the_live_frequency() {
    for (seed, multiplier) in [(4, 3), (1, 4), (2, 5), (3, 6)] {
        let mut auto = AutoMove::new(1, 1, 4, 1);
        auto.rng = seed;
        let mut route = RouteStepper::from_page(&amnezia_data::MoveRouteDef::default(), 4, 7);
        route.set_stop_count(97);
        auto.set_stop_maximum(&mut route);
        assert_eq!(route.stop_maximum(), 4 * multiplier / 5);
        assert_eq!(route.stop_count(), 97);
    }
}

#[test]
fn page_refresh_preserves_elapsed_time_and_the_random_stream() {
    let mut page = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .pages[0]
        .clone();
    page.move_type = 1;
    page.move_frequency = 3;
    let mut route = RouteStepper::from_event_page(Some(&page));
    let mut auto = AutoMove::new(1, 3, 4, 1);
    auto.rng = 1;
    auto.refresh(Some(&page), &mut route);
    assert_eq!((auto.rng, route.stop_maximum()), (270369, 51));
    route.set_stop_count(37);
    page.move_frequency = 7;
    route.refresh_page(Some(&page));
    auto.refresh(Some(&page), &mut route);
    assert_eq!(
        (auto.rng, route.stop_maximum(), route.stop_count()),
        (67634689, 3, 37)
    );
    route.refresh_page(None);
    auto.refresh(None, &mut route);
    assert_eq!(auto.rng, 67634689);
    assert_eq!(route.stop_count(), 37);
}
