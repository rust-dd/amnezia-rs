use bevy::prelude::*;
use bevy::render::{RenderApp, RenderStartup};
use bevy::ui_render::ui_texture_slice_pipeline::{
    UiTextureSlicePipeline, init_ui_texture_slice_pipeline,
};
use bevy::ui_render::{UiPipeline, init_ui_pipeline};

pub(crate) mod hue;
pub(crate) mod smoke;
pub(crate) mod tone;

pub(crate) struct LegacyColorsPlugin;

impl Plugin for LegacyColorsPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Camera2d, CompositingSpace>();
        hue::register(app);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(
                RenderStartup,
                configure_ui
                    .after(init_ui_pipeline)
                    .after(init_ui_texture_slice_pipeline),
            );
        }
    }
}

fn configure_ui(
    asset_server: Res<AssetServer>,
    mut ui: ResMut<UiPipeline>,
    mut slices: ResMut<UiTextureSlicePipeline>,
) {
    // Bevy's sprite pipeline supports sRGB compositing; its UI shaders do not.
    ui.shader = asset_server.load("shaders/legacy_ui.wgsl");
    slices.shader = asset_server.load("shaders/legacy_ui_slices.wgsl");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_game_camera_uses_palette_space_unless_explicitly_overridden() {
        let mut app = App::new();
        app.add_plugins(LegacyColorsPlugin);
        let camera = app.world_mut().spawn(Camera2d).id();
        assert_eq!(
            app.world().get::<CompositingSpace>(camera),
            Some(&CompositingSpace::Srgb)
        );
        let linear = app
            .world_mut()
            .spawn((Camera2d, CompositingSpace::Linear))
            .id();
        assert_eq!(
            app.world().get::<CompositingSpace>(linear),
            Some(&CompositingSpace::Linear)
        );
    }
}
