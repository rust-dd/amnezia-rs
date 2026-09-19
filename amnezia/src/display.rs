use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::ui_render::UiAntiAlias;
use bevy::window::PrimaryWindow;

const NATIVE_SIZE: UVec2 = UVec2::new(320, 240);
const PRESENTATION_LAYER: usize = 4;

pub(crate) mod smoke;
mod window;

pub(crate) use window::primary_window;

pub(crate) struct DisplayPlugin;

#[derive(Component)]
pub(crate) struct PresentationCamera;

#[derive(Component)]
struct PresentationSprite;

#[derive(Resource)]
struct Canvas(Handle<Image>);

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct DisplaySetup;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        window::register(app);
        crate::transitions::render::register(app);
        app.insert_resource(UiScale(1.0 / 3.0))
            .add_systems(PostStartup, setup.in_set(DisplaySetup))
            .add_systems(PreUpdate, (attach_new_cameras, resize));
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<crate::transitions::render::TransitionMaterial>>,
    cameras: Query<Entity, With<Camera2d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let mut image = Image::new_target_texture(
        NATIVE_SIZE.x,
        NATIVE_SIZE.y,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let canvas = images.add(image);
    for camera in &cameras {
        commands.entity(camera).insert((
            RenderTarget::Image(canvas.clone().into()),
            Msaa::Off,
            UiAntiAlias::Off,
        ));
    }
    let size = windows.single().unwrap().resolution.physical_size();
    let material = crate::transitions::render::setup(
        &mut commands,
        &mut images,
        &mut materials,
        canvas.clone(),
    );
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(320.0, 240.0))),
        MeshMaterial2d(material),
        Transform::from_translation(center_offset(size).extend(0.0)),
        RenderLayers::layer(PRESENTATION_LAYER),
        PresentationSprite,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 100,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        projection(size),
        RenderLayers::layer(PRESENTATION_LAYER),
        Msaa::Off,
        UiAntiAlias::Off,
        PresentationCamera,
    ));
    commands.insert_resource(Canvas(canvas));
}

fn projection(size: UVec2) -> Projection {
    let scale = output_scale(size);
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: bevy::camera::ScalingMode::Fixed {
            width: size.x.max(1) as f32 / scale,
            height: size.y.max(1) as f32 / scale,
        },
        ..OrthographicProjection::default_2d()
    })
}

fn output_scale(size: UVec2) -> f32 {
    let fit = (size.x as f32 / NATIVE_SIZE.x as f32).min(size.y as f32 / NATIVE_SIZE.y as f32);
    if fit >= 1.0 {
        fit.floor()
    } else {
        fit.max(1.0 / 320.0)
    }
}

fn center_offset(size: UVec2) -> Vec2 {
    let scale = output_scale(size);
    let padding = (size.as_vec2() - NATIVE_SIZE.as_vec2() * scale) / 2.0;
    (padding.floor() - padding) * Vec2::new(1.0, -1.0) / scale
}

fn attach_new_cameras(
    mut commands: Commands,
    canvas: Option<Res<Canvas>>,
    cameras: Query<Entity, (Added<Camera2d>, Without<PresentationCamera>)>,
) {
    let Some(canvas) = canvas else {
        return;
    };
    for camera in &cameras {
        commands.entity(camera).insert((
            RenderTarget::Image(canvas.0.clone().into()),
            Msaa::Off,
            UiAntiAlias::Off,
        ));
    }
}

fn resize(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Projection, With<PresentationCamera>>,
    mut sprites: Query<&mut Transform, With<PresentationSprite>>,
    mut previous: Local<UVec2>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = window.resolution.physical_size();
    if size.x == 0 || size.y == 0 || size == *previous {
        return;
    }
    for mut camera in &mut cameras {
        *camera = projection(size);
    }
    for mut sprite in &mut sprites {
        sprite.translation = center_offset(size).extend(0.0);
    }
    *previous = size;
}

#[cfg(test)]
mod tests;
