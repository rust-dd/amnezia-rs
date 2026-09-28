use super::{MenuScreen, MenuState, Scene, screen};
use crate::menu::equip::Switch;
use crate::timing::GameFrames;
use crate::transitions::Transition;
use bevy::prelude::*;

#[derive(Debug, PartialEq, Eq)]
struct Frozen {
    slot: u32,
    banks: [(usize, i32, u32, u32, [bool; 2]); 5],
    stats: [u32; 4],
    help: u32,
}

impl Frozen {
    fn of(scene: &Scene) -> Self {
        Self {
            slot: scene.slot_frame,
            banks: std::array::from_fn(|slot| {
                let nav = &scene.lists[slot];
                (
                    nav.index,
                    nav.offset,
                    nav.cursor_frame,
                    nav.arrow_frame,
                    nav.arrows,
                )
            }),
            stats: scene.current,
            help: scene.help_id,
        }
    }
}

struct Pending {
    source: MenuScreen,
    target: MenuScreen,
    started: u32,
    showing: Option<u32>,
    frozen: Frozen,
}

#[derive(Resource, Default)]
pub(super) struct Checks {
    pending: Option<Pending>,
    completed: u8,
    pictures: u8,
}

pub(super) fn verify_frame(world: &mut World, frame: u32) -> (bool, Option<&'static str>) {
    world.resource_scope(|world, mut checks: Mut<Checks>| {
        let request = match frame {
            841 => Some((0, 1, 2)),
            891 => Some((1, 2, 2)),
            921 => Some((2, 0, 2)),
            941 => Some((0, 2, 2)),
            961 => Some((2, 1, 2)),
            991 => Some((1, 2, 0)),
            1011 => Some((2, 0, 0)),
            1061 => Some((0, 1, 1)),
            _ => None,
        };
        let now = world.resource::<GameFrames>().frame;
        let transition = world.resource::<Transition>();
        let scene = world.resource::<Scene>();
        let actual = world.resource::<MenuState>().screen;
        if let Some((source, target, slot)) = request {
            assert!(checks.pending.is_none());
            assert_eq!(actual, screen(source, slot, None));
            assert!(transition.busy());
            assert_eq!(transition.age(), 0);
            checks.pending = Some(Pending {
                source: screen(source, slot, None),
                target: screen(target, slot, None),
                started: now,
                showing: None,
                frozen: Frozen::of(scene),
            });
        }
        let Some(pending) = &mut checks.pending else {
            assert!(!world.resource::<Switch>().active());
            return (false, None);
        };
        assert!(
            now.wrapping_sub(pending.started) < 20,
            "actor fade stalled at {frame}"
        );
        if pending.showing.is_none() && actual != pending.source {
            assert_eq!(actual, pending.target);
            assert!(now.wrapping_sub(pending.started) >= 7);
            assert_eq!(transition.age(), 0);
            assert_eq!(
                (scene.slot_frame, scene.preview, scene.picking),
                (0, None, None)
            );
            for nav in &scene.lists {
                assert_eq!(
                    (nav.index, nav.offset, nav.cursor_frame, nav.arrow_frame),
                    (0, 0, 0, 0)
                );
                assert_eq!(nav.arrows, [false; 2]);
            }
            pending.showing = Some(now);
            pending.frozen = Frozen::of(scene);
        }
        assert_eq!(
            actual,
            if pending.showing.is_some() {
                pending.target
            } else {
                pending.source
            }
        );
        if let Some(started) = pending.showing
            && !transition.busy()
        {
            assert!(now.wrapping_sub(started) >= 7);
            assert!(!world.resource::<Switch>().active());
            assert_eq!(scene.slot_frame, 0);
            checks.pending = None;
            checks.completed += 1;
            return (true, None);
        }
        assert!(transition.busy());
        assert!(world.resource::<Switch>().active());
        assert_eq!(Frozen::of(scene), pending.frozen);
        if checks.completed == 0 && transition.age() == 1 {
            let (bit, label) = if checks.pending.as_ref().unwrap().showing.is_some() {
                (2, "equipment-switch-fade-in")
            } else {
                (1, "equipment-switch-fade-out")
            };
            if checks.pictures & bit == 0 {
                checks.pictures |= bit;
                return (true, Some(label));
            }
        }
        (true, None)
    })
}

pub(super) fn verify_finished(world: &World) {
    let checks = world.resource::<Checks>();
    assert!(checks.pending.is_none());
    assert_eq!((checks.completed, checks.pictures), (8, 3));
    info!(
        "equipment actor fades: eight complete erase/show pairs and two blended references verified"
    );
}
