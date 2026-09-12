use super::*;

#[derive(Component)]
pub(crate) struct Canvas;

#[derive(Resource)]
struct Texture(Handle<Image>);

pub(super) fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(Image::new(
        Extent3d {
            width: 320,
            height: 240,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; 320 * 240 * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    ));
    commands.spawn((
        Sprite::from_image(image.clone()),
        Transform::default(),
        Visibility::Hidden,
        RenderLayers::layer(0),
        Canvas,
    ));
    commands.insert_resource(Texture(image));
}

#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Input<'w> {
    rain: Res<'w, Rain>,
    weather: Res<'w, Weather>,
    strength: Res<'w, WeatherStrength>,
    tint: Option<Res<'w, TintState>>,
    battle: Option<Res<'w, crate::battle::BattleActive>>,
    camera: Option<Res<'w, crate::player::CameraPan>>,
    shake: ScreenShake<'w>,
    texture: Res<'w, Texture>,
}

#[allow(clippy::type_complexity)]
pub(super) fn draw(
    input: Input,
    mut scroll: ResMut<Scroll>,
    mut changed: MessageReader<MapChanged>,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<&Transform, (With<MainCamera>, Without<Canvas>)>,
    mut canvas: Query<(&mut Transform, &mut RenderLayers, &mut Visibility), With<Canvas>>,
) {
    if changed.read().count() != 0 {
        scroll.previous = None;
    }
    let center = cameras
        .single()
        .map_or(Vec2::ZERO, |camera| camera.translation.truncate());
    let position = input.camera.as_ref().and_then(|pan| pan.position);
    if let Some(position) = position {
        if let Some(previous) = scroll.previous {
            let moved = (position - previous) * Vec2::new(-1.0, 1.0);
            scroll.pan = (scroll.pan + moved).rem_euclid(Vec2::new(320.0, 160.0));
        }
        scroll.previous = Some(position);
    }
    let Ok((mut transform, mut layers, mut visibility)) = canvas.single_mut() else {
        return;
    };
    if *input.weather != Weather::Rain {
        *visibility = Visibility::Hidden;
        return;
    }
    *visibility = Visibility::Inherited;
    if input.battle.as_ref().is_some_and(|battle| battle.0) {
        *layers = crate::animation::overlay_layer();
        transform.translation = Vec3::new(0.0, 0.0, 250.0);
    } else {
        *layers = RenderLayers::layer(0);
        transform.translation = center.extend(super::super::WEATHER_Z);
    }
    let surface = raster::surface(&input.rain.drops, input.strength.0);
    let tone = input.tint.as_ref().map_or([100.0; 4], |tint| tint.tone());
    let color = crate::legacy_colors::tone::apply([255; 3], tone);
    let shake = input.shake.offset();
    let offset = [
        -scroll.pan.x as i32 + shake.x as i32,
        -scroll.pan.y as i32 + shake.y as i32,
    ];
    let mut image = images.get_mut(&input.texture.0).unwrap();
    raster::viewport(&surface, offset, color, image.data.as_mut().unwrap());
}
