mod model;
mod render;
#[cfg(test)]
mod tests;

pub use model::{VehicleSave, Vehicles};

use crate::assets::{asset_root, load_ron};
use crate::audio::{AudioRequest, CurrentBgm};
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables, active_page};
use crate::tiles::{DIR_DOWN, DIR_LEFT, DIR_RIGHT, DIR_UP};
use crate::world::{
    Character, MapData, MapEvents, MoveGuards, MoveQueue, RouteAction, StepEffect, dir_delta,
    drive_route, step_secs_for_speed,
};
use amnezia_data::{MusicDef, SystemDef};
use bevy::prelude::*;

#[derive(Resource)]
struct VehicleMusic([MusicDef; 3]);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VehicleInput;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VehicleDisplay;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VehicleSync;

pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        let system = load_ron::<SystemDef>(&format!("{}/system.ron", asset_root()));
        app.init_resource::<Vehicles>()
            .insert_resource(VehicleMusic([
                system.boat_music,
                system.ship_music,
                system.airship_music,
            ]))
            .add_systems(
                PreUpdate,
                keyboard.in_set(VehicleInput).after(crate::save::SaveSet),
            )
            .add_systems(Update, advance)
            .add_systems(
                PostUpdate,
                (
                    render::sync_hero.in_set(VehicleSync),
                    render::draw
                        .in_set(VehicleDisplay)
                        .after(crate::player::CameraFollow)
                        .after(crate::screenfx::ScreenShakeSet)
                        .before(bevy::transform::TransformSystems::Propagate),
                ),
            );
    }
}

impl Vehicles {
    pub fn toggle(
        &mut self,
        data: &MapData,
        hero: (i32, i32, u32),
        blocked: impl Fn(i32, i32) -> bool,
    ) -> bool {
        if let Some(index) = self.save.riding {
            if self.motion[index].queue.busy() {
                return false;
            }
            let vehicle = &self.save.vehicles[index];
            let (mut x, mut y) = vehicle.tile();
            if index != 2 {
                let (dx, dy) = dir_delta(vehicle.dir);
                x += dx;
                y += dy;
            }
            let (x, y) = data.normalize_tile(x, y);
            if !data.passable(x, y) || blocked(x, y) {
                return false;
            }
            self.disembark = Some((x, y, vehicle.dir));
            self.save.riding = None;
            return true;
        }
        let (dx, dy) = dir_delta(hero.2);
        for index in [2, 1, 0] {
            let vehicle = &mut self.save.vehicles[index];
            let tile = if index == 2 {
                (hero.0, hero.1)
            } else {
                data.normalize_tile(hero.0 + dx, hero.1 + dy)
            };
            if vehicle.definition.map_id == data.map_id
                && vehicle.tile() == tile
                && !self.motion[index].queue.busy()
            {
                self.save.riding = Some(index);
                vehicle.dir = if index == 2 { DIR_LEFT } else { hero.2 };
                self.consumed_action = true;
                return true;
            }
        }
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    data: Option<Res<MapData>>,
    map_events: Res<MapEvents>,
    guards: MoveGuards,
    switches: Res<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    music: Res<VehicleMusic>,
    system_bgm: Res<crate::system_bgm::SystemBgm>,
    bgm: Res<CurrentBgm>,
    mut vehicles: ResMut<Vehicles>,
    mut audio: MessageWriter<AudioRequest>,
    players: Query<(&Player, &MoveQueue)>,
) {
    vehicles.consumed_action = false;
    if guards.paused() {
        return;
    }
    let Some(data) = data else { return };
    let Ok((hero, queue)) = players.single() else {
        return;
    };
    if queue.busy() {
        return;
    }
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        let was_riding = vehicles.riding();
        if vehicles.toggle(&data, (hero.tile_x, hero.tile_y, hero.dir), |x, y| {
            map_events.events.iter().any(|event| {
                event.x as i32 == x
                    && event.y as i32 == y
                    && active_page(event, &switches, &variables, &party, &inventory)
                        .is_some_and(|p| p.layer == 1)
            })
        }) {
            vehicles.consumed_action = true;
            if !was_riding {
                vehicles.save.before_music = bgm.track();
                let index = vehicles.save.riding.unwrap();
                audio.write(AudioRequest::from_music(
                    system_bgm.get(3 + index as u32, &music.0[index]),
                ));
            } else {
                audio.write(
                    vehicles
                        .save
                        .before_music
                        .as_ref()
                        .map_or(AudioRequest::StopBgm, |m| m.replay()),
                );
            }
        }
        return;
    }
    let Some(index) = vehicles.save.riding else {
        return;
    };
    if vehicles.motion[index].queue.busy() || vehicles.motion[index].route.active() {
        return;
    }
    let dir = [
        (KeyCode::ArrowUp, DIR_UP),
        (KeyCode::ArrowDown, DIR_DOWN),
        (KeyCode::ArrowLeft, DIR_LEFT),
        (KeyCode::ArrowRight, DIR_RIGHT),
    ]
    .into_iter()
    .find(|(key, _)| keys.pressed(*key))
    .map(|(_, dir)| dir);
    if let Some(dir) = dir {
        let (dx, dy) = dir_delta(dir);
        let (x, y) = vehicles.save.vehicles[index].tile();
        vehicles.save.vehicles[index].dir = dir;
        if data.contains_tile(x + dx, y + dy) {
            let speed = vehicles.save.vehicles[index].speed;
            vehicles.motion[index]
                .queue
                .set_step_secs(step_secs_for_speed(speed));
            vehicles.motion[index]
                .queue
                .push_step(RouteAction::Step { dx, dy, face: dir });
        }
    }
}

fn advance(
    time: Res<Time>,
    data: Res<MapData>,
    guards: MoveGuards,
    mut vehicles: ResMut<Vehicles>,
    mut switches: ResMut<Switches>,
    mut audio: MessageWriter<AudioRequest>,
) {
    if guards.forced_route_paused() {
        return;
    }
    let vehicles = &mut *vehicles;
    for (index, (vehicle, motion)) in vehicles
        .save
        .vehicles
        .iter_mut()
        .zip(&mut vehicles.motion)
        .enumerate()
    {
        if vehicle.definition.map_id != data.map_id {
            continue;
        }
        let (x, y) = vehicle.tile();
        let routed = motion.route.active();
        let driven = drive_route(
            vehicle,
            &mut motion.queue,
            &mut motion.route,
            (x, y),
            time.delta_secs(),
            |dx, dy, _| data.contains_tile(x + dx, y + dy),
        );
        if routed {
            vehicle.speed = motion.route.speed();
        }
        for effect in driven.effects {
            match effect {
                StepEffect::Switch(id, on) => switches.set(id, on),
                StepEffect::Sound { name, params } => {
                    audio.write(AudioRequest::play_sound(&name, &params));
                }
                StepEffect::Transparency(level) => {
                    motion.alpha = crate::tiles::character_alpha(level)
                }
            }
        }
        let moving = motion.queue.busy();
        if let Some(pixel) = motion.queue.advance(vehicle, &data, time.delta_secs()) {
            motion.pixel = Some(pixel);
        }
        let animated = !motion.queue.jumping() && (index != 2 || vehicles.save.riding == Some(2));
        motion
            .route
            .animation
            .advance_vehicle(vehicle, animated, moving, time.delta_secs());
    }
}
