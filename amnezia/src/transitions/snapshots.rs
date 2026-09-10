use bevy::core_pipeline::{Core2d, Core2dSystems};
use bevy::prelude::*;
use bevy::render::{
    RenderApp,
    camera::ExtractedCamera,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_resource::{Extent3d, Origin3d, TexelCopyTextureInfo, TextureAspect},
    renderer::{RenderContext, ViewQuery},
    texture::GpuImage,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Resource, Clone, ExtractResource)]
pub(super) struct Capture {
    live: Handle<Image>,
    before: Handle<Image>,
    after: Handle<Image>,
    pub serial: u64,
    pub active: bool,
    pub erase: bool,
    pub previous_scene: bool,
    completed: Arc<AtomicU64>,
}

impl Capture {
    pub fn new(live: Handle<Image>, before: Handle<Image>, after: Handle<Image>) -> Self {
        Self {
            live,
            before,
            after,
            serial: 0,
            active: false,
            erase: false,
            previous_scene: false,
            completed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn ready(&self, serial: u64) -> bool {
        self.completed.load(Ordering::SeqCst) == serial
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(ExtractResourcePlugin::<Capture>::default());
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(Core2d, copy_scene.before(Core2dSystems::MainPass));
    }
}

fn copy_scene(
    view: ViewQuery<&ExtractedCamera>,
    capture: Option<Res<Capture>>,
    images: Res<RenderAssets<GpuImage>>,
    mut context: RenderContext,
    mut before_serial: Local<u64>,
    mut after_serial: Local<u64>,
) {
    let Some(capture) = capture.filter(|capture| capture.active) else {
        return;
    };
    let camera = view.into_inner();
    let early = camera.order == 0
        && *before_serial != capture.serial
        && (!capture.erase || capture.previous_scene);
    let destination = if early {
        &capture.before
    } else if camera.order == 100 && *after_serial != capture.serial {
        if capture.erase {
            &capture.before
        } else {
            &capture.after
        }
    } else {
        return;
    };
    let (Some(source), Some(destination)) = (images.get(&capture.live), images.get(destination))
    else {
        return;
    };
    context.command_encoder().copy_texture_to_texture(
        TexelCopyTextureInfo {
            texture: &source.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
            texture: &destination.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        Extent3d {
            width: 320,
            height: 240,
            depth_or_array_layers: 1,
        },
    );
    if camera.order == 0 {
        *before_serial = capture.serial;
        if capture.erase {
            *after_serial = capture.serial;
            capture.completed.store(capture.serial, Ordering::SeqCst);
        }
    } else {
        *after_serial = capture.serial;
        capture.completed.store(capture.serial, Ordering::SeqCst);
    }
}
