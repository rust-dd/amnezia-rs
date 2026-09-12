use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource, Clone)]
pub(super) struct Completion {
    scenario: &'static str,
    verified: Arc<AtomicBool>,
}

impl Completion {
    pub(super) fn new(scenario: &'static str) -> Self {
        Self {
            scenario,
            verified: Arc::default(),
        }
    }

    pub(super) fn mark(&self) {
        self.verified.store(true, Ordering::Relaxed);
        info!(
            "smoke scenario '{}' completed all final checks",
            self.scenario
        );
    }
}

pub(crate) fn run(app: &mut App) -> AppExit {
    let completion = app.world().get_resource::<Completion>().cloned();
    let status = app.run();
    if let Some(completion) = completion
        && !completion.verified.load(Ordering::Relaxed)
    {
        error!(
            "smoke scenario '{}' exited before its final checks",
            completion.scenario
        );
        if status.is_success() {
            return AppExit::error();
        }
    }
    status
}

pub(super) fn close_early(world: &mut World, frame: u32) -> bool {
    if frame != 120 || !std::env::args().any(|arg| arg == "--smoke-close-early") {
        return false;
    }
    if world.contains_resource::<super::offscreen::Target>() {
        world.write_message(AppExit::Success);
    } else {
        let windows = world
            .query_filtered::<Entity, With<Window>>()
            .iter(world)
            .collect::<Vec<_>>();
        for window in windows {
            world.despawn(window);
        }
    }
    info!("smoke interruption fixture stopped before final verification");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interrupted_smoke_run_cannot_report_a_successful_exit() {
        let mut app = App::new();
        app.insert_resource(Completion::new("interrupted"))
            .set_runner(|_| AppExit::Success);
        assert_eq!(run(&mut app), AppExit::error());
    }

    #[test]
    fn completion_written_inside_the_runner_survives_the_consumed_app() {
        let mut app = App::new();
        app.insert_resource(Completion::new("verified"))
            .set_runner(|app| {
                app.world().resource::<Completion>().mark();
                AppExit::Success
            });
        assert_eq!(run(&mut app), AppExit::Success);
    }

    #[test]
    fn normal_play_and_existing_error_statuses_are_unchanged() {
        let mut app = App::new();
        app.set_runner(|_| AppExit::Success);
        assert_eq!(run(&mut app), AppExit::Success);
        let mut app = App::new();
        app.insert_resource(Completion::new("failed"))
            .set_runner(|_| AppExit::from_code(7));
        assert_eq!(run(&mut app), AppExit::from_code(7));
    }
}
