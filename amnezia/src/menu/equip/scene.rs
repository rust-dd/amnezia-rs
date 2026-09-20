use super::candidates;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::equipment::Equipment;
use crate::gamedata::GameData;
use crate::menu::{
    MenuOpen, MenuScreen, MenuState, derive,
    list_navigation::{Input, Navigation},
    save_files::SaveFiles,
};
use crate::progression::Progression;
use crate::state::{Inventory, Party};
use crate::vitals::Vitals;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(in crate::menu) struct Scene {
    pub lists: [Navigation<6>; 5],
    pub slot_frame: u32,
    pub current: [u32; 4],
    pub preview: Option<[u32; 4]>,
    pub help_id: u32,
    pub member: Option<usize>,
    actor: u32,
    pub picking: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn refresh_actor(
    open: Res<MenuOpen>,
    state: Res<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    inventory: Res<Inventory>,
    equipment: Res<Equipment>,
    vitals: Res<Vitals>,
    mut scene: ResMut<Scene>,
) {
    if !open.0 {
        return;
    }
    let MenuScreen::Equip { member, slot, .. } = state.screen else {
        return;
    };
    let Some(actor) = party.snapshot().get(member).and_then(|id| data.actor(*id)) else {
        return;
    };
    if scene.member == Some(member) && scene.actor == actor.id {
        return;
    }
    let worn = equipment.slots(actor);
    // Actor switching replaces Scene_Equip; no previous window state survives its first draw.
    *scene = Scene {
        member: Some(member),
        actor: actor.id,
        current: stats(actor, &data, &progression, &vitals, worn),
        help_id: worn[slot.min(4)],
        lists: std::array::from_fn(|slot| {
            Navigation::new(0, candidates(member, slot, &data, &party, &inventory).len())
        }),
        ..default()
    };
}

pub(in crate::menu) fn stats(
    actor: &amnezia_data::ActorDef,
    data: &GameData,
    progression: &Progression,
    vitals: &Vitals,
    slots: [u32; 5],
) -> [u32; 4] {
    let value = derive::field_stats(actor, progression.level(actor), &data.items, slots, vitals);
    [value.attack, value.defense, value.spirit, value.agility]
}

#[allow(clippy::too_many_arguments)]
pub(in crate::menu) fn update(
    input: Res<Input>,
    open: Res<MenuOpen>,
    mut state: ResMut<MenuState>,
    data: Res<GameData>,
    party: Res<Party>,
    progression: Res<Progression>,
    inventory: Res<Inventory>,
    equipment: Res<Equipment>,
    vitals: Res<Vitals>,
    files: Res<SaveFiles>,
    pause: crate::transitions::TransitionPause,
    fade: Res<crate::teleport::Fade>,
    mut scene: ResMut<Scene>,
    sounds: Option<Res<SystemSounds>>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if !open.0 || input.rewound || !matches!(state.screen, MenuScreen::Equip { .. }) {
        *scene = Scene::default();
        return;
    }
    if pause.paused() || fade.busy() || files.active() {
        return;
    }
    let MenuScreen::Equip {
        member,
        mut slot,
        picking,
    } = state.screen
    else {
        return;
    };
    slot = slot.min(4);
    let Some(actor) = party.snapshot().get(member).and_then(|id| data.actor(*id)) else {
        return;
    };
    if scene.member != Some(member) || scene.actor != actor.id {
        *scene = Scene {
            member: Some(member),
            actor: actor.id,
            ..default()
        };
    }
    let entries =
        std::array::from_fn::<_, 5, _>(|slot| candidates(member, slot, &data, &party, &inventory));
    if let Some(cursor) = picking {
        let nav = &scene.lists[slot];
        if scene.picking != Some(slot)
            || nav.index != cursor
            || nav.count() != entries[slot].len().max(1)
        {
            scene.lists[slot].select(cursor, entries[slot].len());
        }
    }
    scene.picking = picking.map(|_| slot);
    let worn = equipment.slots(actor);
    for repeated in input.slot_steps() {
        let mut moves = 0;
        if picking.is_none() {
            scene.slot_frame = (scene.slot_frame + u32::from(input.timed())) % 21;
            for (action, pressed) in repeated.into_iter().enumerate() {
                if !pressed {
                    continue;
                }
                let next = match action {
                    0 => (slot + 1) % 5,
                    1 => (slot + 4) % 5,
                    4 => 4,
                    5 => 0,
                    _ => slot,
                };
                moves += u32::from(next != slot);
                slot = next;
            }
        }
        scene.current = stats(actor, &data, &progression, &vitals, worn);
        scene.preview = picking.map(|_| {
            let id = entries[slot]
                .get(scene.lists[slot].index)
                .copied()
                .unwrap_or(0);
            stats(
                actor,
                &data,
                &progression,
                &vitals,
                crate::equipment::preview_slots(worn, slot, id, &data.items),
            )
        });
        for (index, items) in entries.iter().enumerate() {
            if picking.is_some() && index == slot {
                moves += scene.lists[index].tick(
                    [repeated[0], repeated[1], repeated[2], repeated[3]],
                    input.timed(),
                );
            } else {
                scene.lists[index].idle(items.len(), input.timed());
            }
        }
        scene.help_id = if picking.is_some() {
            entries[slot]
                .get(scene.lists[slot].help_index)
                .copied()
                .unwrap_or(0)
        } else {
            worn[slot]
        };
        if let Some(sounds) = &sounds {
            for _ in 0..moves {
                play_system_se(&mut audio, &sounds.cursor);
            }
        }
    }
    state.screen = MenuScreen::Equip {
        member,
        slot,
        picking: picking.map(|_| scene.lists[slot].index),
    };
}
