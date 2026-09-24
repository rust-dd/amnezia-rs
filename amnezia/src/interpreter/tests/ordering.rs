use super::*;
use crate::appearance::AppearancePlugin;
use crate::player::PlayerPlugin;
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

mod arrival;
mod autoruns;
mod decision;
mod facing;
mod immediate;
mod input;
mod npcs;
mod triggers;
mod unpause;

fn app() -> App {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        crate::gamedata::GameDataPlugin,
        PlayerPlugin,
        AppearancePlugin,
    ))
    .init_asset::<Image>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    crate::world::update::register(&mut app);
    let world = app.world_mut();
    let hero = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .entity_mut(hero)
        .insert((Sprite::default(), Transform::default()));
    app.update();
    app
}

fn hero_x(app: &mut App) -> i32 {
    let world = app.world_mut();
    world.query::<&Player>().single(world).unwrap().tile_x
}

#[test]
fn a_foreground_move_route_starts_on_the_following_character_update() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(0, vec![cmd(11330, 0, vec![10001, 8, 0, 0, 1])]);
    app.update();
    assert_eq!(hero_x(&mut app), 5);
    app.update();
    assert_eq!(hero_x(&mut app), 6);
}

#[test]
fn parallel_position_queries_precede_movement_and_foreground_queries_follow_it() {
    let mut app = app();
    let query = |var| cmd(10220, 0, vec![0, var, var, 0, 6, 10001, 1]);
    app.insert_resource(MapEvents {
        events: vec![map_event(1, 4, vec![query(1)])],
    });
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]);
    world
        .resource_mut::<RunningEvent>()
        .start(0, vec![query(2)]);
    app.update();
    let variables = app.world().resource::<Variables>();
    assert_eq!(variables.get(1), 5);
    assert_eq!(variables.get(2), 6);
}

#[test]
fn a_parallel_message_blocks_keyboard_movement_in_its_opening_update() {
    let mut app = app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![EventCommand {
                string: "Stop".into(),
                ..cmd(10110, 0, vec![])
            }],
        )],
    });
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert!(app.world().resource::<Dialogue>().active);
    assert_eq!(hero_x(&mut app), 5);
}

#[test]
fn field_steps_are_visible_to_foreground_work_in_the_movement_update() {
    #[derive(Resource, Default)]
    struct Observed(u64);
    let mut app = app();
    app.add_plugins(crate::conditions::ConditionsPlugin)
        .init_resource::<Observed>()
        .add_systems(
            Update,
            (|steps: Res<crate::conditions::FieldSteps>, mut seen: ResMut<Observed>| {
                seen.0 = steps.count;
            })
            .in_set(super::super::InterpreterStep),
        );
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowRight);
    app.update();
    assert_eq!(hero_x(&mut app), 6);
    assert_eq!(app.world().resource::<Observed>().0, 1);
}

#[test]
fn a_riders_current_tile_and_tween_are_available_before_foreground_queries() {
    let mut app = app();
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    {
        let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
        vehicles.set_location(0, 0, 5, 5);
        vehicles.save.riding = Some(0);
        vehicles.set_route(10002, RouteStepper::from_move_event(&[10002, 8, 0, 0, 1]));
    }
    app.world_mut().resource_mut::<RunningEvent>().start(
        0,
        vec![
            cmd(10220, 0, vec![0, 1, 1, 0, 6, 10001, 1]),
            cmd(10220, 0, vec![0, 2, 2, 0, 6, 10002, 1]),
        ],
    );
    app.update();
    let variables = app.world().resource::<Variables>();
    assert_eq!(variables.get(1), 6);
    assert_eq!(variables.get(2), 6);
    let world = app.world_mut();
    let (player, transform) = world
        .query::<(&Player, &Transform)>()
        .single(world)
        .unwrap();
    let destination = world
        .resource::<MapData>()
        .tile_center(player.tile_x, player.tile_y);
    assert!(
        transform.translation.x < destination.0,
        "the rider must retain the boat's sub-tile position"
    );
}

#[test]
fn successful_keyboard_movement_precedes_the_boarding_button() {
    let mut app = app();
    app.add_plugins(crate::vehicles::VehiclePlugin)
        .init_resource::<crate::audio::CurrentBgm>();
    app.world_mut()
        .resource_mut::<crate::vehicles::Vehicles>()
        .set_location(0, 0, 5, 6);
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::ArrowRight);
    keys.press(KeyCode::Enter);
    app.update();
    assert_eq!(hero_x(&mut app), 6);
    assert!(!app.world().resource::<crate::vehicles::Vehicles>().riding());
}

#[test]
fn a_pending_save_scene_holds_forced_movement_before_its_opening_fade() {
    let mut app = app();
    let world = app.world_mut();
    *world
        .query::<&mut RouteStepper>()
        .single_mut(world)
        .unwrap() = RouteStepper::from_move_event(&[10001, 8, 0, 0, 1]);
    world.resource_mut::<crate::save::EventSaveRequest>().0 = true;
    app.update();
    assert_eq!(hero_x(&mut app), 5);
    app.world_mut()
        .resource_mut::<crate::save::EventSaveRequest>()
        .0 = false;
    app.update();
    assert_eq!(hero_x(&mut app), 6);
}

#[test]
fn actor_graphics_apply_in_script_order_without_replaying_the_parallel_refresh() {
    use amnezia_data::{MoveCommandDef, MoveRouteDef};
    for foreground in [false, true] {
        let mut app = app();
        let costume = |name: &str, index| EventCommand {
            string: name.into(),
            ..cmd(10630, 0, vec![1, index, 0])
        };
        app.insert_resource(MapEvents {
            events: vec![map_event(1, 4, vec![costume("Chara4", 3)])],
        });
        let world = app.world_mut();
        world
            .query::<&mut RouteStepper>()
            .single_mut(world)
            .unwrap()
            .force_route(RouteStepper::from_page(
                &MoveRouteDef {
                    commands: vec![
                        MoveCommandDef {
                            code: 34,
                            string: "Poses2".into(),
                            params: vec![4],
                        },
                        MoveCommandDef {
                            code: 23,
                            ..default()
                        },
                    ],
                    repeat: false,
                    skippable: false,
                },
                4,
                8,
            ));
        if foreground {
            world
                .resource_mut::<RunningEvent>()
                .start(0, vec![costume("Chara3", 1)]);
        }
        app.update();
        let world = app.world_mut();
        let hero = world.query::<&Player>().single(world).unwrap();
        assert_eq!(
            (hero.charset.as_str(), hero.index),
            if foreground {
                ("Chara3", 1)
            } else {
                ("Poses2", 4)
            }
        );
        assert_eq!(
            world.resource::<crate::appearance::Appearance>().get(1),
            if foreground {
                Some(("Chara3", 1))
            } else {
                Some(("Chara4", 3))
            }
        );
    }
}
