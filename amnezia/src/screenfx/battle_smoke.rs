use super::*;

const TONE: [f32; 4] = [75.0, 100.0, 125.0, 100.0];

pub(crate) fn map_exit(world: &World) -> Vec2 {
    let fx = world.resource::<Fx>();
    assert!(fx.flash.is_some());
    assert_eq!(world.resource::<TintState>().tone(), TONE);
    fx.shake_offset
}

pub(crate) fn battle_entry(world: &World, shake: Vec2) {
    let fx = world.resource::<Fx>();
    assert!(
        fx.flash.is_none(),
        "map flash must end at battle scene entry"
    );
    assert_eq!(fx.shake_offset, shake);
    assert_eq!(world.resource::<TintState>().tone(), TONE);
    info!("battle entry: map flash cleared, tone and paused shake preserved");
}

pub(crate) fn map_return(world: &World) {
    assert!(world.resource::<Fx>().flash.is_none());
    assert_eq!(world.resource::<TintState>().tone(), TONE);
    info!("battle return: cancelled map flash did not resume");
}
