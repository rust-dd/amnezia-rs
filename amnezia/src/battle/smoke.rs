use super::model::{Battle, MenuLevel, Phase};
use bevy::prelude::*;

pub(crate) fn defeat(world: &mut World) {
    let mut battle = world.resource_mut::<Battle>();
    for actor in &mut battle.members {
        actor.hp = 0;
    }
    battle.finish(super::BattleOutcome::Defeat);
}

pub(crate) fn verify_events(world: &World) {
    assert!(!world.resource::<super::BattleActive>().0);
    assert!(
        !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
    );
    assert!(world.resource::<crate::state::Switches>().get(9999));
    let actor = world
        .resource::<crate::gamedata::GameData>()
        .actor(1)
        .unwrap();
    assert!(
        world
            .resource::<crate::progression::Progression>()
            .known_skill_ids(actor)
            .contains(&2)
    );
}

pub(crate) fn prepare(world: &mut World) {
    world
        .resource_mut::<crate::state::Party>()
        .restore(vec![1, 2, 3, 4]);
    let items = world
        .resource::<crate::gamedata::GameData>()
        .items
        .iter()
        .filter(|item| item.item_type == 6)
        .map(|item| item.id)
        .take(12)
        .collect::<Vec<_>>();
    for id in items {
        world
            .resource_mut::<crate::state::Inventory>()
            .add_item(id, 3);
    }
}

pub(crate) fn show(world: &mut World, frame: u32) -> Option<&'static str> {
    status_colors(world, frame);
    if frame == 1040 {
        let mut battle = world.resource_mut::<Battle>();
        let foe = &battle.enemies[0];
        let pos = (foe.x as f32 - 160.0, foe.y as f32 - 120.0);
        battle.pending_blinks.push(pos);
    }
    if frame == 1150 {
        world
            .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
            .single_mut(world)
            .unwrap()
            .resolution
            .set(640.0, 480.0);
    }
    if frame == 300 {
        let skills = world
            .resource::<crate::gamedata::GameData>()
            .skills
            .iter()
            .filter(|skill| skill.skill_type == 0 && !skill.name.starts_with('-'))
            .map(|skill| skill.id)
            .take(16)
            .collect::<Vec<_>>();
        let mut battle = world.resource_mut::<Battle>();
        assert_eq!(battle.members.len(), 4);
        battle.members[0].known_skills = skills;
    }
    let state = match frame {
        400 => Some((MenuLevel::Command, 0)),
        500 => Some((MenuLevel::Skill, 0)),
        650 => Some((MenuLevel::Skill, 11)),
        700 => Some((MenuLevel::Command, 1)),
        800 => Some((MenuLevel::Item, 0)),
        950 => Some((MenuLevel::Target, 0)),
        1100 => Some((MenuLevel::AllyTarget, 3)),
        _ => None,
    };
    if let Some((menu, cursor)) = state {
        let mut battle = world.resource_mut::<Battle>();
        battle.phase = Phase::Command;
        battle.menu = menu;
        battle.cursor = cursor;
    }
    let label = match frame {
        430 => Some("battle-commands"),
        530 => Some("battle-skills"),
        670 => Some("battle-scroll-arrows"),
        680 => Some("battle-skills-scrolled"),
        730 => Some("battle-status-colors"),
        830 => Some("battle-items"),
        980 => Some("battle-target"),
        1010 => Some("battle-target-flash"),
        1018 => Some("battle-target-fade"),
        1026 => Some("battle-target-clear"),
        1041 => Some("battle-hit-visible"),
        1042 => Some("battle-hit-hidden"),
        1061 => Some("battle-hit-restored"),
        1130 => Some("battle-ally-target"),
        1230 => Some("battle-resized"),
        _ => None,
    };
    if label.is_some() {
        super::hud::verify_bounds(world);
    }
    if frame == 430 {
        assert!(
            world
                .query::<&crate::font::bitmap::PixelText>()
                .iter(world)
                .any(|text| text.runs.iter().any(|r| r.text == "Pengetánc"))
        );
    }
    if frame == 670 {
        super::hud::verify_arrows(world);
    }
    label
}

pub(crate) fn verify_skin(image: &Image, label: &str) {
    let xs: &[u32] = match label {
        "battle-commands" | "battle-status-colors" => &[12, 120, 256],
        "battle-skills" | "battle-skills-scrolled" | "battle-scroll-arrows" | "battle-items" => {
            &[12, 150, 308]
        }
        "battle-target" => &[12, 120],
        "battle-ally-target" | "battle-resized" => &[12, 232],
        _ => return,
    };
    for &x in xs {
        let pixel = image
            .get_color_at(x * image.width() / 320, 185 * image.height() / 240)
            .unwrap()
            .to_srgba();
        assert!(
            pixel.red < 0.1 && pixel.blue > pixel.red + 0.05,
            "{label}: windowskin background is not blue at native x={x}: {pixel:?}"
        );
    }
}

#[derive(Resource)]
struct SavedStatus(Vec<StatusValues>);

struct StatusValues {
    hp: i32,
    sp: i32,
    states: Vec<(u32, u32)>,
}

fn status_colors(world: &mut World, frame: u32) {
    if frame == 700 {
        let mut battle = world.resource_mut::<Battle>();
        let saved = SavedStatus(
            battle
                .members
                .iter()
                .map(|m| StatusValues {
                    hp: m.hp,
                    sp: m.sp,
                    states: m.states.clone(),
                })
                .collect(),
        );
        battle.members[0].hp = battle.members[0].max_hp / 4;
        battle.members[1].hp = 0;
        battle.members[2].states = vec![(2, 0)];
        battle.members[3].sp = 0;
        world.insert_resource(saved);
    }
    if frame == 760 {
        let saved = world.remove_resource::<SavedStatus>().unwrap();
        for (member, values) in world
            .resource_mut::<Battle>()
            .members
            .iter_mut()
            .zip(saved.0)
        {
            member.hp = values.hp;
            member.sp = values.sp;
            member.states = values.states;
        }
    }
}
