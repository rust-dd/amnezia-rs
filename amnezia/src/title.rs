//! The title screen: the opaque overlay shown the instant the game opens, so the
//! player picks a command before the intro ever runs. The menu mirrors RM2000's
//! `Scene_Title` — three rows, "Új játék" (New Game) / "Betöltés" (Continue) /
//! "Kilépés" (Shutdown), the game's own terms. The whole world spawns behind the
//! overlay at startup but stays frozen — [`TitleActive`] folds into the same pause
//! set as the menu, shop, and battle — until a choice dismisses it. New Game fades
//! the title BGM and the screen to black together, then unfreezes so the intro
//! autorun fires; Continue asks [`crate::save`] to restore the slot and holds the
//! overlay up until the loaded map is swapped in, so the frozen intro map never
//! flashes; Shutdown exits the app. The cursor opens on Continue when a save slot
//! exists (else New Game), and Continue is disabled (greyed, buzzer) with no save.

use crate::assets::resolve_png;
use crate::audio::{AudioRequest, SystemMusic, SystemSounds, play_system_se};
use crate::font::GameFont;
use crate::save::{LoadOutcome, LoadRequest, save_slot_exists};
use crate::session::NewGameRequest;
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapChanged;
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
/// the overlay stays up (and the world frozen) until [`crate::save`]'s restore has
/// swapped the saved map in; while `starting`, it fades to black over a New Game
/// before releasing the world. Both hold the overlay so nothing behind it runs
/// early. Reset to the opening state on every (re-)entry by [`on_title_entered`].
#[derive(Resource, Default)]
struct TitleState {
    cursor: usize,
    continuing: bool,
    starting: bool,
    start_alpha: f32,
}

/// The three menu rows, in cursor order, using the game's own RM2000 vocabulary
/// (verified against the original `Terms`): New Game / Continue (Load) / Shutdown.
const ROWS: [&str; 3] = ["Új játék", "Betöltés", "Kilépés"];
/// Cursor index of the "Új játék" (New Game) row.
const NEW_GAME: usize = 0;
/// Cursor index of the "Betöltés" (Continue) row.
const CONTINUE: usize = 1;
/// Cursor index of the "Kilépés" (Shutdown) row.
const SHUTDOWN: usize = 2;

/// Alpha per second for the New Game fade-out, matching the teleport fade's
/// cadence so the whole intro transition reads as one motion.
const FADE_SPEED: f32 = 4.0;

/// Seconds the New Game transition lasts: the black cover ramps 0→1 at
/// [`FADE_SPEED`], and the title BGM fades out over the same span so the picture
/// and the music darken as one motion into the (black) intro map.
const START_FADE_SECS: f32 = 1.0 / FADE_SPEED;

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

/// What confirming the current row does, resolved as a pure mapping from the
/// cursor and whether a save exists so the command wiring stays unit-testable.
#[derive(Debug, PartialEq, Eq)]
enum TitleAction {
    /// Start a fresh game: fade the title and its BGM out, then run the intro.
    NewGame,
    /// Load the save slot and resume on the saved map.
    Continue,
    /// Continue chosen with no save slot: a disabled buzz that stays on the title.
    ContinueDisabled,
    /// Quit the application (RM2000 `CommandShutdown`).
    Shutdown,
}

/// Map the confirmed `cursor` row to its [`TitleAction`], greying Continue into a
/// buzzer when `has_save` is false — RM2000's `continue_enabled` gate.
fn action_for(cursor: usize, has_save: bool) -> TitleAction {
    match cursor {
        CONTINUE if has_save => TitleAction::Continue,
        CONTINUE => TitleAction::ContinueDisabled,
        SHUTDOWN => TitleAction::Shutdown,
        _ => TitleAction::NewGame,
    }
}

/// Where the cursor opens: Continue when a save slot exists, else New Game —
/// RM2000 `Scene_Title::Refresh` (`command_window->SetIndex(1)` with a save).
fn default_cursor(has_save: bool) -> usize {
    if has_save { CONTINUE } else { NEW_GAME }
}

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TitleActive>()
            .init_resource::<TitleState>()
            .add_systems(Startup, spawn_ui)
            .add_systems(
                Update,
                (
                    on_title_entered,
                    drive_continue,
                    title_input,
                    drive_start_fade,
                    update_ui,
                )
                    .chain(),
            );
    }
}

