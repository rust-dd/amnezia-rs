use super::{
    flash::SpriteFlash,
    raster::{self, Rasterized},
    tone::SpriteTone,
};
use crate::screenfx::TintState;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
struct WorldImage {
    source: Handle<Image>,
    rendered: Handle<Image>,
}

#[derive(Resource, Default)]
struct Cache(HashMap<AssetId<Image>, Rasterized>);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorldColors;

pub(crate) fn register(app: &mut App) {
    app.init_resource::<Cache>()
        .add_message::<crate::world::MapChanged>()
        .add_systems(
            PostUpdate,
            apply
                .in_set(WorldColors)
                .after(super::hue::HueSet)
                .after(crate::vehicles::VehicleDisplay)
                .after(crate::panorama::PanoramaDraw)
                .before(bevy::transform::TransformSystems::Propagate),
        );
}

#[allow(clippy::type_complexity)]
fn apply(
    mut commands: Commands,
    tint: Res<TintState>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<Cache>,
    mut changed: MessageReader<crate::world::MapChanged>,
    mut sprites: Query<
        (
            Entity,
            &mut Sprite,
            Option<&RenderLayers>,
            Option<&mut WorldImage>,
            Option<&SpriteFlash>,
            Option<&mut Rasterized>,
        ),
        (
            Without<crate::world::BushBottom>,
            Without<super::hue::HueShift>,
        ),
    >,
) {
    if changed.read().count() != 0 {
        cache.0.clear();
    }
    let tone = SpriteTone(tint.tone());
    for (entity, mut sprite, layers, previous, flash, rasterized) in &mut sprites {
        let source = previous
            .as_ref()
            .filter(|previous| sprite.image == previous.rendered)
            .map_or_else(|| sprite.image.clone(), |previous| previous.source.clone());
        if layers.is_some_and(|layers| !layers.intersects(&RenderLayers::layer(0))) {
            if previous.is_some() {
                sprite.image = source;
                commands.entity(entity).remove::<WorldImage>();
            }
            continue;
        }
        let rendered = if let Some(flash) = flash.filter(|flash| flash.0[3] != 0)
            && let Some(mut rasterized) = rasterized
        {
            raster::render(
                &source,
                Some(flash),
                Some(&tone),
                &mut rasterized,
                &mut images,
            )
        } else {
            // Thousands of map quarters can share one toned chipset bitmap.
            raster::render(
                &source,
                None,
                Some(&tone),
                cache.0.entry(source.id()).or_default(),
                &mut images,
            )
        };
        if sprite.image != rendered {
            sprite.image = rendered.clone();
        }
        let value = WorldImage { source, rendered };
        if let Some(mut previous) = previous {
            *previous = value;
        } else {
            commands.entity(entity).insert(value);
        }
    }
}

#[cfg(test)]
mod tests;
