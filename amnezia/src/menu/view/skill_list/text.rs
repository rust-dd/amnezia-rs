use super::*;
use crate::font::bitmap::{DEFAULT, DISABLED, Run};
use crate::menu::view::{MemberField, main_text};

pub(super) fn entries(
    member: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
    vitals: &Vitals,
    equipment: &Equipment,
) -> PixelText {
    let known = skills::known_skills(member, data, party, progression);
    let mut runs = Vec::new();
    if let Some(actor) = party.snapshot().get(member).and_then(|id| data.actor(*id)) {
        for (index, skill) in known.iter().enumerate() {
            let color =
                if skills::can_use(member, skill, data, party, progression, vitals, equipment) {
                    DEFAULT
                } else {
                    DISABLED
                };
            let x = (index % 2 * 160) as i32;
            let y = (index / 2 * 16 + 2) as i32;
            runs.push(Run::clear(x, y, 144, 12));
            runs.push(Run::new(
                format!("-{:>3}", skills::cost(actor, skill, data, equipment)),
                x + 120,
                y,
                color,
            ));
            runs.push(Run::new(crate::i18n::tr(&skill.name), x, y, color));
        }
    }
    PixelText {
        size: UVec2::new(304, known.len().div_ceil(2).max(10) as u32 * 16),
        runs,
    }
}

pub(super) fn help(
    member: usize,
    index: usize,
    data: &GameData,
    party: &Party,
    progression: &Progression,
) -> PixelText {
    let description = skills::skill_at(member, index, data, party, progression)
        .map(|skill| crate::i18n::tr(&skill.description))
        .unwrap_or_default();
    PixelText {
        size: UVec2::new(304, 16),
        runs: vec![Run::new(description, 0, 2, DEFAULT)],
    }
}

pub(in crate::menu) fn status(
    member: &render::MemberView,
    terms: &Terms,
    font: &BitmapFont,
) -> PixelText {
    let mut runs = Vec::new();
    for (field, x) in [
        (MemberField::Name, 0),
        (MemberField::Level, 80),
        (MemberField::Condition, 124),
    ] {
        for mut run in main_text::member(member, field, terms, font) {
            run.position += IVec2::new(x, 2);
            runs.push(run);
        }
    }
    for (label, current, maximum, hp, x) in [
        (&terms.0.hp_short, member.hp, member.max_hp, true, 184),
        (&terms.0.sp_short, member.sp, member.max_sp, false, 250),
    ] {
        for mut run in main_text::vital(label, current, maximum, 3, hp, font) {
            run.position += IVec2::new(x, 2);
            runs.push(run);
        }
    }
    PixelText {
        size: UVec2::new(304, 16),
        runs,
    }
}
