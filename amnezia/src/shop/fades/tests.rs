use super::*;
use crate::shop::{Phase, ShopOutcome, ShopRequest, flow};
use crate::timing::{FrameClockSet, SceneFrames, SceneWait};

#[derive(Resource, Default)]
struct MapPaused(bool);

fn observe_pause(pause: crate::world::ScenePause, mut recorded: ResMut<MapPaused>) {
    recorded.0 = pause.paused();
}

fn fixture(buy: bool, sell: bool) -> App {
    let mut app = App::new();
    app.add_plugins((
        crate::gamedata::GameDataPlugin,
        crate::transitions::TransitionPlugin,
    ))
    .init_resource::<SceneFrames>()
    .init_resource::<SceneWait>()
    .init_resource::<MapPaused>()
    .init_resource::<Inventory>()
    .init_resource::<Screen>()
    .init_resource::<ShopOpen>()
    .init_resource::<ShopOutcome>()
    .init_resource::<crate::menu::DirectionInput>()
    .init_resource::<ButtonInput<KeyCode>>()
    .add_message::<ShopRequest>()
    .add_message::<crate::audio::AudioRequest>()
    .add_systems(PreUpdate, capture_wait.in_set(FrameClockSet))
    .add_systems(
        Update,
        (flow::open_requests, flow::shop_input, observe_pause)
            .chain()
            .in_set(ShopUpdate),
    );
    register(&mut app);
    app.update();
    app.world_mut().resource_mut::<Inventory>().add_item(2, 1);
    app.world_mut().write_message(ShopRequest::OpenShop {
        items: vec![2, 3],
        allow_buy: buy,
        allow_sell: sell,
        shop_type: 0,
    });
    tick(&mut app, &[]);
    app
}

fn capture_wait(flow: Res<Flow>, transition: Res<Transition>, mut wait: ResMut<SceneWait>) {
    wait.0 = flow.active() || transition.busy();
}

fn tick(app: &mut App, keys: &[KeyCode]) {
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    if !app.world().resource::<Flow>().active() {
        app.world_mut().resource_mut::<SceneFrames>().frame += 1;
    }
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

fn state(app: &App) -> &ShopState {
    let Screen::Shop(state) = app.world().resource::<Screen>() else {
        panic!("shop is not visible");
    };
    state
}

fn assert_age(app: &App, age: u32) {
    assert!(app.world().resource::<MapPaused>().0);
    assert!(app.world().resource::<Flow>().active());
    let transition = app.world().resource::<Transition>();
    assert!(transition.busy());
    assert_eq!(transition.age(), age);
}

#[test]
fn all_shop_modes_initialize_at_black_and_wait_six_frames_on_both_sides() {
    for (buy, sell, help) in [(true, true, 0), (true, false, 2), (false, true, 2)] {
        let mut app = fixture(buy, sell);
        let scene_frame = app.world().resource::<SceneFrames>().frame;
        for age in 0_u32..7 {
            assert_age(&app, age.saturating_sub(1));
            assert!(!app.world().resource::<ShopOpen>().0);
            assert!(matches!(app.world().resource::<Screen>(), Screen::Closed));
            tick(&mut app, &[KeyCode::Enter, KeyCode::Escape]);
        }
        for age in 0_u32..7 {
            assert_age(&app, age.saturating_sub(1));
            assert!(app.world().resource::<ShopOpen>().0);
            let scene = &state(&app).scene;
            assert_eq!(scene.help_id, 0);
            assert!(!scene.updated);
            assert_eq!(
                (scene.command_frame, scene.party_frame, scene.number_frame),
                (0, 0, 0)
            );
            assert_eq!((scene.buy.index, scene.sell.index), (0, 0));
            tick(&mut app, &[KeyCode::Enter, KeyCode::Escape]);
        }
        assert!(!app.world().resource::<Flow>().active());
        assert!(!app.world().resource::<Transition>().busy());
        assert!(app.world().resource::<SceneWait>().0);
        assert_eq!(app.world().resource::<SceneFrames>().frame, scene_frame);
        assert_eq!(state(&app).scene.command_frame, 0);
        tick(&mut app, &[]);
        assert!(!app.world().resource::<SceneWait>().0);
        assert_eq!(state(&app).scene.command_frame, 1);
        assert!(state(&app).scene.updated);
        assert_eq!(state(&app).scene.help_id, help);
        assert!(!app.world().resource::<ShopOutcome>().transacted);
    }
}

#[test]
fn leaving_keeps_the_shop_until_black_and_owns_the_entire_return_to_map() {
    let mut app = fixture(true, true);
    for _ in 0..15 {
        tick(&mut app, &[]);
    }
    tick(&mut app, &[KeyCode::Escape]);
    let frame = state(&app).scene.command_frame;
    for age in 0_u32..7 {
        assert_age(&app, age.saturating_sub(1));
        assert!(app.world().resource::<ShopOpen>().0);
        assert_eq!(state(&app).scene.command_frame, frame);
        tick(&mut app, &[KeyCode::Enter]);
    }
    for age in 0_u32..7 {
        assert_age(&app, age.saturating_sub(1));
        assert!(!app.world().resource::<ShopOpen>().0);
        assert!(matches!(app.world().resource::<Screen>(), Screen::Closed));
        tick(&mut app, &[KeyCode::Enter]);
    }
    assert!(!app.world().resource::<Flow>().active());
    assert!(app.world().resource::<SceneWait>().0);
    assert!(app.world().resource::<MapPaused>().0);
    tick(&mut app, &[]);
    assert!(!app.world().resource::<SceneWait>().0);
    assert!(!app.world().resource::<MapPaused>().0);
}

#[test]
fn shop_subwindows_share_the_scene_without_additional_fades() {
    let mut app = fixture(true, true);
    for _ in 0..15 {
        tick(&mut app, &[]);
    }
    tick(&mut app, &[KeyCode::Enter]);
    assert!(matches!(state(&app).phase, Phase::Buy { .. }));
    assert!(!app.world().resource::<Flow>().active());
    tick(&mut app, &[KeyCode::Escape]);
    assert!(matches!(state(&app).phase, Phase::Command { .. }));
    assert!(!app.world().resource::<Transition>().busy());
}
