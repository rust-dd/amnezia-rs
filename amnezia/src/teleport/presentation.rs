use bevy::prelude::*;

pub(super) fn flush(world: &mut World) {
    crate::picture::apply_pending(world);
    crate::screenfx::apply_pending(world);
    crate::animation::start::flush(world);
    crate::panorama::prepare(world);
    crate::dialogue::presentation::flush(world);
    crate::audio::flush(world);
}
