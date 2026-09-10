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
        680 => Some("battle-skills-scrolled"),
        830 => Some("battle-items"),
        980 => Some("battle-target"),
        1130 => Some("battle-ally-target"),
        1230 => Some("battle-resized"),
        _ => None,
    };
    if label.is_some() {
        super::hud::verify_bounds(world);
    }
    label
}

pub(crate) fn verify_skin(image: &Image, label: &str) {
    let xs: &[u32] = match label {
        "battle-commands" => &[12, 120, 256],
        "battle-skills" | "battle-skills-scrolled" | "battle-items" => &[12, 150, 308],
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
