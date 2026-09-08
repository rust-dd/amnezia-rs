use super::{Exec, Flow};
use crate::audio::AudioRequest;
use crate::interpreter::frame::Frame;
use crate::state::active_page;
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
    if vehicles.save.riding == Some(*index as usize)
        && x.subsystems
            .flow
            .map_data
            .as_ref()
            .is_some_and(|data| data.map_id != map)
    {
        x.pending.0 = Some((map, tx, ty));
        return Flow::Yield;
    }
    Flow::Advance
}

pub(super) fn toggle(frame: &mut Frame, x: &mut Exec) -> Flow {
    frame.ip += 1;
    let Some(data) = &x.subsystems.flow.map_data else {
        return Flow::Advance;
    };
    let Ok(hero) = x.subsystems.flow.players.single() else {
        return Flow::Advance;
    };
    let was_riding = x.subsystems.mapfx.vehicles.riding();
    let changed =
        x.subsystems
            .mapfx
            .vehicles
            .toggle(data, (hero.tile_x, hero.tile_y, hero.dir), |tx, ty| {
                x.subsystems.flow.map_events.as_ref().is_some_and(|events| {
                    events.events.iter().any(|event| {
                        event.x as i32 == tx
                            && event.y as i32 == ty
                            && active_page(event, &x.switches, &x.variables, &x.party, &x.inventory)
                                .is_some_and(|page| page.layer == 1)
                    })
                })
            });
    if changed {
        let vehicles = &mut x.subsystems.mapfx.vehicles;
        if was_riding {
            x.audio.write(
                vehicles
                    .save
                    .before_music
                    .as_ref()
                    .map_or(AudioRequest::StopBgm, |m| m.replay()),
            );
        } else {
            vehicles.save.before_music = x
                .subsystems
                .mapfx
                .current_bgm
                .as_ref()
                .and_then(|bgm| bgm.track());
            let system = crate::assets::load_ron::<amnezia_data::SystemDef>(&format!(
                "{}/system.ron",
                crate::assets::asset_root()
            ));
            let music = match vehicles.save.riding {
                Some(0) => system.boat_music,
                Some(1) => system.ship_music,
                _ => system.airship_music,
            };
            let slot = 3 + vehicles.save.riding.unwrap() as u32;
            x.audio.write(AudioRequest::from_music(
                x.subsystems.mapfx.system_bgm.get(slot, &music),
            ));
        }
    }
    Flow::Advance
}
