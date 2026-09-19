use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPosition};
use bevy::winit::{WINIT_WINDOWS, WinitPlugin};
use core_graphics::display::CGDisplay;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::monitor::MonitorHandle;
use winit::platform::macos::MonitorHandleExtMacOS;

pub(super) fn register(app: &mut App) {
    if app.is_plugin_added::<WinitPlugin>() {
        app.add_systems(PreStartup, place_window);
    }
}

fn is_active_builtin(monitor: &MonitorHandle) -> bool {
    let display = CGDisplay::new(monitor.native_id());
    display.is_builtin() && display.is_active()
}

fn place_window(
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    let Ok((entity, mut window)) = windows.single_mut() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        let native = windows.get_window(entity).unwrap();
        let monitor = preferred_monitor(
            native.available_monitors().map(|monitor| {
                let built_in = is_active_builtin(&monitor);
                (monitor, built_in)
            }),
            native.primary_monitor(),
        );
        if let Some(monitor) = monitor {
            let bounds = CGDisplay::new(monitor.native_id()).bounds();
            let position = centered_position(
                LogicalPosition::new(bounds.origin.x, bounds.origin.y),
                LogicalSize::new(bounds.size.width, bounds.size.height),
                native.outer_size().to_logical(native.scale_factor()),
            );
            // Winit interprets physical positions using the old monitor's scale during a move.
            native.set_outer_position(position);
            if let Ok(position) = native.outer_position() {
                window.position = WindowPosition::At(IVec2::new(position.x, position.y));
            }
            info!(
                "Opening game window on display {} (built-in: {})",
                monitor.native_id(),
                is_active_builtin(&monitor)
            );
        }
        window.visible = true;
    });
}

fn preferred_monitor<T>(
    monitors: impl IntoIterator<Item = (T, bool)>,
    primary: Option<T>,
) -> Option<T> {
    monitors
        .into_iter()
        .find_map(|(monitor, built_in)| built_in.then_some(monitor))
        .or(primary)
}

fn centered_position(
    origin: LogicalPosition<f64>,
    monitor: LogicalSize<f64>,
    window: LogicalSize<f64>,
) -> LogicalPosition<f64> {
    LogicalPosition::new(
        origin.x + (monitor.width - window.width).max(0.0) / 2.0,
        origin.y + (monitor.height - window.height).max(0.0) / 2.0,
    )
}

pub(super) fn verify_native_window(world: &mut World) {
    if !world.contains_resource::<bevy::winit::WinitMonitors>() {
        return;
    }
    let entity = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .unwrap();
    WINIT_WINDOWS.with_borrow(|windows| {
        let window = windows.get_window(entity).unwrap();
        let expected = window
            .available_monitors()
            .find(is_active_builtin)
            .or_else(|| window.primary_monitor())
            .expect("native smoke test needs an available display");
        let actual = window.current_monitor().unwrap();
        assert_eq!(actual.native_id(), expected.native_id());
        assert_eq!(window.is_visible(), Some(true));
        info!(
            "native window placement verified: display {}, built-in {}, position {:?}",
            actual.native_id(),
            is_active_builtin(&actual),
            window.outer_position().unwrap()
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_display_wins_regardless_of_monitor_order() {
        let external = 2;
        let built_in = 1;
        for monitors in [
            [(external, false), (built_in, true)],
            [(built_in, true), (external, false)],
        ] {
            assert_eq!(preferred_monitor(monitors, Some(external)), Some(built_in));
        }
    }

    #[test]
    fn unavailable_built_in_display_falls_back_to_primary() {
        assert_eq!(preferred_monitor::<u32>([], None), None);
        assert_eq!(preferred_monitor([], Some(3)), Some(3));
        assert_eq!(
            preferred_monitor([(2, false), (3, false)], Some(3)),
            Some(3)
        );
    }

    #[test]
    fn centering_uses_logical_screen_coordinates_including_negative_origins() {
        use winit::dpi::PhysicalSize;

        for scale in [1.0, 2.0] {
            let physical = PhysicalSize::new(1440.0 * scale, 1108.0 * scale);
            assert_eq!(
                centered_position(
                    LogicalPosition::new(-1800.0, -100.0),
                    LogicalSize::new(1800.0, 1169.0),
                    physical.to_logical(scale),
                ),
                LogicalPosition::new(-1620.0, -69.5)
            );
        }
    }

    #[test]
    fn oversized_windows_start_at_the_monitor_origin() {
        let origin = LogicalPosition::new(3440.0, 200.0);
        assert_eq!(
            centered_position(
                origin,
                LogicalSize::new(1280.0, 720.0),
                LogicalSize::new(1440.0, 1108.0)
            ),
            origin
        );
    }
}
