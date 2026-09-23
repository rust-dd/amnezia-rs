use super::*;
use crate::battle::{BattleActive, BattleResult};
use crate::font::bitmap::PixelText;
use crate::state::Party;

#[derive(Resource, Default)]
struct Trace {
    started: bool,
    ready: bool,
    pause: bool,
    pages: u8,
    states: u32,
    gold: i32,
    items: u32,
    exp: Vec<(u32, u32)>,
}

pub(crate) fn prepare(world: &mut World) {
    world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
    let actors = world
        .resource::<GameData>()
        .actors
        .iter()
        .filter(|actor| actor.id <= 4)
        .cloned()
        .collect::<Vec<_>>();
    for actor in actors {
        let mut progression = world.resource_mut::<Progression>();
        let level = progression.level(&actor);
        let until_next =
            crate::progression::exp_for_level(level + 1, &actor) - progression.total(&actor);
        progression.add(&actor, until_next - 1);
    }
    world.resource_mut::<Vitals>().set(2, 0, 1);
    world.init_resource::<Trace>();
}

pub(crate) fn drive(world: &mut World) -> Option<&'static str> {
    if !world.contains_resource::<Trace>() {
        return None;
    }
    let phase = world.resource::<Battle>().phase;
    if matches!(phase, Phase::PartyCommand | Phase::Command) && !world.resource::<Trace>().started {
        let mut battle = world.resource_mut::<Battle>();
        for enemy in &mut battle.enemies {
            enemy.hp = 0;
            enemy.drop_id = 139;
            enemy.drop_prob = 100;
        }
        battle.phase = Phase::Resolve;
        let gold = world.resource::<Inventory>().gold();
        let items = world.resource::<Inventory>().count(139);
        let data = world.resource::<GameData>();
        let progression = world.resource::<Progression>();
        let exp = [1, 2, 3, 4]
            .into_iter()
            .map(|id| (id, progression.total(data.actor(id).unwrap())))
            .collect();
        let mut trace = world.resource_mut::<Trace>();
        trace.started = true;
        trace.gold = gold;
        trace.items = items;
        trace.exp = exp;
    }
    if phase == Phase::Outcome {
        let battle = world.resource::<Battle>();
        let trace = world.resource::<Trace>();
        assert_eq!(battle.outcome, Some(BattleOutcome::Victory));
        assert!(battle.rewarded);
        assert_eq!(
            world.resource::<Inventory>().gold(),
            trace.gold + battle.reward_gold as i32
        );
        assert_eq!(world.resource::<Inventory>().count(139), trace.items + 2);
        for &(id, before) in &trace.exp {
            let actor = world.resource::<GameData>().actor(id).unwrap();
            assert_eq!(
                world.resource::<Progression>().total(actor),
                before + if id == 2 { 0 } else { battle.reward_exp }
            );
        }
        let dialogue = world.resource::<Dialogue>();
        if dialogue.active {
            assert_eq!(dialogue.boxes.len(), 5);
            assert!(dialogue.boxes.iter().all(|page| page.face.is_none()));
            assert_eq!(dialogue.boxes[0].lines.len(), 4);
            assert_eq!(dialogue.boxes[1].lines.len(), 1);
            let text = dialogue.revealed_text().to_owned();
            let index = dialogue.index;
            let ready = dialogue.ready_to_advance();
            let victory = battle.text.victory.clone();
            let expected = dialogue.boxes[index]
                .lines
                .iter()
                .map(|line| line.replace("\\|", "").replace("\\.", ""))
                .collect::<Vec<_>>()
                .join("\n");
            if ready {
                assert_eq!(text, expected);
                let visible = world
                    .query::<(&PixelText, &InheritedVisibility)>()
                    .iter(world)
                    .filter(|(_, visible)| visible.get())
                    .flat_map(|(text, _)| text.runs.iter().map(|run| run.text.as_str()))
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>();
                assert_eq!(visible, [expected.as_str()]);
            }
            let mut trace = world.resource_mut::<Trace>();
            trace.states += 1;
            if text == victory && !trace.pause {
                trace.pause = true;
                return Some("battle-reward-victory-pause");
            }
            if ready && trace.pages & (1 << index) == 0 {
                trace.pages |= 1 << index;
                return Some(
                    [
                        "battle-reward-first-page",
                        "battle-reward-last-item",
                        "battle-reward-ron-level",
                        "battle-reward-daren-level",
                        "battle-reward-alen-level",
                    ][index],
                );
            }
        }
    }
    if world.resource::<Trace>().started
        && !world.resource::<BattleActive>().0
        && !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
        && !world.resource::<crate::transitions::Transition>().busy()
    {
        world.resource_mut::<Trace>().ready = true;
    }
    None
}

pub(crate) fn ready(world: &World) -> bool {
    world
        .get_resource::<Trace>()
        .is_some_and(|trace| trace.ready)
}

pub(crate) fn verify_finished(world: &World) {
    let trace = world.resource::<Trace>();
    assert!(trace.ready && trace.pause);
    assert_eq!(trace.pages, 0b11111);
    assert!(trace.states > 120);
    assert!(world.resource::<BattleResult>().0.is_none());
    info!(
        "battle rewards: five typed pages, victory pause, three level-ups, fallen-member exclusion and single payout verified over {} states",
        trace.states
    );
}
