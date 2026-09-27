use super::*;

#[test]
fn a_transfer_restarts_closing_without_discarding_an_accepted_parallel_choice() {
    let mut app = app();
    let mut commands = choices();
    commands.extend([
        cmd(10220, 0, vec![0, 90, 90, 1, 0, 1]),
        switch_cmd(300, 1, 0),
    ]);
    parallel(&mut app, commands);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    {
        let mut choice = app.world_mut().resource_mut::<Choice>();
        choice.active = false;
        choice.result = Some(0);
    }
    crate::dialogue::testing::update_prompt(app.world_mut());
    app.world_mut().resource_mut::<GameFrames>().frame += 2;
    crate::dialogue::testing::tick(app.world_mut());
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .lifecycle
            .message
            .half_height(80),
        22
    );
    arrive(&mut app, 4, false);
    assert_eq!(
        app.world()
            .resource::<Dialogue>()
            .lifecycle
            .message
            .half_height(80),
        34
    );
    assert_eq!(app.world().resource::<Choice>().result, Some(0));
    finish_transfer(&mut app);
    crate::dialogue::testing::finish_window_close(app.world_mut());
    for _ in 0..3 {
        app.update();
    }
    assert!(switch_on(&app, 40));
    assert!(!switch_on(&app, 41) && !switch_on(&app, 44));
    assert_eq!(app.world().resource::<Variables>().get(90), 1);
    assert!(!app.world().resource::<Dialogue>().active);
}
