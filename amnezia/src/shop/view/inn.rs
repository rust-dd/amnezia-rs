use crate::font::GameFont;
use crate::shop::{Screen, logic, messages};
use crate::state::Inventory;
use crate::terms::Terms;
use bevy::prelude::*;
use bevy::text::FontSource;

#[derive(Component)]
pub(in crate::shop) struct InnPanel;
#[derive(Component)]
pub(in crate::shop) struct InnText;

pub(in crate::shop) fn update(
    screen: Res<Screen>,
    inventory: Res<Inventory>,
    terms: Res<Terms>,
    mut panels: Query<&mut Visibility, With<InnPanel>>,
    mut texts: Query<&mut Text, With<InnText>>,
) {
    let active = matches!(*screen, Screen::Inn { .. });
    for mut visibility in &mut panels {
        *visibility = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if let Screen::Inn { cost, yes, done } = *screen {
        for mut text in &mut texts {
            **text = render_inn(cost, yes, done, &inventory, &terms);
        }
    }
}

fn render_inn(cost: i32, yes: bool, done: bool, inventory: &Inventory, terms: &Terms) -> String {
    let inn = messages::inn_vocab(terms);
    let unit = messages::currency(terms);
    let cost = cost.max(0);
    let gold = format!("{} {unit}", inventory.gold());
    if done {
        return format!("{}\n\n(-{cost} {unit})\n\n{gold}", inn.rested);
    }
    let affordable = logic::inn_afford(cost, inventory.gold()).is_some();
    let accept = if affordable {
        inn.accept.clone()
    } else {
        format!("{} {}", inn.accept, inn.broke)
    };
    format!(
        "Egy szoba {cost} {unit}.\nKipihened magad?\n\n{}{accept}\n{}{}\n\n{gold}",
        cursor_mark(yes),
        cursor_mark(!yes),
        inn.cancel,
    )
}

fn cursor_mark(selected: bool) -> &'static str {
    if selected { "▶ " } else { "  " }
}

pub fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let system = asset_server.load::<Image>("graphics/System/System.png");
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(40.0),
                right: Val::Px(40.0),
                top: Val::Px(40.0),
                bottom: Val::Px(40.0),
                padding: UiRect::all(Val::Px(16.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(110),
            InnPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                inset_node(0.0),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(32.0, 0.0, 64.0, 32.0)),
                    image_mode: NodeImageMode::Sliced(TextureSlicer {
                        border: BorderRect::all(8.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Stretch,
                        max_corner_scale: 1.0,
                    }),
                    ..default()
                },
            ));
            panel.spawn((
                inset_node(4.0),
                ImageNode {
                    image: system.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 32.0, 32.0)),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            panel.spawn((
                Text::new(String::new()),
                TextFont {
                    font: FontSource::Handle(font.0.clone()),
                    font_size: FontSize::Px(crate::font::UI_FONT_PX),
                    ..default()
                },
                TextColor(Color::WHITE),
                bevy::text::LineHeight::Px(crate::font::UI_LINE_PX),
                TextLayout::no_wrap(),
                InnText,
            ));
        });
}

fn inset_node(px: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(px),
        right: Val::Px(px),
        top: Val::Px(px),
        bottom: Val::Px(px),
        ..default()
    }
}
