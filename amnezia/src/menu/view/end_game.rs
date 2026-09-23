use super::*;
use crate::font::bitmap::{BitmapFont, DEFAULT};
use crate::timing::GameFrames;

pub(crate) mod smoke;
#[cfg(test)]
mod tests;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(in crate::menu) enum EndWindow {
    Help,
    Commands,
}

#[derive(Component)]
pub(in crate::menu) struct EndText(usize);

#[derive(Component)]
pub(in crate::menu) struct EndCursor;

pub(in crate::menu) type CursorOnly = (With<EndCursor>, Without<EndWindow>, Without<EndText>);

#[derive(Resource, Default)]
pub(in crate::menu) struct Clock {
    last: Option<u32>,
    active: bool,
    phase: u32,
}

impl Clock {
    fn advance(&mut self, now: u32, active: bool, paused: bool) {
        let delta = self
            .last
            .replace(now)
            .map_or(0, |last| now.wrapping_sub(last));
        if !active || !self.active {
            self.phase = 0;
        } else if !paused {
            self.phase = (self.phase + delta % 21) % 21;
        }
        self.active = active;
    }

    fn source_x(&self) -> f32 {
        if self.phase <= 10 { 64.0 } else { 96.0 }
    }
}

struct Layout {
    labels: [String; 3],
    widths: [u32; 2],
}

impl Layout {
    fn new(terms: &Terms, font: &BitmapFont) -> Self {
        let labels = [
            terms.label(&terms.0.exit_game_message, "Do you really want to quit?"),
            terms.label(&terms.0.yes, "Yes"),
            terms.label(&terms.0.no, "No"),
        ];
        let widths = [
            font.width(&labels[0]) as u32 + 16,
            font.width(&labels[1]).max(font.width(&labels[2])) as u32 + 16,
        ];
        Self { labels, widths }
    }

    fn rect(&self, window: EndWindow) -> (i32, i32, u32, u32) {
        let (width, y, height) = match window {
            EndWindow::Help => (self.widths[0], 72, 32),
            EndWindow::Commands => (self.widths[1], 120, 48),
        };
        (160 - width as i32 / 2, y, width, height)
    }
}

pub(super) fn spawn(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    for kind in [EndWindow::Help, EndWindow::Commands] {
        panel
            .spawn((Node::default(), kind, Visibility::Hidden))
            .with_children(|window| {
                crate::windowskin::fixed_frame(window, system, UVec2::ONE);
                if kind == EndWindow::Commands {
                    window
                        .spawn((Node::default(), EndCursor))
                        .with_children(|cursor| crate::windowskin::cursor(cursor, system));
                }
                let labels = if kind == EndWindow::Help { 0..1 } else { 1..3 };
                for (row, label) in labels.enumerate() {
                    window.spawn((
                        main_text::at(24.0, 30.0 + row as f32 * 48.0, 1),
                        EndText(label),
                    ));
                }
            });
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    terms: Res<Terms>,
    font: Res<BitmapFont>,
    frames: Res<GameFrames>,
    pause: crate::menu::scene::Pause,
    mut clock: ResMut<Clock>,
    mut windows: Query<(&EndWindow, &mut Node, &mut Visibility, &Children)>,
    mut backgrounds: Query<&mut crate::windowskin::background::Pixels>,
    mut texts: Query<(&EndText, &mut PixelText, &mut Node), Without<EndWindow>>,
    mut cursor: Query<(&mut Node, &Children), CursorOnly>,
    mut images: Query<&mut ImageNode, Without<EndText>>,
) {
    let selected = match state.screen {
        MenuScreen::EndGame { cursor } if open.0 => Some(cursor),
        _ => None,
    };
    clock.advance(frames.frame, selected.is_some(), pause.paused());
    for (_, _, mut visibility, _) in &mut windows {
        *visibility = vis(selected.is_some());
    }
    let Some(selected) = selected else { return };
    let layout = Layout::new(&terms, &font);
    for (kind, mut node, _, children) in &mut windows {
        let (x, y, width, height) = layout.rect(*kind);
        for child in children {
            if let Ok(mut background) = backgrounds.get_mut(*child) {
                background.0 = UVec2::new(width, height);
            }
        }
        *node = window_node(
            x as f32 * 3.0,
            y as f32 * 3.0,
            width as f32 * 3.0,
            height as f32 * 3.0,
        );
    }
    for (label, mut text, mut node) in &mut texts {
        let width = layout.widths[usize::from(label.0 != 0)] - 16;
        node.width = Val::Px(width as f32 * 3.0);
        let next = PixelText {
            size: UVec2::new(width, 16),
            runs: vec![Run::new(&layout.labels[label.0], 0, 0, DEFAULT)],
        };
        if *text != next {
            *text = next;
        }
    }
    for (mut node, children) in &mut cursor {
        *node = window_node(
            12.0,
            (8 + selected * 16) as f32 * 3.0,
            (layout.widths[1] - 8) as f32 * 3.0,
            48.0,
        );
        for child in children {
            if let Ok(mut image) = images.get_mut(*child) {
                crate::windowskin::cursor_phase(&mut image, clock.source_x());
            }
        }
    }
}
