use super::*;
use crate::timing::{SceneFrames, SceneWait};

#[derive(Debug, PartialEq, Eq)]
struct Frozen {
    windows: [u32; 5],
    lists: [(usize, i32, u32, u32, [bool; 2]); 2],
}

impl Frozen {
    fn of(screen: &Screen) -> Option<Self> {
        let Screen::Shop(state) = screen else {
            return None;
        };
        let scene = &state.scene;
        let list = |index, offset, cursor, arrow, arrows| (index, offset, cursor, arrow, arrows);
        Some(Self {
            windows: [
                scene.command_frame,
                scene.number_frame,
                scene.party_frame,
                scene.help_id,
                scene.item_id,
            ],
            lists: [
                list(
                    scene.buy.index,
                    scene.buy.offset,
                    scene.buy.cursor_frame,
                    scene.buy.arrow_frame,
                    scene.buy.arrows,
                ),
                list(
                    scene.sell.index,
                    scene.sell.offset,
                    scene.sell.cursor_frame,
                    scene.sell.arrow_frame,
                    scene.sell.arrows,
                ),
            ],
        })
    }
}

struct Pending {
    entering: bool,
    started: u32,
    scene_frame: u32,
    showing: Option<u32>,
    frozen: Option<Frozen>,
}

#[derive(Resource, Default)]
struct Checks {
    pending: Option<Pending>,
    completed: u32,
    pictures: u8,
}

pub(in crate::shop) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        world.init_resource::<Checks>();
    }
    if frame <= 300 {
        return None;
    }
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let flow = world.resource::<Flow>();
        let transition = world.resource::<Transition>();
        let now = world.resource::<GameFrames>().frame;
        let scene_frame = world.resource::<SceneFrames>().frame;
        let frozen = Frozen::of(world.resource::<Screen>());
        let open = world.resource::<ShopOpen>().0;
        assert_eq!(open, frozen.is_some());
        if checks.pending.is_none()
            && matches!(flow.0, Stage::ErasingToMap | Stage::ErasingToShop(_))
        {
            let entering = matches!(flow.0, Stage::ErasingToShop(_));
            assert_eq!(entering, checks.completed.is_multiple_of(2));
            assert_eq!(open, !entering);
            assert_eq!(transition.age(), 0);
            checks.pending = Some(Pending {
                entering,
                started: now,
                scene_frame,
                showing: None,
                frozen: Frozen::of(world.resource::<Screen>()),
            });
        }
        let Some(pending) = checks.pending.as_mut() else {
            assert!(!flow.active());
            return None;
        };
        assert!(
            now.wrapping_sub(pending.started) < 24,
            "shop fade stalled at {frame}"
        );
        assert_eq!(scene_frame, pending.scene_frame);
        if pending.showing.is_none() && matches!(flow.0, Stage::Showing) {
            assert!(now.wrapping_sub(pending.started) >= 6);
            assert_eq!(transition.age(), 0);
            pending.showing = Some(now);
            pending.frozen = Frozen::of(world.resource::<Screen>());
        }
        assert_eq!(open, pending.entering == pending.showing.is_some());
        assert_eq!(frozen, pending.frozen);
        if let Some(started) = pending.showing
            && !flow.active()
        {
            assert!(!transition.busy());
            assert!(world.resource::<SceneWait>().0);
            assert!(now.wrapping_sub(started) >= 6);
            checks.pending = None;
            checks.completed += 1;
            return None;
        }
        assert!(flow.active());
        assert!(transition.busy());
        let showing = pending.showing;
        let (bit, label) = match (checks.completed, transition.age(), showing) {
            (0, 1, Some(_)) => (1, "shop-scene-fade-in"),
            (1, 1, None) => (2, "shop-scene-fade-out"),
            (6, 1, Some(_)) => (4, "shop-buy-fade-in"),
            _ => return None,
        };
        if checks.pictures & bit == 0 {
            checks.pictures |= bit;
            return Some(label);
        }
        None
    })
}

pub(in crate::shop) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert!(checks.pending.is_none());
    assert_eq!((checks.completed, checks.pictures), (14, 7));
    info!(
        "shop fades: fourteen six-frame erase/show pairs, frozen windows and three blended images verified"
    );
}
