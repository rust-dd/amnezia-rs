use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowOccluded};

#[derive(Resource, Default)]
struct State(Option<bool>);

pub(super) fn configure(app: &mut App) {
    if !super::offscreen::enabled() {
        app.init_resource::<State>().add_systems(PreUpdate, observe);
    }
}

fn observe(
    mut events: MessageReader<WindowOccluded>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if windows.contains(event.window) {
            state.0 = Some(event.occluded);
            info!("native smoke window occluded: {}", event.occluded);
        }
    }
}

pub(super) fn describe(world: &mut World, label: &str) {
    let Some(state) = world.get_resource::<State>() else {
        return;
    };
    let occluded = state.0;
    if let Ok(window) = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
    {
        info!(
            "native capture {label}: visible={}, focused={}, occluded={occluded:?}, size={:?}",
            window.visible,
            window.focused,
            window.resolution.physical_size(),
        );
    }
}
