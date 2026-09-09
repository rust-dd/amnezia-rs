use crate::assets::resolve_png;
use crate::world::{MainCamera, MapData, ScenePause};
use amnezia_data::PanoramaDef;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Panorama {
    map_id: Option<u32>,
    definition: Option<PanoramaDef>,
    scroll: (f32, f32),
}

impl Panorama {
    pub fn change(&mut self, map_id: u32, definition: PanoramaDef) {
        self.map_id = Some(map_id);
        self.definition = (!definition.name.is_empty()).then_some(definition);
        self.scroll = (0.0, 0.0);
    }
}

#[derive(Component)]
struct PanoramaTile(i32, i32);

pub struct PanoramaPlugin;

impl Plugin for PanoramaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panorama>().add_systems(
            PostUpdate,
            draw.after(crate::screenfx::ScreenShakeSet)
                .before(bevy::transform::TransformSystems::Propagate),
        );
    }
}

fn speed(value: i32) -> f32 {
    if value == 0 {
        0.0
    } else {
        value.signum() as f32 * 2f32.powi(value.saturating_abs().min(12)) * 60.0 / 32.0
    }
}

fn camera_scroll(
    position: f32,
    map_size: f32,
    view_size: f32,
    image_size: f32,
    looping: bool,
) -> f32 {
    let offset = (position + (map_size - view_size) / 2.0).max(0.0);
    if looping {
        -offset / 2.0
    } else if image_size > view_size && map_size > view_size {
        -offset * (image_size - view_size) / (map_size - view_size)
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<MapData>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    scene: ScenePause,
    mut panorama: ResMut<Panorama>,
    mut previous: Local<Option<(PanoramaDef, UVec2)>>,
    camera: Query<&Transform, (With<MainCamera>, Without<PanoramaTile>)>,
    mut tiles: Query<(Entity, &PanoramaTile, &mut Transform), Without<MainCamera>>,
) {
    if panorama.map_id != Some(data.map_id) {
        panorama.map_id = Some(data.map_id);
        panorama.definition = data.panorama.clone();
        panorama.scroll = (0.0, 0.0);
    }
    let Some(definition) = panorama.definition.clone() else {
        if previous.take().is_some() {
            for (entity, _, _) in &tiles {
                commands.entity(entity).despawn();
            }
        }
        return;
    };
    let image = server.load(resolve_png("Panorama", &definition.name));
    let Some(loaded) = images.get(&image) else {
        return;
    };
    let size = loaded.size();
    if size.x == 0 || size.y == 0 {
        return;
    }
    if previous.as_ref() != Some(&(definition.clone(), size)) {
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
                    Transform::from_xyz(0.0, 0.0, -10.0),
                ));
            }
        }
        *previous = Some((definition.clone(), size));
        return;
    }
    let Ok(camera) = camera.single() else { return };
    if !scene.paused() {
        if definition.loop_x && definition.auto_x {
            panorama.scroll.0 += speed(definition.speed_x) * time.delta_secs();
        }
        if definition.loop_y && definition.auto_y {
            panorama.scroll.1 += speed(definition.speed_y) * time.delta_secs();
        }
    }
    panorama.scroll.0 = panorama.scroll.0.rem_euclid(size.x as f32);
    panorama.scroll.1 = panorama.scroll.1.rem_euclid(size.y as f32);
    let x_scroll = camera_scroll(
        camera.translation.x,
        data.width as f32 * 16.0,
        320.0,
        size.x as f32,
        definition.loop_x,
    );
    let y_scroll = camera_scroll(
        -camera.translation.y,
        data.height as f32 * 16.0,
        240.0,
        size.y as f32,
        definition.loop_y,
    );
    let x_offset = (x_scroll + panorama.scroll.0).rem_euclid(size.x as f32);
    let y_offset = (y_scroll + panorama.scroll.1).rem_euclid(size.y as f32);
    for (_, tile, mut transform) in &mut tiles {
        transform.translation = Vec3::new(
            camera.translation.x - 160.0 + (tile.0 as f32 + 0.5) * size.x as f32 + x_offset,
            camera.translation.y + 120.0 - (tile.1 as f32 + 0.5) * size.y as f32 - y_offset,
            -10.0,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallax_speed_and_camera_tracking_match_original_subpixels() {
        assert_eq!(speed(0), 0.0);
        assert_eq!(speed(3), 15.0);
        assert_eq!(speed(-3), -15.0);
        assert_eq!(camera_scroll(0.0, 640.0, 320.0, 320.0, true), -80.0);
    }

    #[test]
    fn command_configuration_and_scroll_survive_save() {
        let mut panorama = Panorama::default();
        panorama.change(
            126,
            PanoramaDef::from_command("Sky".into(), &[1, 1, 1, -3, 1, 2]),
        );
        panorama.scroll = (12.0, 8.0);
        let restored = ron::from_str::<Panorama>(&ron::to_string(&panorama).unwrap()).unwrap();
        assert_eq!(restored, panorama);
    }
}
