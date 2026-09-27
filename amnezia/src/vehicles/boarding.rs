use super::*;
use bevy::ecs::system::SystemParam;

#[derive(SystemParam)]
pub(super) struct Boarding<'w, 's> {
    data: Option<Res<'w, MapData>>,
    obstacles: obstacles::Obstacles<'w, 's>,
    switches: Res<'w, Switches>,
    music: Option<Res<'w, VehicleMusic>>,
    system_bgm: Res<'w, crate::system_bgm::SystemBgm>,
    bgm: Option<Res<'w, CurrentBgm>>,
    vehicles: ResMut<'w, Vehicles>,
    audio: MessageWriter<'w, AudioRequest>,
    heroes: Query<
        'w,
        's,
        (
            &'static mut Player,
            &'static mut MoveQueue,
            &'static mut RouteStepper,
        ),
    >,
}

enum Change {
    Board(usize),
    Disembark,
    Descend,
}

impl Boarding<'_, '_> {
    fn toggle(&mut self) -> bool {
        let Some(data) = self.data.as_deref() else {
            return false;
        };
        let Ok((mut hero, mut queue, mut route)) = self.heroes.single_mut() else {
            return false;
        };
        let bodies = self
            .obstacles
            .bodies(&self.vehicles, data.map_id, route.through());
        let collision = self.obstacles.collision(data, &self.switches, &bodies);
        let change = toggle(
            &mut self.vehicles,
            data,
            &collision,
            &mut hero,
            &mut queue,
            &mut route,
        );
        match change {
            Some(Change::Board(index)) => {
                self.vehicles.save.before_music = self.bgm.as_ref().and_then(|bgm| bgm.track());
                let fallback;
                let music = if let Some(music) = self.music.as_ref() {
                    &music.0[index]
                } else {
                    let system = load_ron::<SystemDef>(&format!("{}/system.ron", asset_root()));
                    fallback = [system.boat_music, system.ship_music, system.airship_music];
                    &fallback[index]
                };
                self.audio.write(AudioRequest::from_music(
                    self.system_bgm.get(3 + index as u32, music),
                ));
            }
            Some(Change::Disembark) => {
                self.audio.write(
                    self.vehicles
                        .save
                        .before_music
                        .as_ref()
                        .map_or(AudioRequest::StopBgm, |music| music.replay()),
                );
            }
            Some(Change::Descend) => {}
            None => return false,
        }
        true
    }
}

pub(super) fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    guards: MoveGuards,
    mut boarding: Boarding,
) {
    boarding.vehicles.consumed_action = false;
    if guards.paused() || boarding.vehicles.blocks_movement() {
        return;
    }
    let Ok((_, queue, route)) = boarding.heroes.single() else {
        return;
    };
    if queue.busy() || route.active() {
        return;
    }
    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space)) && boarding.toggle()
    {
        boarding.vehicles.consumed_action = true;
    }
}

pub(crate) fn flush(world: &mut World) {
    super::rider::relocate(world);
    let pending = world
        .get_resource_mut::<Vehicles>()
        .is_some_and(|mut vehicles| std::mem::take(&mut vehicles.toggle_pending));
    if pending {
        world.run_system_cached(script).unwrap();
    }
}

fn script(mut boarding: Boarding) {
    boarding.toggle();
}

fn toggle(
    vehicles: &mut Vehicles,
    data: &MapData,
    collision: &crate::world::collision::MapCollision,
    hero: &mut Player,
    queue: &mut MoveQueue,
    route: &mut RouteStepper,
) -> Option<Change> {
    let direction = route.normalize_direction(hero);
    let (dx, dy) = dir_delta(direction);
    let front = (hero.tile_x + dx, hero.tile_y + dy);
    if vehicles.aboard() {
        let index = vehicles.save.riding.unwrap();
        if index == 2 {
            if vehicles.airship_transitioning() {
                return None;
            }
            hero.dir = DIR_LEFT;
            vehicles.save.airship_flight.descend();
            return Some(Change::Descend);
        }
        if !collision.can_disembark(hero.tile(), front) {
            return None;
        }
        vehicles.save.vehicles[index].dir = DIR_LEFT;
        vehicles.save.boarding = false;
        route.set_speed(vehicles.save.preboard_speed);
        vehicles.save.unboarding = true;
        step(hero, queue, route, data, (dx, dy));
        vehicles.save.riding = None;
        return Some(Change::Disembark);
    }
    let in_position = |index: usize, tile| {
        let vehicle = &vehicles.save.vehicles[index];
        vehicle.definition.map_id == data.map_id && vehicle.tile() == tile
    };
    if in_position(2, hero.tile()) && !queue.busy() && !vehicles.motion[2].queue.busy() {
        vehicles.save.riding = Some(2);
        vehicles.save.boarding = false;
        hero.dir = DIR_LEFT;
        vehicles.save.preboard_speed = route.speed();
        route.set_speed(vehicles.save.vehicles[2].speed);
        vehicles.save.airship_flight.ascend();
        return Some(Change::Board(2));
    }
    let index = [1, 0]
        .into_iter()
        .find(|index| in_position(*index, data.normalize_tile(front.0, front.1)))?;
    if !collision.can_embark(hero.tile(), front) {
        return None;
    }
    step(hero, queue, route, data, (dx, dy));
    vehicles.save.riding = Some(index);
    vehicles.save.preboard_speed = route.speed();
    vehicles.save.boarding = true;
    Some(Change::Board(index))
}

fn step(
    hero: &mut Player,
    queue: &mut MoveQueue,
    route: &RouteStepper,
    data: &MapData,
    delta: (i32, i32),
) {
    if !queue.busy() {
        queue.set_step_secs(step_secs_for_speed(route.speed()));
        queue.begin_from(
            hero,
            data,
            hero.tile(),
            RouteAction::Step {
                dx: delta.0,
                dy: delta.1,
                face: hero.dir,
            },
        );
    }
}
