use crate::assets::{asset_root, load_ron};
use crate::dialogue::{Dialogue, MessagePosition, MessageTransparent};
use crate::interpreter::{ParallelPool, RunningEvent};
use crate::player::{CameraPan, HeroHidden, Player};
use crate::save::{EventSaveRequest, LoadRequest, SaveAccess, SaveRequest};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::PendingTeleport;
use crate::world::{MoveQueue, RouteStepper};
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct NewGameRequest(pub bool);

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NewGameRequest>()
            .add_systems(PreUpdate, start_new_game.before(crate::save::SaveSet));
    }
}

fn start_new_game(world: &mut World) {
    if !std::mem::take(&mut world.resource_mut::<NewGameRequest>().0) {
        return;
    }
    clear_for_reload(world);
    reset::<Switches>(world);
    reset::<Variables>(world);
    reset::<Party>(world);
    reset::<Inventory>(world);
    reset::<crate::progression::Progression>(world);
    reset::<crate::equipment::Equipment>(world);
    reset::<crate::vitals::Vitals>(world);
    reset::<crate::conditions::FieldSteps>(world);
    reset::<crate::vehicles::Vehicles>(world);
    reset::<crate::appearance::Appearance>(world);
    reset::<HeroHidden>(world);
    reset::<crate::audio::MemorizedBgm>(world);
    reset::<crate::system_bgm::SystemBgm>(world);
    reset::<crate::transitions::Settings>(world);
    reset::<crate::panorama::Panorama>(world);
    reset::<crate::screenfx::TintState>(world);
    reset::<crate::screenfx::Weather>(world);
    reset::<crate::screenfx::WeatherStrength>(world);
    reset::<crate::timer::PlayTime>(world);
    reset::<crate::timer::GameClock>(world);
    reset::<crate::timing::GameFrames>(world);
    reset::<crate::menu::MenuAccess>(world);
    reset::<SaveAccess>(world);
    reset::<LoadRequest>(world);
    reset::<crate::teleport::Fade>(world);
    let hero = load_ron::<amnezia_data::Hero>(&format!("{}/hero.ron", asset_root()));
    world.insert_resource(crate::text::HeroName(hero.name));
    for mut player in world.query::<&mut Player>().iter_mut(world) {
        player.charset = "Chara1".into();
        player.index = 0;
        player.dir = crate::tiles::DIR_DOWN;
        player.frame = 1;
    }
    let start = load_ron::<amnezia_data::Start>(&format!("{}/start.ron", asset_root()));
    world
        .resource_mut::<PendingTeleport>()
        .reload(start.map_id, start.x, start.y);
}

/// Discard commands and overlays belonging to the previous play session.
pub(crate) fn clear_transient(world: &mut World) {
    world.remove_resource::<crate::player::saved_camera::Pending>();
    world.remove_resource::<crate::audio::saved::Pending>();
    reset::<RunningEvent>(world);
    reset::<ParallelPool>(world);
    reset::<Dialogue>(world);
    reset::<MessagePosition>(world);
    reset::<MessageTransparent>(world);
    reset::<crate::dialogue::MessageOptions>(world);
    reset::<crate::choice::Choice>(world);
    reset::<crate::inputnumber::InputNumber>(world);
    reset::<crate::menu::MenuOpen>(world);
    reset::<crate::shop::ShopOpen>(world);
    reset::<crate::shop::ShopOutcome>(world);
    reset::<crate::battle::BattleActive>(world);
    reset::<crate::battle::BattleResult>(world);
    crate::battle::reset_session(world);
    reset::<crate::gameover::GameOverActive>(world);
    reset::<crate::gameover::GameOverFlow>(world);
    reset::<CameraPan>(world);
    reset::<EventSaveRequest>(world);
    reset::<SaveRequest>(world);
    clear_messages::<crate::audio::AudioRequest>(world);
    clear_messages::<crate::appearance::SpriteChange>(world);
    clear_messages::<crate::screenfx::ScreenEffect>(world);
    clear_messages::<crate::picture::PictureCommand>(world);
    clear_messages::<crate::battle::BattleRequest>(world);
    clear_messages::<crate::shop::ShopRequest>(world);
    clear_messages::<crate::world::RelocateEvent>(world);
    clear_messages::<crate::animation::ShowMapAnimation>(world);
    clear_messages::<crate::animation::PlayAnimation>(world);
    crate::animation::reset_transient(world);
    crate::screenfx::reset_transient(world);
    if let Some(mut transition) = world.get_resource_mut::<crate::transitions::Transition>() {
        transition.clear();
    }
    if let Some(mut vehicles) = world.get_resource_mut::<crate::vehicles::Vehicles>() {
        vehicles.clear_motion();
    }
    for (mut queue, mut route) in world
        .query::<(&mut MoveQueue, &mut RouteStepper)>()
        .iter_mut(world)
    {
        *queue = MoveQueue::default();
        *route = RouteStepper::default();
    }
}

pub(crate) fn clear_for_reload(world: &mut World) {
    let erased = world
        .get_resource::<crate::transitions::Transition>()
        .is_some_and(|t| t.erased());
    clear_transient(world);
    if erased
        && let Some(mut transition) = world.get_resource_mut::<crate::transitions::Transition>()
    {
        transition.hold_black();
    }
}

fn reset<T: Resource + Default>(world: &mut World) {
    world.insert_resource(T::default());
}

fn clear_messages<T: Message>(world: &mut World) {
    if let Some(mut messages) = world.get_resource_mut::<Messages<T>>() {
        messages.clear();
    }
}

#[cfg(test)]
mod tests;
