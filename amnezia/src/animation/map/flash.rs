use super::*;
use crate::legacy_colors::flash::SpriteFlash;
use crate::timing::GameFrames;

#[derive(Component)]
struct CharacterFlash {
    frame: u32,
}

pub(in crate::animation) fn register(app: &mut App) {
    app.add_message::<crate::world::MapChanged>().add_systems(
        PostUpdate,
        expire.before(crate::legacy_colors::world::WorldColors),
    );
}

pub(in crate::animation) fn color(def: &AnimationDef, tick: u32, duration: u32) -> [u8; 4] {
    if tick >= duration {
        return [0; 4];
    }
    let timing = def
        .timings
        .iter()
        .enumerate()
        .filter(|(_, timing)| {
            timing.flash_scope == FLASH_SCOPE_TARGET
                && timing.frame > 0
                && (timing.frame - 1) * 2 <= tick
        })
        .max_by_key(|(index, timing)| (timing.frame, *index));
    let Some((_, timing)) = timing else {
        return [0; 4];
    };
    let age = tick - (timing.frame - 1) * 2;
    if age > 10 {
        return [0; 4];
    }
    [
        flash_channel(timing.flash_red),
        flash_channel(timing.flash_green),
        flash_channel(timing.flash_blue),
        (flash_power_level(age, timing.flash_power) * 8) as u8,
    ]
}

pub(in crate::animation) fn write(
    commands: &mut Commands,
    target: AnimTarget,
    color: [u8; 4],
    frame: u32,
) {
    commands.queue(move |world: &mut World| {
        let entity = match target {
            AnimTarget::Hero => world
                .query_filtered::<Entity, (With<Player>, With<Sprite>)>()
                .single(world)
                .ok(),
            AnimTarget::Event(id) => world
                .query_filtered::<(Entity, &EventSprite), With<Sprite>>()
                .iter(world)
                .find_map(|(entity, event)| (event.id == id).then_some(entity)),
        };
        if let Some(entity) = entity {
            world
                .entity_mut(entity)
                .insert((CharacterFlash { frame }, SpriteFlash(color)));
        }
    });
}

fn expire(
    mut commands: Commands,
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    mut changes: MessageReader<crate::world::MapChanged>,
    mut flashes: Query<(Entity, &mut CharacterFlash, &mut SpriteFlash)>,
) {
    let changed = changes.read().count() != 0;
    for (entity, mut state, mut flash) in &mut flashes {
        if changed || (!pause.paused() && state.frame != frames.frame) {
            // Character flashes last one game tick; a live animation refreshes them.
            flash.0 = [0; 4];
            commands.entity(entity).remove::<CharacterFlash>();
        } else {
            state.frame = frames.frame;
        }
    }
}

pub(in crate::animation) fn reset(world: &mut World) {
    let entities = world
        .query_filtered::<Entity, With<CharacterFlash>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in entities {
        world.entity_mut(entity).remove::<CharacterFlash>();
        if let Some(mut flash) = world.get_mut::<SpriteFlash>(entity) {
            flash.0 = [0; 4];
        }
    }
}
