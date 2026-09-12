use super::*;
use amnezia_data::EventCommand;

#[derive(Resource, Default)]
struct Trace {
    shots: u8,
    ready_at: Option<u32>,
    finished: bool,
    shake: Option<Vec2>,
    screen_checks: u8,
}

pub(crate) fn entry() -> Vec<EventCommand> {
    [
        (10810, "", vec![3, 15, 12]),
        (11030, "", vec![75, 100, 125, 100, 0, 0]),
        (11040, "", vec![31, 5, 10, 31, 600, 0]),
        (11050, "", vec![3, 5, 600, 0]),
        (10710, "Cave1", vec![0, 2, 1, 0, 1, 0]),
        (10210, "", vec![0, 9997, 9997, 0]),
    ]
    .into_iter()
    .map(|(code, text, params)| EventCommand {
        code,
        indent: 0,
        string: text.into(),
        params,
    })
    .collect()
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 150 {
        world.insert_resource(Trace::default());
    }
    if frame < 150 {
        return None;
    }
    let age = world.resource::<crate::transitions::Transition>().age();
    let flow = world.resource::<BattleFlow>();
    let shot = match flow.stage {
        Stage::MapErase if (2..20).contains(&age) => Some((0, "battle-transition-flash")),
        Stage::MapErase if age >= 36 => Some((1, "battle-transition-zoom-out")),
        Stage::BattleShow if age >= 16 => Some((2, "battle-transition-mosaic-in")),
        Stage::Battle => Some((3, "battle-transition-ready")),
        Stage::BattleErase if age >= 16 => Some((4, "battle-transition-mosaic-out")),
        Stage::MapShow if age >= 16 => Some((5, "battle-transition-zoom-in")),
        _ => None,
    };
    let finished = flow.stage == Stage::Map && world.resource::<crate::state::Switches>().get(9997);
    if finished && !world.resource::<Trace>().finished {
        crate::screenfx::battle_smoke::map_return(world);
        world.resource_mut::<Trace>().screen_checks |= 4;
    }
    if let Some((index, _)) = shot
        && world.resource::<Trace>().shots & (1 << index) == 0
    {
        if index == 0 {
            let shake = crate::screenfx::battle_smoke::map_exit(world);
            let mut trace = world.resource_mut::<Trace>();
            trace.shake = Some(shake);
            trace.screen_checks |= 1;
        } else if index == 2 {
            let shake = world
                .resource::<Trace>()
                .shake
                .expect("map exit was checked");
            crate::screenfx::battle_smoke::battle_entry(world, shake);
            world.resource_mut::<Trace>().screen_checks |= 2;
        }
    }
    let mut trace = world.resource_mut::<Trace>();
    if finished {
        trace.finished = true;
    }
    if let Some((index, label)) = shot
        && trace.shots & (1 << index) == 0
    {
        trace.shots |= 1 << index;
        if index == 3 {
            trace.ready_at = Some(frame);
        }
        return Some(label);
    }
    if trace.ready_at == Some(frame.saturating_sub(20)) {
        world.resource_mut::<Battle>().finish(BattleOutcome::Abort);
    }
    None
}

pub(crate) fn verify_finished(world: &World) {
    let trace = world.resource::<Trace>();
    assert_eq!(trace.shots, 0b111111);
    assert!(trace.finished);
    assert_eq!(trace.screen_checks, 0b111);
    assert!(!world.resource::<BattleActive>().0);
    assert!(!world.resource::<crate::transitions::Transition>().erased());
    info!("battle flashes, zoom, mosaic and return completed before the map event resumed");
}
