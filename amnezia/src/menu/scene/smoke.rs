use super::*;

struct Pending {
    before: Snapshot,
    after: Snapshot,
    started: u32,
    showing: Option<u32>,
}

#[derive(Resource, Default)]
struct Checks {
    pending: Option<Pending>,
    completed: u32,
    pictures: u8,
}

pub(in crate::menu) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        world.init_resource::<Checks>();
    }
    if frame <= 300 {
        return None;
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let flow = world.resource::<Flow>();
        let now = world.resource::<GameFrames>().frame;
        let transition = world.resource::<Transition>();
        let actual = Snapshot::capture(world.resource::<MenuOpen>(), world.resource::<MenuState>());
        if checks.pending.is_none()
            && let Stage::Erasing(after) = flow.stage
        {
            assert_eq!(transition.age(), 0);
            assert_eq!(
                (after.open, after.screen),
                match checks.completed {
                    0 | 3 | 6 | 7 => (false, MenuScreen::Command),
                    1 | 2 | 5 => (true, MenuScreen::Command),
                    4 => (true, MenuScreen::EndGame { cursor: 1 }),
                    _ => panic!("unexpected menu scene transition at {frame}"),
                },
                "scene requested at {frame}"
            );
            checks.pending = Some(Pending {
                before: actual,
                after,
                started: now,
                showing: None,
            });
        }
        let Some(pending) = checks.pending.as_mut() else {
            assert!(!flow.active());
            return None;
        };
        assert!(now.wrapping_sub(pending.started) < 24, "menu fade stalled");
        if pending.showing.is_none() && matches!(flow.stage, Stage::Showing) {
            assert!(now.wrapping_sub(pending.started) >= 7);
            assert_eq!(transition.age(), 0);
            pending.showing = Some(now);
        }
        assert_eq!(
            actual,
            if pending.showing.is_some() {
                pending.after
            } else {
                pending.before
            },
            "menu scene at {frame}"
        );
        if let Some(started) = pending.showing
            && !flow.active()
        {
            assert!(!transition.busy());
            assert!(world.resource::<SceneWait>().0);
            assert!(now.wrapping_sub(started) >= 7);
            checks.pending = None;
            checks.completed += 1;
            return None;
        }
        assert!(flow.active());
        assert!(transition.busy());
        let showing = pending.showing;
        let (bit, label) = match (checks.completed, transition.age(), showing) {
            (0, 1, None) => (1, "menu-scene-fade-out"),
            (1, 1, Some(_)) => (2, "menu-scene-fade-in"),
            _ => return None,
        };
        if checks.pictures & bit == 0 {
            checks.pictures |= bit;
            return Some(label);
        }
        None
    })
}

pub(in crate::menu) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert!(checks.pending.is_none());
    assert_eq!((checks.completed, checks.pictures), (8, 3));
    info!(
        "field menu fades: eight six-frame erase/show pairs and two full blended images verified"
    );
}
