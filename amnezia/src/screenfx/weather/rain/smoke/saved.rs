use super::*;

pub(crate) fn poison(world: &mut World) {
    world.resource_mut::<Scroll>().pan = Vec2::new(100.0, 110.0);
    world.resource_mut::<Rain>().drops.fill(model::Drop {
        x: -999,
        y: 999,
        life: 0,
    });
}

pub(crate) fn verify_fresh(world: &World) {
    let rain = world.resource::<Rain>();
    assert_eq!(rain.drops.len(), 100);
    assert_eq!(rain.fraction, 0.0);
    assert!(
        rain.drops
            .iter()
            .all(|drop| (0..320).contains(&drop.x) && (0..160).contains(&drop.y) && drop.life < 40)
    );
}

pub(crate) fn reset_reference(world: &mut World) {
    let position = world
        .resource::<crate::player::CameraPan>()
        .position
        .unwrap();
    world.resource_mut::<Checks>().camera = position;
}

pub(crate) fn verify_hidden(world: &mut World) {
    let visible = world
        .query_filtered::<&InheritedVisibility, With<Canvas>>()
        .single(world)
        .unwrap();
    assert!(!visible.get());
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().pixels.load(Ordering::SeqCst), 3);
}
