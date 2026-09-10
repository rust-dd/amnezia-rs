use super::{Kind, Transition, snapshots};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, TextureFormat, TextureUsages};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(crate) struct TransitionMaterial {
    #[uniform(0)]
    control: Vec4,
    #[uniform(1)]
    crop: Vec4,
    #[uniform(2)]
    mosaic: Vec4,
    #[texture(3)]
    live: Handle<Image>,
    #[texture(4)]
    before: Handle<Image>,
    #[texture(5)]
    after: Handle<Image>,
}

impl Material2d for TransitionMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/transition.wgsl".into()
    }
}

#[derive(Resource)]
struct OutputMaterial(Handle<TransitionMaterial>);

pub(crate) fn register(app: &mut App) {
    app.add_plugins(Material2dPlugin::<TransitionMaterial>::default())
        .add_systems(Last, sync);
    snapshots::register(app);
}

pub(crate) fn setup(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    materials: &mut Assets<TransitionMaterial>,
    live: Handle<Image>,
) -> Handle<TransitionMaterial> {
    let mut texture = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
    texture.texture_descriptor.usage |= TextureUsages::COPY_DST | TextureUsages::COPY_SRC;
    let before = images.add(texture.clone());
    let after = images.add(texture);
    let material = materials.add(TransitionMaterial {
        control: Vec4::new(20.0, 0.0, 0.0, 0.0),
        crop: Vec4::new(0.0, 0.0, 320.0, 240.0),
        mosaic: Vec4::ZERO,
        live: live.clone(),
        before: before.clone(),
        after: after.clone(),
    });
    commands.insert_resource(snapshots::Capture::new(live, before, after));
    commands.insert_resource(OutputMaterial(material.clone()));
    material
}

fn sync(
    transition: Option<Res<Transition>>,
    output: Option<Res<OutputMaterial>>,
    capture: Option<ResMut<snapshots::Capture>>,
    mut materials: ResMut<Assets<TransitionMaterial>>,
) {
    let (Some(transition), Some(output), Some(mut capture)) = (transition, output, capture) else {
        return;
    };
    let Some(mut material) = materials.get_mut(&output.0) else {
        return;
    };
    capture.serial = transition.serial;
    capture.active = transition.busy();
    if let Some(effect) = &transition.effect {
        capture.erase = effect.erase;
        material.control = Vec4::new(
            effect.kind as u32 as f32,
            u8::from(effect.erase) as f32,
            u8::from(effect.from_erased) as f32,
            effect.fade_alpha(transition.frame) as f32 / 255.0,
        );
        match effect.kind {
            Kind::Mosaic => {
                let (size, offset) = effect.mosaic(transition.frame);
                material.mosaic = Vec4::new(size as f32, offset as f32, 0.0, 0.0);
            }
            Kind::Zoom => material.crop = effect.zoom_rect(transition.frame).as_vec4(),
            _ => {}
        }
    } else {
        material.control = Vec4::new(if transition.erased { 21.0 } else { 20.0 }, 0.0, 0.0, 0.0);
    }
}
