use super::*;
use crate::player::Player;
use crate::screenfx::ScreenEffect;
use crate::vehicles::Vehicles;
use crate::world::{Character, MapData, MoveQueue, RouteStepper};

fn fixture() -> App {
    let mut app = crate::world::test_support::app(vec![], false);
    app.add_plugins((
        crate::gamedata::GameDataPlugin,
        ConditionsPlugin,
        crate::vehicles::VehiclePlugin,
    ))
    .init_resource::<crate::audio::CurrentBgm>();
    app.update();
    app
}

fn hero(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap()
}

fn poison(app: &mut App) {
    app.world_mut().resource_mut::<FieldSteps>().count = 3;
    let mut vitals = app.world_mut().resource_mut::<Vitals>();
    vitals.set(1, 2, 5);
    vitals.set_states(1, vec![2]);
}

fn tick(app: &mut App, key: Option<KeyCode>) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
    app.update();
}

#[test]
fn scripted_walks_jumps_and_adjacent_relocations_do_not_count_as_manual_steps() {
    for kind in 0..3 {
        let mut app = fixture();
        let hero = hero(&mut app);
        poison(&mut app);
        if kind == 2 {
            app.world_mut().get_mut::<Player>(hero).unwrap().tile_x += 1;
        } else {
            let params = if kind == 0 {
                vec![10001, 8, 0, 0, 1]
            } else {
                vec![10001, 8, 0, 0, 24, 1, 25]
            };
            app.world_mut()
                .get_mut::<RouteStepper>(hero)
                .unwrap()
                .force_route(RouteStepper::from_move_event(&params));
        }
        tick(&mut app, None);
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
        assert_eq!(app.world().resource::<FieldSteps>().count, 3, "{kind}");
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 5)));
    }
}

#[test]
fn manual_steps_across_both_loop_seams_count_and_apply_poison() {
    for (x, key, destination) in [(9, KeyCode::ArrowRight, 0), (0, KeyCode::ArrowLeft, 9)] {
        let mut app = fixture();
        app.world_mut().resource_mut::<MapData>().scroll_type = 2;
        let hero = hero(&mut app);
        app.world_mut().get_mut::<Player>(hero).unwrap().tile_x = x;
        tick(&mut app, None);
        poison(&mut app);
        tick(&mut app, Some(key));
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile_x, destination);
        assert_eq!(app.world().resource::<FieldSteps>().count, 4);
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((1, 5)));
    }
}

#[test]
fn manual_vehicle_steps_apply_party_state_damage_for_every_vehicle_kind() {
    for index in 0..3 {
        let mut app = fixture();
        app.insert_resource(crate::world::test_support::water_map(10, 10));
        let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
        vehicles.set_location(index, 0, 5, 5);
        vehicles.save.riding = Some(index);
        tick(&mut app, None);
        poison(&mut app);
        tick(&mut app, Some(KeyCode::ArrowRight));
        assert_eq!(app.world().resource::<FieldSteps>().count, 4, "{index}");
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((1, 5)));
        let hero = hero(&mut app);
        assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
    }
}

#[test]
fn unboarding_does_not_become_a_manual_step_on_the_next_update() {
    let mut app = fixture();
    let mut vehicles = app.world_mut().resource_mut::<Vehicles>();
    vehicles.set_location(0, 0, 5, 5);
    vehicles.save.vehicles[0].dir = 1;
    vehicles.save.riding = Some(0);
    tick(&mut app, None);
    poison(&mut app);
    tick(&mut app, Some(KeyCode::Enter));
    tick(&mut app, None);
    assert!(!app.world().resource::<Vehicles>().riding());
    let hero = hero(&mut app);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (6, 5));
    assert_eq!(app.world().resource::<FieldSteps>().count, 3);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 5)));
}

