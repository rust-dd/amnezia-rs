//! Integration tests for the foreground/background interpreter split: that a
//! common autostart gates, parallel loops, scene pauses, prompt ownership and
//! shared foreground/background state.

use super::{CommonEvents, InterpreterPlugin, ParallelPool, RunningEvent};
use crate::animation::{AnimationLibrary, ShowMapAnimation};
use crate::appearance::SpriteChange;
use crate::audio::AudioRequest;
use crate::battle::{BattleActive, BattleRequest, BattleResult};
use crate::choice::Choice;
use crate::dialogue::{Dialogue, MessagePosition, MessageTransparent};
use crate::gamedata::GameData;
use crate::gameover::GameOverActive;
use crate::inputnumber::InputNumber;
use crate::menu::{MenuAccess, MenuOpen};
use crate::picture::PictureCommand;
use crate::player::{CameraPan, HeroHidden, Player};
use crate::progression::Progression;
use crate::save::{EventSaveRequest, SaveAccess};
use crate::screenfx::{ScreenEffect, Weather, WeatherStrength};
use crate::shop::{ShopOpen, ShopOutcome, ShopRequest};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport};
use crate::text::HeroName;
use crate::timer::GameClock;
use crate::title::TitleActive;
use crate::vitals::Vitals;
use crate::world::{MapData, MapEvents, MoveQueue, RouteStepper};
use amnezia_data::{CommonEvent, Event, EventCommand, EventPage};
use bevy::prelude::*;

mod actor_commands;
mod call_scope;
mod camera;
mod embedded_prompts;
mod inns;
mod key_input;
mod message_handoff;
mod message_options;
mod message_ownership;
mod movement;
mod outcomes;
mod save_boundary;
mod saved;
mod screen_coordinates;
mod transfers;
mod transitions;
mod vehicles;

const CONTROL_SWITCHES: u32 = 10210;
const CONDITIONAL_BRANCH: u32 = 12010;
const END_BRANCH: u32 = 22011;

/// One command with integer params and no string.
fn cmd(code: u32, indent: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: String::new(),
        params,
    }
}

/// Set switch `id` to `on`/off (`op` 0 = on, 1 = off, 2 = toggle) at `indent`.
fn switch_cmd(id: i32, op: i32, indent: u32) -> EventCommand {
    cmd(CONTROL_SWITCHES, indent, vec![0, id, id, op])
}

/// A common event with the given trigger and gating switch.
fn common(id: u32, trigger: u32, switch_id: u32, commands: Vec<EventCommand>) -> CommonEvent {
    CommonEvent {
        id,
        name: String::new(),
        trigger,
        switch_id,
        commands,
    }
}

/// A one-page map event at the origin whose page carries `trigger` and `commands`.
fn map_event(id: u32, trigger: u32, commands: Vec<EventCommand>) -> Event {
    Event {
        id,
        x: 0,
        y: 0,
        name: String::new(),
        pages: vec![EventPage {
            trigger,
            graphic_name: String::new(),
            graphic_index: 0,
            direction: 2,
            pattern: 1,
            animation_type: 0,
            translucent: false,
            overlap_forbidden: false,
            move_type: 0,
            move_frequency: 3,
            move_speed: 3,
            move_route: Default::default(),
            layer: 0,
            condition: amnezia_data::EventCondition::default(),
            commands,
        }],
    }
}

/// Build a headless app carrying every resource the interpreter systems read,
/// with an empty map and no common events; tests insert their own scripts.
fn interp_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(InterpreterPlugin);
    app.init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Inventory>()
        .init_resource::<Party>()
        .init_resource::<PendingTeleport>()
        .init_resource::<Dialogue>()
        .init_resource::<Choice>()
        .init_resource::<BattleResult>()
        .init_resource::<ShopOutcome>()
        .init_resource::<GameOverActive>()
        .init_resource::<Vitals>()
        .init_resource::<InputNumber>()
        .init_resource::<CameraPan>()
        .init_resource::<HeroHidden>()
        .init_resource::<Weather>()
        .init_resource::<WeatherStrength>()
        .init_resource::<MessagePosition>()
        .init_resource::<MessageTransparent>()
        .init_resource::<GameClock>()
        .init_resource::<EventSaveRequest>()
        .init_resource::<SaveAccess>()
        .init_resource::<MenuAccess>()
        .init_resource::<Progression>()
        .init_resource::<crate::equipment::Equipment>()
        .init_resource::<Fade>()
        .init_resource::<MenuOpen>()
        .init_resource::<ShopOpen>()
        .init_resource::<BattleActive>()
        .init_resource::<MapEvents>();
    app.insert_resource(AnimationLibrary(Vec::new()))
        .insert_resource(HeroName(String::new()))
        .insert_resource(GameData {
            actors: Vec::new(),
            items: Vec::new(),
            skills: Vec::new(),
        })
        .insert_resource(MapData::for_test(10, 10))
        .insert_resource(TitleActive(false))
        .insert_resource(ButtonInput::<KeyCode>::default());
    app.add_message::<AudioRequest>()
        .add_message::<ShopRequest>()
        .add_message::<BattleRequest>()
        .add_message::<ScreenEffect>()
        .add_message::<PictureCommand>()
        .add_message::<SpriteChange>()
        .add_message::<ShowMapAnimation>()
        .add_message::<crate::world::RelocateEvent>();
    app.insert_resource(CommonEvents::default());
    app.world_mut().spawn((
        Player {
            tile_x: 5,
            tile_y: 5,
            dir: 2,
            frame: 1,
            charset: "C".into(),
            index: 0,
        },
        MoveQueue::default(),
        RouteStepper::default(),
    ));
    app
}

