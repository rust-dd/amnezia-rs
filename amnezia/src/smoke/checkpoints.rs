use super::{SmokeRun, capture::capture};
use bevy::prelude::*;

pub(super) fn drive(world: &mut World, frame: u32, scenario: &str) {
    let (label, ready, delay) = match scenario {
        "airship-free" => (
            super::free_flight::drive(world, frame),
            super::free_flight::ready(world),
            30,
        ),
        "airship-sky" => (
            super::sky_castle::drive(world, frame).map(str::to_owned),
            super::sky_castle::ready(world),
            30,
        ),
        "airship-return" => (
            super::return_trip::drive(world, frame).map(str::to_owned),
            super::return_trip::ready(world),
            30,
        ),
        "overlap" => (
            crate::world::overlap_smoke::drive(world, frame).map(str::to_owned),
            crate::world::overlap_smoke::ready(world),
            30,
        ),
        "terrain" => (
            crate::world::terrain_smoke::drive(world, frame),
            crate::world::terrain_smoke::ready(world),
            30,
        ),
        "airship-journey" => (
            super::journey::drive(world).map(str::to_owned),
            super::journey::ready(world),
            30,
        ),
        "map-passages" => (
            crate::world::passage_smoke::drive(world, frame).map(str::to_owned),
            crate::world::passage_smoke::ready(world),
            30,
        ),
        "map-scenes" => (
            crate::world::scene_smoke::drive(world, frame),
            crate::world::scene_smoke::ready(world),
            30,
        ),
        "crystals" => (
            crate::save::crystal_smoke::drive(world, frame),
            crate::save::crystal_smoke::ready(world, frame),
            1,
        ),
        _ => return,
    };
    if let Some(label) = label {
        capture(world, &label);
    }
    if ready && world.resource::<SmokeRun>().finish_at.is_none() {
        world.resource_mut::<SmokeRun>().finish_at = Some(frame + delay);
    }
}
