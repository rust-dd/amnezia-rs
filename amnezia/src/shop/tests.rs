use super::*;
use crate::audio::{AudioRequest, SystemSounds};
use crate::gamedata::{GameData, GameDataPlugin};
use crate::state::Inventory;
use crate::timing::{GameFrames, SceneFrames, SceneWait};

mod quantity;

fn app(phase: Phase) -> App {
    let mut app = App::new();
    let sound = |name: &str| amnezia_data::SoundDef {
        name: name.into(),
        volume: 100,
        tempo: 100,
        ..default()
    };
    app.add_plugins(GameDataPlugin)
        .init_resource::<Time>()
        .init_resource::<GameFrames>()
        .init_resource::<SceneFrames>()
        .init_resource::<SceneWait>()
        .init_resource::<crate::menu::DirectionInput>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Inventory>()
        .init_resource::<crate::vitals::Vitals>()
        .init_resource::<ShopOutcome>()
        .insert_resource(ShopOpen(true))
        .insert_resource(SystemSounds {
            cursor: sound("CURSOR"),
            decision: sound("DECISION"),
            cancel: sound("CANCEL"),
            buzzer: sound("BUZZER"),
            ..default()
        })
        .insert_resource(Screen::Shop(ShopState {
            items: vec![1, 2, 3, 4],
            allow_buy: true,
            allow_sell: true,
            shop_type: 0,
            phase,
        }))
        .add_message::<AudioRequest>()
        .add_systems(
            Update,
            (crate::menu::update_directions, flow::shop_input).chain(),
        );
    app.world_mut()
        .resource_mut::<Inventory>()
        .add_gold(100_000);
    app.update();
    app
}

fn tick(app: &mut App, ticks: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame += ticks;
    app.world_mut().resource_mut::<SceneFrames>().frame += ticks;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f64(ticks as f64 / 60.0));
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.reset_all();
    for key in keys {
        input.press(*key);
    }
    tick(app, 1);
}

fn sounds(app: &mut App) -> Vec<String> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .map(|request| match request {
            AudioRequest::Sound { name, .. } => name,
            _ => panic!("unexpected shop audio"),
        })
        .collect()
}

fn phase(app: &App) -> &Phase {
    let Screen::Shop(state) = app.world().resource::<Screen>() else {
        panic!("shop closed unexpectedly");
    };
    &state.phase
}

fn number(mode: Mode, count: u32, max: u32, origin: usize) -> Phase {
    Phase::Number(NumberState {
        mode,
        item_id: 1,
        count,
        max,
        unit_price: 10,
        origin,
    })
}

#[test]
fn sell_list_keeps_disabled_items_and_refuses_to_destroy_them() {
    let mut app = app(Phase::Sell { cursor: 0 });
    app.world_mut().resource_mut::<GameData>().items[0].price = 0;
    app.world_mut().resource_mut::<Inventory>().add_item(1, 3);
    assert_eq!(
        logic::sell_ids(
            app.world().resource::<GameData>(),
            app.world().resource::<Inventory>()
        ),
        [1]
    );
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(phase(&app), Phase::Sell { cursor: 0 }));
    assert_eq!(sounds(&mut app), ["BUZZER"]);
    assert_eq!(app.world().resource::<Inventory>().count(1), 3);
    assert!(!app.world().resource::<ShopOutcome>().transacted);
    app.world_mut()
        .resource_scope(|world, data: Mut<GameData>| {
            assert!(!logic::apply_trade(
                Mode::Sell,
                1,
                &data,
                &mut world.resource_mut::<Inventory>()
            ));
        });
}

#[test]
fn empty_buy_and_sell_lists_buzz_without_opening_quantity() {
    for sell in [false, true] {
        let mut app = app(if sell {
            Phase::Sell { cursor: 0 }
        } else {
            Phase::Buy { cursor: 0 }
        });
        if let Screen::Shop(state) = &mut *app.world_mut().resource_mut::<Screen>() {
            state.items.clear();
        }
        press(&mut app, &[KeyCode::Enter]);
        assert!(!matches!(phase(&app), Phase::Number(_)));
        assert_eq!(sounds(&mut app), ["BUZZER"]);
    }
}

