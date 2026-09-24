use super::*;

#[test]
fn every_original_event_page_produces_a_valid_character_snapshot() {
    let (mut app, _) = app("campaign-fields");
    let template = snapshot(app.world_mut()).remove(0);
    let mut maps = 0;
    let mut pages = 0;
    for entry in std::fs::read_dir(format!("{}/maps", asset_root())).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let map = load_ron::<Map>(path.to_str().unwrap());
        maps += 1;
        for event in &map.events {
            for (index, page) in event.pages.iter().enumerate() {
                let mut saved = template.clone();
                saved.character = EventSprite {
                    id: event.id,
                    tile_x: event.x as i32,
                    tile_y: event.y as i32,
                    dir: page.direction,
                    frame: page.pattern,
                    charset: page.graphic_name.clone(),
                    index: page.graphic_index,
                    layer: page.layer,
                };
                saved.route = RouteStepper::from_event_page(Some(page));
                saved.autonomy = AutoMove::new(
                    page.move_type,
                    page.move_frequency,
                    page.move_speed,
                    event.id,
                );
                saved.page = ron::from_str::<PageState>(&format!("(Some({index}))")).unwrap();
                assert!(
                    valid(&[saved], &map),
                    "{} event {} page {}",
                    path.display(),
                    event.id,
                    index
                );
                pages += 1;
            }
        }
    }
    assert_eq!(maps, 276);
    assert!(pages > 1000);
}

#[test]
fn invalid_motion_route_and_animation_values_are_rejected() {
    let (mut app, _) = app("numeric-fields");
    let mut base = snapshot(app.world_mut()).remove(0);
    base.route = RouteStepper::default();
    base.autonomy = AutoMove::new(1, 4, 4, base.character.id);
    let map = load_ron::<Map>(&format!("{}/maps/map_0003.ron", asset_root()));
    let original = ron::to_string(&base).unwrap();
    assert!(valid(&[base], &map));
    for (from, to) in [
        ("step_secs:0.13333334", "step_secs:0.0"),
        ("step_secs:0.13333334", "step_secs:NaN"),
        ("speed:4", "speed:0"),
        ("frequency:4", "frequency:9"),
        ("transparency:0", "transparency:8"),
        ("timer:0.0", "timer:inf"),
        ("facing_lock:None", "facing_lock:Some(4)"),
        ("direction:None", "direction:Some(8)"),
        ("count:0", "count:24"),
        ("fraction:0.0", "fraction:NaN"),
    ] {
        assert!(original.contains(from), "{from}: {original}");
        let broken = ron::from_str::<EventState>(&original.replace(from, to)).unwrap();
        assert!(!valid(&[broken], &map), "{from} -> {to}");
    }
}

#[test]
fn saved_npcs_accept_diagonal_movement_but_only_cardinal_sprite_faces() {
    let (mut app, _) = app("diagonal-fields");
    let mut base = snapshot(app.world_mut()).remove(0);
    base.route = RouteStepper::default();
    let original = ron::to_string(&base).unwrap();
    let map = load_ron::<Map>(&format!("{}/maps/map_0003.ron", asset_root()));
    for direction in 4..8 {
        let encoded = original.replace("direction:None", &format!("direction:Some({direction})"));
        let mut saved = ron::from_str::<EventState>(&encoded).unwrap();
        assert_eq!(saved.route.direction(&saved.character), direction);
        assert!(valid(&[saved.clone()], &map));
        saved.character.dir = direction;
        assert!(!valid(&[saved], &map));
    }
}

#[test]
fn off_map_event_locations_are_not_clamped_to_the_players_save_bounds() {
    let (mut app, path) = app("off-map");
    let entity = npc(app.world_mut());
    let mut event = app.world_mut().get_mut::<EventSprite>(entity).unwrap();
    event.tile_x = 2000;
    event.tile_y = -4;
    save_and_load(&mut app);
    std::fs::remove_file(path).unwrap();
    let entity = npc(app.world_mut());
    let event = app.world().get::<EventSprite>(entity).unwrap();
    assert_eq!((event.tile_x, event.tile_y), (2000, -4));
}
