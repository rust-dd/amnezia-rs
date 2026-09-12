use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

const RGB: [u8; 3] = [32, 156, 0];

#[derive(Resource)]
struct Fixture {
    anchors: [Vec2; 2],
    checked: Arc<AtomicU32>,
}

pub(crate) fn setup(world: &mut World) {
    let camera = world
        .query_filtered::<&Transform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[RGB[0], RGB[1], RGB[2], 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    let image = world.resource_mut::<Assets<Image>>().add(image);
    let anchors = [
        camera + Vec2::new(-96.0, 80.0),
        camera + Vec2::new(-32.0, 80.0),
    ];
    for (anchor, layer) in anchors.into_iter().zip([0, crate::screenfx::PICTURE_LAYER]) {
        world.spawn((
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::splat(24.0)),
                ..default()
            },
            Transform::from_translation(anchor.extend(200.0)),
            bevy::camera::visibility::RenderLayers::layer(layer),
        ));
    }
    world.insert_resource(Fixture {
        anchors,
        checked: Arc::new(AtomicU32::new(0)),
    });
}

pub(crate) fn verify_camera(world: &mut World) {
    let pan = world.resource::<crate::player::CameraPan>();
    let expected = pan.position.unwrap() + world.resource::<Fx>().shake_offset;
    let actual = world
        .query_filtered::<&Transform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    assert!(
        actual.abs_diff_eq(expected, 0.001),
        "saved shake camera: {actual:?}, expected {expected:?}"
    );
}

pub(crate) struct Snapshot {
    centers: [Vec2; 2],
    tone: [f32; 4],
    flash: [u8; 4],
    checked: Arc<AtomicU32>,
}

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if !matches!(
        label,
        "save-screen-restored" | "save-screen-resumed" | "save-screen-legacy"
    ) {
        return None;
    }
    verify_camera(world);
    let camera = world
        .query_filtered::<&Transform, With<crate::world::MainCamera>>()
        .single(world)
        .unwrap()
        .translation
        .truncate();
    let fixture = world.resource::<Fixture>();
    let color = world
        .resource::<Fx>()
        .flash
        .as_ref()
        .map_or(Color::NONE, |flash| flash.color())
        .to_srgba();
    Some(Snapshot {
        centers: fixture
            .anchors
            .map(|anchor| Vec2::new(anchor.x - camera.x + 160.0, camera.y - anchor.y + 120.0)),
        tone: world.resource::<TintState>().tone(),
        flash: [color.red, color.green, color.blue, color.alpha].map(|v| (v * 255.0).round() as u8),
        checked: fixture.checked.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let bases = [crate::legacy_colors::tone::apply(RGB, self.tone), RGB];
        for (center, base) in self.centers.into_iter().zip(bases) {
            let expected = std::array::from_fn::<_, 3, _>(|i| {
                ((u32::from(base[i]) * (255 - u32::from(self.flash[3]))
                    + u32::from(self.flash[i]) * u32::from(self.flash[3]))
                    / 255) as u8
            });
            for dy in -10..10 {
                for dx in -10..10 {
                    let x = (center.x.round() as i32 + dx) as u32;
                    let y = (center.y.round() as i32 + dy) as u32;
                    let actual = crate::display::smoke::pixel_at(image, x, y);
                    assert!(
                        actual[..3]
                            .iter()
                            .zip(expected)
                            .all(|(a, b)| a.abs_diff(b) <= 1),
                        "saved screen ({x},{y}): {actual:?}, expected {expected:?}"
                    );
                }
            }
        }
        self.checked.fetch_add(1, Ordering::SeqCst);
        info!(
            "saved screen: 800 toned world and untinted foreground pixels, screen flash and camera shake verified"
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Fixture>().checked.load(Ordering::SeqCst),
        3
    );
}
