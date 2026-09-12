use super::{Weather, WeatherStrength};
use crate::screenfx::{ScreenShake, TintState};
use crate::world::{MainCamera, MapChanged, ScenePause};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use model::Rain;

mod model;
mod raster;
pub(crate) mod smoke;
mod view;

pub(crate) use view::Canvas;

#[derive(Resource, Default, Clone, Debug, PartialEq)]
struct Scroll {
    pan: Vec2,
    previous: Option<Vec2>,
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<Rain>()
        .init_resource::<Scroll>()
        .add_message::<MapChanged>()
        .add_systems(Startup, view::setup)
        .add_systems(Update, step.after(crate::interpreter::InterpreterStep))
        .add_systems(
            PostUpdate,
            view::draw
                .after(crate::screenfx::ScreenShakeSet)
                .before(bevy::transform::TransformSystems::Propagate),
        );
}

fn step(time: Res<Time>, scene: ScenePause, weather: Res<Weather>, mut rain: ResMut<Rain>) {
    if *weather == Weather::Rain && !scene.screen_effects_paused() {
        rain.advance(time.delta_secs_f64());
    }
}

pub(in crate::screenfx) fn reset(world: &mut World) {
    if world.contains_resource::<Rain>() {
        world.insert_resource(Rain::default());
        world.insert_resource(Scroll::default());
    }
}

#[cfg(test)]
mod tests;
