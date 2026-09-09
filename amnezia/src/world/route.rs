//! Driving the move-route [`RouteStepper`] each frame: the two Bevy systems that
//! pump every character's forced route — [`route_events`] for event NPCs (a
//! `move_type == 6` custom route or a loaded `MoveEvent`) and [`route_hero`] for
//! the hero. Both gate on the same pauses as autonomous movement, advance the
//! stepper only while the character stands idle, enqueue the resulting tile step
//! into the shared [`MoveQueue`] (which `walk` tweens), and apply the command's
//! side effects — a game-switch toggle, a sound, or a transparency change.
//!
//! The stepper state machine itself lives in the [`stepper`] submodule.

mod stepper;

pub use stepper::RouteStepper;

use super::collision::{CollisionBodies, MapCollision, Mover};
use super::{Character, EventSprite, MapData, MapEvents, MoveQueue};
use crate::audio::AudioRequest;
use crate::player::Player;
use crate::state::{Inventory, Party, Switches, Variables};
use bevy::prelude::*;
pub(crate) use stepper::StepEffect;

/// One frame of a character's stepper: the tile delta of the step it enqueued
/// this tick (for the caller to sync the logical event tile), plus the side
/// effects to apply.
pub(crate) struct Driven {
    pub(crate) moved: Option<(i32, i32)>,
    pub(crate) effects: Vec<StepEffect>,
}

impl Driven {
    /// Nothing happened this tick (inactive, mid-step, or still in its delay).
    fn idle() -> Self {
        Self {
            moved: None,
            effects: Vec::new(),
        }
    }
}

/// Drive one character's stepper for a frame: yield (nothing) while it is
/// inactive, mid-step (queue busy), or still in its inter-command delay; otherwise
/// advance the route, enqueue any resulting step, and return its delta and side
/// effects for the caller to apply. The effects are returned rather than applied
/// here so `can_step` — which borrows the switches and events — is dropped before
/// the caller mutates them.
pub(crate) fn drive<C: Character>(
    ch: &mut C,
    queue: &mut MoveQueue,
    stepper: &mut RouteStepper,
    hero: (i32, i32),
    dt: f32,
    can_step: impl Fn(&C, i32, i32, bool, bool) -> bool,
) -> Driven {
    if queue.busy() {
        return Driven::idle();
    }
    if stepper.settle_movement() || !stepper.active() {
        return Driven::idle();
    }
    if !stepper.tick_ready(dt) {
        return Driven::idle();
    }
    let mut effects = Vec::new();
    let moved = stepper
        .advance(ch, hero, &can_step, &mut effects)
        .map(|(action, secs)| {
            let delta = action.delta();
            queue.set_step_secs(secs);
            queue.enqueue_route([action]);
            delta
        });
    Driven { moved, effects }
}

