use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowPosition, WindowResolution};

#[cfg(target_os = "macos")]
mod macos;

pub(crate) fn primary_window(offscreen: bool) -> Window {
    Window {
        resolution: if offscreen {
            WindowResolution::new(960, 720)
        } else {
            WindowResolution::new(1440, 1080)
        },
        resizable: true,
        title: "Amnézia".to_string(),
        position: if cfg!(target_os = "macos") && !offscreen {
            WindowPosition::Centered(MonitorSelection::Primary)
        } else {
            WindowPosition::Automatic
        },
        // The Mac window stays hidden until its monitor is selected, avoiding an external-screen flash.
        visible: offscreen || !cfg!(target_os = "macos"),
        ..default()
    }
}

pub(super) fn register(_app: &mut App) {
    #[cfg(target_os = "macos")]
    macos::register(_app);
}

pub(super) fn verify_native_window(_world: &mut World) {
    #[cfg(target_os = "macos")]
    macos::verify_native_window(_world);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_window_starts_hidden_until_the_mac_display_is_selected() {
        let window = primary_window(false);
        assert_eq!(window.visible, !cfg!(target_os = "macos"));
        if cfg!(target_os = "macos") {
            assert_eq!(
                window.position,
                WindowPosition::Centered(MonitorSelection::Primary)
            );
        }
        assert_eq!(window.resolution.physical_size(), UVec2::new(1440, 1080));
        assert!(window.resizable);
    }

    #[test]
    fn offscreen_rendering_does_not_wait_for_a_native_monitor() {
        let window = primary_window(true);
        assert!(window.visible);
        assert_eq!(window.resolution.physical_size(), UVec2::new(960, 720));
    }
}
