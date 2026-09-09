use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::window::PrimaryWindow;

#[derive(Resource)]
pub(super) struct Target(pub Handle<Image>);

pub(crate) fn enabled() -> bool {
    cfg!(debug_assertions)
        && std::env::args().any(|arg| arg == "--smoke-test")
        && std::env::args().any(|arg| arg == "--smoke-offscreen")
}

pub(super) fn configure(app: &mut App) {
    if !enabled() {
        return;
    }
    app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ))
    .add_systems(PostStartup, setup)
    .add_systems(PreUpdate, resize);
}

fn texture(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<Entity, With<Camera>>,
) {
    let size = windows.single().unwrap().resolution.physical_size();
    let target = images.add(texture(size));
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(RenderTarget::Image(target.clone().into()));
    }
    commands.insert_resource(Target(target));
    info!("offscreen smoke rendering at {}x{}", size.x, size.y);
}

fn resize(
    target: Res<Target>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let size = windows.single().unwrap().resolution.physical_size();
    if size.x > 0 && size.y > 0 && images.get(&target.0).unwrap().size() != size {
        let mut image = images.get_mut(&target.0).unwrap();
        *image = texture(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_camera_layers_share_one_resizable_capture_target() {
        let mut app = App::new();
        app.init_resource::<Assets<Image>>()
            .add_systems(Startup, setup)
            .add_systems(Update, resize);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(960, 720),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        for order in 0..4 {
            app.world_mut().spawn(Camera { order, ..default() });
        }
        app.update();
        let handle = app.world().resource::<Target>().0.clone();
        let world = app.world_mut();
        for target in world.query::<&RenderTarget>().iter(world) {
            assert!(matches!(target, RenderTarget::Image(image) if image.handle == handle));
        }
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        assert_eq!(image.size(), UVec2::new(960, 720));
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::COPY_SRC)
        );
        world
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(640.0, 480.0);
        app.update();
        assert_eq!(app.world().resource::<Target>().0, handle);
        assert_eq!(
            app.world()
                .resource::<Assets<Image>>()
                .get(&handle)
                .unwrap()
                .size(),
            UVec2::new(640, 480)
        );
    }
}
