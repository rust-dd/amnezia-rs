use super::{MemberField, command, render};
use crate::font::bitmap::{BitmapFont, CRITICAL, DEFAULT, DISABLED, KNOCKOUT, PixelText, Run};
use crate::i18n;
use crate::terms::Terms;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(in crate::menu) struct Drawing<'w> {
    pub font: Res<'w, BitmapFont>,
    pub server: Res<'w, AssetServer>,
    pub save_access: Res<'w, crate::save::SaveAccess>,
}

pub(super) fn at(x: f32, y: f32, width: u32) -> impl Bundle {
    (
        PixelText {
            size: UVec2::new(width, 16),
            runs: Vec::new(),
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            width: Val::Px(width as f32 * 3.0),
            height: Val::Px(48.0),
            ..default()
        },
    )
}

pub(super) fn command_color(index: usize, members: usize, save: bool) -> u32 {
    if command::enabled(command::COMMANDS[index], members, save) {
        DEFAULT
    } else {
        DISABLED
    }
}

fn right(value: impl ToString, edge: i32, color: u32, font: &BitmapFont) -> Run {
    let value = value.to_string();
    Run::new(value.clone(), edge - font.width(&value), 0, color)
}

pub(super) fn gold(amount: i32, terms: &Terms, font: &BitmapFont) -> Vec<Run> {
    let label = i18n::tr(&terms.0.gold);
    vec![
        right(amount, 72 - font.width(&label), DEFAULT, font),
        right(label, 72, 1, font),
    ]
}

pub(super) fn member(
    member: &render::MemberView,
    field: MemberField,
    terms: &Terms,
    font: &BitmapFont,
) -> Vec<Run> {
    let t = &terms.0;
    match field {
        MemberField::Name => vec![Run::new(&member.name, 0, 0, DEFAULT)],
        MemberField::Title => vec![Run::new(&member.title, 0, 0, DEFAULT)],
        MemberField::Level => vec![
            Run::new(i18n::tr(&t.lvl_short), 0, 0, 1),
            right(member.level, 24, DEFAULT, font),
        ],
        MemberField::Condition => vec![Run::new(
            if member.condition_color.is_some() {
                member.condition.clone()
            } else {
                i18n::tr(&t.normal_status)
            },
            0,
            0,
            member.condition_color.unwrap_or(DEFAULT),
        )],
        MemberField::Hp => vital(
            &t.hp_short,
            member.hp,
            member.max_hp,
            digits(member),
            true,
            font,
        ),
        MemberField::Sp => vital(
            &t.sp_short,
            member.sp,
            member.max_sp,
            digits(member),
            false,
            font,
        ),
        MemberField::Exp => {
            let numbers = member.exp.map_or_else(
                || "------/------".into(),
                |(total, next)| format!("{total:>6}/{next:>6}"),
            );
            vec![
                Run::new(i18n::tr(&t.exp_short), 0, 0, 1),
                Run::new(numbers, 12, 0, DEFAULT),
            ]
        }
    }
}

fn digits(member: &render::MemberView) -> i32 {
    if member.max_hp >= 1000 || member.max_sp >= 1000 {
        4
    } else {
        3
    }
}

pub(super) fn vital_width(member: &render::MemberView) -> u32 {
    (18 + digits(member) * 12) as u32
}

pub(super) fn vital(
    label: &str,
    current: i32,
    maximum: i32,
    digits: i32,
    hp: bool,
    font: &BitmapFont,
) -> Vec<Run> {
    let color = if hp && current == 0 {
        KNOCKOUT
    } else if maximum > 0 && current <= maximum / 4 {
        CRITICAL
    } else {
        DEFAULT
    };
    vec![
        Run::new(i18n::tr(label), 0, 0, 1),
        right(current, 12 + digits * 6, color, font),
        Run::new("/", 12 + digits * 6, 0, DEFAULT),
        right(maximum, 18 + digits * 12, DEFAULT, font),
    ]
}

#[cfg(test)]
mod tests;
