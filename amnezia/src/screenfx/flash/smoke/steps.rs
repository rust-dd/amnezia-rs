use super::*;
use crate::conditions::FieldSteps;
use crate::player::{CameraPan, Player};
use crate::vitals::Vitals;
use crate::world::{Character, MapData, MoveQueue, RouteStepper};

#[derive(Resource)]
struct Probe {
    origin: (i32, i32),
    checks: u8,
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    (frame == 780).then_some(KeyCode::ArrowRight)
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 780 {
        assert_eq!(world.resource::<MapData>().map_id, 98);
        assert!(!world.resource::<crate::teleport::Fade>().busy());
        assert!(
            !world
                .resource::<crate::interpreter::RunningEvent>()
                .active()
        );
        let (hero, queue, mut route) = world
            .query::<(&Player, &MoveQueue, &mut RouteStepper)>()
            .single_mut(world)
            .unwrap();
        assert!(!queue.busy());
        let origin = hero.tile();
        route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 36]));
        world.insert_resource(Probe { origin, checks: 0 });
        world.resource_mut::<CameraPan>().locked = true;
        world.resource_mut::<FieldSteps>().count = 3;
        let mut vitals = world.resource_mut::<Vitals>();
        vitals.set(1, 2, 37);
        vitals.set_states(1, vec![2]);
    }
    let label = match frame {
        781 => Some("screen-flash-poison-first"),
        783 => Some("screen-flash-poison-later"),
        786 => Some("screen-flash-poison-ended"),
        _ => None,
    };
    if label.is_some() || frame == 790 {
        assert_eq!(world.resource::<FieldSteps>().count, 4);
        assert_eq!(world.resource::<Vitals>().get_stored(1), Some((1, 37)));
        world.resource_mut::<Probe>().checks += 1;
    }
    if frame == 790 {
        let (hero, queue) = world
            .query::<(&Player, &MoveQueue)>()
            .single(world)
            .unwrap();
        let origin = world.resource::<Probe>().origin;
        assert_eq!(hero.tile(), (origin.0 + 1, origin.1));
        assert!(!queue.busy());
        assert!(world.resource::<Fx>().flash.is_none());
    }
    label
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().checks, 4);
    info!(
        "manual fourth step: nonlethal poison, six-frame flash and unchanged step count verified"
    );
}
