use super::*;

/// Per-strength fog layer opacities (0..255), EasyRPG's `fog_opacity` rows: the
/// back layer fades out entirely at the hacked strength 3, the front peaks opaque.
const FOG_BACK_OPACITY: [u8; 4] = [32, 64, 96, 0];
const FOG_FRONT_OPACITY: [u8; 4] = [64, 80, 160, 255];

/// Duplicate the left half for seamless horizontal wrapping; extra height covers vertical bob.
const FOG_W: usize = 640;
const FOG_H: usize = 280;
const FOG_HALF: usize = 320;

/// One fog overlay layer: how far it has scrolled, its leftward speed, and its
/// vertical bob (the front layer bobs; the back layer does not).
#[derive(Component)]
pub(super) struct FogLayer {
    scroll: f32,
    speed: f32,
    bob_amp: f32,
    bob_freq: f32,
    time: f32,
    z: f32,
}

/// The generated scrolling fog texture, built once at startup.
#[derive(Resource)]
pub(super) struct FogAssets {
    pub(super) texture: Handle<Image>,
}

/// Generate the fog texture (a subtle near-white noise, horizontally periodic)
/// and stash its handle for the fog layers to share.
pub(super) fn init_fog(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(FogAssets {
        texture: images.add(fog_image()),
    });
}

/// Spawn the two fog overlay layers at strength-derived opacities.
pub(super) fn spawn_fog(commands: &mut Commands, fog: &FogAssets, strength: i32, base: Vec2) {
    let level = strength.clamp(0, 3) as usize;
    commands.spawn(fog_sprite(
        fog,
        FOG_BACK_OPACITY[level] as f32 / 255.0,
        FogLayer {
            scroll: 0.0,
            speed: 15.0,
            bob_amp: 0.0,
            bob_freq: 0.0,
            time: 0.0,
            z: WEATHER_Z - 2.0,
        },
        base,
    ));
    commands.spawn(fog_sprite(
        fog,
        FOG_FRONT_OPACITY[level] as f32 / 255.0,
        FogLayer {
            scroll: 0.0,
            speed: 7.5,
            bob_amp: 8.0,
            bob_freq: 0.6,
            time: 0.0,
            z: WEATHER_Z - 1.0,
        },
        base,
    ));
}

/// One fog layer's sprite bundle: the shared texture at `alpha`, screen-pinned to
/// the camera `base` at its initial scroll offset.
fn fog_sprite(
    fog: &FogAssets,
    alpha: f32,
    layer: FogLayer,
    base: Vec2,
) -> (Sprite, Transform, WeatherEntity, FogLayer) {
    let sx = base.x + layer.scroll.rem_euclid(SCREEN_W) - SCREEN_W / 2.0;
    let z = layer.z;
    (
        Sprite {
            image: fog.texture.clone(),
            custom_size: Some(Vec2::new(FOG_W as f32, FOG_H as f32)),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            ..default()
        },
        Transform::from_xyz(sx, base.y, z),
        WeatherEntity,
        layer,
    )
}

/// Scroll and place the fog layers, screen-pinned to the camera. The horizontal
/// wrap uses the texture's [`SCREEN_W`] period so it is seamless.
pub(super) fn scroll_fog(
    time: Res<Time>,
    cameras: Query<&Transform, (With<MainCamera>, Without<FogLayer>)>,
    mut layers: Query<(&mut FogLayer, &mut Transform)>,
) {
    let Ok(base) = cameras.single().map(|t| t.translation.truncate()) else {
        return;
    };
    let dt = time.delta_secs();
    for (mut layer, mut transform) in &mut layers {
        layer.scroll -= layer.speed * dt;
        layer.time += dt;
        let sx = base.x + layer.scroll.rem_euclid(SCREEN_W) - SCREEN_W / 2.0;
        let bob = layer.bob_amp * (layer.time * layer.bob_freq).sin();
        transform.translation = Vec3::new(sx, base.y + bob, layer.z);
    }
}

/// Near-white noise with a duplicated right half for a [`SCREEN_W`]-pixel scroll period.
fn fog_image() -> Image {
    let mut data = vec![0u8; FOG_W * FOG_H * 4];
    let greys = [230u8, 240, 255];
    let mut rng = 0x9E37_79B9u32;
    for y in 0..FOG_H {
        for x in 0..FOG_HALF {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let grey = greys[(rng >> 16) as usize % greys.len()];
            let left = (y * FOG_W + x) * 4;
            let right = (y * FOG_W + x + FOG_HALF) * 4;
            for start in [left, right] {
                data[start] = grey;
                data[start + 1] = grey;
                data[start + 2] = grey;
                data[start + 3] = 255;
            }
        }
    }
    Image::new(
        Extent3d {
            width: FOG_W as u32,
            height: FOG_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}
