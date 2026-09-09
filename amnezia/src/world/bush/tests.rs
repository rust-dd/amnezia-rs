use super::*;
use crate::world::RouteAction;

fn forest() -> MapData {
    let mut data = MapData::for_test(10, 10);
    data.terrains[0].bush_depth = 1;
    data
}

fn player() -> Player {
    Player {
        tile_x: 4,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    }
}

#[test]
fn bush_depth_uses_integer_sprite_thirds_and_ignores_jump_flight_and_other_layers() {
    let mut data = forest();
    let mut visual = Visual {
        source: Rect::new(0.0, 0.0, 24.0, 32.0),
        tile: (4, 4),
        layer: 1,
        jumping: false,
        flying: false,
    };
    for (mode, expected) in [(0, 0), (1, 10), (2, 16), (3, 32)] {
        data.terrains[0].bush_depth = mode;
        assert_eq!(depth(&data, &visual), expected);
    }
    data.terrains[0].bush_depth = 1;
    for layer in [0, 2] {
        visual.layer = layer;
        assert_eq!(depth(&data, &visual), 0);
    }
    visual.layer = 1;
    visual.jumping = true;
    assert_eq!(depth(&data, &visual), 0);
    visual.jumping = false;
    visual.flying = true;
    assert_eq!(depth(&data, &visual), 0);
    visual.flying = false;
    visual.source = Rect::new(0.0, 0.0, 16.0, 16.0);
    assert_eq!(depth(&data, &visual), 5);
    visual.tile = (-1, 4);
    assert_eq!(depth(&data, &visual), 0);
}

#[test]
fn split_preserves_pixel_positions_and_original_rounded_bottom_opacity() {
    let full = Rect::new(24.0, 64.0, 48.0, 96.0);
    for (level, opacity) in [128, 112, 96, 80, 64, 48, 32, 16].into_iter().enumerate() {
        let mut sprite = Sprite {
            color: Color::WHITE.with_alpha(tiles::character_alpha(level as u8)),
            ..default()
        };
        let mut anchor = Anchor::CENTER;
        let (bottom, transform) = split(&mut sprite, &mut anchor, full, 10).unwrap();
        assert_eq!(sprite.rect, Some(Rect::new(24.0, 64.0, 48.0, 86.0)));
        assert_eq!(bottom.rect, Some(Rect::new(24.0, 86.0, 48.0, 96.0)));
        assert_eq!(sprite.custom_size, Some(Vec2::new(24.0, 22.0)));
        assert_eq!(bottom.custom_size, Some(Vec2::new(24.0, 10.0)));
        assert!((bottom.color.alpha() * 255.0 - opacity as f32).abs() < 1e-5);
        assert!((-anchor.as_vec().y * 22.0 - 5.0).abs() < 1e-5);
        assert_eq!(transform.translation, Vec3::new(0.0, -11.0, 0.0));
        assert!(split(&mut sprite, &mut anchor, full, 0).is_none());
        assert_eq!(sprite.rect, Some(full));
        assert_eq!(sprite.custom_size, Some(Vec2::new(24.0, 32.0)));
        assert_eq!(anchor, Anchor::CENTER);
    }
}

#[test]
fn full_bush_mode_has_no_division_by_zero_or_double_draw() {
    let mut sprite = Sprite::default();
    let mut anchor = Anchor::CENTER;
    let full = Rect::new(0.0, 0.0, 16.0, 16.0);
    let (bottom, transform) = split(&mut sprite, &mut anchor, full, 16).unwrap();
    assert_eq!(sprite.custom_size.unwrap().y, 0.0);
    assert_eq!(anchor, Anchor::CENTER);
    assert_eq!(bottom.rect, Some(full));
    assert_eq!(transform.translation, Vec3::ZERO);
}

#[test]
fn mounted_airship_disables_bush_for_both_rider_and_vehicle() {
    let data = forest();
    let mut vehicles = Vehicles::default();
    vehicles.set_location(2, 0, 4, 4);
    let hero = player();
    let vehicle = VehicleSprite(2);
    assert_eq!(
        depth(
            &data,
            &visual(Some(&hero), None, None, None, Some(&vehicles)).unwrap()
        ),
        10
    );
    assert_eq!(
        depth(
            &data,
            &visual(None, None, Some(&vehicle), None, Some(&vehicles)).unwrap()
        ),
        10
    );
    vehicles.save.riding = Some(2);
    assert_eq!(
        depth(
            &data,
            &visual(Some(&hero), None, None, None, Some(&vehicles)).unwrap()
        ),
        0
    );
    assert_eq!(
        depth(
            &data,
            &visual(None, None, Some(&vehicle), None, Some(&vehicles)).unwrap()
        ),
        0
    );
}

#[test]
fn bevy_bush_parts_follow_animation_restore_after_jump_and_despawn_with_the_character() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin))
        .insert_resource(forest())
        .add_systems(
            PostUpdate,
            update.before(bevy::transform::TransformSystems::Propagate),
        );
    let entity = app
        .world_mut()
        .spawn((
            player(),
            Sprite::default(),
            MoveQueue::default(),
            Transform::from_xyz(80.0, 80.0, 10.0),
        ))
        .id();
    app.update();
    let child = app.world().get::<BushChild>(entity).unwrap().0;
    assert_eq!(app.world().get::<ChildOf>(child).unwrap().parent(), entity);
    assert_eq!(
        app.world()
            .get::<GlobalTransform>(child)
            .unwrap()
            .translation(),
        Vec3::new(80.0, 69.0, 10.0)
    );
    assert_eq!(
        app.world()
            .get::<Sprite>(entity)
            .unwrap()
            .rect
            .unwrap()
            .max
            .y,
        86.0
    );
    app.world_mut().get_mut::<Player>(entity).unwrap().frame = 2;
    app.update();
    assert_eq!(app.world().get::<BushChild>(entity).unwrap().0, child);
    assert_eq!(
        app.world()
            .get::<Sprite>(child)
            .unwrap()
            .rect
            .unwrap()
            .min
            .x,
        48.0
    );
    let mut jumping = player();
    let mut queue = MoveQueue::default();
    queue.enqueue_route([RouteAction::Jump {
        dx: 0,
        dy: 0,
        face: 2,
    }]);
    queue.advance(&mut jumping, app.world().resource::<MapData>(), 0.0);
    app.world_mut().entity_mut(entity).insert(queue);
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(child),
        Some(&Visibility::Hidden)
    );
    assert_eq!(
        app.world().get::<Sprite>(entity).unwrap().custom_size,
        Some(Vec2::new(24.0, 32.0))
    );
    assert_eq!(app.world().get::<Anchor>(entity), Some(&Anchor::CENTER));
    app.world_mut()
        .entity_mut(entity)
        .insert(MoveQueue::default());
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(child),
        Some(&Visibility::Inherited)
    );
    app.world_mut().resource_mut::<MapData>().terrains[0].bush_depth = 0;
    app.update();
    assert_eq!(
        app.world().get::<Visibility>(child),
        Some(&Visibility::Hidden)
    );
    app.world_mut().despawn(entity);
    assert!(app.world().get_entity(child).is_err());
}
