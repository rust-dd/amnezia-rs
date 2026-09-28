use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Transition, TransitionPlugin};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransitionPlugin))
        .init_resource::<GameOverActive>()
        .init_resource::<GameOverFlow>()
        .insert_resource(TitleActive(false))
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<AudioRequest>()
        .add_systems(Update, drive);
    app
}

fn frame(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

#[test]
fn game_over_fades_in_and_out_for_eighty_frames_without_early_confirmation() {
    let mut app = app();
    app.world_mut().resource_mut::<GameOverActive>().0 = true;
    frame(&mut app, 0);
    frame(&mut app, 35);
    assert!(!app.world().resource::<GameOverFlow>().visible());
    frame(&mut app, 36);
    assert!(app.world().resource::<GameOverFlow>().visible());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    frame(&mut app, 116);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Revealing);
    frame(&mut app, 117);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Showing);
    frame(&mut app, 118);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Leaving);
    assert!(!app.world().resource::<TitleActive>().0);
    frame(&mut app, 198);
    assert!(!app.world().resource::<TitleActive>().0);
    frame(&mut app, 199);
    assert!(app.world().resource::<TitleActive>().0);
    assert!(!app.world().resource::<GameOverActive>().0);
    assert!(app.world().resource::<Transition>().erased());
}

#[test]
fn erased_map_skips_a_second_erase_and_still_reveals_game_over() {
    let mut app = app();
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    app.world_mut().resource_mut::<GameOverActive>().0 = true;
    frame(&mut app, 0);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Revealing);
    assert!(!app.world().resource::<Transition>().event_erased);
    frame(&mut app, 79);
    assert!(app.world().resource::<Transition>().busy());
    frame(&mut app, 80);
    assert!(!app.world().resource::<Transition>().erased());
    frame(&mut app, 81);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Showing);
}

#[test]
fn battle_handoff_does_not_add_a_map_erase_or_restore_its_music() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<GameOverFlow>()
        .prepare_from_battle();
    app.world_mut().resource_mut::<GameOverActive>().0 = true;
    frame(&mut app, 0);
    frame(&mut app, 81);
    assert_eq!(app.world().resource::<GameOverFlow>().0, Stage::Showing);
    assert!(!app.world().resource::<Transition>().busy());
}
