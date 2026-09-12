use super::*;
use crate::world::{Character, MapChanged, MapData, MapRebuilt, MoveQueue, RouteAction};

pub(super) fn map_app(fps: u32) -> (App, Entity, Entity) {
    let mut app = app(fps);
    app.add_message::<ShowMapAnimation>()
        .add_message::<MapChanged>()
        .add_systems(
            Update,
            (playback::clear_map_animations, resolve_map_animation)
                .chain()
                .before(start_animations),
        )
        .add_systems(
            PostUpdate,
            (playback::follow_map_animations, track_active_animations).chain(),
        );
    let camera = app
        .world_mut()
        .spawn((MainCamera, Transform::default()))
        .id();
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 5,
                tile_y: 5,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            Transform::from_xyz(32.0, 24.0, 0.0),
        ))
        .id();
    (app, hero, camera)
}

fn play(app: &mut App, target: AnimTarget, global: bool) {
    app.world_mut().write_message(ShowMapAnimation {
        anim_id: 1,
        target,
        global,
    });
    app.update();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
}

fn positions(app: &mut App) -> Vec<(Entity, Vec2)> {
    let mut values = app
        .world_mut()
        .query_filtered::<(Entity, &Transform), With<MeshMaterial2d<cells::CellMaterial>>>()
        .iter(app.world())
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();
    values.sort_by_key(|(entity, _)| *entity);
    values
}

#[test]
fn map_cells_follow_the_live_hero_and_camera_even_without_a_new_logical_frame() {
    for fps in [15, 30, 60, 144] {
        let (mut app, hero, camera) = map_app(fps);
        play(&mut app, AnimTarget::Hero, false);
        let initial = positions(&mut app);
        assert_eq!(initial[0].1, Vec2::new(32.0, 20.0));
        let frame = app.world().resource::<GameFrames>().frame;
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation += Vec3::new(16.25, -8.25, 0.0);
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation = Vec3::new(3.5, -2.5, 0.0);
        app.update();
        assert_eq!(app.world().resource::<GameFrames>().frame, frame);
        assert_eq!(positions(&mut app), [(initial[0].0, Vec2::new(45.0, 14.0))]);
    }
}

#[test]
fn event_head_center_and_feet_anchors_do_not_depend_on_charset_or_tile_graphics() {
    for charset in ["Chara1", ""] {
        for (position, expected_y) in [(0, 36.0), (1, 24.0), (2, 12.0)] {
            let (mut app, _, _) = map_app(60);
            app.world_mut().resource_mut::<AnimationLibrary>().0[0].position = position;
            let event = EventSprite {
                id: 7,
                tile_x: 3,
                tile_y: 4,
                dir: 2,
                frame: 1,
                charset: charset.into(),
                index: 0,
                layer: 1,
            };
            let offset = event.y_offset();
            let entity = app
                .world_mut()
                .spawn((event, Transform::from_xyz(-30.0, 20.0 + offset, 0.0)))
                .id();
            play(&mut app, AnimTarget::Event(7), false);
            assert_eq!(positions(&mut app)[0].1, Vec2::new(-30.0, expected_y));
            app.world_mut()
                .get_mut::<Transform>(entity)
                .unwrap()
                .translation += Vec3::new(16.0, -16.0, 0.0);
            app.update();
            assert_eq!(
                positions(&mut app)[0].1,
                Vec2::new(-14.0, expected_y - 16.0)
            );
        }
    }
}

