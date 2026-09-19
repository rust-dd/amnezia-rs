use super::model::{Key, Selection};
use crate::font::bitmap::{BitmapFont, DEFAULT, PixelText, Run};
use crate::menu::{
    render::MemberView,
    view::{MemberField, main_text},
};
use crate::terms::Terms;
use bevy::prelude::*;

pub(super) fn name(selection: &Selection) -> PixelText {
    PixelText {
        size: UVec2::new(120, 16),
        runs: vec![Run::new(&selection.name, 0, 2, DEFAULT)],
    }
}

pub(super) fn value(selection: &Selection, terms: &Terms, font: &BitmapFont) -> PixelText {
    let label = match selection.key {
        Key::Item(_) => &terms.0.possessed_items,
        Key::Skill(..) => &terms.0.sp_cost,
    };
    let value = selection.value.to_string();
    PixelText {
        size: UVec2::new(120, 16),
        runs: vec![
            Run::new(crate::i18n::tr(label), 0, 2, 1),
            Run::new(&value, 120 - font.width(&value), 2, DEFAULT),
        ],
    }
}

pub(in crate::menu) fn party(
    members: &[MemberView],
    terms: &Terms,
    font: &BitmapFont,
) -> PixelText {
    let mut runs = Vec::new();
    for (index, member) in members.iter().take(4).enumerate() {
        let vital_x = if member.max_hp >= 1000 || member.max_sp >= 1000 {
            102
        } else {
            114
        };
        for (field, x, y) in [
            (MemberField::Name, 56, 2),
            (MemberField::Level, 56, 18),
            (MemberField::Condition, 56, 34),
            (MemberField::Hp, vital_x, 18),
            (MemberField::Sp, vital_x, 34),
        ] {
            for mut run in main_text::member(member, field, terms, font) {
                run.position += IVec2::new(x, index as i32 * 58 + y);
                runs.push(run);
            }
        }
    }
    PixelText {
        size: UVec2::new(168, 224),
        runs,
    }
}
