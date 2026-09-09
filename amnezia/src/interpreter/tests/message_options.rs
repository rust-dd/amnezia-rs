use super::*;

#[test]
fn original_message_options_preserve_transparency_position_fixed_and_continuation() {
    let mut app = interp_app();
    for params in [
        vec![0, 0, 1, 0],
        vec![1, 2, 0, 1],
        vec![0, 0, 0, 1],
        vec![0, 2, 1, 1],
    ] {
        app.world_mut()
            .resource_mut::<RunningEvent>()
            .start(1, vec![cmd(10120, 0, params.clone())]);
        app.update();
        let options = app.world().resource::<crate::dialogue::MessageOptions>();
        assert_eq!(options.fixed, params[2] == 0);
        assert_eq!(options.continue_events, params[3] != 0);
        assert_eq!(
            app.world().resource::<MessageTransparent>().0,
            params[0] != 0
        );
        assert_eq!(
            *app.world().resource::<MessagePosition>(),
            if params[1] == 0 {
                MessagePosition::Top
            } else {
                MessagePosition::Bottom
            }
        );
    }
}