#[test]
fn a_jumping_target_keeps_the_animation_on_its_interpolated_ground_position() {
    let (mut app, hero, _) = map_app(60);
    let data = MapData::for_test(20, 15);
    let mut queue = MoveQueue::default();
    queue.set_step_secs(1.0);
    queue.enqueue_route([RouteAction::Jump {
        dx: 2,
        dy: 0,
        face: 1,
    }]);
    let mut player = app.world_mut().get_mut::<Player>(hero).unwrap();
    queue.advance(&mut *player, &data, 0.0);
    queue.advance(&mut *player, &data, 0.5);
    let ground = queue.ground_position(&*player, &data);
    let elevated = queue.render_position(&*player, &data);
    assert!(elevated.y > ground.y);
    let position = elevated + Vec2::Y * player.y_offset();
    app.world_mut()
        .entity_mut(hero)
        .insert((queue, Transform::from_xyz(position.x, position.y, 0.0)));
    app.insert_resource(data);
    play(&mut app, AnimTarget::Hero, false);
    assert_eq!(positions(&mut app)[0].1, ground + Vec2::Y * 4.0);
}

#[test]
fn looped_map_targets_use_the_visible_copy_beside_the_camera() {
    let (mut app, hero, camera) = map_app(60);
    let mut map = MapData::for_test(140, 140);
    map.scroll_type = 3;
    app.insert_resource(map);
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(0, 0);
    app.world_mut()
        .entity_mut(hero)
        .insert(MoveQueue::default());
    app.world_mut()
        .get_mut::<Transform>(hero)
        .unwrap()
        .translation = Vec3::new(-1112.0, 1120.0, 0.0);
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation = Vec3::new(1120.0, -1120.0, 0.0);
    play(&mut app, AnimTarget::Hero, false);
    assert_eq!(positions(&mut app)[0].1, Vec2::new(8.0, -4.0));
}

#[test]
fn screen_scope_animations_stay_fixed_while_their_character_moves() {
    let (mut app, hero, camera) = map_app(60);
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].scope = 1;
    play(&mut app, AnimTarget::Hero, false);
    let initial = positions(&mut app);
    assert_eq!(initial.len(), 1);
    app.world_mut()
        .get_mut::<Transform>(hero)
        .unwrap()
        .translation
        .x += 50.0;
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation
        .y += 30.0;
    app.update();
    assert_eq!(positions(&mut app), initial);
}

#[test]
fn missing_targets_and_map_transfers_remove_map_cells() {
    for transfer in [false, true] {
        let (mut app, hero, _) = map_app(60);
        play(&mut app, AnimTarget::Hero, false);
        if transfer {
            app.world_mut().write_message(MapRebuilt);
        } else {
            app.world_mut().despawn(hero);
        }
        app.update();
        assert_eq!(app.world().resource::<ActiveAnimations>().total, 0);
        assert!(positions(&mut app).is_empty());
    }
}

#[test]
fn requesting_a_missing_character_cancels_the_map_slot() {
    let (mut app, _, _) = map_app(60);
    play(&mut app, AnimTarget::Event(999), false);
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 0);
}

#[test]
fn the_last_valid_id_controls_target_validation_even_in_one_update() {
    for id in [1, u32::MAX] {
        let (mut app, _, _) = map_app(60);
        app.world_mut().write_message(ShowMapAnimation {
            anim_id: 1,
            target: AnimTarget::Hero,
            global: false,
        });
        app.world_mut().write_message(ShowMapAnimation {
            anim_id: id,
            target: AnimTarget::Event(999),
            global: false,
        });
        app.update();
        assert_eq!(
            app.world().resource::<ActiveAnimations>().total,
            usize::from(id != 1)
        );
    }
}

#[test]
fn reloading_a_session_removes_live_animations_and_pending_target_flashes() {
    let (mut app, _, _) = map_app(60);
    play(&mut app, AnimTarget::Hero, false);
    app.world_mut().write_message(BattlerFlash {
        pos: Vec2::ZERO,
        rgb: [248; 3],
        power: 31,
        age: 0,
    });
    crate::session::clear_transient(app.world_mut());
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 0);
    assert!(positions(&mut app).is_empty());
    assert_eq!(
        app.world_mut()
            .query::<&LiveAnimation>()
            .iter(app.world())
            .count(),
        0
    );
    let messages = app.world().resource::<Messages<BattlerFlash>>();
    assert_eq!(messages.get_cursor().read(messages).count(), 0);
}