fn switch_on(app: &App, id: u32) -> bool {
    app.world().resource::<Switches>().get(id)
}

fn set_switch(app: &mut App, id: u32, value: bool) {
    app.world_mut().resource_mut::<Switches>().set(id, value);
}

#[test]
fn common_autostart_fires_only_while_its_switch_is_on() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        1,
        5,
        vec![switch_cmd(10, 0, 0), cmd(0, 0, vec![])],
    )]));

    for _ in 0..3 {
        app.update();
    }
    assert!(
        !switch_on(&app, 10),
        "autostart must not fire with its switch off"
    );

    set_switch(&mut app, 5, true);
    app.update();
    assert!(switch_on(&app, 10), "autostart fires once its switch is on");

    set_switch(&mut app, 5, false);
    set_switch(&mut app, 10, false);
    for _ in 0..3 {
        app.update();
    }
    assert!(
        !switch_on(&app, 10),
        "autostart stops when its switch goes off"
    );
}

#[test]
fn parallel_common_event_steps_and_loops_every_frame() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        2,
        0,
        vec![switch_cmd(20, 2, 0), cmd(0, 0, vec![])],
    )]));

    // One toggle per frame proves the page runs, ends, and loops from the top.
    app.update();
    assert!(switch_on(&app, 20), "first pass toggles the switch on");
    assert_eq!(app.world().resource::<ParallelPool>().count(), 1);
    app.update();
    assert!(
        !switch_on(&app, 20),
        "the looped second pass toggles it back off"
    );
    app.update();
    assert!(
        switch_on(&app, 20),
        "and the third pass toggles it on again"
    );

    assert!(!app.world().resource::<RunningEvent>().active());
}

#[test]
fn parallel_map_page_runs_and_pauses_under_a_scene() {
    let mut app = interp_app();
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            4,
            vec![switch_cmd(21, 2, 0), cmd(0, 0, vec![])],
        )],
    });

    app.update();
    assert!(switch_on(&app, 21), "the trigger-4 page steps each frame");
    assert_eq!(app.world().resource::<ParallelPool>().count(), 1);

    app.world_mut().resource_mut::<BattleActive>().0 = true;
    let frozen = switch_on(&app, 21);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        switch_on(&app, 21),
        frozen,
        "a parallel page must not step while a battle owns the scene"
    );

    app.world_mut().resource_mut::<BattleActive>().0 = false;
    app.update();
    assert_ne!(
        switch_on(&app, 21),
        frozen,
        "the page resumes once the scene clears"
    );
}

#[test]
fn parallel_switches_continue_during_another_interpreters_message() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        2,
        0,
        vec![switch_cmd(22, 2, 0), cmd(0, 0, vec![])],
    )]));

    app.update();
    app.world_mut().resource_mut::<Dialogue>().active = true;
    for continue_events in [false, true] {
        app.world_mut()
            .resource_mut::<crate::dialogue::MessageOptions>()
            .continue_events = continue_events;
        for _ in 0..3 {
            let before = switch_on(&app, 22);
            app.update();
            assert_ne!(switch_on(&app, 22), before);
        }
    }
}

#[test]
fn parallel_writes_are_visible_to_the_foreground() {
    let mut app = interp_app();
    app.insert_resource(CommonEvents(vec![common(
        1,
        2,
        0,
        vec![switch_cmd(30, 0, 0), cmd(0, 0, vec![])],
    )]));
    app.update();
    assert!(
        switch_on(&app, 30),
        "the parallel event set the shared switch"
    );

    app.world_mut().resource_mut::<RunningEvent>().start(
        99,
        vec![
            cmd(CONDITIONAL_BRANCH, 0, vec![0, 30, 0]),
            switch_cmd(31, 0, 1),
            cmd(END_BRANCH, 0, vec![]),
        ],
    );
    app.update();
    assert!(
        switch_on(&app, 31),
        "the foreground interpreter observed the parallel event's switch write"
    );
}

#[test]
fn move_event_is_fire_and_forget() {
    let mut app = interp_app();
    // MoveEvent is fire-and-forget; the switch must run in the same burst.
    app.insert_resource(MapEvents {
        events: vec![map_event(
            1,
            3,
            vec![
                cmd(11330, 0, vec![10001, 8, 0, 0, 2]),
                switch_cmd(40, 0, 0),
                cmd(0, 0, vec![]),
            ],
        )],
    });
    app.update();
    assert!(
        switch_on(&app, 40),
        "MoveEvent must not block the command that follows it"
    );
    let world = app.world_mut();
    let mut q = world.query_filtered::<&RouteStepper, With<Player>>();
    assert!(
        q.single(world).unwrap().active(),
        "the decoded route is armed on the hero for the background stepper"
    );
}
