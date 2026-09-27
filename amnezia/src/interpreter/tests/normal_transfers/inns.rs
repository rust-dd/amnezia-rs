use super::*;

fn prepare(app: &mut App) {
    app.init_resource::<crate::shop::inn::State>()
        .insert_resource(crate::terms::Terms(crate::assets::load_ron(&format!(
            "{}/terms.ron",
            crate::assets::asset_root()
        ))));
    app.world_mut().resource_mut::<Inventory>().add_gold(100);
    app.world_mut().resource_mut::<Vitals>().set(1, 2, 0);
}

fn commands() -> Vec<EventCommand> {
    vec![
        cmd(10730, 0, vec![1, 30, 1]),
        cmd(20730, 0, vec![]),
        switch_cmd(40, 0, 1),
        cmd(20731, 0, vec![]),
        switch_cmd(41, 0, 1),
        cmd(20732, 0, vec![]),
        cmd(10220, 0, vec![0, 90, 90, 1, 0, 1]),
        switch_cmd(300, 1, 0),
    ]
}

fn open_parallel(app: &mut App) {
    set_switch(app, 300, true);
    app.insert_resource(CommonEvents(vec![common(1, 4, 300, commands())]));
    super::super::super::parallel::run_parallel(app.world_mut());
    crate::shop::inn::open_pending(app.world_mut());
    assert!(
        app.world()
            .resource::<crate::shop::inn::State>()
            .prompting()
    );
    assert!(!app.world().resource::<Dialogue>().from_foreground);
}

#[test]
fn abandoned_parallel_inn_closes_gold_without_rest_payment_or_outcome_handlers() {
    let mut app = app();
    prepare(&mut app);
    open_parallel(&mut app);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    arrive(&mut app, 4, false);
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(app.world().resource::<Dialogue>().lifecycle.gold.closing());
    assert!(!app.world().resource::<crate::shop::inn::State>().active());
    finish_transfer(&mut app);
    crate::dialogue::testing::finish_window_close(app.world_mut());
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<Inventory>().gold(), 100);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
    assert_eq!(app.world().resource::<Variables>().get(90), 1);
    assert!(!switch_on(&app, 40) && !switch_on(&app, 41));
    assert!(!app.world().resource::<ShopOpen>().0);
    assert!(app.world().resource::<Messages<AudioRequest>>().is_empty());
}

#[test]
fn a_foreground_inn_keeps_its_question_gold_window_and_owner_during_transfer() {
    let mut app = app();
    prepare(&mut app);
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, commands());
    super::super::super::driver::foreground(app.world_mut());
    crate::shop::inn::open_pending(app.world_mut());
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    arrive(&mut app, 4, false);
    assert!(app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<Dialogue>().lifecycle.gold.closing());
    assert!(
        app.world()
            .resource::<crate::shop::inn::State>()
            .prompting()
    );
    assert!(app.world().resource::<Choice>().active());
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(0));
}

#[test]
fn a_parallel_inn_that_replaced_a_foreground_choice_cancels_both_callbacks() {
    let mut app = app();
    prepare(&mut app);
    let mut commands = choices();
    commands.push(switch_cmd(60, 0, 0));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, commands);
    super::super::super::driver::foreground(app.world_mut());
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    open_parallel(&mut app);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    arrive(&mut app, 4, false);
    finish_transfer(&mut app);
    crate::dialogue::testing::finish_window_close(app.world_mut());
    for _ in 0..3 {
        app.update();
    }
    assert!(switch_on(&app, 60));
    assert_eq!(app.world().resource::<Variables>().get(90), 1);
    for id in [40, 41, 44] {
        assert!(!switch_on(&app, id));
    }
    assert_eq!(app.world().resource::<Inventory>().gold(), 100);
    assert!(!app.world().resource::<Dialogue>().active);
}
