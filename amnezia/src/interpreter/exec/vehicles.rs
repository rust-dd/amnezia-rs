use super::{Exec, Flow};
use crate::interpreter::frame::Frame;
use amnezia_data::EventCommand;

pub(super) fn locate(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    frame.ip += 1;
    let [index, mode, map, tx, ty, ..] = command.params.as_slice() else {
        return Flow::Advance;
    };
    if !(0..3).contains(index) {
        return Flow::Advance;
    }
    let value = |n: i32| {
        if *mode == 1 {
            x.variables.get(n as u32)
        } else {
            n
        }
        .max(0) as u32
    };
    let (map, tx, ty) = (value(*map), value(*tx), value(*ty));
    let vehicles = &mut x.subsystems.mapfx.vehicles;
    vehicles.set_location(*index as usize, map, tx, ty);
    if vehicles.save.riding == Some(*index as usize) {
        if x.subsystems
            .flow
            .map_data
            .as_ref()
            .is_some_and(|data| data.map_id != map)
        {
            if frame.parallel && frame.base_event_id() != 0 {
                bevy::log::warn!("Quick vehicle transfer is not valid in a parallel map event");
                return Flow::Stop;
            }
            x.pending.quick(map, tx, ty);
            return Flow::Advance;
        }
        vehicles.relocate_pending = Some((tx, ty));
    }
    Flow::Advance
}

pub(super) fn toggle(frame: &mut Frame, x: &mut Exec) -> Flow {
    frame.ip += 1;
    x.subsystems.mapfx.vehicles.toggle_pending = true;
    Flow::Advance
}
