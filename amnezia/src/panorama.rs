use crate::assets::resolve_png;
use crate::world::{MainCamera, MapData, ScenePause};
use amnezia_data::PanoramaDef;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

mod motion;
mod render;
pub(crate) mod smoke;

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Panorama {
    map_id: Option<u32>,
    definition: Option<PanoramaDef>,
    scroll: (f32, f32),
    #[serde(default)]
    motion: Option<motion::Motion>,
    #[serde(default)]
    clock: crate::timing::GameFrames,
}

impl Default for Panorama {
    fn default() -> Self {
        Self {
            map_id: None,
            definition: None,
            scroll: (0.0, 0.0),
            motion: Some(default()),
            clock: default(),
        }
    }
}

impl Panorama {
    pub(crate) fn change(
        &mut self,
        map: &MapData,
        camera: &mut crate::player::CameraPan,
        definition: PanoramaDef,
    ) {
        self.sync_camera(map, Some(&mut *camera));
        self.definition = if definition.name.is_empty() {
            map.panorama.clone()
        } else {
            Some(definition)
        }
        .filter(|definition| !definition.name.is_empty());
        if self.definition.is_none() {
            self.motion.get_or_insert_default().initialize(
                "",
                UVec2::ZERO,
                None,
                map,
                camera.background_position(map).unwrap_or_default(),
            );
        }
    }

    fn sync_camera(&mut self, map: &MapData, camera: Option<&mut crate::player::CameraPan>) {
        if self.map_id != Some(map.map_id) {
            *self = Self {
                map_id: Some(map.map_id),
                definition: map
                    .panorama
                    .clone()
                    .filter(|definition| !definition.name.is_empty()),
                ..default()
            };
        }
        if let Some(camera) = camera {
            for event in camera.take_background_scroll() {
                if let Some(motion) = &mut self.motion {
                    motion.scroll(&event, self.definition.as_ref(), map);
                }
            }
        }
    }

    pub(crate) fn valid(&self) -> bool {
        self.scroll.0.is_finite()
            && self.scroll.1.is_finite()
            && self.motion.as_ref().is_none_or(motion::Motion::valid)
    }
}

#[derive(Component)]
struct PanoramaTile(i32, i32);

#[derive(Resource, Default)]
struct BackgroundImage(Option<(String, Handle<Image>)>);

impl BackgroundImage {
    fn load(&mut self, server: &AssetServer, name: &str) -> &Handle<Image> {
        if self.0.as_ref().is_none_or(|(previous, _)| previous != name) {
            self.0 = Some((name.to_owned(), server.load(resolve_png("Panorama", name))));
        }
        &self.0.as_ref().unwrap().1
    }
}

pub struct PanoramaPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PanoramaDraw;

impl Plugin for PanoramaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Panorama>()
            .init_resource::<BackgroundImage>()
            .add_systems(
                PostUpdate,
                render::draw
                    .in_set(PanoramaDraw)
                    .after(advance)
                    .after(crate::screenfx::ScreenShakeSet)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
        crate::timing::logical::post(app, || advance);
    }
}

fn camera_scroll(
    position: f32,
    map_size: f32,
    view_size: f32,
    image_size: f32,
    looping: bool,
) -> f32 {
    let offset = position + (map_size - view_size) / 2.0;
    if looping {
        -offset / 2.0
    } else if image_size > view_size && map_size > view_size {
        -offset.max(0.0) * (image_size - view_size).min(map_size - view_size)
            / (map_size - view_size)
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
fn advance(
    time: Res<Time>,
    data: Res<MapData>,
    server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    scene: ScenePause,
    mut panorama: ResMut<Panorama>,
    mut texture: ResMut<BackgroundImage>,
    mut camera: Option<ResMut<crate::player::CameraPan>>,
    transforms: Query<&Transform, With<MainCamera>>,
) {
    panorama.sync_camera(&data, camera.as_deref_mut());
    let display = camera
        .as_ref()
        .and_then(|camera| camera.background_position(&data))
        .unwrap_or_else(|| {
            let point = transforms
                .single()
                .map_or(Vec2::ZERO, |transform| transform.translation.truncate());
            Vec2::new(
                point.x + (data.width as f32 * 16.0 - 320.0) / 2.0,
                (data.height as f32 * 16.0 - 240.0) / 2.0 - point.y,
            )
        });
    let Some(definition) = panorama.definition.clone() else {
        texture.0 = None;
        panorama
            .motion
            .get_or_insert_default()
            .initialize("", UVec2::ZERO, None, &data, display);
        return;
    };
    let image = texture.load(&server, &definition.name);
    let Some(image) = images.get(image) else {
        return;
    };
    let size = image.size();
    if size.min_element() == 0 {
        return;
    }
    if panorama.motion.is_none() {
        let tracked = camera
            .as_ref()
            .and_then(|camera| {
                camera.panorama_position(&data, [definition.loop_x, definition.loop_y])
            })
            .unwrap_or(display);
        panorama.motion = Some(motion::Motion::migrate(
            &definition,
            &data,
            size,
            tracked,
            panorama.scroll,
        ));
        panorama.scroll = (0.0, 0.0);
    }
    panorama.motion.as_mut().unwrap().initialize(
        &definition.name,
        size,
        Some(&definition),
        &data,
        display,
    );
    if !scene.paused() {
        let before = panorama.clock.frame;
        panorama.clock.advance(time.delta_secs_f64());
        let frames = panorama.clock.frame.wrapping_sub(before);
        panorama.motion.as_mut().unwrap().step(&definition, frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod loading;
    mod logical;
    mod motion;
    mod ordering;
    mod phase;

    #[test]
    fn parallax_speed_and_camera_tracking_match_original_subpixels() {
        assert_eq!(super::motion::amount(0), 0);
        assert_eq!(super::motion::amount(3), -8);
        assert_eq!(super::motion::amount(-3), 8);
        assert_eq!(camera_scroll(0.0, 640.0, 320.0, 320.0, true), -80.0);
    }

    #[test]
    fn command_configuration_and_scroll_survive_save() {
        let mut panorama = Panorama::default();
        let mut map = MapData::for_test(20, 26);
        map.map_id = 126;
        panorama.change(
            &map,
            &mut crate::player::CameraPan::default(),
            PanoramaDef::from_command("Sky".into(), &[1, 1, 1, -3, 1, 2]),
        );
        panorama.scroll = (12.0, 8.0);
        let restored = ron::from_str::<Panorama>(&ron::to_string(&panorama).unwrap()).unwrap();
        assert_eq!(restored, panorama);
    }

    #[test]
    fn scrolling_backgrounds_keep_negative_phase_and_bounded_ones_cap_their_ratio() {
        assert_eq!(camera_scroll(-170.0, 640.0, 320.0, 320.0, true), 5.0);
        assert_eq!(camera_scroll(0.0, 640.0, 320.0, 960.0, false), -160.0);
        assert_eq!(camera_scroll(0.0, 640.0, 320.0, 480.0, false), -80.0);
    }
}
