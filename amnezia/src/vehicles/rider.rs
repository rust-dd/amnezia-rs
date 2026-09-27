use super::*;

pub(super) fn relocate(world: &mut World) {
    let target = world
        .get_resource_mut::<Vehicles>()
        .and_then(|mut vehicles| vehicles.relocate_pending.take());
    if let Some(target) = target {
        world.run_system_cached_with(relocate_hero, target).unwrap();
        crate::player::relocate_camera(world);
        crate::appearance::reset_player(world);
    }
}

fn relocate_hero(
    In((x, y)): In<(u32, u32)>,
    data: Res<MapData>,
    mut pan: ResMut<crate::player::CameraPan>,
    mut calling: Option<ResMut<crate::menu::Calling>>,
    mut heroes: Query<(&mut Player, &mut MoveQueue, Option<&mut Transform>)>,
) {
    let Ok((mut hero, mut queue, transform)) = heroes.single_mut() else {
        return;
    };
    queue.relocate(hero.tile());
    hero.set_tile(x as i32, y as i32);
    if let Some(mut transform) = transform {
        let point = queue.render_position(&*hero, &data);
        transform.translation =
            Vec3::new(point.x, point.y + hero.y_offset(), hero.draw_z(hero.tile_y));
    }
    pan.recenter(false);
    if let Some(calling) = calling.as_mut() {
        calling.cancel();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn flight(
    time: Res<Time>,
    data: Res<MapData>,
    guards: MoveGuards,
    obstacles: obstacles::Obstacles,
    switches: Res<Switches>,
    mut vehicles: ResMut<Vehicles>,
    mut audio: MessageWriter<AudioRequest>,
    mut heroes: Query<(&mut Player, &MoveQueue, &mut RouteStepper)>,
) {
    vehicles.movement_blocked = false;
    if guards.forced_route_paused() {
        return;
    }
    let Ok((mut hero, queue, mut route)) = heroes.single_mut() else {
        return;
    };
    if queue.busy() {
        return;
    }
    vehicles.movement_blocked = vehicles.airship_transitioning();
    if vehicles.advance_flight(time.delta_secs(), &data, |x, y| {
        obstacles.blocks_landing((x, y), &switches)
    }) {
        hero.dir = DIR_DOWN;
        route.set_speed(vehicles.save.preboard_speed);
        audio.write(
            vehicles
                .save
                .before_music
                .as_ref()
                .map_or(AudioRequest::StopBgm, |music| music.replay()),
        );
    }
}

pub(super) fn sync(
    time: Res<Time>,
    data: Res<MapData>,
    guards: MoveGuards,
    mut vehicles: ResMut<Vehicles>,
    mut updates: Option<ResMut<advance::Updates>>,
    mut heroes: Query<(&mut Player, &MoveQueue, &mut RouteStepper)>,
) {
    if guards.forced_route_paused() {
        return;
    }
    let Ok((mut hero, queue, mut route)) = heroes.single_mut() else {
        return;
    };
    if !queue.busy() {
        if vehicles.save.boarding {
            vehicles.save.boarding = false;
            hero.dir = DIR_LEFT;
            if let Some(index) = vehicles.save.riding {
                route.set_speed(vehicles.save.vehicles[index].speed);
            }
        }
        vehicles.save.unboarding = false;
    }
    if !vehicles.aboard() {
        return;
    }
    let index = vehicles.save.riding.unwrap();
    if let Some(updates) = updates.as_mut() {
        updates.0[index] = true;
    }
    let vehicles = &mut *vehicles;
    let vehicle = &mut vehicles.save.vehicles[index];
    let motion = &mut vehicles.motion[index];
    let previous = vehicle.tile();
    vehicle.definition.map_id = data.map_id;
    vehicle.set_tile(hero.tile_x, hero.tile_y);
    motion.route.set_direction(vehicle, route.direction(&*hero));
    vehicle.dir = hero.dir;
    motion
        .queue
        .use_character_motion(vehicle.speed, motion.route.direction(vehicle));
    motion.queue.sync_remaining(queue, previous, vehicle, &data);
    motion.pixel = Some(motion.queue.render_position(vehicle, &data));
    motion.route.animation.advance_vehicle(
        vehicle,
        !motion.queue.jumping(),
        motion.route.stop_count() == 0,
        time.delta_secs(),
    );
    motion.route.cancel_for_rider();
}

pub(crate) struct Graphic<'a> {
    pub(crate) hero: &'a mut Player,
    pub(crate) vehicles: Option<&'a mut Vehicles>,
}

impl Character for Graphic<'_> {
    fn tile(&self) -> (i32, i32) {
        self.hero.tile()
    }
    fn set_tile(&mut self, x: i32, y: i32) {
        self.hero.set_tile(x, y);
    }
    fn dir(&self) -> u32 {
        self.hero.dir()
    }
    fn set_dir(&mut self, dir: u32) {
        self.hero.set_dir(dir);
    }
    fn frame(&self) -> u32 {
        self.hero.frame()
    }
    fn set_frame(&mut self, frame: u32) {
        self.hero.set_frame(frame);
    }
    fn index(&self) -> u32 {
        self.hero.index()
    }
    fn charset(&self) -> &str {
        self.hero.charset()
    }
    fn set_graphic(&mut self, name: String, index: u32) {
        if let Some(vehicles) = self.vehicles.as_mut()
            && let Some(riding) = vehicles.save.riding
        {
            vehicles.save.vehicles[riding].set_graphic(name, index);
        } else {
            self.hero.set_graphic(name, index);
        }
    }
}
