use super::*;
use crate::world::EventSprite;
use amnezia_data::{MoveCommandDef, MoveRouteDef};

#[derive(Resource, Default)]
struct RouteEffects(Vec<crate::world::StepEffect>);

fn drive_hero(
    data: Res<MapData>,
    mut hero: Query<(&mut Player, &mut MoveQueue, &mut RouteStepper)>,
    mut effects: ResMut<RouteEffects>,
) {
    let (mut player, mut queue, mut route) = hero.single_mut().unwrap();
    let driven = crate::world::drive_route(
        &mut *player,
        &mut queue,
        &mut route,
        (0, 0),
        1.0 / 60.0,
        |_, _, _, _, through| through,
    );
    effects.0.extend(driven.effects);
    queue.advance(&mut *player, &data, 1.0 / 60.0);
}

fn spawn_npc(app: &mut App, route: RouteStepper) -> Entity {
    app.world_mut()
        .spawn((
            EventSprite {
                id: 10,
                tile_x: 3,
                tile_y: 4,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
                layer: 1,
            },
            MoveQueue::default(),
            route,
        ))
        .id()
}

#[test]
fn proceed_with_movement_ignores_an_npcs_repeating_page_route() {
    let mut app = interp_app();
    let npc = spawn_npc(
        &mut app,
        RouteStepper::from_page(
            &MoveRouteDef {
                commands: vec![MoveCommandDef {
                    code: 1,
                    ..default()
                }],
                repeat: true,
                skippable: false,
            },
            3,
            3,
        ),
    );
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11340, 0, vec![]), switch_cmd(77, 0, 0)]);
    app.update();
    app.update();
    assert!(switch_on(&app, 77));
    assert!(app.world().get::<RouteStepper>(npc).unwrap().active());
}

#[test]
fn proceed_with_movement_still_waits_for_a_forced_route() {
    let mut app = interp_app();
    let npc = spawn_npc(&mut app, RouteStepper::from_move_event(&[10, 8, 0, 0, 1]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(1, vec![cmd(11340, 0, vec![]), switch_cmd(77, 0, 0)]);
    app.update();
    app.update();
    assert!(!switch_on(&app, 77));
    *app.world_mut().get_mut::<RouteStepper>(npc).unwrap() = RouteStepper::default();
    app.update();
    assert!(switch_on(&app, 77));
}

#[test]
fn consecutive_move_events_keep_through_facing_lock_and_transparency() {
    let mut app = interp_app();
    app.init_resource::<RouteEffects>()
        .add_systems(PostUpdate, drive_hero);
    app.world_mut().resource_mut::<RunningEvent>().start(
        1,
        vec![
            cmd(11330, 0, vec![10001, 8, 0, 0, 36, 26, 40, 29]),
            cmd(11340, 0, vec![]),
            cmd(11330, 0, vec![10001, 8, 0, 0, 1, 40]),
            cmd(11340, 0, vec![]),
            switch_cmd(77, 0, 0),
        ],
    );
    for _ in 0..60 {
        app.update();
    }
    assert!(switch_on(&app, 77));
    let world = app.world_mut();
    let (player, route) = world
        .query::<(&Player, &RouteStepper)>()
        .single(world)
        .unwrap();
    assert_eq!((player.tile_x, player.tile_y, player.dir), (6, 5, 2));
    assert_eq!(route.speed(), 3);
    assert!(!route.forced());
    assert!(matches!(
        world.resource::<RouteEffects>().0.as_slice(),
        [
            crate::world::StepEffect::Transparency(1),
            crate::world::StepEffect::Transparency(2)
        ]
    ));
}
