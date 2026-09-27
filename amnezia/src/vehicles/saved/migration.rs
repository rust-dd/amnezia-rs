use super::*;
use crate::world::{MoveQueue, saved::hero::HeroState};

pub(crate) fn migrate_rider(
    version: u32,
    vehicles: &mut crate::vehicles::VehicleSave,
    motion: &mut Option<State>,
    hero: &mut Option<HeroState>,
) {
    if version >= 28 {
        return;
    }
    let Some(index) = vehicles.riding else { return };
    vehicles.boarding = false;
    vehicles.unboarding = false;
    vehicles.preboard_speed = hero.as_ref().map_or(4, |hero| hero.route.speed());
    let (queue, mut route) = if let Some(state) = motion {
        let vehicle = &mut state.motion[index];
        let saved = (vehicle.queue.clone(), vehicle.route.clone());
        vehicle.route.cancel_for_rider();
        saved
    } else {
        (MoveQueue::default().snapshot(), RouteStepper::default())
    };
    route.set_speed(vehicles.vehicles[index].speed);
    *hero = Some(HeroState {
        frame: hero.as_ref().map_or(1, |hero| hero.frame),
        motion: queue,
        route,
    });
}

#[cfg(test)]
mod tests;
