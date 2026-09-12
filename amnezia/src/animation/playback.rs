use super::cells::CellRenderer;
use super::*;
use crate::timing::GameFrames;

pub(crate) mod saved;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AnimationSet {
    Advance,
    Start,
}

#[derive(Component)]
pub(super) struct LiveAnimation {
    pub(super) slot: AnimationSlot,
    pub(super) map_target: Option<AnimTarget>,
    global: bool,
    index: usize,
    draw_anchors: Vec<Vec2>,
    flash_anchors: Vec<Vec2>,
    pub(super) frame: usize,
    elapsed: u32,
    last: u32,
    duration: u32,
    sound_only: bool,
    pub(super) cells: Vec<Entity>,
}

pub(super) fn start_animations(
    frames: Res<GameFrames>,
    mut commands: Commands,
    mut renderer: CellRenderer,
    library: Res<AnimationLibrary>,
    mut requests: MessageReader<PlayAnimation>,
    mut animations: Query<(Entity, &mut LiveAnimation)>,
) {
    let mut pending = Vec::<(&PlayAnimation, usize)>::new();
    for request in requests.read() {
        let Some(index) = library.0.iter().position(|a| a.id == request.anim_id) else {
            continue;
        };
        pending.retain(|(previous, _)| previous.slot != request.slot);
        pending.push((request, index));
    }
    for (request, index) in pending {
        for (entity, mut previous) in &mut animations {
            if previous.slot == request.slot {
                cancel(&mut commands, entity, &mut previous);
            }
        }
        let def = &library.0[index];
        let mut duration = def.frames.len() as u32 * 2;
        if request.sound_only {
            duration = duration.min(40);
        }
        if duration == 0 {
            continue;
        }
        let draw_anchors = if request.sound_only {
            Vec::new()
        } else {
            draw_anchors(def, &request.targets, request.screen_center, request.global)
        };
        let cells = spawn_cells_at(&mut commands, &mut renderer, def, 0, &draw_anchors);
        commands.spawn(LiveAnimation {
            slot: request.slot,
            map_target: request.map_target,
            global: request.global,
            index,
            draw_anchors,
            flash_anchors: request.targets.iter().map(|t| t.pos).collect(),
            frame: 0,
            elapsed: 0,
            last: frames.frame,
            duration,
            sound_only: request.sound_only,
            cells,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn step_animations(
    frames: Res<GameFrames>,
    pause: crate::transitions::TransitionPause,
    scene: super::scene::Scenes,
    mut commands: Commands,
    mut renderer: CellRenderer,
    library: Res<AnimationLibrary>,
    mut audio: MessageWriter<AudioRequest>,
    mut battler_flash: MessageWriter<BattlerFlash>,
    mut animations: Query<(Entity, &mut LiveAnimation)>,
) {
    for (entity, mut anim) in &mut animations {
        let delta = frames.frame.wrapping_sub(anim.last);
        anim.last = frames.frame;
        if pause.paused() || scene.frozen() || delta == 0 {
            continue;
        }
        let def = &library.0[anim.index];
        let end = anim.elapsed.saturating_add(delta).min(anim.duration);
        // Timings run before the reference increments its frame; drawing runs after.
        for tick in anim.elapsed..end {
            if tick.is_multiple_of(2) {
                fire_timings(
                    &mut commands,
                    &mut audio,
                    &mut battler_flash,
                    def,
                    tick as usize / 2,
                    if anim.map_target.is_some() {
                        &[]
                    } else {
                        &anim.flash_anchors
                    },
                    (!anim.sound_only).then_some(FlashStamp {
                        age: anim.elapsed.saturating_add(delta) - tick - 1,
                        frame: frames.frame,
                    }),
                );
            }
        }
        if let Some(target) = anim.map_target {
            let tick = anim.elapsed.saturating_add(delta) - 1;
            let color = map::flash::color(def, tick, anim.duration);
            map::flash::write(&mut commands, target, color, frames.frame);
        }
        anim.elapsed = end;
        let frame = end as usize / 2;
        if frame == anim.frame && end < anim.duration {
            continue;
        }
        for cell in anim.cells.drain(..) {
            commands.entity(cell).despawn();
        }
        if end >= anim.duration {
            commands.entity(entity).despawn();
        } else {
            anim.frame = frame;
            anim.cells =
                spawn_cells_at(&mut commands, &mut renderer, def, frame, &anim.draw_anchors);
        }
    }
}

fn spawn_cells_at(
    commands: &mut Commands,
    renderer: &mut CellRenderer,
    def: &AnimationDef,
    frame: usize,
    bases: &[Vec2],
) -> Vec<Entity> {
    let mut cells = Vec::new();
    for &base in bases {
        cells.extend(renderer.spawn_frame(commands, def, frame, base));
    }
    cells
}

pub(super) fn cancel(commands: &mut Commands, entity: Entity, animation: &mut LiveAnimation) {
    for cell in animation.cells.drain(..) {
        commands.entity(cell).despawn();
    }
    commands.entity(entity).despawn();
}

pub(super) fn clear_map_animations(
    mut changes: MessageReader<crate::world::MapRebuilt>,
    mut commands: Commands,
    mut animations: Query<(Entity, &mut LiveAnimation)>,
    flashes: Query<Entity, With<render::FlashQuad>>,
) {
    if changes.read().count() == 0 {
        return;
    }
    for (entity, mut animation) in &mut animations {
        if animation.slot == AnimationSlot::Map {
            cancel(&mut commands, entity, &mut animation);
        }
    }
    for entity in &flashes {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn reset_transient(world: &mut World) {
    saved::reset(world);
    map::flash::reset(world);
    let entities = world
        .query::<(Entity, &LiveAnimation)>()
        .iter(world)
        .flat_map(|(entity, animation)| {
            std::iter::once(entity).chain(animation.cells.iter().copied())
        })
        .collect::<Vec<_>>();
    for entity in entities {
        world.despawn(entity);
    }
    let flashes = world
        .query_filtered::<Entity, With<render::FlashQuad>>()
        .iter(world)
        .collect::<Vec<_>>();
    for entity in flashes {
        world.despawn(entity);
    }
    world.insert_resource(ActiveAnimations::default());
    if let Some(mut flashes) = world.get_resource_mut::<Messages<BattlerFlash>>() {
        flashes.clear();
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn follow_map_animations(
    mut commands: Commands,
    targets: map::Targets,
    library: Res<AnimationLibrary>,
    mut animations: Query<(Entity, &mut LiveAnimation)>,
    mut cells: Query<&mut Transform, (Without<MainCamera>, Without<Player>, Without<EventSprite>)>,
) {
    for (entity, mut animation) in &mut animations {
        let Some(target) = animation.map_target else {
            continue;
        };
        let Some(anchor) = targets.anchor(target) else {
            cancel(&mut commands, entity, &mut animation);
            continue;
        };
        animation.flash_anchors = vec![anchor.pos];
        let def = &library.0[animation.index];
        if animation.global || animation.sound_only || def.scope == SCOPE_SCREEN {
            continue;
        }
        let next = anchor.pos + Vec2::new(0.0, position_offset(def.position, anchor.height));
        let delta = overlay_translation(next - animation.draw_anchors[0], 0.0);
        if delta == Vec3::ZERO {
            continue;
        }
        animation.draw_anchors[0] = next;
        for &cell in &animation.cells {
            if let Ok(mut transform) = cells.get_mut(cell) {
                transform.translation += delta;
            }
        }
    }
}

pub(super) fn track_active_animations(
    animations: Query<&LiveAnimation>,
    mut active: ResMut<ActiveAnimations>,
) {
    let mut next = ActiveAnimations::default();
    for animation in &animations {
        next.total += 1;
        next.battle += usize::from(animation.slot != AnimationSlot::Map);
    }
    active.set_if_neq(next);
}
