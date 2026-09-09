use super::*;
use crate::world::EventSprite;
use amnezia_data::{MoveCommandDef, MoveRouteDef};

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
