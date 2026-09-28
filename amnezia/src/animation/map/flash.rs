use super::*;
use crate::legacy_colors::flash::SpriteFlash;
use crate::timing::GameFrames;

#[derive(Component)]
struct CharacterFlash {
    frame: u32,
    callback: bool,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::animation) struct Expire;

pub(in crate::animation) fn register(app: &mut App) {
    app.add_message::<crate::world::MapRebuilt>()
        .add_message::<crate::world::MapEffectsReset>();
    crate::teleport::rebuild::register(
        app,
        crate::teleport::rebuild::Stage::Reset,
        clear_on_map_change,
    );
    crate::timing::logical::post(app, || {
        expire
            .in_set(Expire)
            .before(crate::legacy_colors::world::WorldColors)
    });
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
            let callback = world
                .get_resource::<crate::timing::logical::Step>()
                .is_some_and(|step| step.callback);
            world
                .entity_mut(entity)
                .insert((CharacterFlash { frame, callback }, SpriteFlash(color)));
        }
    });
}

fn expire(
    mut commands: Commands,
    frames: Res<GameFrames>,
    step: Option<Res<crate::timing::logical::Step>>,
    pause: crate::transitions::TransitionPause,
    scene: crate::animation::scene::Scenes,
    mut flashes: Query<(Entity, &mut CharacterFlash, &mut SpriteFlash)>,
) {
    let callback = step.is_some_and(|step| step.callback);
    for (entity, mut state, mut flash) in &mut flashes {
        if !pause.paused()
            && !scene.frozen()
            && (state.frame != frames.frame || state.callback != callback)
        {
            // Character flashes last one game tick; a live animation refreshes them.
            flash.0 = [0; 4];
            commands.entity(entity).remove::<CharacterFlash>();
        } else {
            state.frame = frames.frame;
            state.callback = callback;
        }
    }
}

fn clear_on_map_change(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<crate::world::MapEffectsReset>>,
) {
    if cursor
        .read(world.resource::<Messages<crate::world::MapEffectsReset>>())
        .count()
        != 0
    {
        reset(world);
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn callback_expires_old_flashes_but_keeps_the_flash_written_in_that_visit() {
        let mut world = World::new();
        world.init_resource::<GameFrames>();
        world.resource_mut::<GameFrames>().frame = 42;
        world.init_resource::<crate::timing::logical::Step>();
        world
            .resource_mut::<crate::timing::logical::Step>()
            .callback = true;
        let spawn = |world: &mut World, callback| {
            world
                .spawn((
                    CharacterFlash {
                        frame: 42,
                        callback,
                    },
                    SpriteFlash([248; 4]),
                ))
                .id()
        };
        let old = spawn(&mut world, false);
        let current = spawn(&mut world, true);
        world.run_system_once(expire).unwrap();
        assert_eq!(world.get::<SpriteFlash>(old).unwrap().0, [0; 4]);
        assert!(world.get::<CharacterFlash>(old).is_none());
        assert_eq!(world.get::<SpriteFlash>(current).unwrap().0, [248; 4]);
        world.resource_mut::<GameFrames>().frame = 43;
        world
            .resource_mut::<crate::timing::logical::Step>()
            .callback = false;
        world.run_system_once(expire).unwrap();
        assert_eq!(world.get::<SpriteFlash>(current).unwrap().0, [0; 4]);
    }
}
