use super::model::{Battle, BattleSe, Phase};
use crate::font::bitmap::PixelText;
use crate::timing::SceneFrames;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod cases;
mod pixels;
pub(crate) use pixels::snapshot;

const ENDS: [u32; 7] = [97, 126, 108, 165, 125, 125, 156];

#[derive(Resource, Default)]
struct Trace {
    epoch: Option<u32>,
    case: usize,
    states: u32,
    sounds: Vec<(usize, u32, BattleSe)>,
    lines: Vec<String>,
    pixels: Arc<AtomicUsize>,
}

pub(crate) fn configure(app: &mut App) {
    app.init_resource::<Trace>().add_systems(
        Update,
        record
            .after(super::systems::resolve_tick)
            .before(super::systems::drain_pending_se),
    );
}

pub(crate) fn prepare(world: &mut World) {
    world
        .resource_mut::<crate::state::Party>()
        .restore(vec![1, 2]);
}

fn record(battle: Res<Battle>, frames: Res<SceneFrames>, mut trace: ResMut<Trace>) {
    if let Some(epoch) = trace.epoch
        && trace.case < ENDS.len()
    {
        let case = trace.case;
        for &sound in &battle.pending_se {
            trace
                .sounds
                .push((case, frames.frame.wrapping_sub(epoch), sound));
        }
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame < 400 || ready(world) {
        return None;
    }
    if world.resource::<Trace>().epoch.is_none() {
        if world.resource::<Battle>().phase != Phase::PartyCommand {
            return None;
        }
        cases::start(world, 0);
        return None;
    }
    let case = world.resource::<Trace>().case;
    let now = world.resource::<SceneFrames>().frame;
    let age = now.wrapping_sub(world.resource::<Trace>().epoch.unwrap());
    if age > ENDS[case] {
        world.resource_mut::<Trace>().case += 1;
        if !ready(world) {
            cases::start(world, case + 1);
        }
        return None;
    }
    assert!(age > 0);
    let battle = world.resource::<Battle>();
    assert_eq!(
        battle.phase,
        if age < ENDS[case] {
            Phase::Resolve
        } else {
            Phase::PartyCommand
        },
        "case {case}, update {age}"
    );
    cases::verify_state(battle, case, age);
    let lines = cases::lines(battle, case, age.saturating_sub(1));
    if age < ENDS[case] {
        assert_eq!(
            battle.messages.console.visible(),
            lines,
            "case {case}, update {age}"
        );
        let visible = visible_lines(world);
        assert_eq!(visible, lines, "rendered case {case}, update {age}");
    }
    let mut trace = world.resource_mut::<Trace>();
    trace.states += 1;
    trace.lines = lines;
    cases::capture(case, age)
}

fn visible_lines(world: &mut World) -> Vec<String> {
    let mut rows = world
        .query::<(&PixelText, &Node, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, _, visibility)| visibility.get())
        .map(|(text, node, _)| {
            let Val::Px(top) = node.top else {
                panic!("battle text row must have a pixel offset")
            };
            (
                top,
                text.runs
                    .iter()
                    .map(|run| run.text.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter()
        .flat_map(|(_, text)| text)
        .filter(|line| !line.is_empty())
        .collect()
}

pub(crate) fn ready(world: &World) -> bool {
    world.resource::<Trace>().case == ENDS.len()
}

pub(crate) fn verify_finished(world: &World) {
    let trace = world.resource::<Trace>();
    assert!(ready(world));
    assert_eq!(trace.states, ENDS.iter().sum::<u32>());
    assert_eq!(trace.pixels.load(Ordering::Relaxed), 21);
    assert_eq!(
        trace.sounds,
        [
            (0, 49, BattleSe::EnemyDamaged),
            (1, 78, BattleSe::EnemyDamaged),
            (2, 49, BattleSe::Dodge),
            (3, 66, BattleSe::EnemyDamaged),
            (3, 117, BattleSe::EnemyDamaged),
            (5, 4, BattleSe::UseItem),
            (6, 49, BattleSe::EnemyDamaged),
            (6, 88, BattleSe::EnemyDefeated),
        ]
    );
    info!(
        "battle action timeline: {} exact states, eight sounds and 21 complete message-window images verified",
        trace.states
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::bitmap::{DEFAULT, Run};

    #[test]
    fn rendered_message_checks_use_screen_rows_not_entity_iteration_order() {
        let mut world = World::new();
        for (top, text) in [(48.0, "result"), (0.0, "usage")] {
            world.spawn((
                PixelText {
                    size: UVec2::new(300, 16),
                    runs: vec![Run::new(text, 0, 0, DEFAULT)],
                },
                Node {
                    top: Val::Px(top),
                    ..default()
                },
                InheritedVisibility::VISIBLE,
            ));
        }
        assert_eq!(visible_lines(&mut world), ["usage", "result"]);
    }
}
