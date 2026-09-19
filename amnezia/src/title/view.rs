use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT, DISABLED, PixelText, Run};
use crate::terms::Terms;
use frame::WindowFrame;

pub(super) mod clock;
mod frame;
pub(super) mod smoke;

#[cfg(test)]
mod tests;

#[derive(Component)]
pub(super) struct TitleRoot;

#[derive(Component)]
struct TitleWindow;

#[derive(Component)]
pub(super) struct TitleRow(usize);

#[derive(Component)]
struct TitleCursor;

type RootOnly = (With<TitleRoot>, Without<TitleRow>, Without<TitleCursor>);
type RowsOnly = (Without<TitleWindow>, Without<TitleCursor>);
type CursorOnly = (With<TitleCursor>, Without<TitleWindow>, Without<TitleRow>);

struct Layout {
    labels: [String; 3],
    width: u32,
}

impl Layout {
    fn new(terms: &Terms, font: &BitmapFont) -> Self {
        let labels = [
            terms.label(&terms.0.new_game, ROWS[0]),
            terms.label(&terms.0.load_game, ROWS[1]),
            terms.label(&terms.0.exit_game, ROWS[2]),
        ];
        let width = labels
            .iter()
            .map(|label| font.width(label))
            .max()
            .unwrap_or(0) as u32
            + 16;
        Self { labels, width }
    }
}

fn command_color(row: usize, has_save: bool) -> u32 {
    if row != CONTINUE || has_save {
        DEFAULT
    } else {
        DISABLED
    }
}

fn at(x: f32, y: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x * 3.0),
        top: Val::Px(y * 3.0),
        width: Val::Px(width * 3.0),
        height: Val::Px(height * 3.0),
        ..default()
    }
}

fn fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<clock::Clock>()
        .add_systems(Update, update.after(TitleFlow))
        .add_systems(
            PostUpdate,
            frame::rasterize.before(bevy::ui::UiSystems::Content),
        );
}

pub(super) fn spawn(mut commands: Commands, assets: Res<AssetServer>) {
    let system = assets.load("graphics/System/System.png");
    commands
        .spawn((
            fill_node(),
            BackgroundColor(Color::BLACK),
            GlobalZIndex(2000),
            Visibility::Hidden,
            TitleRoot,
        ))
        .with_children(|root| {
            root.spawn((
                fill_node(),
                ImageNode {
                    image: assets.load(crate::assets::resolve_png("Title", "Title")),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            root.spawn((Node::default(), WindowFrame::default(), TitleWindow))
                .with_children(|window| {
                    window
                        .spawn((Node::default(), TitleCursor, Visibility::Hidden))
                        .with_children(|cursor| crate::windowskin::cursor(cursor, &system));
                    for i in 0..ROWS.len() {
                        window.spawn((
                            at(8.0, 10.0 + i as f32 * 16.0, 1.0, 16.0),
                            PixelText::default(),
                            Visibility::Hidden,
                            TitleRow(i),
                        ));
                    }
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn update(
    title: Res<TitleActive>,
    location: Res<crate::save::SaveLocation>,
    state: Res<TitleState>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    clock: Res<clock::Clock>,
    mut roots: Query<&mut Visibility, RootOnly>,
    mut windows: Query<(&mut Node, &mut WindowFrame), With<TitleWindow>>,
    mut rows: Query<(&TitleRow, &mut PixelText, &mut Node, &mut Visibility), RowsOnly>,
    mut cursors: Query<(&mut Node, &mut Visibility, &Children), CursorOnly>,
    mut images: Query<&mut ImageNode, Without<TitleRow>>,
) {
    let visible = title.0 && state.stage.visible();
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }
    let layout = Layout::new(&terms, &font);
    let contents = if clock.opened == 8 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (mut node, mut frame) in &mut windows {
        *node = at(
            (160 - layout.width as i32 / 2) as f32,
            148.0,
            layout.width as f32,
            64.0,
        );
        *frame = WindowFrame {
            width: layout.width,
            opened: clock.opened,
        };
    }
    let has_save = location.has_saves();
    for (row, mut text, mut node, mut visibility) in &mut rows {
        *visibility = contents;
        node.width = Val::Px((layout.width - 16) as f32 * 3.0);
        let next = PixelText {
            size: UVec2::new(layout.width - 16, 16),
            runs: vec![Run::new(
                &layout.labels[row.0],
                0,
                0,
                command_color(row.0, has_save),
            )],
        };
        if *text != next {
            *text = next;
        }
    }
    for (mut node, mut visibility, children) in &mut cursors {
        *visibility = contents;
        *node = at(
            4.0,
            (8 + state.cursor * 16) as f32,
            (layout.width - 8) as f32,
            16.0,
        );
        for child in children {
            if let Ok(mut image) = images.get_mut(*child) {
                crate::windowskin::cursor_phase(&mut image, clock.source_x());
            }
        }
    }
}
