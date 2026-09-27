use super::*;

#[derive(Resource)]
struct Trace {
    entity: Entity,
    cells: Vec<Entity>,
    frame: usize,
    held: u32,
    captured: bool,
    cleared: bool,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1010 {
        let (entity, animation) = world
            .query::<(Entity, &playback::LiveAnimation)>()
            .single(world)
            .unwrap();
        let trace = Trace {
            entity,
            cells: animation.cells.clone(),
            frame: animation.frame,
            held: 0,
            captured: false,
            cleared: false,
        };
        world.insert_resource(trace);
        world.resource_mut::<crate::teleport::PendingTeleport>().0 = Some((13, 61, 60));
    }
    if (1011..1110).contains(&frame) {
        let trace = world.resource::<Trace>();
        let animation = world.get::<playback::LiveAnimation>(trace.entity).unwrap();
        if world.resource::<crate::teleport::Fade>().busy() {
            assert_eq!(animation.frame, trace.frame);
            assert_eq!(animation.cells, trace.cells);
            for &cell in &trace.cells {
                assert!(world.get::<InheritedVisibility>(cell).unwrap().get());
            }
            world.resource_mut::<Trace>().held += 1;
        } else if !trace.captured && animation.frame == 10 {
            assert_eq!(world.resource::<MapData>().map_id, 13);
            assert!(trace.held > 60);
            assert_ne!(animation.cells, trace.cells);
            world.resource_mut::<Trace>().captured = true;
            return Some("map-animation-transfer-same");
        }
    }
    if frame == 1110 {
        assert!(world.resource::<Trace>().captured);
        assert_eq!(world.resource::<ActiveAnimations>().total, 1);
        world.resource_mut::<crate::teleport::PendingTeleport>().0 = Some((3, 15, 6));
    }
    if frame == 1200 {
        assert_eq!(world.resource::<MapData>().map_id, 3);
        assert!(!world.resource::<crate::teleport::Fade>().busy());
        assert_eq!(world.resource::<ActiveAnimations>().total, 0);
        assert!(
            world
                .get::<playback::LiveAnimation>(world.resource::<Trace>().entity)
                .is_none()
        );
        world.resource_mut::<Trace>().cleared = true;
        return Some("map-animation-transferred");
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    let trace = world.resource::<Trace>();
    assert!(trace.held > 60 && trace.captured && trace.cleared);
    info!(
        "map animation transfers: visible frozen fade, same-map retention and cross-map clearing verified"
    );
}
