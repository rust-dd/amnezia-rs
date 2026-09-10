use super::MainCamera;
use crate::screenfx::{FrontCamera, PICTURE_LAYER, ScreenTone};
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

/// The fixed 320×240 orthographic projection shared by the world and front
/// cameras — RM2000's native screen. Maps larger than this scroll; the whole view
/// is rendered to the native canvas before the display plugin scales it.
fn fixed_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: bevy::camera::ScalingMode::Fixed {
            width: 320.0,
            height: 240.0,
        },
        ..OrthographicProjection::default_2d()
    })
}

pub(super) fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        MainCamera,
        fixed_projection(),
        ScreenTone::default(),
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        fixed_projection(),
        IsDefaultUiCamera,
        RenderLayers::layer(PICTURE_LAYER),
        FrontCamera,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_clears_to_black_without_erasing_it_in_the_front_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_systems(Startup, setup);
        app.update();
        let world = app.world_mut();
        let camera = world
            .query_filtered::<&Camera, With<MainCamera>>()
            .single(world)
            .unwrap();
        assert!(
            matches!(camera.clear_color, ClearColorConfig::Custom(color) if color == Color::BLACK)
        );
        let world_order = camera.order;
        let front = world
            .query_filtered::<&Camera, With<FrontCamera>>()
            .single(world)
            .unwrap();
        assert!(matches!(front.clear_color, ClearColorConfig::None));
        assert!(front.order > world_order);
    }
}
