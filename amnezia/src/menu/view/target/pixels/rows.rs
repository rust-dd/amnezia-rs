use crate::font::bitmap::{BitmapFont, PixelText, Run};
use crate::gamedata::GameData;
use crate::state::Party;
use bevy::prelude::*;

pub(super) fn text(world: &World, label: &str) -> PixelText {
    let single = matches!(label, "target-item" | "target-used" | "target-empty");
    let casting = matches!(
        label,
        "target-cast-ready" | "target-cast-first" | "target-cast-second" | "target-cast-full"
    );
    let roster = if single { vec![1] } else { vec![1, 2, 3, 4] };
    assert_eq!(world.resource::<Party>().snapshot(), roster);
    let data = world.resource::<GameData>();
    let font = world.resource::<BitmapFont>();
    let vitals = world.resource::<crate::vitals::Vitals>();
    let mut runs = Vec::new();
    for (index, id) in roster.into_iter().enumerate() {
        let actor = data.actor(id).unwrap();
        let level = actor.level;
        assert_eq!(
            world
                .resource::<crate::progression::Progression>()
                .level(actor),
            level
        );
        let (max_hp, max_sp) = crate::menu::derive::max_hp_sp(actor, level);
        let (hp, sp) = match id {
            1 => (
                match label {
                    "target-used" | "target-cast-first" => 57,
                    "target-empty" | "target-cast-second" | "target-cast-full" => 63,
                    _ => 7,
                },
                if casting { 30 } else { 5 },
            ),
            2 if casting => (
                30,
                match label {
                    "target-cast-ready" => 75,
                    "target-cast-first" => 60,
                    _ => 45,
                },
            ),
            2 => (0, 0),
            3 => (1, 0),
            _ => (max_hp, max_sp),
        };
        assert_eq!(vitals.get_stored(id).unwrap_or((max_hp, max_sp)), (hp, sp));
        let states = match id {
            2 if !casting => vec![1],
            3 => vec![2],
            _ => Vec::new(),
        };
        assert_eq!(vitals.states(id), states);
        let name = if id == 1 {
            let expected = if single {
                "Ron"
            } else if label == "target-long-name" {
                "Árvíztűrő tükörfúrógép rendkívül hosszú név"
            } else {
                "Áron"
            };
            assert_eq!(world.resource::<crate::text::HeroName>().0, expected);
            expected
        } else {
            &actor.name
        };
        let row = index as i32 * 58;
        runs.push(Run::new(crate::i18n::tr(name), 56, row + 2, 0));
        runs.push(Run::new("Sz", 56, row + 18, 1));
        runs.push(right(level, 80, row + 18, 0, font));
        if let Some(state_id) = states.first() {
            let state = crate::conditions::definitions()
                .iter()
                .find(|state| state.id == *state_id)
                .unwrap();
            runs.push(Run::new(
                crate::i18n::tr(&state.name),
                56,
                row + 34,
                state.color,
            ));
        }
        let digits = if max_hp >= 1000 || max_sp >= 1000 {
            4
        } else {
            3
        };
        let left = 150 - digits * 12;
        for (name, current, maximum, y) in
            [("HP", hp, max_hp, row + 18), ("SP", sp, max_sp, row + 34)]
        {
            let color = if name == "HP" && current == 0 {
                5
            } else if maximum > 0 && current <= maximum / 4 {
                4
            } else {
                0
            };
            runs.push(Run::new(name, left, y, 1));
            runs.push(right(current, 162 - digits * 6, y, color, font));
            runs.push(Run::new("/", 162 - digits * 6, y, 0));
            runs.push(right(maximum, 168, y, 0, font));
        }
    }
    PixelText {
        size: UVec2::new(168, 224),
        runs,
    }
}

fn right(value: impl ToString, edge: i32, y: i32, color: u32, font: &BitmapFont) -> Run {
    let value = value.to_string();
    Run::new(&value, edge - font.width(&value), y, color)
}