#[test]
fn a_poison_tick_flashes_once_including_at_one_hp_without_touching_reserves_or_the_dead() {
    let mut app = fixture();
    let mut party = app.world_mut().resource_mut::<Party>();
    party.add(2);
    party.add(4);
    app.world_mut().resource_mut::<FieldSteps>().count = 3;
    let mut vitals = app.world_mut().resource_mut::<Vitals>();
    for (id, hp) in [(1, 1), (2, 30), (3, 10), (4, 0)] {
        vitals.set(id, hp, 5);
        if hp > 0 {
            vitals.set_states(id, vec![2]);
        }
    }
    tick(&mut app, Some(KeyCode::ArrowRight));
    let vitals = app.world().resource::<Vitals>();
    for (id, hp) in [(1, 1), (2, 29), (3, 10), (4, 0)] {
        assert_eq!(vitals.get_stored(id), Some((hp, 5)));
    }
    assert_eq!(
        app.world_mut()
            .resource_mut::<Messages<ScreenEffect>>()
            .drain()
            .collect::<Vec<_>>(),
        vec![ScreenEffect::Flash {
            r: 31,
            g: 10,
            b: 10,
            intensity: 20,
            secs: 0.1,
        }]
    );
    let hero = hero(&mut app);
    assert!(app.world().get::<MoveQueue>(hero).unwrap().busy());
}

#[test]
fn blocked_steps_idle_updates_and_boarding_do_not_apply_poison() {
    for kind in 0..3 {
        let mut app = fixture();
        let hero = hero(&mut app);
        poison(&mut app);
        let key = match kind {
            0 => {
                app.world_mut().get_mut::<Player>(hero).unwrap().tile_x = 9;
                Some(KeyCode::ArrowRight)
            }
            1 => None,
            _ => {
                app.world_mut()
                    .resource_mut::<Vehicles>()
                    .set_location(2, 0, 5, 5);
                Some(KeyCode::Enter)
            }
        };
        tick(&mut app, key);
        assert_eq!(app.world().resource::<FieldSteps>().count, 3, "{kind}");
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 5)));
        assert!(app.world().resource::<Messages<ScreenEffect>>().is_empty());
        assert_eq!(app.world().resource::<Vehicles>().riding(), kind == 2);
    }
}

#[derive(Resource, Default)]
struct BeforeMovement(Vec<Sample>);

#[derive(Debug, PartialEq)]
struct Sample {
    steps: u64,
    hp_sp: Option<(i32, i32)>,
    position: Vec2,
}

fn observe_before_movement(
    steps: Res<FieldSteps>,
    vitals: Res<Vitals>,
    heroes: Query<&Transform, With<Player>>,
    mut samples: ResMut<BeforeMovement>,
) {
    let transform = heroes.single().unwrap();
    samples.0.push(Sample {
        steps: steps.count,
        hp_sp: vitals.get_stored(1),
        position: transform.translation.truncate(),
    });
}

#[test]
fn step_side_effects_precede_the_first_movement_update() {
    let mut app = fixture();
    let hero = hero(&mut app);
    let origin = app.world().get::<Transform>(hero).unwrap().translation;
    poison(&mut app);
    app.init_resource::<BeforeMovement>().add_systems(
        crate::player::update::CharacterUpdate,
        observe_before_movement
            .after(step)
            .before(crate::player::PlayerStep),
    );
    tick(&mut app, Some(KeyCode::ArrowRight));
    assert_eq!(
        app.world().resource::<BeforeMovement>().0,
        vec![Sample {
            steps: 4,
            hp_sp: Some((1, 5)),
            position: origin.truncate(),
        }]
    );
    assert_eq!(
        app.world().get::<Transform>(hero).unwrap().translation,
        origin + Vec3::X * 2.0
    );
}

#[test]
fn healthy_steps_do_not_materialize_an_implicit_full_health_pool() {
    let mut app = fixture();
    tick(&mut app, Some(KeyCode::ArrowRight));
    assert_eq!(app.world().resource::<FieldSteps>().count, 1);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), None);
    assert!(app.world().resource::<Messages<ScreenEffect>>().is_empty());
}
