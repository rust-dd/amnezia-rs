use super::*;
use crate::timing::{GameFrames, SceneFrames};
use crate::transitions::Transition;

#[derive(Resource, Default)]
struct Checks {
    selected_at: Option<u32>,
    player_updates: u32,
    fade_ages: u8,
    captured: bool,
    rebuilt: bool,
}

pub(super) fn configure(app: &mut App) {
    crate::timing::logical::pre(app, || count_player_updates);
}

fn count_player_updates(checks: Option<ResMut<Checks>>) {
    if let Some(mut checks) = checks {
        checks.player_updates += 1;
    }
}

pub(super) fn begin(world: &mut World) {
    world.insert_resource(Checks::default());
}

pub(crate) fn observe(world: &mut World, frame: u32) -> Option<&'static str> {
    let checks = world.get_resource::<Checks>()?;
    let first = checks.selected_at.unwrap_or(frame);
    assert!(checks.player_updates > 0);
    assert_eq!(
        world.resource::<GameFrames>().frame,
        checks.player_updates - 1
    );
    let stage = world.resource::<TitleState>().stage;
    if matches!(
        stage,
        flow::Stage::Leaving(TitleAction::NewGame) | flow::Stage::Loading
    ) {
        assert_eq!(world.resource::<SceneFrames>().frame, 0);
    }
    let transition = world.resource::<Transition>();
    let age = transition.age();
    if checks.selected_at.is_none() {
        assert_eq!(stage, flow::Stage::Leaving(TitleAction::NewGame));
        assert!(transition.busy());
        assert_eq!(age, 0);
    }
    let rebuilt = world.resource::<crate::teleport::PendingTeleport>().0 == Some((5, 0, 0))
        || world.resource::<crate::world::MapData>().map_id == 5;
    let mut checks = world.resource_mut::<Checks>();
    checks.selected_at = Some(first);
    if stage == flow::Stage::Leaving(TitleAction::NewGame) {
        assert!(age < 6);
        checks.fade_ages |= 1 << age;
        if age == 1 && !checks.captured {
            checks.captured = true;
            return Some("title-new-game-fade");
        }
    }
    checks.rebuilt |= rebuilt;
    None
}

pub(crate) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert!(checks.selected_at.is_some());
    assert_eq!(checks.fade_ages, 0b111111);
    assert!(checks.captured && checks.rebuilt);
    super::super::view::smoke::verify_new_game_finished(world);
    info!(
        "new game clock: decision reset, six-frame exit, frozen scene and continuous raw clock verified"
    );
}
