use super::weather::rain::Scroll;
use super::{Fx, TintState, flash::Flashing, shake::ShakeState};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub(crate) mod smoke;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct ScreenState {
    pub tone: TintState,
    flash: Option<Flashing>,
    shake: ShakeState,
    #[serde(default)]
    pub(crate) weather_pan: [f32; 2],
}

impl ScreenState {
    pub(crate) fn valid(&self) -> bool {
        self.tone.valid()
            && self.flash.as_ref().is_none_or(Flashing::valid)
            && self.shake.valid()
            && self
                .weather_pan
                .into_iter()
                .zip([320.0, 160.0])
                .all(|(value, limit)| value.is_finite() && (0.0..limit).contains(&value))
    }

    fn restore(self, world: &mut World) {
        Scroll::restore(world, self.weather_pan);
        world.insert_resource(self.tone);
        world.insert_resource(Fx {
            flash: self.flash,
            shake_offset: Vec2::new(self.shake.position(), 0.0),
            shake: self.shake,
        });
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Capture<'w> {
    fx: Option<Res<'w, Fx>>,
    scroll: Option<Res<'w, Scroll>>,
}

impl Capture<'_> {
    pub(crate) fn snapshot(&self, tone: &TintState) -> ScreenState {
        ScreenState {
            weather_pan: self
                .scroll
                .as_ref()
                .map_or([0.0; 2], |scroll| scroll.snapshot()),
            tone: tone.clone(),
            flash: self.fx.as_ref().and_then(|fx| fx.flash.clone()),
            shake: self
                .fx
                .as_ref()
                .map(|fx| fx.shake.clone())
                .unwrap_or_default(),
        }
    }
}

pub(crate) fn snapshot(world: &World) -> ScreenState {
    let fx = world.resource::<Fx>();
    ScreenState {
        weather_pan: world
            .get_resource::<Scroll>()
            .map_or([0.0; 2], Scroll::snapshot),
        tone: world.resource::<TintState>().clone(),
        flash: fx.flash.clone(),
        shake: fx.shake.clone(),
    }
}

#[derive(Resource)]
pub(crate) struct Pending {
    map_id: u32,
    screen: ScreenState,
}

pub(crate) fn prepare(world: &mut World, map_id: u32, screen: Option<ScreenState>) {
    world.remove_resource::<Pending>();
    if let Some(screen) = screen {
        world.insert_resource(Pending { map_id, screen });
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_message::<crate::world::MapRebuilt>().add_systems(
        Update,
        restore
            .after(crate::teleport::MapTransfer)
            .after(super::MapScreenReset)
            .before(super::flash::channel::Advance)
            .before(crate::interpreter::InterpreterStep)
            .before(super::ScreenEffectsSet),
    );
}

fn restore(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<crate::world::MapRebuilt>>,
) {
    if cursor
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
    pending.screen.restore(world);
}
