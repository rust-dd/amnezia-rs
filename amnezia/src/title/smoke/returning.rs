use super::*;
use crate::menu::{MenuOpen, SceneFlow};
use crate::timing::SceneWait;
use crate::transitions::Transition;

#[derive(Resource, Default)]
struct Probe {
    stage: u8,
    pictures: u8,
    faded: bool,
    repetition_at: Option<u32>,
}

pub(super) fn repetition_frame(world: &World, frame: u32) -> Option<u32> {
    let start = world.get_resource::<Probe>()?.repetition_at?;
    (frame >= start).then(|| frame - start + 560)
}

pub(super) fn input(world: &mut World, frame: u32) -> Option<KeyCode> {
    if let Some(frame) = repetition_frame(world, frame) {
        return repetition::input(frame);
    }
    if frame < 305 {
        return None;
    }
    world.init_resource::<Probe>();
    if world.resource::<SceneFlow>().active()
        || world.resource::<Transition>().busy()
        || world.resource::<SceneWait>().0
    {
        return None;
    }
    let probe = world.resource::<Probe>();
    let (command, end) = crate::menu::end_smoke::cursors(world);
    let open = world.resource::<MenuOpen>().0;
    let key = match probe.stage {
        0 if !open => KeyCode::Escape,
        1 if open && command.is_some() => KeyCode::PageDown,
        2 if open && command == Some(4) => KeyCode::Enter,
        3 if end == Some(1) && probe.pictures & 3 == 3 => KeyCode::Enter,
        4 | 6 if open && command.is_some() => {
            crate::menu::end_smoke::assert_cancelled(world);
            KeyCode::Enter
        }
        5 if end == Some(1) => KeyCode::Escape,
        7 if end == Some(1) => KeyCode::ArrowUp,
        8 if end == Some(0) && probe.pictures == 7 && frame >= 462 => KeyCode::Enter,
        _ => return None,
    };
    world.resource_mut::<Probe>().stage += 1;
    Some(key)
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if let Some(frame) = repetition_frame(world, frame) {
        return repetition::drive(world, frame);
    }
    let probe = world.get_resource::<Probe>()?;
    if probe.repetition_at.is_some() {
        return None;
    }
    assert!(frame < 1000, "return-to-title fixture did not settle");
    if probe.stage == 9 {
        if !probe.faded && world.resource::<Transition>().busy() {
            assert!(world.resource::<TitleActive>().0);
            assert!(!world.resource::<MenuOpen>().0);
            world.resource_mut::<Probe>().faded = true;
            return Some("title-return-fade");
        }
        if ready(world) && crate::title::view::smoke::fully_open(world) {
            assert!(probe.faded);
            world.resource_mut::<Probe>().repetition_at = Some(frame + 10);
            return Some("title-return-ready");
        }
    }
    if world.resource::<SceneFlow>().active() || world.resource::<Transition>().busy() {
        return None;
    }
    let (_, end) = crate::menu::end_smoke::cursors(world);
    let phase = crate::menu::end_smoke::cursor_source(world);
    let (bit, label) = match (probe.stage, end, phase) {
        (3, Some(1), 64) => (1, "title-return-menu"),
        (3, Some(1), 96) => (2, "end-game-no-blink"),
        (8, Some(0), 96) => (4, "end-game-yes"),
        _ => return None,
    };
    if probe.pictures & bit != 0 {
        return None;
    }
    world.resource_mut::<Probe>().pictures |= bit;
    Some(label)
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!((probe.stage, probe.pictures), (9, 7));
    assert!(probe.faded && probe.repetition_at.is_some());
}
