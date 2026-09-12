use super::{Anim, Picture, Tween, effects::EffectState, render};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Asset handles and texture dimensions are rebuilt from the saved graphic name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct PictureState {
    pub id: u32,
    pub name: String,
    pub visual: Anim,
    pub use_transparent_color: bool,
    pub fixed_to_map: bool,
    pub world_anchor: Option<[f32; 2]>,
    pub tween: Option<Tween>,
    pub effect: EffectState,
    pub frame_fraction: f64,
}

impl Picture {
    fn snapshot(&self) -> PictureState {
        PictureState {
            id: self.id,
            name: self.name.clone(),
            visual: self.anim(),
            use_transparent_color: self.use_transparent_color,
            fixed_to_map: self.fixed_to_map,
            world_anchor: self.world_anchor.map(|point| point.to_array()),
            tween: self.tween,
            effect: self.effect,
            frame_fraction: self.frame_fraction,
        }
    }
}

impl Anim {
    fn valid(&self) -> bool {
        [
            self.x,
            self.y,
            self.transparency,
            self.zoom,
            self.tone.r,
            self.tone.g,
            self.tone.b,
            self.tone.sat,
        ]
        .into_iter()
        .all(f32::is_finite)
    }
}

impl PictureState {
    fn valid(&self) -> bool {
        self.id > 0
            && !self.name.is_empty()
            && self.visual.valid()
            && self.fixed_to_map == self.world_anchor.is_some()
            && self
                .world_anchor
                .iter()
                .flatten()
                .all(|value| value.is_finite())
            && self.frame_fraction.is_finite()
            && (0.0..1.0).contains(&self.frame_fraction)
            && self.effect.valid()
            && self.tween.is_none_or(|tween| {
                tween.frames > 0
                    && tween.elapsed < tween.frames
                    && tween.from.valid()
                    && tween.to.valid()
            })
    }

    fn into_picture(self) -> Picture {
        Picture {
            id: self.id,
            name: self.name,
            x: self.visual.x,
            y: self.visual.y,
            transparency: self.visual.transparency,
            zoom: self.visual.zoom,
            tone: self.visual.tone,
            use_transparent_color: self.use_transparent_color,
            fixed_to_map: self.fixed_to_map,
            world_anchor: self.world_anchor.map(Vec2::from_array),
            base_size: None,
            tween: self.tween,
            effect: self.effect,
            frame_fraction: self.frame_fraction,
        }
    }
}

pub(crate) fn valid(pictures: &[PictureState]) -> bool {
    let mut ids = std::collections::HashSet::new();
    pictures
        .iter()
        .all(|picture| picture.valid() && ids.insert(picture.id))
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Capture<'w, 's> {
    pictures: Query<'w, 's, &'static Picture>,
}

impl Capture<'_, '_> {
    pub(crate) fn snapshot(&self) -> Vec<PictureState> {
        let mut pictures = self
            .pictures
            .iter()
            .map(Picture::snapshot)
            .collect::<Vec<_>>();
        pictures.sort_unstable_by_key(|picture| picture.id);
        pictures
    }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    pictures: Vec<PictureState>,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, pictures: Vec<PictureState>) {
    world.remove_resource::<Pending>();
    if !pictures.is_empty() {
        world.insert_resource(Pending { map_id, pictures });
    }
}

pub(super) fn restore(
    world: &mut World,
    mut changes: Local<bevy::ecs::message::MessageCursor<crate::world::MapRebuilt>>,
) {
    if changes
        .read(world.resource::<Messages<crate::world::MapRebuilt>>())
        .count()
        == 0
    {
        return;
    }
    let Some(pending) = world.get_resource::<Pending>() else {
        return;
    };
    if world
        .get_resource::<crate::world::MapData>()
        .is_none_or(|map| map.map_id != pending.map_id)
    {
        return;
    }
    let pending = world.remove_resource::<Pending>().unwrap();
    for picture in pending.pictures {
        render::spawn_picture(world, picture.into_picture());
    }
}
