use super::State;
use crate::dialogue::{Dialogue, DialoguePanel};
use crate::font::bitmap::{BitmapFont, DEFAULT, PixelText, Run};
use crate::terms::Terms;
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Gold;

#[derive(Component)]
pub(super) struct Contents;

#[derive(Component)]
pub(super) struct Frame;

pub(super) fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(232.0 * 3.0),
                top: Val::Px(0.0),
                width: Val::Px(88.0 * 3.0),
                height: Val::Px(32.0 * 3.0),
                ..default()
            },
            Gold,
            GlobalZIndex(101),
            Visibility::Hidden,
        ))
        .with_children(|window| {
            window.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                crate::windowskin::motion::Pixels {
                    size: UVec2::new(88, 32),
                    half: 0,
                },
                Frame,
            ));
            window.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    top: Val::Px(24.0),
                    width: Val::Px(72.0 * 3.0),
                    height: Val::Px(48.0),
                    ..default()
                },
                PixelText::default(),
                Contents,
            ));
        });
}

pub(super) fn contents(gold: i32, terms: &Terms, font: &BitmapFont) -> PixelText {
    let label = crate::i18n::tr(&terms.0.gold);
    let gold = gold.to_string();
    let edge = 72 - font.width(&label);
    PixelText {
        size: UVec2::new(72, 16),
        runs: vec![
            Run::new(label, edge, 2, 1),
            Run::new(&gold, edge - font.width(&gold), 2, DEFAULT),
        ],
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update(
    state: Res<State>,
    dialogue: Res<Dialogue>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    message: Query<&Node, (With<DialoguePanel>, Without<Gold>)>,
    mut window: Query<(&mut Node, &mut Visibility), With<Gold>>,
    mut text: Query<(&mut PixelText, &mut Visibility), (With<Contents>, Without<Gold>)>,
    mut frames: Query<&mut crate::windowskin::motion::Pixels, With<Frame>>,
) {
    for (mut node, mut visible) in &mut window {
        if *visible != Visibility::Visible && dialogue.lifecycle.gold.visible() {
            node.top = if message
                .single()
                .is_ok_and(|message| message.top == Val::Px(0.0))
            {
                Val::Px(208.0 * 3.0)
            } else {
                Val::Px(0.0)
            };
        }
        *visible = if dialogue.lifecycle.gold.visible() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut frame in &mut frames {
        frame.half = dialogue.lifecycle.gold.half_height(32);
    }
    for (mut text, mut visibility) in &mut text {
        *visibility = if dialogue.lifecycle.gold.ready() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let next = contents(state.gold, &terms, &font);
        if *text != next {
            *text = next;
        }
    }
}
