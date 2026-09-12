use super::*;
use render::{FlashQuad, saved::FlashState};
use serde::{Deserialize, Serialize};

pub(crate) mod smoke;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MapState {
    pub(crate) cast: Option<CastState>,
    pub(crate) screen_flash: Option<FlashState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CastState {
    pub(crate) id: u32,
    pub(crate) target: AnimTarget,
    pub(crate) global: bool,
    pub(crate) elapsed: u32,
}

impl LiveAnimation {
    fn snapshot(&self, library: &AnimationLibrary) -> Option<CastState> {
        if self.slot != AnimationSlot::Map {
            return None;
        }
        Some(CastState {
            id: library.0[self.index].id,
            target: self.map_target?,
            global: self.global,
            elapsed: self.elapsed,
        })
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Capture<'w, 's> {
    library: Option<Res<'w, AnimationLibrary>>,
    animations: Query<'w, 's, &'static LiveAnimation>,
    flashes: Query<'w, 's, &'static FlashQuad>,
}

impl Capture<'_, '_> {
    pub(crate) fn valid(&self, state: &MapState, map: &amnezia_data::Map) -> bool {
        state.screen_flash.as_ref().is_none_or(FlashState::valid)
            && state.cast.as_ref().is_none_or(|cast| {
                self.library.as_ref().is_some_and(|library| {
                    library
                        .0
                        .iter()
                        .any(|def| def.id == cast.id && cast.elapsed < def.frames.len() as u32 * 2)
                }) && match cast.target {
                    AnimTarget::Hero => true,
                    AnimTarget::Event(id) => map.events.iter().any(|event| event.id == id),
                }
            })
    }

    pub(crate) fn snapshot(&self) -> MapState {
        MapState {
            cast: self.library.as_ref().and_then(|library| {
                self.animations
                    .iter()
                    .find_map(|animation| animation.snapshot(library))
            }),
            screen_flash: self.flashes.iter().next().map(FlashQuad::snapshot),
        }
    }
}

pub(crate) fn snapshot(world: &mut World) -> MapState {
    let mut animations = world.query::<&LiveAnimation>();
    let cast = world
        .get_resource::<AnimationLibrary>()
        .and_then(|library| {
            animations
                .iter(world)
                .find_map(|animation| animation.snapshot(library))
        });
    let screen_flash = world
        .query::<&FlashQuad>()
        .iter(world)
        .next()
        .map(FlashQuad::snapshot);
    MapState { cast, screen_flash }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    state: MapState,
}

#[derive(Resource)]
struct TargetFlash {
    target: AnimTarget,
    color: [u8; 4],
}

pub(crate) fn prepare(world: &mut World, map_id: u32, state: MapState) {
    reset(world);
    if state != MapState::default() {
        world.insert_resource(Pending { map_id, state });
    }
}

pub(super) fn reset(world: &mut World) {
    world.remove_resource::<Pending>();
    world.remove_resource::<TargetFlash>();
}

pub(crate) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (restore, track_active_animations)
            .chain()
            .after(AnimationSet::Advance)
            .before(AnimationSet::Start),
    )
    .add_systems(
        PostUpdate,
        restore_target_flash
            .after(map::flash::Expire)
            .before(crate::legacy_colors::world::WorldColors),
    );
}

#[allow(clippy::too_many_arguments)]
fn restore(
    mut changes: MessageReader<crate::world::MapRebuilt>,
    pending: Option<Res<Pending>>,
    map: Option<Res<crate::world::MapData>>,
    frames: Res<GameFrames>,
    library: Res<AnimationLibrary>,
    targets: map::Targets,
    mut renderer: CellRenderer,
    mut commands: Commands,
) {
    if changes.read().count() == 0 {
        return;
    }
    let Some(pending) = pending else {
        return;
    };
    if map.is_none_or(|map| map.map_id != pending.map_id) {
        return;
    }
    commands.remove_resource::<Pending>();
    if let Some(flash) = pending.state.screen_flash.clone() {
        flash.restore(&mut commands, frames.frame);
    }
    let Some(cast) = pending.state.cast.as_ref() else {
        return;
    };
    let Some(index) = library.0.iter().position(|def| def.id == cast.id) else {
        return;
    };
    let Some(anchor) = targets.anchor(cast.target) else {
        return;
    };
    let def = &library.0[index];
    let duration = def.frames.len() as u32 * 2;
    if cast.elapsed >= duration {
        return;
    }
    let draw_anchors = draw_anchors(def, &[anchor], MAP_SCREEN_CENTER, cast.global);
    let frame = cast.elapsed as usize / 2;
    let cells = spawn_cells_at(&mut commands, &mut renderer, def, frame, &draw_anchors);
    commands.spawn(LiveAnimation {
        slot: AnimationSlot::Map,
        map_target: Some(cast.target),
        global: cast.global,
        index,
        draw_anchors,
        flash_anchors: vec![anchor.pos],
        frame,
        elapsed: cast.elapsed,
        last: frames.frame,
        duration,
        sound_only: false,
        cells,
    });
    if let Some(tick) = cast.elapsed.checked_sub(1) {
        commands.insert_resource(TargetFlash {
            target: cast.target,
            color: map::flash::color(def, tick, duration),
        });
    }
}

fn restore_target_flash(
    pending: Option<Res<TargetFlash>>,
    frames: Res<GameFrames>,
    mut commands: Commands,
) {
    let Some(pending) = pending else {
        return;
    };
    map::flash::write(&mut commands, pending.target, pending.color, frames.frame);
    commands.remove_resource::<TargetFlash>();
}
