use super::cells::CellRenderer;
use super::*;
use crate::timing::GameFrames;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AnimationSet {
    Advance,
    Start,
}

#[derive(Component)]
pub(super) struct LiveAnimation {
    index: usize,
    draw_anchors: Vec<Vec2>,
    flash_anchors: Vec<Vec2>,
    pub(super) frame: usize,
    elapsed: u32,
    last: u32,
    duration: u32,
    sound_only: bool,
    cells: Vec<Entity>,
}

pub(super) fn start_animations(
    frames: Res<GameFrames>,
    mut commands: Commands,
    mut renderer: CellRenderer,
    library: Res<AnimationLibrary>,
    mut requests: MessageReader<PlayAnimation>,
) {
    for request in requests.read() {
        let Some(index) = library.0.iter().position(|a| a.id == request.anim_id) else {
            continue;
        };
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
        if pause.paused() || delta == 0 {
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
                    &anim.flash_anchors,
                    (!anim.sound_only).then_some(FlashStamp {
                        age: anim.elapsed.saturating_add(delta) - tick - 1,
                        frame: frames.frame,
                    }),
                );
            }
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

pub(super) fn track_active_animations(
    animations: Query<(), With<LiveAnimation>>,
    mut active: ResMut<ActiveAnimations>,
) {
    let count = animations.iter().count();
    if active.0 != count {
        active.0 = count;
    }
}
