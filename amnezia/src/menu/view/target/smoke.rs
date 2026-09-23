use crate::menu::{MenuOpen, MenuScreen, MenuState};
use bevy::prelude::*;

pub(crate) use super::pixels::snapshot;

pub(in crate::menu) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 980 {
        assert!(!world.resource::<MenuOpen>().0);
        world.resource_mut::<MenuOpen>().0 = true;
        world
            .resource_mut::<crate::state::Party>()
            .restore(vec![1, 2, 3, 4]);
        world.resource_mut::<MenuState>().screen = MenuScreen::ItemTarget {
            item_id: 105,
            cursor: 3,
        };
        world.resource_mut::<crate::text::HeroName>().0 = "Áron".into();
        world.resource_scope(|world, data: Mut<crate::gamedata::GameData>| {
            let mut equipment = world.resource_mut::<crate::equipment::Equipment>();
            for actor in data.actors.iter().take(4) {
                for slot in 0..5 {
                    equipment.set_slot(actor, slot, 0);
                }
            }
        });
        let mut vitals = world.resource_mut::<crate::vitals::Vitals>();
        vitals.set(1, 7, 5);
        vitals.set(2, 0, 0);
        vitals.set(3, 1, 0);
        vitals.set_states(3, vec![2]);
    }
    if frame == 1010 {
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
            member: 0,
            skill_id: 9,
            cursor: 3,
        };
    }
    if frame == 1040 {
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
            member: 2,
            skill_id: 49,
            cursor: 0,
        };
    }
    if frame == 1080 {
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillTarget {
            member: 0,
            skill_id: 7,
            cursor: 2,
        };
        world.resource_scope(|world, data: Mut<crate::gamedata::GameData>| {
            world
                .resource_mut::<crate::equipment::Equipment>()
                .set_slot(data.actor(1).unwrap(), 1, 152);
        });
    }
    if frame == 1120 {
        world.resource_mut::<crate::text::HeroName>().0 =
            "Árvíztűrő tükörfúrógép rendkívül hosszú név".into();
    }
    if frame == 1130 {
        world.resource_mut::<crate::text::HeroName>().0 = "Áron".into();
        let data = world.resource::<crate::gamedata::GameData>();
        assert_eq!(
            crate::menu::skills::known_skills(
                1,
                data,
                world.resource::<crate::state::Party>(),
                world.resource::<crate::progression::Progression>()
            )[0]
            .id,
            7
        );
        world.resource_mut::<MenuState>().screen = MenuScreen::SkillList {
            member: 1,
            cursor: 0,
        };
        let mut vitals = world.resource_mut::<crate::vitals::Vitals>();
        vitals.set(1, 7, 30);
        vitals.set(2, 30, 75);
    }
    match frame {
        546 => Some("target-item"),
        560 => Some("target-used"),
        630 => Some("target-empty"),
        995 => Some("target-four"),
        1025 => Some("target-party"),
        1064 => Some("target-self"),
        1095 => Some("target-skill"),
        1125 => Some("target-long-name"),
        1147 => Some("target-cast-ready"),
        1151 => Some("target-cast-first"),
        1156 => Some("target-cast-second"),
        1161 => Some("target-cast-full"),
        _ => None,
    }
}
