//! The title screen: the opaque overlay shown the instant the game opens, so the
//! player picks "Új játék" (New Game) or "Folytatás" (Continue) before the intro
//! ever runs. The whole world spawns behind it at startup but stays frozen —
//! [`TitleActive`] folds into the same pause set as the menu, shop, and battle —
//! until a choice dismisses it. New Game fades the title to black and then
//! unfreezes so the intro autorun fires; Continue asks [`crate::save`] to restore
//! the slot, then unfreezes onto the saved map. Continue is disabled (greyed,
//! no-op) when no save slot exists.

use crate::assets::resolve_png;
use crate::audio::{AudioRequest, SystemMusic, SystemSounds, play_system_se};
use crate::font::GameFont;
use crate::save::{LoadRequest, save_slot_exists};
use amnezia_data::SoundDef;
use bevy::prelude::*;
use bevy::text::FontSource;

/// Whether the title overlay owns the screen. Default `true`, so the game opens
/// on the title; every world system that pauses on the menu/shop/battle also
/// pauses on this, keeping the intro map behind the title frozen until a choice.
#[derive(Resource)]
pub struct TitleActive(pub bool);

impl Default for TitleActive {
    fn default() -> Self {
        Self(true)
    }
}

/// The title's row cursor and the two dismissal handshakes. While `continuing`,
/// the overlay stays up (and the world frozen) until [`crate::save`] applies the
/// restore; while `starting`, it fades to black over a New Game before releasing
/// the world. Both hold the overlay so nothing behind it runs early.
#[derive(Resource, Default)]
struct TitleState {
    cursor: usize,
    continuing: bool,
    starting: bool,
    start_alpha: f32,
}

/// The two menu rows, in cursor order.
const ROWS: [&str; 2] = ["Új játék", "Folytatás"];
/// Cursor index of the "Folytatás" (Continue) row.
const CONTINUE: usize = 1;

/// Alpha per second for the New Game fade-out, matching the teleport fade's
/// cadence so the whole intro transition reads as one motion.
const FADE_SPEED: f32 = 4.0;

/// White for a selectable row, grey for a disabled one (Continue with no save).
const ENABLED: Color = Color::WHITE;
const DISABLED: Color = Color::srgb(0.5, 0.5, 0.5);

#[derive(Component)]
struct TitleRoot;

#[derive(Component)]
struct TitleRow(usize);

/// The black sheet over the title that the New Game fade ramps up, so the title
/// dissolves into the (black) intro map instead of hard-cutting to it.
#[derive(Component)]
struct TitleFadeCover;

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleActive>()
            .init_resource::<TitleState>()
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (title_input, drive_start_fade, drive_title_music, update_ui),
            );
    }
}

