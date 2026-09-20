use super::*;
use crate::appearance::Appearance;
use crate::equipment::Equipment;
use crate::progression::Progression;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(in crate::shop) struct Cache {
    sources: HashMap<AssetId<Image>, Handle<Image>>,
    gray: HashMap<AssetId<Image>, Handle<Image>>,
}

impl Cache {
    fn retain(&mut self, handle: &Handle<Image>) {
        self.sources
            .entry(handle.id())
            .or_insert_with(|| handle.clone());
    }
}

fn gray(
    source: &Handle<Image>,
    images: &mut Assets<Image>,
    cache: &mut Cache,
) -> Option<Handle<Image>> {
    cache.retain(source);
    if let Some(handle) = cache.gray.get(&source.id()) {
        return Some(handle.clone());
    }
    let mut image = images.get(source)?.clone();
    for pixel in image.data.as_mut()?.as_chunks_mut::<4>().0 {
        let color = crate::legacy_colors::tone::apply(
            [pixel[0], pixel[1], pixel[2]],
            [100.0, 100.0, 100.0, 0.0],
        );
        pixel[..3].copy_from_slice(&color);
    }
    let handle = images.add(image);
    cache.gray.insert(source.id(), handle.clone());
    Some(handle)
}

#[derive(SystemParam)]
pub(in crate::shop) struct Context<'w> {
    screen: Res<'w, Screen>,
    data: Res<'w, GameData>,
    party: Res<'w, Party>,
    appearance: Res<'w, Appearance>,
    equipment: Res<'w, Equipment>,
    progression: Res<'w, Progression>,
    server: Res<'w, AssetServer>,
    images: ResMut<'w, Assets<Image>>,
    cache: ResMut<'w, Cache>,
}

pub(in crate::shop) fn update(
    mut context: Context,
    mut events: MessageReader<AssetEvent<Image>>,
    mut parts: Query<(&Part, &mut Node, &mut Visibility, &mut ImageNode)>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } | AssetEvent::Removed { id } = event {
            context.cache.gray.remove(id);
        }
    }
    let Screen::Shop(state) = &*context.screen else {
        return;
    };
    let item_id = state.scene.item_id;
    let cycle = state.scene.party_frame / 12;
    let phase = if cycle == 3 { 1 } else { cycle };
    let members = context.party.snapshot();
    for (part, mut node, mut visibility, mut image) in &mut parts {
        let (Part::Character(member) | Part::Indicator(member)) = part else {
            continue;
        };
        let actor = members
            .get(*member)
            .filter(|_| item_id != 0 || *member == 0)
            .and_then(|id| context.data.actor(*id));
        *visibility = visible(actor.is_some());
        let Some(actor) = actor else { continue };
        let item = context.data.item(item_id);
        let usable = item.is_none_or(|item| {
            item.usable_by_actor(actor.id) && !(actor.two_weapons && item.item_type == 2)
        });
        if matches!(part, Part::Character(_)) {
            let (name, index) = context
                .appearance
                .get(actor.id)
                .unwrap_or((&actor.character_name, actor.character_index));
            if name.is_empty() {
                *visibility = Visibility::Hidden;
                continue;
            }
            let source = context
                .server
                .load::<Image>(crate::assets::resolve_png("CharSet", name));
            context.cache.retain(&source);
            let Some(bitmap) = context.images.get(&source) else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let width = bitmap.width() / 12;
            let height = bitmap.height() / 8;
            let x = (index % 4 * 3 + if usable { phase } else { 1 }) * width;
            let y = (index / 4 * 4 + 2) * height;
            image.rect = Some(Rect::new(
                x as f32,
                y as f32,
                (x + width) as f32,
                (y + height) as f32,
            ));
            node.width = Val::Px(width as f32 * 3.0);
            node.height = Val::Px(height as f32 * 3.0);
            if usable {
                image.image = source;
            } else {
                let Context { images, cache, .. } = &mut context;
                if let Some(handle) = gray(&source, images, cache) {
                    image.image = handle;
                }
            }
        } else {
            let Some(item) = item.filter(|item| usable && (1..=5).contains(&item.item_type)) else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let slots = context.equipment.slots(actor);
            let (row, phase) = if slots.contains(&item.id) {
                (24, phase)
            } else {
                let comparison = comparison(
                    actor,
                    context.progression.level(actor),
                    &context.data,
                    slots,
                    item,
                );
                (
                    match comparison {
                        std::cmp::Ordering::Greater => 0,
                        std::cmp::Ordering::Equal => 8,
                        std::cmp::Ordering::Less => 16,
                    },
                    cycle,
                )
            };
            let x = 128 + phase * 8;
            image.rect = Some(Rect::new(
                x as f32,
                row as f32,
                (x + 8) as f32,
                (row + 8) as f32,
            ));
        }
    }
}

pub(super) fn comparison(
    actor: &amnezia_data::ActorDef,
    level: u32,
    data: &GameData,
    slots: [u32; 5],
    item: &amnezia_data::ItemDef,
) -> std::cmp::Ordering {
    let mut next = slots;
    let slot = item.item_type.saturating_sub(1).min(4) as usize;
    next[slot] = item.id;
    if slot < 2 {
        let other = 1 - slot;
        if data.item(slots[other]).is_some_and(|old| {
            (old.item_type == 1 && old.two_handed) || (item.item_type == 1 && item.two_handed)
        }) {
            next[other] = 0;
        }
    }
    score(actor, level, data, next).cmp(&score(actor, level, data, slots))
}

fn score(actor: &amnezia_data::ActorDef, level: u32, data: &GameData, slots: [u32; 5]) -> u32 {
    let index = level.saturating_sub(1) as usize;
    let mut values = [
        &actor.curves.attack,
        &actor.curves.defense,
        &actor.curves.spirit,
        &actor.curves.agility,
    ]
    .map(|curve| curve.get(index).copied().unwrap_or(1).clamp(1, 999));
    for item in slots.into_iter().filter_map(|id| data.item(id)) {
        for (value, bonus) in values
            .iter_mut()
            .zip([item.atk, item.def, item.spi, item.agi])
        {
            *value = value.saturating_add(bonus);
        }
    }
    values.map(|value| value.clamp(1, 999)).into_iter().sum()
}
