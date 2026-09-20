use super::*;
use crate::timing::GameFrames;
use crate::transitions::{Kind, Transition};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Stage {
    Transition { raw_start: u32 },
    Paused { captured_after: u32 },
    Resuming { raw_start: u32 },
    Settling { captured_after: u32 },
    Finished,
}

#[derive(Resource)]
struct Probe {
    stage: Stage,
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1020 {
        let raw_start = world.resource::<GameFrames>().frame;
        world.resource_mut::<SceneFrames>().frame = 0;
        world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        world.resource_mut::<Transition>().start_for(
            Kind::Fade,
            false,
            raw_start,
            IVec2::new(160, 120),
            36,
        );
        world.insert_resource(Probe {
            stage: Stage::Transition { raw_start },
        });
    }
    let stage = world.get_resource::<Probe>()?.stage;
    let scene = world.resource::<SceneFrames>().frame;
    let raw = world.resource::<GameFrames>().frame;
    let next = match stage {
        Stage::Transition { raw_start } => {
            assert_eq!(
                scene, 0,
                "water advanced during the asynchronous transition"
            );
            if world.resource::<Transition>().busy() {
                assert!(frame < 1170, "water clock transition never completed");
                return None;
            }
            assert!(raw.wrapping_sub(raw_start) >= 36);
            world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
            Stage::Paused {
                captured_after: frame + 12,
            }
        }
        Stage::Paused { captured_after } => {
            assert_eq!(scene, 0);
            if frame < captured_after {
                return None;
            }
            world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / 60.0,
            )));
            world.resource_mut::<Probe>().stage = Stage::Resuming { raw_start: raw };
            return Some("water-clock-paused");
        }
        Stage::Resuming { raw_start } => {
            assert_eq!(scene, raw.wrapping_sub(raw_start));
            if scene < 12 {
                return None;
            }
            assert_eq!(scene, 12);
            world.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
            Stage::Settling {
                captured_after: frame + 12,
            }
        }
        Stage::Settling { captured_after } => {
            assert_eq!(scene, 12);
            if frame < captured_after {
                return None;
            }
            world.resource_mut::<Probe>().stage = Stage::Finished;
            return Some("water-clock-resumed");
        }
        Stage::Finished => return None,
    };
    world.resource_mut::<Probe>().stage = next;
    None
}

pub(super) fn verify_finished(world: &World) {
    assert!(matches!(world.resource::<Probe>().stage, Stage::Finished));
    info!("water clock: raw transition ticks, frozen scene and exact twelve-frame resume verified");
}
