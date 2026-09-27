use super::{ParallelPool, RunningEvent, frame::Frame};
use crate::battle::BattleRequest;
use crate::shop::ShopRequest;
use bevy::prelude::*;

pub(super) enum Scene {
    Menu,
    Battle(BattleRequest),
    Shop(ShopRequest),
    Save,
    GameOver,
}

struct Call {
    ticket: u64,
    scene: Scene,
}

#[derive(Resource, Default)]
pub(crate) struct Requests {
    next_ticket: u64,
    pending: Option<Call>,
    cancelled: Vec<u64>,
}

impl Requests {
    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn menu(&mut self) {
        self.replace(Scene::Menu);
    }

    pub(super) fn replace(&mut self, scene: Scene) -> u64 {
        self.next_ticket = self.next_ticket.wrapping_add(1);
        let ticket = self.next_ticket;
        if let Some(previous) = self.pending.replace(Call { ticket, scene }) {
            self.cancelled.push(previous.ticket);
        }
        ticket
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Commit;

struct SceneRequestsPlugin;

impl Plugin for SceneRequestsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Requests>()
            .init_resource::<crate::menu::SceneFlow>()
            .init_resource::<crate::save::EventSaveRequest>()
            .init_resource::<crate::gameover::GameOverActive>()
            .add_message::<BattleRequest>()
            .add_message::<ShopRequest>()
            .add_systems(
                Update,
                commit
                    .in_set(Commit)
                    .after(super::InterpreterStep)
                    .after(crate::menu::MapMenuRequest)
                    .before(crate::audio::AudioRequests),
            );
    }
}

pub(crate) fn register(app: &mut App) {
    if !app.is_plugin_added::<SceneRequestsPlugin>() {
        app.add_plugins(SceneRequestsPlugin);
    }
}

pub(super) fn cancel_replaced(world: &mut World) {
    let cancelled = std::mem::take(&mut world.resource_mut::<Requests>().cancelled);
    for ticket in cancelled {
        settle(world, ticket, true);
    }
}

fn settle(world: &mut World, ticket: u64, cancelled: bool) {
    if let Some(mut running) = world.get_resource_mut::<RunningEvent>() {
        running.frame.settle_scene(ticket, cancelled);
    }
    if let Some(mut pool) = world.get_resource_mut::<ParallelPool>() {
        pool.settle_scene(ticket, cancelled);
    }
}

impl Frame {
    pub(super) fn settle_scene(&mut self, ticket: u64, cancelled: bool) {
        if self.scene_request != Some(ticket) {
            return;
        }
        self.scene_request = None;
        if cancelled {
            if std::mem::take(&mut self.battle_pending) {
                self.battle_outcome = None;
            }
            if std::mem::take(&mut self.shop_pending) {
                self.shop_transacted = None;
                self.ip += 1;
            }
        }
    }
}

fn commit(world: &mut World) {
    cancel_replaced(world);
    if world
        .get_resource::<crate::title::TitleActive>()
        .is_some_and(|title| title.0)
    {
        if let Some(call) = world.resource_mut::<Requests>().pending.take() {
            settle(world, call.ticket, true);
        }
        return;
    }
    if suspended(world) {
        return;
    }
    let Some(call) = world.resource_mut::<Requests>().pending.take() else {
        return;
    };
    settle(world, call.ticket, false);
    match call.scene {
        Scene::Menu => world
            .resource_mut::<crate::menu::SceneFlow>()
            .request_main_menu(),
        Scene::Battle(request) => {
            world.write_message(request);
        }
        Scene::Shop(request) => {
            world.write_message(request);
        }
        Scene::Save => world.resource_mut::<crate::save::EventSaveRequest>().0 = true,
        Scene::GameOver => world.resource_mut::<crate::gameover::GameOverActive>().0 = true,
    }
}

fn suspended(world: &World) -> bool {
    let waiting = world
        .get_resource::<crate::timing::SceneWait>()
        .is_some_and(|wait| wait.0);
    world
        .get_resource::<crate::teleport::PendingTeleport>()
        .is_some_and(|pending| pending.0.is_some())
        || world
            .get_resource::<crate::teleport::Fade>()
            .is_some_and(|fade| fade.busy())
        || world
            .get_resource::<crate::transitions::Transition>()
            .is_some_and(|transition| transition.busy())
        || world
            .get_resource::<super::continuation::Continuation>()
            .map_or(waiting, |state| state.tail_paused(waiting))
        || world
            .get_resource::<crate::shop::inn::State>()
            .is_some_and(|inn| inn.resting())
}