/// Spawn the fullscreen title overlay above every other layer (fade is 1000, the
/// menu 100): the `Title` background image with the two menu rows near the bottom.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let background: Handle<Image> = asset_server.load(resolve_png("Title", "Title"));
    commands
        .spawn((
            fill_node(),
            GlobalZIndex(2000),
            Visibility::Visible,
            TitleRoot,
        ))
        .with_children(|root| {
            root.spawn((
                fill_node(),
                ImageNode {
                    image: background,
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(56.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|menu| {
                for (i, label) in ROWS.iter().enumerate() {
                    menu.spawn((
                        Text::new(row_text(label, i == 0)),
                        TextFont {
                            font: FontSource::Handle(font.0.clone()),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(ENABLED),
                        TitleRow(i),
                    ));
                }
            });
            root.spawn((
                fill_node(),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                TitleFadeCover,
            ));
        });
}

/// An absolutely-positioned node filling its parent on every side.
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

/// Drive the title while it owns the screen: move the cursor with up/down and
/// confirm with Enter/Space. New Game begins the fade-out (`starting`); Continue
/// (only with a save) asks [`crate::save`] to restore, holding the overlay up via
/// `continuing` until the request clears so the restore lands first.
fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut title: ResMut<TitleActive>,
    mut state: ResMut<TitleState>,
    mut load_request: ResMut<LoadRequest>,
    mut audio: MessageWriter<AudioRequest>,
    sounds: Option<Res<SystemSounds>>,
) {
    if state.continuing {
        if !load_request.0 {
            state.continuing = false;
            title.0 = false;
        }
        return;
    }
    if state.starting || !title.0 {
        return;
    }
    let sounds = sounds.as_deref();
    if keys.just_pressed(KeyCode::ArrowUp) {
        state.cursor = wrap_cursor(state.cursor, -1, ROWS.len());
        play_se(&mut audio, sounds, |s| &s.cursor);
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        state.cursor = wrap_cursor(state.cursor, 1, ROWS.len());
        play_se(&mut audio, sounds, |s| &s.cursor);
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        match state.cursor {
            CONTINUE if save_slot_exists() => {
                play_se(&mut audio, sounds, |s| &s.decision);
                // Stop the title theme now, while the world is still frozen, so the
                // restored map takes over cleanly with no same-frame stop race.
                audio.write(AudioRequest::StopBgm);
                load_request.0 = true;
                state.continuing = true;
            }
            // Continue with no save is disabled: buzz and stay on the title.
            CONTINUE => play_se(&mut audio, sounds, |s| &s.buzzer),
            _ => {
                play_se(&mut audio, sounds, |s| &s.decision);
                audio.write(AudioRequest::StopBgm);
                state.starting = true;
            }
        }
    }
}

/// Play a [`SystemSounds`] effect if the resource is loaded; a small helper so
/// each navigation branch reads as one line.
fn play_se(
    audio: &mut MessageWriter<AudioRequest>,
    sounds: Option<&SystemSounds>,
    pick: impl FnOnce(&SystemSounds) -> &SoundDef,
) {
    if let Some(sounds) = sounds {
        play_system_se(audio, pick(sounds));
    }
}

/// Play the title theme whenever the title takes the screen (startup, and every
/// return from End Game or Game Over), tracked by a one-shot transition so it is
/// not re-issued each frame. Leaving the title stops the theme from
/// [`title_input`] directly, so this system only ever starts it.
fn drive_title_music(
    title: Res<TitleActive>,
    music: Option<Res<SystemMusic>>,
    mut audio: MessageWriter<AudioRequest>,
    mut was_active: Local<bool>,
) {
    let active = title.0;
    let just_entered = active && !*was_active;
    *was_active = active;
    if just_entered && let Some(music) = music {
        audio.write(AudioRequest::from_music(&music.title));
    }
}

/// Ramp the black cover up over a New Game so the title fades into the (also
/// black) intro map, then release the world once it is fully black. The overlay
/// stays up — the world frozen — for the whole fade.
fn drive_start_fade(
    time: Res<Time>,
    mut title: ResMut<TitleActive>,
    mut state: ResMut<TitleState>,
    mut covers: Query<&mut BackgroundColor, With<TitleFadeCover>>,
) {
    if !state.starting {
        return;
    }
    state.start_alpha = (state.start_alpha + FADE_SPEED * time.delta_secs()).min(1.0);
    if let Ok(mut cover) = covers.single_mut() {
        cover.0 = Color::srgba(0.0, 0.0, 0.0, state.start_alpha);
    }
    if state.start_alpha >= 1.0 {
        state.starting = false;
        title.0 = false;
    }
}

/// Show or hide the overlay to match [`TitleActive`], and repaint each row with
/// its cursor marker; the Continue row greys out when no save slot exists.
fn update_ui(
    title: Res<TitleActive>,
    state: Res<TitleState>,
    mut roots: Query<&mut Visibility, With<TitleRoot>>,
    mut rows: Query<(&TitleRow, &mut Text, &mut TextColor)>,
) {
    if !title.is_changed() && !state.is_changed() {
        return;
    }
    if let Ok(mut visibility) = roots.single_mut() {
        *visibility = if title.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let has_save = save_slot_exists();
    for (row, mut text, mut color) in &mut rows {
        let enabled = row.0 != CONTINUE || has_save;
        **text = row_text(ROWS[row.0], row.0 == state.cursor);
        *color = TextColor(if enabled { ENABLED } else { DISABLED });
    }
}

/// A menu row rendered with the `▶` cursor when selected, matching the menu and
/// dialogue rows.
fn row_text(label: &str, selected: bool) -> String {
    let marker = if selected { "▶ " } else { "  " };
    format!("{marker}{label}")
}

/// Move `cursor` by `delta` within `len` rows, wrapping around either end.
fn wrap_cursor(cursor: usize, delta: i32, len: usize) -> usize {
    let len = len as i32;
    (((cursor as i32 + delta) % len + len) % len) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_starts_active_so_the_world_boots_paused() {
        assert!(TitleActive::default().0);
    }

    #[test]
    fn cursor_wraps_around_both_ends() {
        assert_eq!(wrap_cursor(0, 1, 2), 1);
        assert_eq!(wrap_cursor(1, 1, 2), 0);
        assert_eq!(wrap_cursor(1, -1, 2), 0);
        assert_eq!(wrap_cursor(0, -1, 2), 1);
    }

    #[test]
    fn row_text_marks_only_the_selected_row() {
        assert_eq!(row_text("Új játék", true), "▶ Új játék");
        assert_eq!(row_text("Folytatás", false), "  Folytatás");
    }
}