/// Spawn the fullscreen title overlay above every other layer (fade is 1000, the
/// menu 100): the `Title` background image with the three menu rows near the
/// bottom, the cursor already on its opening row.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let background: Handle<Image> = asset_server.load(resolve_png("Title", "Title"));
    let has_save = save_slot_exists();
    let cursor = default_cursor(has_save);
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
                    let enabled = i != CONTINUE || has_save;
                    menu.spawn((
                        Text::new(row_text(label, i == cursor)),
                        TextFont {
                            font: FontSource::Handle(font.0.clone()),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(if enabled { ENABLED } else { DISABLED }),
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
/// confirm with Enter/Space, dispatching the confirmed row through [`action_for`].
/// New Game begins the fade-out (`starting`); Continue (only with a save) asks
/// [`crate::save`] to restore and raises `continuing` so [`drive_continue`] holds
/// the overlay until the saved map lands; Shutdown quits the app. Input is skipped
/// while a New Game fade or a Continue restore is already resolving.
fn title_input(
    keys: Res<ButtonInput<KeyCode>>,
    title: Res<TitleActive>,
    mut state: ResMut<TitleState>,
    mut load_request: ResMut<LoadRequest>,
    mut audio: MessageWriter<AudioRequest>,
    mut exit: MessageWriter<AppExit>,
    sounds: Option<Res<SystemSounds>>,
) {
    if !title.0 || state.starting || state.continuing {
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
        match action_for(state.cursor, save_slot_exists()) {
            TitleAction::NewGame => {
                play_se(&mut audio, sounds, |s| &s.decision);
                // Fade the title theme out over the same span as the screen cover
                // rather than cutting it, so New Game reads as a single motion.
                audio.write(AudioRequest::FadeOutBgm {
                    duration: START_FADE_SECS,
                });
                state.starting = true;
            }
            TitleAction::Continue => {
                play_se(&mut audio, sounds, |s| &s.decision);
                // Stop the title theme now, while the world is still frozen, so the
                // restored map's own BGM takes over cleanly with no stop race.
                audio.write(AudioRequest::StopBgm);
                load_request.0 = true;
                state.continuing = true;
            }
            // Continue with no save is disabled: buzz and stay on the title.
            TitleAction::ContinueDisabled => play_se(&mut audio, sounds, |s| &s.buzzer),
            TitleAction::Shutdown => {
                play_se(&mut audio, sounds, |s| &s.decision);
                exit.write(AppExit::Success);
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

/// Re-open the menu on every (re-)entry to the title — startup, and each return
/// from End Game or Game Over — the way RM2000 `Scene_Title` re-runs `Refresh` and
/// `PlayTitleMusic`: cursor on Continue when a save exists (else New Game), the
/// dismissal handshakes cleared, the New Game fade cover wiped so a prior fade-out
/// never leaves the title black, and the title theme (re)started. Tracked by a
/// one-shot transition so it fires once per entry, not every frame.
fn on_title_entered(
    title: Res<TitleActive>,
    music: Option<Res<SystemMusic>>,
    mut state: ResMut<TitleState>,
    mut audio: MessageWriter<AudioRequest>,
    mut covers: Query<&mut BackgroundColor, With<TitleFadeCover>>,
    mut was_active: Local<bool>,
) {
    let active = title.0;
    let just_entered = active && !*was_active;
    *was_active = active;
    if !just_entered {
        return;
    }
    state.cursor = default_cursor(save_slot_exists());
    state.continuing = false;
    state.starting = false;
    state.start_alpha = 0.0;
    if let Ok(mut cover) = covers.single_mut() {
        cover.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
    }
    if let Some(music) = music {
        audio.write(AudioRequest::from_music(&music.title));
    }
}

/// Hold the title until a new game or loaded map arrives. A failed load returns
/// control to the title menu and restarts its theme.
fn drive_continue(
    mut title: ResMut<TitleActive>,
    mut state: ResMut<TitleState>,
    load_request: Res<LoadRequest>,
    fade: Res<Fade>,
    pending: Res<PendingTeleport>,
    new_game: Res<NewGameRequest>,
    mut load_outcome: ResMut<LoadOutcome>,
    music: Res<SystemMusic>,
    mut audio: MessageWriter<AudioRequest>,
    mut map_changed: MessageReader<MapChanged>,
) {
    if !state.continuing {
        // Stay current so the load's own MapChanged is the first one we read.
        map_changed.clear();
        return;
    }
    let swapped = !map_changed.is_empty();
    map_changed.clear();
    if load_outcome.0.take() == Some(false) {
        state.continuing = false;
        audio.write(AudioRequest::from_music(&music.title));
        return;
    }
    let settled = !load_request.0 && !new_game.0 && !fade.busy() && pending.0.is_none();
    if swapped || settled {
        state.continuing = false;
        title.0 = false;
    }
}

/// Fade to black before resetting the session, keeping the world paused until
/// the starting map is rebuilt.
fn drive_start_fade(
    time: Res<Time>,
    mut state: ResMut<TitleState>,
    mut new_game: ResMut<NewGameRequest>,
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
        state.continuing = true;
        new_game.0 = true;
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
    fn failed_continue_keeps_the_title_open() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<TitleActive>()
            .init_resource::<LoadRequest>()
            .init_resource::<NewGameRequest>()
            .init_resource::<Fade>()
            .init_resource::<PendingTeleport>()
            .init_resource::<SystemMusic>()
            .insert_resource(LoadOutcome(Some(false)))
            .insert_resource(TitleState {
                continuing: true,
                ..default()
            })
            .add_message::<AudioRequest>()
            .add_message::<MapChanged>()
            .add_systems(Update, drive_continue);
        app.update();
        assert!(app.world().resource::<TitleActive>().0);
        assert!(!app.world().resource::<TitleState>().continuing);
    }

    #[test]
    fn title_starts_active_so_the_world_boots_paused() {
        assert!(TitleActive::default().0);
    }

    #[test]
    fn title_menu_has_three_commands_ending_in_shutdown() {
        assert_eq!(ROWS.len(), 3);
        assert_eq!(ROWS, ["Új játék", "Betöltés", "Kilépés"]);
        assert_eq!(ROWS[SHUTDOWN], "Kilépés");
    }

    #[test]
    fn cursor_opens_on_continue_only_when_a_save_exists() {
        assert_eq!(default_cursor(true), CONTINUE);
        assert_eq!(default_cursor(false), NEW_GAME);
    }

    #[test]
    fn action_for_maps_every_row_and_disables_continue_without_a_save() {
        assert_eq!(action_for(NEW_GAME, false), TitleAction::NewGame);
        assert_eq!(action_for(NEW_GAME, true), TitleAction::NewGame);
        assert_eq!(action_for(CONTINUE, true), TitleAction::Continue);
        assert_eq!(action_for(CONTINUE, false), TitleAction::ContinueDisabled);
        assert_eq!(action_for(SHUTDOWN, false), TitleAction::Shutdown);
        assert_eq!(action_for(SHUTDOWN, true), TitleAction::Shutdown);
    }

    #[test]
    fn cursor_wraps_around_all_three_rows() {
        assert_eq!(wrap_cursor(NEW_GAME, -1, ROWS.len()), SHUTDOWN);
        assert_eq!(wrap_cursor(SHUTDOWN, 1, ROWS.len()), NEW_GAME);
        assert_eq!(wrap_cursor(CONTINUE, 1, ROWS.len()), SHUTDOWN);
        assert_eq!(wrap_cursor(CONTINUE, -1, ROWS.len()), NEW_GAME);
    }

    #[test]
    fn row_text_marks_only_the_selected_row() {
        assert_eq!(row_text("Új játék", true), "▶ Új játék");
        assert_eq!(row_text("Betöltés", false), "  Betöltés");
    }

    #[test]
    fn selecting_shutdown_requests_app_exit() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AudioRequest>()
            .add_message::<AppExit>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(TitleActive(true))
            .insert_resource(TitleState {
                cursor: SHUTDOWN,
                ..default()
            })
            .init_resource::<LoadRequest>()
            .add_systems(Update, title_input);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(
            app.should_exit().is_some(),
            "confirming Kilépés must request an app exit"
        );
    }
}
