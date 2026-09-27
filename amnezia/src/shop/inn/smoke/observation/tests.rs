use super::*;
use crate::timing::{TimingPlugin, logical::LogicalPlugin};
use bevy::time::TimeUpdateStrategy;
use std::time::Duration;

#[derive(Resource, Default)]
struct Inputs(Vec<[bool; 3]>);

const KEYS: [KeyCode; 3] = [KeyCode::Escape, KeyCode::ArrowDown, KeyCode::Enter];

fn observe_and_finish_rest(
    keys: Res<ButtonInput<KeyCode>>,
    probe: Option<ResMut<Probe>>,
    mut inputs: ResMut<Inputs>,
) {
    let Some(mut probe) = probe else { return };
    inputs.0.push(KEYS.map(|key| keys.pressed(key)));
    probe.handoffs |= 1 << probe.case;
}

#[test]
fn rest_probe_keys_stop_at_the_handoff_even_inside_a_batched_render() {
    for fps in [15, 30, 60, 144] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TimingPlugin, LogicalPlugin))
            .init_resource::<Inputs>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .add_systems(Update, observe_and_finish_rest);
        crate::timing::logical::pre(&mut app, || release_finished_input);
        app.update();
        let mut probe = Probe::new(Entity::PLACEHOLDER, MessageCursor::default());
        probe.step = Step::Rest;
        probe.case = 3;
        probe.handoffs = 1 << 2;
        app.insert_resource(probe)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
                1.0 / f64::from(fps),
            )));
        for _ in 0..fps {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            for key in KEYS {
                keys.press(key);
            }
            app.update();
        }
        let inputs = &app.world().resource::<Inputs>().0;
        assert_eq!(inputs.len(), 60, "{fps} FPS");
        assert_eq!(inputs[0], [true; 3], "{fps} FPS");
        assert!(
            inputs[1..].iter().all(|keys| *keys == [false; 3]),
            "{fps} FPS"
        );
    }
}

#[test]
fn completed_rest_does_not_filter_keys_for_later_prompt_steps() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    let mut probe = Probe::new(Entity::PLACEHOLDER, MessageCursor::default());
    probe.step = Step::Ready;
    probe.case = 3;
    probe.handoffs = 1 << 3;
    world.insert_resource(probe);
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::Enter);
    world.insert_resource(keys);
    world.run_system_once(release_finished_input).unwrap();
    assert!(
        world
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::Enter)
    );
}
