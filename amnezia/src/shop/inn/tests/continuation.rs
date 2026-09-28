use super::*;
use crate::interpreter::{CommonEvents, RunningEvent};
use crate::state::Variables;
use amnezia_data::{CommonEvent, EventCommand};

mod clocks;
mod destination;
mod messages;
mod music;

fn map_event(commands: Vec<EventCommand>) -> amnezia_data::Event {
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0052.ron",
        crate::assets::asset_root()
    ));
    let mut event = map.events.into_iter().next().unwrap();
    event.id = 1;
    event.pages.truncate(1);
    event.pages[0].condition = default();
    event.pages[0].trigger = 4;
    event.pages[0].commands = commands;
    event
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        params,
        string: String::new(),
    }
}

fn increment(id: i32) -> EventCommand {
    command(10220, vec![0, id, id, 1, 0, 1])
}

fn common(id: u32, commands: Vec<EventCommand>) -> CommonEvent {
    CommonEvent {
        id,
        name: String::new(),
        trigger: 4,
        switch_flag: false,
        switch_id: 0,
        commands,
    }
}

fn interpreter_app() -> App {
    let mut app = crate::interpreter::tests::interp_app();
    app.add_plugins(crate::transitions::TransitionPlugin)
        .init_resource::<State>()
        .init_resource::<CurrentBgm>()
        .init_resource::<SystemMusic>()
        .insert_resource(Terms(crate::assets::load_ron(&format!(
            "{}/terms.ron",
            crate::assets::asset_root()
        ))));
    register_flow(&mut app);
    crate::dialogue::testing::register_playback(&mut app);
    app.update();
    app.world_mut().resource_mut::<Inventory>().add_gold(100);
    app.world_mut().resource_mut::<Vitals>().set(1, 2, 0);
    app
}

fn tick(app: &mut App, frame: u32) {
    let waiting =
        app.world().resource::<Transition>().busy() || app.world().resource::<State>().resting();
    app.insert_resource(crate::timing::SceneWait(waiting));
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

fn counts(app: &App) -> [i32; 3] {
    let variables = app.world().resource::<Variables>();
    std::array::from_fn(|index| variables.get(index as u32 + 1))
}

#[test]
fn accepting_an_inn_fades_music_while_the_message_is_still_closing() {
    let mut app = app();
    open(&mut app, 30);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    crate::dialogue::testing::dismiss(app.world_mut());
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    app.world_mut().run_system_once(flow::accept).unwrap();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(app.world().resource::<Dialogue>().busy());
    assert!(!app.world().resource::<ShopOpen>().0);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::FadeOutBgm { duration: 0.8 }]
    );
}

#[test]
fn accepting_an_inn_runs_the_second_window_close_step_in_the_decision_frame() {
    let mut app = app();
    open(&mut app, 30);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    crate::dialogue::testing::dismiss(app.world_mut());
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    app.world_mut().run_system_once(flow::accept).unwrap();
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .lifecycle
            .message
            .half_height(80),
        28
    );
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .lifecycle
            .gold
            .half_height(32),
        11
    );
}

#[test]
fn free_inn_resumes_foreground_on_the_terminal_fade_frame_without_parallel_replay() {
    let mut app = interpreter_app();
    app.insert_resource(CommonEvents(vec![common(1, vec![increment(1)])]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![command(10730, vec![0, 0, 1]), increment(2)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 0, 0]);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 0, 0]);
    tick(&mut app, 70);
    assert_eq!(counts(&app), [1, 1, 0]);
    assert!(!app.world().resource::<RunningEvent>().active());
    tick(&mut app, 71);
    assert_eq!(counts(&app), [2, 1, 0]);
}

#[test]
fn free_inn_suspends_and_resumes_the_common_interpreter_before_later_events() {
    let mut app = interpreter_app();
    app.insert_resource(CommonEvents(vec![
        common(
            1,
            vec![
                command(10730, vec![0, 0, 1]),
                increment(1),
                command(11410, vec![100]),
            ],
        ),
        common(2, vec![increment(2)]),
    ]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(3)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [0, 0, 0]);
    tick(&mut app, 35);
    tick(&mut app, 70);
    assert_eq!(counts(&app), [1, 1, 1]);
}

#[test]
fn a_free_silent_inn_on_an_erased_screen_heals_and_starts_show_without_an_extra_tick() {
    let mut app = interpreter_app();
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![command(10730, vec![0, 0, 1]), increment(1)]);
    tick(&mut app, 0);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), None);
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::FadeIn
    ));
    assert!(!app.world().resource::<Transition>().event_erased);
    tick(&mut app, 35);
    assert_eq!(counts(&app), [1, 0, 0]);
}

