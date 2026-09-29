use super::*;
use crate::battle::model::{Action, Command, Source};
use crate::font::bitmap::PixelText;

#[derive(Resource)]
struct SkillImpact {
    collapse: String,
    other_enemy_hp: i32,
    captured: bool,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1231 {
        let mut battle = world.resource_mut::<Battle>();
        battle.phase = Phase::Resolve;
        battle.log.clear();
        battle.queue = vec![
            Action {
                source: Source::Enemy(0),
                kind: Command::DoNothing,
                agility: 1,
            },
            Action {
                source: Source::Party(0),
                kind: Command::DoNothing,
                agility: 1,
            },
        ];
        battle.queue_at = 0;
        battle.timeline = default();
        battle.advance_action(
            super::super::message::Controls::default(),
            |_| 0,
            |_, _| true,
        );
    }
    if frame == 1232 {
        return Some("battle-action-flash");
    }
    if frame == 1234 {
        return Some("battle-action-fade");
    }
    if frame == 1235 {
        let mut battle = world.resource_mut::<Battle>();
        battle.phase = Phase::Resolve;
        battle.log.clear();
        battle.enemies[0].hp = 1;
        let check = SkillImpact {
            collapse: format!("{} összeesik!", battle.enemies[0].name),
            other_enemy_hp: battle.enemies[1].hp,
            captured: false,
        };
        battle.queue = vec![Action {
            source: Source::Party(0),
            kind: Command::Skill {
                skill_id: 1,
                target: 0,
            },
            agility: 1,
        }];
        battle.queue_at = 0;
        battle.timeline = default();
        world.insert_resource(check);
    }
    if frame <= 1250 || world.resource::<SkillImpact>().captured {
        return None;
    }
    let collapse = world.resource::<SkillImpact>().collapse.clone();
    if !world
        .query::<(&PixelText, &InheritedVisibility)>()
        .iter(world)
        .any(|(text, visible)| visible.get() && text.runs.iter().any(|run| run.text == collapse))
    {
        return None;
    }
    let battle = world.resource::<Battle>();
    assert_eq!(battle.phase, Phase::Resolve);
    assert_eq!(battle.members[0].sp, 17);
    assert_eq!(battle.enemies[0].hp, 0);
    assert_eq!(
        battle.enemies[1].hp,
        world.resource::<SkillImpact>().other_enemy_hp
    );
    assert!(battle.death_in_progress());
    assert_eq!(
        battle
            .log
            .join("\n")
            .lines()
            .filter(|line| *line == collapse)
            .count(),
        1
    );
    world.resource_mut::<SkillImpact>().captured = true;
    Some("battle-skill-impact")
}

pub(crate) fn verify_finished(world: &World) {
    assert!(
        world.resource::<SkillImpact>().captured,
        "the original X-strike never showed its collapse result"
    );
}