/// Apply the side effects a route command produced: toggle a game switch, play a
/// sound, or set the character's sprite transparency.
fn apply_effects(
    effects: Vec<StepEffect>,
    switches: &mut Switches,
    audio: &mut MessageWriter<AudioRequest>,
    sprite: &mut Sprite,
) {
    for effect in effects {
        match effect {
            StepEffect::Switch(id, on) => switches.set(id, on),
            StepEffect::Sound { name, params } => {
                audio.write(AudioRequest::play_sound(&name, &params));
            }
            StepEffect::Transparency(level) => {
                let alpha = crate::tiles::character_alpha(level);
                sprite.color = sprite.color.with_alpha(alpha);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn tile_open(
    ex: i32,
    ey: i32,
    dx: i32,
    dy: i32,
    jumping: bool,
    mover: (u32, u32),
    hero: (i32, i32),
    data: &MapData,
    map_events: &MapEvents,
    state: (&Switches, &Variables, &Party, &Inventory),
) -> bool {
    let character = EventSprite {
        id: mover.0,
        layer: mover.1,
        tile_x: ex,
        tile_y: ey,
        dir: 2,
        frame: 1,
        charset: "C".into(),
        index: 0,
    };
    MapCollision::new(data, map_events, state, &CollisionBodies::default()).can_move(
        (ex, ey),
        (ex + dx, ey + dy),
        Mover::event(&character, false),
        Some(hero),
        jumping,
    )
}

/// Step every event NPC's forced route (custom `move_type == 6` or a loaded
/// `MoveEvent`). Paused by the same guards as autonomous movement.
#[allow(clippy::too_many_arguments)]
pub(super) fn route_events(
    time: Res<Time>,
    data: Res<MapData>,
    mut map_events: ResMut<MapEvents>,
    mut switches: ResMut<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: super::autonomy::MoveGuards,
    mut touches: Option<ResMut<super::TouchEvents>>,
    mut audio: MessageWriter<AudioRequest>,
    players: Query<(&Player, Option<&RouteStepper>), Without<EventSprite>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut movers: Query<
        (
            &mut EventSprite,
            &mut MoveQueue,
            &mut RouteStepper,
            &mut Sprite,
        ),
        Without<Player>,
    >,
) {
    if guards.forced_route_paused() {
        return;
    }
    let hero = players
        .single()
        .map(|(p, _)| (p.tile_x, p.tile_y))
        .unwrap_or((-1, -1));
    let mut bodies = CollisionBodies::from_events(
        movers
            .iter()
            .map(|(event, _, route, _)| (event, Some(route))),
    );
    bodies.hero_through = players
        .single()
        .ok()
        .and_then(|(_, route)| route)
        .is_some_and(RouteStepper::through);
    bodies.include_vehicles(vehicles.as_deref(), data.map_id);
    let dt = time.delta_secs();
    for (mut sprite_c, mut queue, mut stepper, mut sprite) in &mut movers {
        if !stepper.forced() && guards.autonomous_paused(sprite_c.id) {
            continue;
        }
        let (ex, ey) = (sprite_c.tile_x, sprite_c.tile_y);
        let self_id = sprite_c.id;
        let layer = sprite_c.layer;
        let delta = data.tile_delta((ex, ey), hero);
        let near_hero = (ex + delta.0, ey + delta.1);
        let touched = std::cell::Cell::new(false);
        let driven = {
            let collision = MapCollision::new(
                &data,
                &map_events,
                (&switches, &variables, &party, &inventory),
                &bodies,
            );
            let can_step =
                |character: &EventSprite, dx: i32, dy: i32, jumping: bool, through: bool| {
                    if !through
                        && !bodies.hero_through
                        && layer == 1
                        && data.normalize_tile(ex + dx, ey + dy) == hero
                    {
                        touched.set(true);
                    }
                    collision.can_move(
                        (ex, ey),
                        (ex + dx, ey + dy),
                        Mover::event(character, through),
                        Some(hero),
                        jumping,
                    )
                };
            drive(
                &mut *sprite_c,
                &mut queue,
                &mut stepper,
                near_hero,
                dt,
                can_step,
            )
        };
        bodies.update(&sprite_c, &stepper);
        if touched.get()
            && let Some(touches) = touches.as_mut()
        {
            touches.0.push(self_id);
        }
        if let Some((dx, dy)) = driven.moved
            && let Some(event) = map_events.events.iter_mut().find(|e| e.id == self_id)
        {
            let (x, y) = data.normalize_tile(ex + dx, ey + dy);
            event.x = x.max(0) as u32;
            event.y = y.max(0) as u32;
        }
        apply_effects(driven.effects, &mut switches, &mut audio, &mut sprite);
    }
}

/// Step the hero's forced route (a `MoveEvent` targeting the hero). Same guards
/// as event routes; the hero is `self_id` 0 (no event) for the collision test.
#[allow(clippy::too_many_arguments)]
pub(super) fn route_hero(
    time: Res<Time>,
    data: Res<MapData>,
    map_events: Res<MapEvents>,
    mut switches: ResMut<Switches>,
    variables: Res<Variables>,
    party: Res<Party>,
    inventory: Res<Inventory>,
    guards: super::autonomy::MoveGuards,
    mut audio: MessageWriter<AudioRequest>,
    mut hero: Query<
        (&mut Player, &mut MoveQueue, &mut RouteStepper, &mut Sprite),
        Without<EventSprite>,
    >,
    events: Query<(&EventSprite, Option<&RouteStepper>), Without<Player>>,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
) {
    // The hero's only routes come from a `MoveEvent`, which is always forced, so they
    // must keep advancing through the very cutscene that issued them — RM2000 steps
    // an overwritten route even while the event interpreter runs and a message shows.
    if guards.forced_route_paused() || vehicles.as_ref().is_some_and(|v| v.airship_transitioning())
    {
        return;
    }
    let Ok((mut player, mut queue, mut stepper, mut sprite)) = hero.single_mut() else {
        return;
    };
    let (ex, ey) = (player.tile_x, player.tile_y);
    let pos = (ex, ey);
    let dt = time.delta_secs();
    let mut bodies = CollisionBodies::from_events(events.iter());
    bodies.include_vehicles(vehicles.as_deref(), data.map_id);
    let driven = {
        let collision = MapCollision::new(
            &data,
            &map_events,
            (&switches, &variables, &party, &inventory),
            &bodies,
        );
        let can_step = |_: &Player, dx: i32, dy: i32, jumping: bool, through: bool| {
            collision.can_move(pos, (ex + dx, ey + dy), Mover::hero(through), None, jumping)
        };
        drive(&mut *player, &mut queue, &mut stepper, pos, dt, can_step)
    };
    // The hero is not a map event, so only its side effects need applying — its
    // tile is tracked by the `Player` component that `walk` updates.
    apply_effects(driven.effects, &mut switches, &mut audio, &mut sprite);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleActive;
    use crate::dialogue::Dialogue;
    use crate::gameover::GameOverActive;
    use crate::interpreter::RunningEvent;
    use crate::menu::MenuOpen;
    use crate::shop::ShopOpen;
    use crate::teleport::Fade;
    use crate::tiles::DIR_DOWN;
    use crate::title::TitleActive;
    use amnezia_data::{Event, EventCommand, MoveCommandDef, MoveRouteDef};

    #[test]
    fn forced_routes_check_the_wrapped_destination_for_hero_collision() {
        let mut data = MapData::for_test(140, 140);
        data.scroll_type = 3;
        let state = (
            &Switches::default(),
            &Variables::default(),
            &Party::default(),
            &Inventory::default(),
        );
        for jumping in [false, true] {
            assert!(!tile_open(
                0,
                2,
                -1,
                0,
                jumping,
                (1, 1),
                (139, 2),
                &data,
                &MapEvents::default(),
                state
            ));
            assert!(tile_open(
                0,
                2,
                -1,
                0,
                jumping,
                (1, 0),
                (139, 2),
                &data,
                &MapEvents::default(),
                state
            ));
            assert!(tile_open(
                0,
                2,
                -1,
                0,
                jumping,
                (1, 1),
                (138, 2),
                &data,
                &MapEvents::default(),
                state
            ));
        }
    }

    /// A `RunningEvent` mid-execution — a one-command frame is enough to make
    /// `active()` hold, standing in for a cutscene that is still running.
    fn running_event() -> RunningEvent {
        let mut running = RunningEvent::default();
        running.start(
            1,
            vec![EventCommand {
                code: 10110,
                indent: 0,
                string: String::new(),
                params: Vec::new(),
            }],
        );
        running
    }

    #[test]
    fn move_type_six_npc_follows_its_route() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(MapData::for_test(10, 10));
        app.insert_resource(MapEvents {
            events: vec![Event {
                id: 1,
                x: 5,
                y: 5,
                name: String::new(),
                pages: Vec::new(),
            }],
        });
        app.init_resource::<Switches>();
        app.init_resource::<Variables>();
        app.init_resource::<Party>();
        app.init_resource::<Inventory>();
        app.init_resource::<Dialogue>();
        app.init_resource::<Fade>();
        app.init_resource::<MenuOpen>();
        app.init_resource::<ShopOpen>();
        app.init_resource::<BattleActive>();
        app.init_resource::<GameOverActive>();
        app.init_resource::<RunningEvent>();
        app.insert_resource(TitleActive(false));
        app.add_message::<AudioRequest>();
        app.add_systems(Update, route_events);
        app.world_mut().spawn(Player {
            tile_x: 0,
            tile_y: 0,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
        });
        let route = MoveRouteDef {
            commands: vec![MoveCommandDef {
                code: 2,
                params: Vec::new(),
                string: String::new(),
            }],
            repeat: true,
            skippable: false,
        };
        app.world_mut().spawn((
            EventSprite {
                id: 1,
                tile_x: 5,
                tile_y: 5,
                dir: DIR_DOWN,
                frame: 1,
                charset: "C".into(),
                index: 0,
                layer: 1,
            },
            MoveQueue::default(),
            RouteStepper::from_page(&route, 6, 8),
            Sprite::default(),
        ));
        app.update();
        let world = app.world_mut();
        let logical = &world.resource::<MapEvents>().events[0];
        assert_eq!(
            (logical.x, logical.y),
            (5, 6),
            "the routed NPC stepped down and its logical tile followed",
        );
        let (sprite, queue) = world
            .query::<(&EventSprite, &MoveQueue)>()
            .single(world)
            .unwrap();
        assert_eq!(sprite.dir, DIR_DOWN, "faced its move direction");
        assert!(queue.busy(), "the tile step is queued for the walk tween");
    }

    #[test]
    fn forced_hero_route_advances_while_an_event_runs() {
        // The intro walks the hero in via a MoveEvent while its own event is still
        // running. A forced route must step regardless of the running interpreter —
        // RM2000 updates an overwritten route every frame (the bug: it was paused).
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(MapData::for_test(10, 10));
        app.insert_resource(MapEvents { events: Vec::new() });
        app.init_resource::<Switches>();
        app.init_resource::<Variables>();
        app.init_resource::<Party>();
        app.init_resource::<Inventory>();
        app.init_resource::<Dialogue>();
        app.init_resource::<Fade>();
        app.init_resource::<MenuOpen>();
        app.init_resource::<ShopOpen>();
        app.init_resource::<BattleActive>();
        app.init_resource::<GameOverActive>();
        app.insert_resource(TitleActive(false));
        app.insert_resource(running_event());
        app.add_message::<AudioRequest>();
        app.add_systems(Update, route_hero);
        app.world_mut().spawn((
            Player {
                tile_x: 5,
                tile_y: 5,
                dir: DIR_DOWN,
                frame: 1,
                charset: "C".into(),
                index: 0,
            },
            MoveQueue::default(),
            RouteStepper::from_move_event(&[10001, 6, 0, 0, 2]),
            Sprite::default(),
        ));
        app.update();
        let world = app.world_mut();
        let queue = world.query::<&MoveQueue>().single(world).unwrap();
        assert!(
            queue.busy(),
            "a forced hero route must step during a running event (cutscene movement)",
        );
    }

    #[test]
    fn page_route_pauses_while_an_event_runs() {
        // A move_type-6 NPC's own route is not forced, so a running event pauses it
        // like autonomous movement — only forced MoveEvent routes ignore the pause.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(MapData::for_test(10, 10));
        app.insert_resource(MapEvents {
            events: vec![Event {
                id: 1,
                x: 5,
                y: 5,
                name: String::new(),
                pages: Vec::new(),
            }],
        });
        app.init_resource::<Switches>();
        app.init_resource::<Variables>();
        app.init_resource::<Party>();
        app.init_resource::<Inventory>();
        app.init_resource::<Dialogue>();
        app.init_resource::<Fade>();
        app.init_resource::<MenuOpen>();
        app.init_resource::<ShopOpen>();
        app.init_resource::<BattleActive>();
        app.init_resource::<GameOverActive>();
        app.insert_resource(TitleActive(false));
        app.insert_resource(running_event());
        app.add_message::<AudioRequest>();
        app.add_systems(Update, route_events);
        app.world_mut().spawn(Player {
            tile_x: 0,
            tile_y: 0,
            dir: DIR_DOWN,
            frame: 1,
            charset: "C".into(),
            index: 0,
        });
        let route = MoveRouteDef {
            commands: vec![MoveCommandDef {
                code: 2,
                params: Vec::new(),
                string: String::new(),
            }],
            repeat: true,
            skippable: false,
        };
        app.world_mut().spawn((
            EventSprite {
                id: 1,
                tile_x: 5,
                tile_y: 5,
                dir: DIR_DOWN,
                frame: 1,
                charset: "C".into(),
                index: 0,
                layer: 1,
            },
            MoveQueue::default(),
            RouteStepper::from_page(&route, 6, 8),
            Sprite::default(),
        ));
        app.update();
        let world = app.world_mut();
        let logical = &world.resource::<MapEvents>().events[0];
        assert_eq!(
            (logical.x, logical.y),
            (5, 5),
            "a page route must not step while an event runs",
        );
    }
}
