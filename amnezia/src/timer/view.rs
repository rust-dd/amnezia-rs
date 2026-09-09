use super::{ClockScene, GameClock};
use crate::dialogue::MessagePosition;
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct TimerPanel;

#[derive(Component)]
pub(super) struct TimerDigit(usize);

pub(super) fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    camera: Query<Entity, With<crate::battle::HudCamera>>,
) {
    let Ok(camera) = camera.single() else { return };
    let image = assets.load("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(12.0),
                ..default()
            },
            Visibility::Hidden,
            UiTargetCamera(camera),
            GlobalZIndex(500),
            TimerPanel,
        ))
        .with_children(|panel| {
            for index in 0..5 {
                panel.spawn((
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(48.0),
                        ..default()
                    },
                    ImageNode {
                        image: image.clone(),
                        ..default()
                    },
                    TimerDigit(index),
                ));
            }
        });
}

pub(super) fn update(
    clock: Res<GameClock>,
    scene: ClockScene,
    position: Option<Res<MessagePosition>>,
    mut panels: Query<(&mut Node, &mut Visibility), With<TimerPanel>>,
    mut digits: Query<(&TimerDigit, &mut ImageNode, &mut Visibility), Without<TimerPanel>>,
) {
    let show = clock.visible && !scene.paused() && (!scene.in_battle() || clock.in_battle);
    for (mut node, mut visibility) in &mut panels {
        *visibility = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        node.top = Val::Px(if scene.in_battle() {
            420.0
        } else if position.as_deref() == Some(&MessagePosition::Top) {
            660.0
        } else {
            12.0
        });
    }
    if !show {
        return;
    }
    let seconds = clock.seconds().min(5999);
    let values = [
        seconds / 600,
        seconds / 60 % 10,
        10,
        seconds / 10 % 6,
        seconds % 10,
    ];
    for (digit, mut image, mut visibility) in &mut digits {
        let x = 32.0 + values[digit.0] as f32 * 8.0;
        image.rect = Some(Rect::new(x, 32.0, x + 8.0, 48.0));
        *visibility = if digit.0 == 2 && clock.remaining.fract() < 0.5 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
