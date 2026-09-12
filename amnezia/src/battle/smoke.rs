use super::model::{Battle, MenuLevel, Phase};
use bevy::prelude::*;

mod messages;
pub(in crate::battle) mod shake;

pub(crate) fn verify_finished(world: &World) {
    messages::verify_finished(world);
    shake::verify_finished(world);
}

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

pub(crate) fn events_ready(world: &World) -> bool {
    world.resource::<crate::state::Switches>().get(9999)
        && world.resource::<crate::world::MapData>().map_id == 3
        && !world.resource::<super::BattleActive>().0
        && !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
        && !world.resource::<crate::dialogue::Dialogue>().active
        && !world.resource::<crate::teleport::Fade>().busy()
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

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        400 | 403 | 443 | 470 | 540 | 586 | 592 | 626 | 835 => Some(KeyCode::Enter),
        407 | 447 | 591 | 625 => Some(KeyCode::ArrowDown),
        585 => Some(KeyCode::ArrowRight),
        440 | 565 | 610 | 620 | 860 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn show(world: &mut World, frame: u32) -> Option<&'static str> {
    let impact = messages::drive(world, frame);
    let shake = shake::drive(world, frame);
    status_colors(world, frame);
    if let Some(tone) = match frame {
        900 => Some([50.0, 100.0, 150.0, 0.0]),
        920 => Some([200.0, 200.0, 200.0, 100.0]),
        940 => Some([100.0; 4]),
        _ => None,
    } {
        world
            .resource_mut::<crate::screenfx::TintState>()
            .set_tone(tone);
    }
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
        battle.pending_skill = None;
        battle.pending_item = None;
        if menu == MenuLevel::AllyTarget {
            battle.pending_item = battle
                .items
                .iter()
                .find(|item| item.item_type == 6)
                .map(|item| item.id);
        }
    }
    let label = match frame {
        430 => Some("battle-commands"),
        530 => Some("battle-skills"),
        550 => Some("battle-skill-target-overlay"),
        580 => Some("battle-skill-target-return"),
        600 => Some("battle-second-actor-skills"),
        630 => Some("battle-restored-actor-skills"),
        670 => Some("battle-scroll-arrows"),
        680 => Some("battle-skills-scrolled"),
        730 => Some("battle-status-colors"),
        830 => Some("battle-items"),
        845 => Some("battle-item-target-overlay"),
        880 => Some("battle-item-target-return"),
        910 => Some("battle-tone"),
        930 => Some("battle-tone-light"),
        980 => Some("battle-target"),
        1010 => Some("battle-target-flash"),
        1018 => Some("battle-target-fade"),
        1026 => Some("battle-target-clear"),
        1041 => Some("battle-hit-visible"),
        1042 => Some("battle-hit-hidden"),
        1061 => Some("battle-hit-restored"),
        1130 => Some("battle-ally-target"),
        1230 => Some("battle-resized"),
        1250 => Some("battle-skill-usage"),
        _ => super::hud::movement_label(frame).or(impact).or(shake),
    };
    if matches!(label, Some("battle-action-flash" | "battle-action-fade")) {
        assert!(world.resource::<Battle>().log.is_empty());
        let visible = world
            .query::<(
                Entity,
                &crate::font::bitmap::PixelText,
                &InheritedVisibility,
                Option<&ChildOf>,
            )>()
            .iter(world)
            .filter(|(_, text, visible, _)| {
                visible.get() && text.runs.iter().any(|run| !run.text.is_empty())
            })
            .map(|(entity, text, _, parent)| {
                format!(
                    "{entity}: {:?}, parent {:?}",
                    text.runs,
                    parent.and_then(|parent| world.get::<Visibility>(parent.parent()))
                )
            })
            .collect::<Vec<_>>();
        assert!(visible.is_empty(), "unexpected action text: {visible:?}");
    } else if label.is_some() {
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
    if frame == 1250 {
        let battle = world.resource::<Battle>();
        assert_eq!(battle.members[0].sp, 17);
        assert_eq!(battle.log, ["Ron X-csapást alkalmaz"]);
        assert!(battle.anim_hold_active());
        assert!(
            world
                .query::<(&crate::font::bitmap::PixelText, &InheritedVisibility)>()
                .iter(world)
                .any(|(text, visible)| visible.get()
                    && text.runs.iter().any(|r| r.text == "Ron X-csapást alkalmaz"))
        );
    }
    if matches!(frame, 600 | 630) {
        let battle = world.resource::<Battle>();
        assert!(battle.menu == MenuLevel::Skill);
        assert_eq!(
            (battle.turn, battle.cursor),
            if frame == 600 { (1, 0) } else { (0, 1) }
        );
        assert_eq!(
            battle.members[0].sp, 37,
            "selection and undo must not spend Ron's SP"
        );
        assert_eq!(battle.skill_cursors[0], 1);
        if frame == 600 {
            assert!(matches!(
                battle.members[0].command,
                Some(super::model::Command::Skill { skill_id: 2, .. })
            ));
        } else {
            assert!(battle.members[0].command.is_none());
        }
    }
    if matches!(frame, 550 | 580 | 845 | 880) {
        super::hud::verify_layers(world);
        let battle = world.resource::<Battle>();
        let expected = match frame {
            550 => MenuLevel::Target,
            580 => MenuLevel::Skill,
            845 => MenuLevel::AllyTarget,
            _ => MenuLevel::Item,
        };
        assert!(
            battle.menu == expected,
            "frame {frame}: target overlay/return"
        );
    }
    label
}

pub(crate) fn verify_actor_names(world: &mut World) {
    let battle = world.resource::<Battle>();
    assert_eq!(battle.phase, super::model::Phase::PartyCommand);
    assert_eq!(
        battle
            .members
            .iter()
            .map(|m| (m.actor_id, m.name.as_str()))
            .collect::<Vec<_>>(),
        [(2, "Tiffany"), (1, "Áron"), (3, "Daren"), (4, "Alen")]
    );
    let visible = world
        .query::<(&crate::font::bitmap::PixelText, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, visibility)| visibility.get())
        .flat_map(|(text, _)| &text.runs)
        .map(|run| run.text.as_str())
        .collect::<Vec<_>>();
    assert!(visible.contains(&"Áron"), "{visible:?}");
    assert!(!visible.contains(&"Ron"), "{visible:?}");
}

pub(crate) fn verify_skin(image: &Image, label: &str) {
    let xs: &[u32] = match label {
        "battle-commands" | "battle-status-colors" => &[12, 120, 256],
        "battle-skills"
        | "battle-skills-scrolled"
        | "battle-scroll-arrows"
        | "battle-items"
        | "battle-tone"
        | "battle-tone-light" => &[12, 150, 308],
        "battle-target" | "battle-skill-target-overlay" | "battle-item-target-overlay" => {
            &[12, 140, 256]
        }
        "battle-ally-target" | "battle-resized" => &[12, 232],
        _ => return,
    };
    for &x in xs {
        let pixel = crate::display::smoke::pixel_at(image, x, 185).map(|v| v as f32 / 255.0);
        assert!(
            pixel[0] < 0.1 && pixel[2] > pixel[0] + 0.05,
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
