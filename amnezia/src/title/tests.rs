use super::flow::Stage;
use super::*;
use crate::audio::SystemMusic;
use crate::save::{LoadOutcome, LoadRequest};
use crate::session::NewGameRequest;
use crate::teleport::{Fade, PendingTeleport};
use crate::world::MapChanged;

mod loading;
mod new_game_clock;
mod repetition;

fn flow_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, crate::transitions::TransitionPlugin))
        .add_plugins(crate::gamedata::GameDataPlugin)
        .init_resource::<TitleActive>()
        .init_resource::<TitleState>()
        .init_resource::<crate::save::SaveLocation>()
        .init_resource::<crate::save::slots::ActiveSlot>()
        .init_resource::<crate::menu::MenuOpen>()
        .init_resource::<crate::menu::save_files::SaveFiles>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::menu::DirectionInput>()
        .init_resource::<LoadRequest>()
        .init_resource::<LoadOutcome>()
        .init_resource::<NewGameRequest>()
        .init_resource::<Fade>()
        .init_resource::<PendingTeleport>()
        .add_message::<AudioRequest>()
        .add_message::<AppExit>()
        .add_message::<MapChanged>()
        .add_systems(
            Update,
            (
                crate::menu::update_directions,
                flow::entered,
                flow::loaded,
                flow::input,
                flow::drive,
                files::update,
            )
                .chain(),
        );
    app
}

fn frame(app: &mut App, frame: u32) {
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = frame;
    app.update();
}

#[test]
fn continue_waits_for_a_slot_choice_without_stopping_the_title_music() {
    let mut app = flow_app();
    frame(&mut app, 0);
    frame(&mut app, 35);
    app.world_mut().resource_mut::<TitleState>().stage = Stage::Leaving(TitleAction::Continue);
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .clear();
    frame(&mut app, 36);
    assert!(
        !app.world().resource::<LoadRequest>().0,
        "Continue must open the file chooser before requesting a load"
    );
    assert!(app.world().resource::<TitleActive>().0);
    assert!(
        app.world_mut()
            .resource_mut::<Messages<AudioRequest>>()
            .drain()
            .all(|request| !matches!(request, AudioRequest::StopBgm))
    );
}

#[test]
fn title_waits_for_its_initial_fade_and_new_game_erases_for_six_frames() {
    let mut app = flow_app();
    frame(&mut app, 0);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Showing);
    app.world_mut().resource_mut::<TitleState>().cursor = NEW_GAME;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    frame(&mut app, 34);
    assert!(!app.world().resource::<NewGameRequest>().requested);
    frame(&mut app, 35);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
    frame(&mut app, 36);
    assert_eq!(
        app.world().resource::<TitleState>().stage,
        Stage::Leaving(TitleAction::NewGame)
    );
    let audio = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect::<Vec<_>>();
    assert!(
        audio.iter().any(
            |audio| matches!(audio, AudioRequest::FadeOutBgm { duration } if *duration == 0.8)
        )
    );
    frame(&mut app, 5);
    assert!(!app.world().resource::<NewGameRequest>().requested);
    frame(&mut app, 6);
    assert!(app.world().resource::<NewGameRequest>().requested);
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .erased()
    );
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Loading);
}

#[test]
fn return_to_title_keeps_its_twenty_frame_delay_between_erase_and_show() {
    let mut app = flow_app();
    frame(&mut app, 0);
    frame(&mut app, 35);
    app.world_mut().resource_mut::<TitleActive>().0 = false;
    frame(&mut app, 36);
    app.world_mut().resource_mut::<TitleActive>().0 = true;
    frame(&mut app, 37);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Enter);
    frame(&mut app, 43);
    frame(&mut app, 62);
    assert!(!app.world().resource::<TitleState>().stage.visible());
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .erased()
    );
    frame(&mut app, 63);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Showing);
    frame(&mut app, 97);
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .busy()
    );
    frame(&mut app, 98);
    assert_eq!(app.world().resource::<TitleState>().stage, Stage::Ready);
}

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
            stage: Stage::Loading,
            ..default()
        })
        .add_message::<AudioRequest>()
        .add_message::<MapChanged>()
        .add_systems(Update, flow::loaded);
    app.update();
    assert!(app.world().resource::<TitleActive>().0);
    assert!(app.world().resource::<TitleState>().stage == Stage::Prepare);
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
fn selecting_shutdown_requests_app_exit_after_thirty_five_frames() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<AudioRequest>()
        .add_message::<AppExit>()
        .add_plugins(crate::transitions::TransitionPlugin)
        .init_resource::<NewGameRequest>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::menu::DirectionInput>()
        .insert_resource(TitleActive(true))
        .insert_resource(TitleState {
            cursor: SHUTDOWN,
            stage: Stage::Ready,
        })
        .init_resource::<LoadRequest>()
        .init_resource::<crate::save::SaveLocation>()
        .add_systems(Update, (flow::input, flow::drive).chain());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert!(app.should_exit().is_none());
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 35;
    app.update();
    assert!(
        app.should_exit().is_some(),
        "confirming Kilépés must request an app exit"
    );
}
