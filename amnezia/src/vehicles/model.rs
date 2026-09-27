use crate::assets::{asset_root, load_ron};
use crate::audio::BgmTrack;
use crate::tiles::DIR_LEFT;
use crate::world::{Character, MoveQueue, RouteStepper};
use amnezia_data::VehicleDef;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleState {
    pub definition: VehicleDef,
    pub dir: u32,
    pub frame: u32,
    pub speed: u32,
}

impl Character for VehicleState {
    fn tile(&self) -> (i32, i32) {
        (self.definition.x as i32, self.definition.y as i32)
    }
    fn set_tile(&mut self, x: i32, y: i32) {
        self.definition.x = x.max(0) as u32;
        self.definition.y = y.max(0) as u32;
    }
    fn dir(&self) -> u32 {
        self.dir
    }
    fn set_dir(&mut self, dir: u32) {
        self.dir = dir;
    }
    fn frame(&self) -> u32 {
        self.frame
    }
    fn set_frame(&mut self, frame: u32) {
        self.frame = frame;
    }
    fn index(&self) -> u32 {
        self.definition.index
    }
    fn charset(&self) -> &str {
        &self.definition.charset
    }
    fn set_graphic(&mut self, name: String, index: u32) {
        self.definition.charset = name;
        self.definition.index = index;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleSave {
    pub vehicles: [VehicleState; 3],
    pub riding: Option<usize>,
    #[serde(default)]
    pub boarding: bool,
    #[serde(default)]
    pub unboarding: bool,
    #[serde(default = "preboard_speed")]
    pub preboard_speed: u32,
    pub before_music: Option<BgmTrack>,
    #[serde(default)]
    pub(super) airship_flight: super::flight::AirshipFlight,
}

impl Default for VehicleSave {
    fn default() -> Self {
        let definitions = load_ron::<[VehicleDef; 3]>(&format!("{}/vehicles.ron", asset_root()));
        Self {
            vehicles: std::array::from_fn(|index| VehicleState {
                definition: definitions[index].clone(),
                speed: if index == 2 { 5 } else { 4 },
                dir: DIR_LEFT,
                frame: 1,
            }),
            riding: None,
            boarding: false,
            unboarding: false,
            preboard_speed: preboard_speed(),
            before_music: None,
            airship_flight: default(),
        }
    }
}

fn preboard_speed() -> u32 {
    4
}

#[derive(Clone)]
pub(super) struct Motion {
    pub queue: MoveQueue,
    pub route: RouteStepper,
    pub pixel: Option<Vec2>,
    pub alpha: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            queue: default(),
            route: default(),
            pixel: None,
            alpha: 1.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct Vehicles {
    pub save: VehicleSave,
    pub(super) motion: [Motion; 3],
    pub(super) consumed_action: bool,
    pub(super) movement_blocked: bool,
    pub(crate) toggle_pending: bool,
    pub(crate) relocate_pending: Option<(u32, u32)>,
}

impl Vehicles {
    pub(crate) fn route_through(&self, index: usize) -> bool {
        self.motion[index].route.through()
    }

    pub(crate) fn collision_tiles(
        &self,
        map_id: u32,
    ) -> impl Iterator<Item = (usize, (i32, i32))> + '_ {
        self.save
            .vehicles
            .iter()
            .enumerate()
            .filter_map(move |(index, vehicle)| {
                (vehicle.definition.map_id == map_id
                    && !self.motion[index].route.through()
                    && !(index == 2 && self.save.riding == Some(2)))
                .then_some((index, vehicle.tile()))
            })
    }

    pub(crate) fn jumping(&self, index: usize) -> bool {
        self.motion
            .get(index)
            .is_some_and(|motion| motion.queue.jumping())
    }

    pub fn riding(&self) -> bool {
        self.save.riding.is_some()
    }

    pub(crate) fn aboard(&self) -> bool {
        self.riding() && !self.save.boarding
    }

    pub(crate) fn blocks_movement(&self) -> bool {
        self.movement_blocked || self.airship_transitioning()
    }

    pub fn blocks_action(&self) -> bool {
        self.save.riding == Some(2) || self.consumed_action
    }

    pub fn character(&self, reference: i32) -> Option<(i32, i32, u32)> {
        let index = usize::try_from(reference - 10002).ok()?;
        let vehicle = self.save.vehicles.get(index)?;
        Some((vehicle.tile().0, vehicle.tile().1, vehicle.dir))
    }

    pub(crate) fn pixel(&self, reference: i32, data: &crate::world::MapData) -> Option<Vec2> {
        let index = usize::try_from(reference - 10002).ok()?;
        let vehicle = self.save.vehicles.get(index)?;
        Some(
            self.motion[index].pixel.unwrap_or_else(|| {
                Vec2::from(data.tile_center(vehicle.tile().0, vehicle.tile().1))
            }) + Vec2::Y
                * if index == 2 {
                    self.airship_altitude()
                } else {
                    0.0
                },
        )
    }

    pub fn set_location(&mut self, index: usize, map_id: u32, x: u32, y: u32) {
        let Some(vehicle) = self.save.vehicles.get_mut(index) else {
            return;
        };
        self.motion[index].queue.relocate(vehicle.tile());
        vehicle.definition.map_id = map_id;
        vehicle.definition.x = x;
        vehicle.definition.y = y;
        self.motion[index].pixel = None;
    }

    pub fn set_route(&mut self, reference: i32, route: RouteStepper) {
        if let Ok(index) = usize::try_from(reference - 10002)
            && let Some(motion) = self.motion.get_mut(index)
        {
            motion.route.set_speed(self.save.vehicles[index].speed);
            motion.route.force_route(route);
        }
    }

    pub fn routes_pending(&self) -> bool {
        self.motion.iter().any(|m| m.route.pending())
    }

    pub fn restore(&mut self, save: VehicleSave) {
        *self = Self { save, ..default() };
        self.save.airship_flight.sanitize();
        if self.save.riding.is_some_and(|index| index >= 3) {
            self.save.riding = None;
        }
    }

    pub fn clear_motion(&mut self) {
        self.motion = std::array::from_fn(|_| default());
        self.consumed_action = false;
        self.movement_blocked = false;
        self.toggle_pending = false;
        self.relocate_pending = None;
    }
}
