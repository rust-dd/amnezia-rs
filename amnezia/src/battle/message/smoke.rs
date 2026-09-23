use crate::battle::{Battle, Phase};
use crate::font::bitmap::PixelText;
use crate::timing::SceneFrames;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Trace {
    first: Option<u32>,
    second: Option<u32>,
    finished: bool,
}

pub(in crate::battle) fn drive(world: &mut World) -> Option<&'static str> {
    if !world.contains_resource::<Trace>() {
        world.init_resource::<Trace>();
    }
    let battle = world.resource::<Battle>();
    let now = world.resource::<SceneFrames>().frame;
    if battle.phase == Phase::Encounter {
        let lines = battle.messages.console.visible().to_vec();
        let expected = battle
            .enemies
            .iter()
            .map(|enemy| format!("{}{}", enemy.name, battle.text.encounter))
            .collect::<Vec<_>>();
        if lines.is_empty() {
            return None;
        }
        assert_eq!(lines, expected[..lines.len()]);
        let visible = world
            .query::<(&PixelText, &InheritedVisibility)>()
            .iter(world)
            .filter(|(_, visible)| visible.get())
            .flat_map(|(text, _)| text.runs.iter().map(|run| run.text.clone()))
            .collect::<Vec<_>>();
        assert_eq!(visible, lines);
        let mut trace = world.resource_mut::<Trace>();
        if lines.len() == 1 && trace.first.is_none() {
            trace.first = Some(now);
            return Some("battle-encounter-one");
        }
        if lines.len() == 2 && trace.second.is_none() {
            assert_eq!(now.wrapping_sub(trace.first.unwrap()), 7);
            trace.second = Some(now);
            return Some("battle-encounter-two");
        }
    } else if battle.phase == Phase::PartyCommand {
        let mut trace = world.resource_mut::<Trace>();
        if !trace.finished
            && let Some(second) = trace.second
        {
            assert_eq!(now.wrapping_sub(second), 68);
            trace.finished = true;
        }
    }
    None
}

pub(in crate::battle) fn verify_finished(world: &World) {
    let trace = world.resource::<Trace>();
    assert!(trace.first.is_some() && trace.second.is_some() && trace.finished);
    info!(
        "battle encounter: both original enemy lines, seven-frame spacing and exact command handoff verified"
    );
}
