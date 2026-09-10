use super::{MenuOpen, MenuScreen, MenuState};
use crate::text::HeroName;
use bevy::prelude::*;

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        world
            .resource_mut::<crate::state::Party>()
            .restore(vec![2, 1, 3, 4]);
        let mut running = world.resource_mut::<crate::interpreter::RunningEvent>();
        assert!(!running.active());
        running.start(
            0,
            vec![amnezia_data::EventCommand {
                code: 10610,
                indent: 0,
                string: "Áron".into(),
                params: vec![1],
            }],
        );
    }
    if frame == 300 {
        assert_eq!(world.resource::<HeroName>().0, "Áron");
        world.resource_mut::<MenuOpen>().0 = true;
    }
    let screen = match frame {
        400 => Some(MenuScreen::ItemTarget {
            item_id: 105,
            cursor: 1,
        }),
        500 => Some(MenuScreen::SkillList {
            member: 1,
            cursor: 0,
        }),
        600 => Some(MenuScreen::SkillTarget {
            member: 0,
            skill_id: 9,
            cursor: 1,
        }),
        700 => Some(MenuScreen::Equip {
            member: 1,
            slot: 0,
            picking: None,
        }),
        800 => Some(MenuScreen::Status { member: 1 }),
        _ => None,
    };
    if let Some(screen) = screen {
        world.resource_mut::<MenuState>().screen = screen;
    }
    if frame == 900 {
        world.resource_mut::<MenuOpen>().0 = false;
        world.write_message(crate::battle::BattleRequest {
            troop_id: 2,
            allow_escape: true,
            ..default()
        });
    }
    let label = match frame {
        320 => "actor-name-main",
        420 => "actor-name-items",
        520 => "actor-name-skills",
        620 => "actor-name-skill-target",
        720 => "actor-name-equipment",
        820 => "actor-name-status",
        1100 => {
            crate::battle::smoke::verify_actor_names(world);
            return Some("actor-name-battle");
        }
        _ => return None,
    };
    let visible = world
        .query::<(&Text, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, v)| v.get())
        .map(|(t, _)| t.0.as_str())
        .collect::<Vec<_>>();
    assert!(
        visible.iter().any(|text| text.contains("Áron")),
        "{label}: {visible:?}"
    );
    assert!(
        !visible.iter().any(|text| text.contains("Ron")),
        "{label}: {visible:?}"
    );
    Some(label)
}
