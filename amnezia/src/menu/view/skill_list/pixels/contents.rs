use super::Fixture;
use crate::font::bitmap::{BitmapFont, PixelText, Run};
use crate::gamedata::GameData;
use crate::menu::skills::smoke::LONG_NAME;
use crate::progression::Progression;
use crate::vitals::Vitals;
use bevy::prelude::*;

pub(super) fn text(world: &World, fixture: &Fixture) -> [PixelText; 3] {
    let data = world.resource::<GameData>();
    let font = world.resource::<BitmapFont>();
    let id = fixture.member as u32 + 1;
    let actor = data.actor(id).unwrap();
    let ids = if fixture.empty {
        Vec::new()
    } else if fixture.member == 1 {
        vec![7, 11]
    } else {
        (1..=24).chain([49]).collect::<Vec<_>>()
    };
    let mut learned = world.resource::<Progression>().known_skill_ids(actor);
    learned.sort_unstable();
    assert_eq!(learned, ids);
    assert_eq!(
        world.resource::<crate::state::Party>().snapshot(),
        [1, 2, 3, 4]
    );
    let skill = |id| data.skills.iter().find(|skill| skill.id == id).unwrap();
    let description = ids
        .get(fixture.help)
        .map(|id| skill(*id).description.as_str())
        .unwrap_or("");
    let mut entries = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let skill = skill(*id);
        let cost = if fixture.half {
            skill.sp_cost.div_ceil(2)
        } else {
            skill.sp_cost
        };
        let color = if [7, 8, 9, 10, 11, 49].contains(id) && cost <= fixture.sp as u32 {
            0
        } else {
            3
        };
        let x = (index % 2 * 160) as i32;
        let y = (index / 2 * 16 + 2) as i32;
        entries.push(Run::clear(x, y, 144, 12));
        entries.push(Run::new(format!("-{cost:>3}"), x + 120, y, color));
        entries.push(Run::new(&skill.name, x, y, color));
    }
    let (name, level, hp, max_hp, max_sp) = if fixture.member == 1 {
        ("Tiffany", 3, 30, 38, 75)
    } else {
        (if fixture.long { LONG_NAME } else { "Áron" }, 2, 7, 63, 37)
    };
    if fixture.member == 0 {
        assert_eq!(world.resource::<crate::text::HeroName>().0, name);
    }
    assert_eq!(world.resource::<Progression>().level(actor), level);
    assert_eq!(
        world.resource::<Vitals>().get_stored(id),
        Some((hp, fixture.sp))
    );
    assert_eq!(
        world.resource::<Vitals>().states(id),
        if fixture.poison { vec![2] } else { Vec::new() }
    );
    let mut status = vec![
        Run::new(name, 0, 2, 0),
        Run::new("Sz", 80, 2, 1),
        right(level, 104, 0, font),
    ];
    if fixture.poison {
        let poison = crate::conditions::definitions()
            .iter()
            .find(|state| state.id == 2)
            .unwrap();
        status.push(Run::new(&poison.name, 124, 2, poison.color));
    }
    for (label, current, maximum, x) in [("HP", hp, max_hp, 184), ("SP", fixture.sp, max_sp, 250)] {
        let color = if current <= maximum / 4 { 4 } else { 0 };
        status.extend([
            Run::new(label, x, 2, 1),
            right(current, x + 30, color, font),
            Run::new("/", x + 30, 2, 0),
            right(maximum, x + 54, 0, font),
        ]);
    }
    [
        PixelText {
            size: UVec2::new(304, 16),
            runs: vec![Run::new(description, 0, 2, 0)],
        },
        PixelText {
            size: UVec2::new(304, 16),
            runs: status,
        },
        PixelText {
            size: UVec2::new(304, ids.len().div_ceil(2).max(10) as u32 * 16),
            runs: entries,
        },
    ]
}

fn right(value: impl ToString, edge: i32, color: u32, font: &BitmapFont) -> Run {
    let value = value.to_string();
    Run::new(&value, edge - font.width(&value), 2, color)
}
