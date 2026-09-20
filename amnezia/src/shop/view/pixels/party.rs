use super::*;
use amnezia_data::{ActorDef, ItemDef};

pub(super) fn draw(canvas: &mut Canvas, world: &World, skin: &Image, item_id: u32, frame: u32) {
    let data = world.resource::<GameData>();
    let images = world.resource::<Assets<Image>>();
    let server = world.resource::<AssetServer>();
    let item = data.item(item_id);
    let equipment = world.resource::<crate::equipment::Equipment>();
    let progression = world.resource::<crate::progression::Progression>();
    let appearance = world.resource::<crate::appearance::Appearance>();
    let cycle = frame / 12;
    let phase = [0, 1, 2, 1][cycle as usize];
    for (slot, id) in world
        .resource::<Party>()
        .snapshot()
        .into_iter()
        .take(4)
        .enumerate()
    {
        let actor = data.actor(id).unwrap();
        let usable = item.is_none_or(|item| {
            item.actor_set
                .get((id - 1) as usize)
                .copied()
                .unwrap_or(true)
                && !(actor.two_weapons && item.item_type == 2)
        });
        let (name, index) = appearance
            .get(id)
            .unwrap_or((&actor.character_name, actor.character_index));
        if !name.is_empty() {
            let image = server.load::<Image>(crate::assets::resolve_png("CharSet", name));
            let image = images.get(&image).unwrap();
            let (width, height) = (image.width() / 12, image.height() / 8);
            let x = (index % 4 * 3 + if usable { phase } else { 1 }) * width;
            let y = (index / 4 * 4 + 2) * height;
            canvas.blit(
                image,
                (188 + slot as u32 * 32, 36),
                (x, y, width.min(128 - slot as u32 * 32), height.min(32)),
                !usable,
            );
        }
        let Some(item) = item else { break };
        if usable && (1..=5).contains(&item.item_type) {
            let slots = equipment.slots(actor);
            let (row, phase) = if slots.contains(&item.id) {
                (24, phase)
            } else {
                let delta = compare(data, actor, progression.level(actor), slots, item);
                (
                    if delta > 0 {
                        0
                    } else if delta < 0 {
                        16
                    } else {
                        8
                    },
                    cycle,
                )
            };
            canvas.blit(
                skin,
                (208 + slot as u32 * 32, 60),
                (128 + phase * 8, row, 8, 8),
                false,
            );
        }
    }
}

fn compare(data: &GameData, actor: &ActorDef, level: u32, slots: [u32; 5], item: &ItemDef) -> i32 {
    let base = [
        &actor.curves.attack,
        &actor.curves.defense,
        &actor.curves.spirit,
        &actor.curves.agility,
    ]
    .map(|curve| {
        curve
            .get(level.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(1)
            .clamp(1, 999) as i32
    });
    let mut before = base;
    for current in slots.into_iter().filter_map(|id| data.item(id)) {
        add(&mut before, current, 1);
    }
    let old_score = before
        .map(|value| value.clamp(1, 999))
        .into_iter()
        .sum::<i32>();
    if let Some(current) = data.item(slots[(item.item_type - 1) as usize]) {
        add(&mut before, current, -1);
    }
    if item.item_type <= 2
        && let Some(other) = data.item(slots[(2 - item.item_type) as usize])
        && ((other.item_type == 1 && other.two_handed) || (item.item_type == 1 && item.two_handed))
    {
        add(&mut before, other, -1);
    }
    add(&mut before, item, 1);
    before
        .map(|value| value.clamp(1, 999))
        .into_iter()
        .sum::<i32>()
        - old_score
}

fn add(stats: &mut [i32; 4], item: &ItemDef, factor: i32) {
    for (value, bonus) in stats
        .iter_mut()
        .zip([item.atk, item.def, item.spi, item.agi])
    {
        *value += bonus as i32 * factor;
    }
}
