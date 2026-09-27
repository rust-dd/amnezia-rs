use super::*;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn draw(
    mut commands: Commands,
    texture: Res<BackgroundImage>,
    images: Res<Assets<Image>>,
    panorama: Res<Panorama>,
    shake: crate::screenfx::ScreenShake,
    mut previous: Local<Option<(String, UVec2)>>,
    camera: Query<&Transform, (With<MainCamera>, Without<PanoramaTile>)>,
    mut tiles: Query<(Entity, &PanoramaTile, &mut Transform), Without<MainCamera>>,
) {
    if panorama.definition.is_none() {
        if previous.take().is_some() {
            for (entity, _, _) in &tiles {
                commands.entity(entity).despawn();
            }
        }
        return;
    }
    let Some((name, image)) = &texture.0 else {
        return;
    };
    let Some(loaded) = images.get(image) else {
        return;
    };
    let size = loaded.size();
    if size.min_element() == 0 {
        return;
    }
    let Some(motion) = panorama.motion.as_ref() else {
        return;
    };
    let Ok(camera) = camera.single() else { return };
    let origin = camera.translation.truncate() - shake.offset();
    let offset = motion.offset();
    let translation = |x, y| {
        Vec3::new(
            origin.x - 160.0 + (x as f32 + 0.5) * size.x as f32 + offset.x,
            origin.y + 120.0 - (y as f32 + 0.5) * size.y as f32 - offset.y,
            -10.0,
        )
    };
    if previous.as_ref() != Some(&(name.clone(), size)) {
        for (entity, _, _) in &tiles {
            commands.entity(entity).despawn();
        }
        for y in -1..=(240 / size.y + 1) as i32 {
            for x in -1..=(320 / size.x + 1) as i32 {
                commands.spawn((
                    PanoramaTile(x, y),
                    Sprite {
                        image: image.clone(),
                        custom_size: Some(size.as_vec2()),
                        ..default()
                    },
                    Transform::from_translation(translation(x, y)),
                ));
            }
        }
        *previous = Some((name.clone(), size));
        return;
    }
    for (_, tile, mut transform) in &mut tiles {
        transform.translation = translation(tile.0, tile.1);
    }
}
