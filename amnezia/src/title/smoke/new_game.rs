use super::*;
use crate::timing::GameFrames;
use crate::transitions::Transition;

#[derive(Resource, Default)]
struct Checks {
    selected_at: Option<u32>,
    fade_ages: u8,
    captured: bool,
    rebuilt: bool,
}

pub(super) fn begin(world: &mut World) {
    world.insert_resource(Checks::default());
}

pub(crate) fn observe(world: &mut World, frame: u32) -> Option<&'static str> {
    let checks = world.get_resource::<Checks>()?;
    let first = checks.selected_at.unwrap_or(frame);
    assert_eq!(world.resource::<GameFrames>().frame, frame - first);
    let stage = world.resource::<TitleState>().stage;
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
    info!("new game clock: decision reset, six-frame exit and continuous map rebuild verified");
}