#[test]
fn a_free_inn_keeps_the_interpreter_budget_even_when_it_is_the_last_command() {
    for parallel in [false, true] {
        for before in [9_998, 9_999] {
            let mut app = interpreter_app();
            let mut commands = vec![increment(1); before];
            commands.extend([command(10730, vec![0, 0, 1]), increment(2), increment(3)]);
            if parallel {
                app.insert_resource(CommonEvents(vec![common(1, commands)]));
            } else {
                app.world_mut()
                    .resource_mut::<RunningEvent>()
                    .start(7, commands);
            }
            tick(&mut app, 0);
            assert_eq!(counts(&app), [before as i32, 0, 0]);
            tick(&mut app, 35);
            tick(&mut app, 70);
            assert_eq!(counts(&app), [before as i32, i32::from(before == 9_998), 0]);
            tick(&mut app, 71);
            assert_eq!(counts(&app), [before as i32, 1, 1]);
        }
    }
}

#[test]
fn a_free_map_inn_resumes_its_disabled_page_without_repeating_common_events() {
    let mut app = interpreter_app();
    app.insert_resource(CommonEvents(vec![common(1, vec![increment(1)])]));
    let mut event = map_event(vec![
        command(10210, vec![0, 5, 5, 1]),
        command(10730, vec![0, 0, 1]),
        increment(2),
    ]);
    event.pages[0].condition.flags = 1;
    event.pages[0].condition.switch_a = 5;
    app.insert_resource(crate::world::MapEvents {
        events: vec![event],
    });
    app.world_mut()
        .resource_mut::<crate::state::Switches>()
        .set(5, true);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(3)]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 0, 0]);
    tick(&mut app, 35);
    tick(&mut app, 70);
    assert_eq!(counts(&app), [1, 1, 1]);
    tick(&mut app, 71);
    assert_eq!(counts(&app), [2, 1, 1]);
}

#[test]
fn nested_make_way_discards_free_rest_but_retains_the_stay_branch_and_command_cursor() {
    let mut app = interpreter_app();
    let mut stay = increment(1);
    stay.indent = 1;
    let mut no_stay = increment(2);
    no_stay.indent = 1;
    let event = map_event(vec![
        command(10730, vec![0, 0, 1]),
        command(20730, vec![]),
        stay,
        command(20731, vec![]),
        no_stay,
        command(20732, vec![]),
        command(11410, vec![100]),
    ]);
    app.insert_resource(crate::world::MapEvents {
        events: vec![event],
    });
    assert!(!crate::interpreter::update_map_event(app.world_mut(), 1));
    assert!(!app.world().resource::<State>().active());
    assert_eq!(counts(&app), [0; 3]);
    tick(&mut app, 0);
    assert_eq!(counts(&app), [1, 0, 0]);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
    assert!(!app.world().resource::<Transition>().busy());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn an_ordinary_transfer_waits_for_inn_continuation_and_the_remaining_map_visit() {
    let mut app = interpreter_app();
    app.add_plugins((AssetPlugin::default(), crate::teleport::TeleportPlugin))
        .init_asset::<Image>()
        .add_message::<crate::world::MapChanged>();
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<crate::player::Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(hero)
        .insert(Transform::default());
    app.insert_resource(CommonEvents(vec![
        common(
            1,
            vec![
                command(10810, vec![3, 7, 8]),
                command(10730, vec![0, 0, 1]),
                increment(1),
            ],
        ),
        common(2, vec![increment(2)]),
    ]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, vec![increment(3)]);
    for frame in [0, 35, 69] {
        tick(&mut app, frame);
        assert_eq!(counts(&app), [0; 3]);
        assert!(!app.world().resource::<crate::teleport::Fade>().busy());
        assert_eq!(
            app.world().resource::<crate::teleport::PendingTeleport>().0,
            Some((3, 7, 8))
        );
    }
    tick(&mut app, 70);
    assert_eq!(counts(&app), [1; 3]);
    assert!(app.world().resource::<crate::teleport::Fade>().busy());
    assert!(!app.world().resource::<State>().active());
}
