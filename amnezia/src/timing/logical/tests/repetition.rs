use super::*;

#[derive(Resource, Default)]
struct Repeats(Vec<u32>);

fn record(
    frames: Res<GameFrames>,
    input: Res<crate::menu::DirectionInput>,
    mut repeats: ResMut<Repeats>,
) {
    for step in input.steps() {
        if step[0] {
            repeats.0.push(frames.frame);
        }
    }
}

#[test]
fn shared_key_repeats_keep_their_exact_tick_inside_low_rate_renders() {
    for fps in [15, 30, 60, 144] {
        let mut app = app();
        app.init_resource::<crate::menu::DirectionInput>()
            .init_resource::<Repeats>()
            .add_systems(Update, (crate::menu::update_directions, record).chain());
        duration(&mut app, 1.0 / 60.0);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        duration(&mut app, 1.0 / f64::from(fps));
        for _ in 0..fps {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Repeats>().0,
            [2, 25, 29, 33, 37, 41, 45, 49, 53, 57, 61],
            "{fps} FPS"
        );
    }
}