#[test]
fn purchase_and_sale_hold_sixty_scene_frames_then_restore_the_item_cursor() {
    for mode in [Mode::Buy, Mode::Sell] {
        for fps in [15, 30, 60, 120, 144] {
            let mut app = app(number(mode, 1, 9, 2));
            app.world_mut().resource_mut::<GameData>().items[2].price = 10;
            if let Screen::Shop(state) = &mut *app.world_mut().resource_mut::<Screen>()
                && let Phase::Number(number) = &mut state.phase
            {
                number.item_id = 3;
            }
            for id in 1..=3 {
                app.world_mut().resource_mut::<Inventory>().add_item(id, 5);
            }
            press(&mut app, &[KeyCode::Enter]);
            assert_eq!(sounds(&mut app), ["DECISION"]);
            let mut previous = 0;
            for render in 1..=fps {
                let elapsed = render * 60 / fps;
                tick(&mut app, elapsed - previous);
                previous = elapsed;
                if elapsed < 60 {
                    assert!(
                        matches!(phase(&app), Phase::Bought { .. } | Phase::Sold { .. }),
                        "confirmation ended at {elapsed} ticks, {fps} FPS"
                    );
                } else {
                    assert!(matches!(
                        phase(&app),
                        Phase::Buy { cursor: 2 } | Phase::Sell { cursor: 2 }
                    ));
                }
            }
            assert_eq!(
                app.world().resource::<Inventory>().count(3),
                if mode == Mode::Buy { 6 } else { 4 }
            );
            assert_eq!(
                app.world().resource::<Inventory>().gold(),
                if mode == Mode::Buy { 99_990 } else { 100_005 }
            );
        }
    }
}

#[test]
fn confirmation_holds_through_async_waits_and_discards_its_return_frame_keys() {
    let mut app = app(number(Mode::Buy, 1, 9, 2));
    app.world_mut().resource_mut::<GameData>().items[0].price = 10;
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(sounds(&mut app), ["DECISION"]);
    app.world_mut().resource_mut::<SceneWait>().0 = true;
    tick(&mut app, 240);
    assert!(matches!(phase(&app), Phase::Bought { remaining: 60, .. }));
    app.world_mut().resource_mut::<SceneWait>().0 = false;
    tick(&mut app, 59);
    assert!(matches!(phase(&app), Phase::Bought { remaining: 1, .. }));
    press(&mut app, &[KeyCode::Enter, KeyCode::ArrowDown]);
    assert!(matches!(phase(&app), Phase::Buy { cursor: 2 }));
    assert!(sounds(&mut app).is_empty());
    assert_eq!(app.world().resource::<Inventory>().count(1), 1);
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(phase(&app), Phase::Number(_)));
    assert_eq!(sounds(&mut app), ["DECISION"]);
}

#[test]
fn selling_the_last_selected_stack_clamps_the_return_cursor_to_the_remaining_list() {
    let mut app = app(number(Mode::Sell, 1, 1, 2));
    if let Screen::Shop(state) = &mut *app.world_mut().resource_mut::<Screen>()
        && let Phase::Number(number) = &mut state.phase
    {
        number.item_id = 3;
    }
    for id in 1..=3 {
        app.world_mut().resource_mut::<Inventory>().add_item(id, 1);
    }
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(app.world().resource::<Inventory>().count(3), 0);
    tick(&mut app, 60);
    assert!(matches!(phase(&app), Phase::Sell { cursor: 1 }));
}

#[test]
fn an_empty_trade_never_reports_a_transaction_or_starts_confirmation() {
    for mode in [Mode::Buy, Mode::Sell] {
        let mut app = app(number(mode, 1, 9, 0));
        app.world_mut().resource_mut::<GameData>().items[0].price = 10;
        app.world_mut()
            .resource_mut::<Inventory>()
            .remove_gold(100_000);
        press(&mut app, &[KeyCode::Enter]);
        assert!(matches!(phase(&app), Phase::Number(_)));
        assert!(!app.world().resource::<ShopOutcome>().transacted);
        assert_eq!(sounds(&mut app), ["BUZZER"]);
    }
}
