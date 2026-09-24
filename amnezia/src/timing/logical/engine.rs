use bevy::prelude::*;

#[derive(Resource)]
struct EngineUpdates(Schedule);

/// Install after Bevy's plugins and before gameplay plugins so engine callbacks
/// such as screenshot delivery still run on renders with no logical tick.
pub(crate) struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        if let Some(schedule) = app.world_mut().resource_mut::<Schedules>().remove(Update) {
            app.insert_resource(EngineUpdates(schedule));
        }
    }
}

pub(super) fn update(world: &mut World) {
    if world.contains_resource::<EngineUpdates>() {
        world.resource_scope(|world, mut updates: Mut<EngineUpdates>| updates.0.run(world));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::{GameFrames, TimingPlugin, logical::LogicalPlugin};
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[derive(Resource, Default)]
    struct Probe {
        engine: Vec<Duration>,
        game: usize,
    }

    #[test]
    fn engine_callbacks_keep_render_time_and_run_without_logical_ticks() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Probe>()
            .add_systems(Update, |time: Res<Time>, mut probe: ResMut<Probe>| {
                probe.engine.push(time.delta());
            })
            .add_plugins((EnginePlugin, TimingPlugin, LogicalPlugin))
            .add_systems(Update, |mut probe: ResMut<Probe>| probe.game += 1)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();
        *app.world_mut().resource_mut::<Probe>() = Probe::default();
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(app.world().resource::<Probe>().engine, [Duration::ZERO; 20]);
        assert_eq!(app.world().resource::<Probe>().game, 0);
        let render_delta = Duration::from_secs_f64(1.0 / 15.0);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(render_delta));
        app.update();
        let probe = app.world().resource::<Probe>();
        assert_eq!(probe.engine.len(), 21);
        assert_eq!(probe.engine[20], render_delta);
        assert_eq!(probe.game, 4);
        assert_eq!(app.world().resource::<GameFrames>().frame, 4);
    }
}
