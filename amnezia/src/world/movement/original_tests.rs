use super::*;
use crate::world::test_support::*;
use crate::world::{EventSprite, RouteStepper};

#[test]
fn jumps_subtract_integer_subpixels_and_rasterize_each_original_frame_at_every_speed() {
    for (speed, amount) in [(1, 8), (2, 12), (3, 16), (4, 24), (5, 32), (6, 64)] {
        let mut jumping = page(vec![
            command(24, 0),
            command(1, 0),
            command(1, 0),
            command(25, 0),
        ]);
        jumping.move_speed = speed;
        let mut app = app(vec![event(1, 1, vec![jumping])], false);
        let npc = entity(&mut app, 1);
        let mut remaining = 256_i32;
        while remaining > 0 {
            app.update();
            remaining = (remaining - amount).max(0);
            let character = app.world().get::<EventSprite>(npc).unwrap();
            let queue = app.world().get::<MoveQueue>(npc).unwrap();
            let data = app.world().resource::<MapData>();
            let ground = Vec2::from(data.tile_center(0, 1))
                + Vec2::X * ((3 * 256 - 2 * remaining) / 16) as f32;
            let height = remaining.min(256 - remaining) / 8;
            let height = if height < 5 {
                height * 2
            } else if height < 13 {
                height + 4
            } else {
                16
            };
            assert_eq!(
                queue.ground_position(character, data),
                ground,
                "speed {speed}, remaining {remaining}"
            );
            assert_eq!(
                queue.render_position(character, data),
                ground + Vec2::Y * height as f32
            );
            assert_eq!(queue.busy(), remaining != 0);
        }
    }
}

#[test]
fn slow_walking_keeps_whole_pixel_positions_between_subpixel_updates() {
    let mut walking = page(vec![command(1, 0)]);
    walking.move_speed = 1;
    let mut app = app(vec![event(1, 1, vec![walking])], false);
    let npc = entity(&mut app, 1);
    for frame in 1..=8 {
        app.update();
        let character = app.world().get::<EventSprite>(npc).unwrap();
        let queue = app.world().get::<MoveQueue>(npc).unwrap();
        let data = app.world().resource::<MapData>();
        assert_eq!(
            queue.render_position(character, data).x - data.tile_center(1, 1).0,
            (frame / 4) as f32
        );
    }
}

#[test]
fn an_ongoing_walk_uses_the_live_direction_without_changing_its_destination_or_remaining_step() {
    let mut app = app(vec![event(1, 1, vec![page(vec![command(1, 0)])])], false);
    let npc = entity(&mut app, 1);
    app.update();
    let world = app.world_mut();
    let (mut character, mut route) = world
        .query::<(&mut EventSprite, &mut RouteStepper)>()
        .get_mut(world, npc)
        .unwrap();
    route.set_direction(&mut *character, DIR_LEFT);
    app.update();
    let character = app.world().get::<EventSprite>(npc).unwrap();
    let queue = app.world().get::<MoveQueue>(npc).unwrap();
    let data = app.world().resource::<MapData>();
    assert_eq!(character.tile(), (2, 1));
    assert_eq!(
        queue.render_position(character, data).x,
        data.tile_center(2, 1).0 + 12.0
    );
}

#[test]
fn page_speed_changes_affect_the_current_step_without_restarting_it() {
    let original = page(vec![command(1, 0)]);
    let mut faster = gated(original.clone());
    faster.move_speed = 6;
    let mut app = app(vec![event(1, 1, vec![original, faster])], false);
    let npc = entity(&mut app, 1);
    app.update();
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(7, true);
    app.update();
    let character = app.world().get::<EventSprite>(npc).unwrap();
    let queue = app.world().get::<MoveQueue>(npc).unwrap();
    let data = app.world().resource::<MapData>();
    assert_eq!(
        queue.render_position(character, data).x - data.tile_center(1, 1).0,
        10.0
    );
}

#[test]
fn hero_and_vehicle_jumps_use_the_same_subpixel_clock() {
    let mut app = app(vec![], false);
    app.init_resource::<crate::audio::CurrentBgm>()
        .add_plugins(crate::vehicles::VehiclePlugin);
    let world = app.world_mut();
    world
        .query_filtered::<&mut RouteStepper, With<crate::player::Player>>()
        .single_mut(world)
        .unwrap()
        .force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 1, 25]));
    let mut vehicles = world.resource_mut::<crate::vehicles::Vehicles>();
    for index in 0..3 {
        vehicles.set_location(index, 0, 2 + index as u32, 3);
        vehicles.save.vehicles[index].speed = 4;
        vehicles.set_route(
            10002 + index as i32,
            RouteStepper::from_move_event(&[10002 + index as i32, 8, 0, 0, 24, 1, 25]),
        );
    }
    app.update();
    let world = app.world_mut();
    let (hero, queue) = world
        .query::<(&crate::player::Player, &MoveQueue)>()
        .single(world)
        .unwrap();
    let data = world.resource::<MapData>();
    assert_eq!(
        queue.render_position(hero, data),
        Vec2::from(data.tile_center(5, 5)) + Vec2::new(1.0, 6.0)
    );
    for index in 0..3 {
        assert_eq!(
            world
                .resource::<crate::vehicles::Vehicles>()
                .pixel(10002 + index, data)
                .unwrap(),
            Vec2::from(data.tile_center(2 + index, 3)) + Vec2::new(1.0, 6.0)
        );
    }
}
