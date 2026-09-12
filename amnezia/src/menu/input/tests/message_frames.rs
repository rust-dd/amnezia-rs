use super::*;

fn close_prompts(
    mut dialogue: ResMut<Dialogue>,
    mut choice: ResMut<Choice>,
    mut number: ResMut<InputNumber>,
) {
    dialogue.close();
    choice.active = false;
    number.active = false;
}

#[test]
fn a_message_choice_or_number_close_cannot_reuse_cancel_to_open_the_menu() {
    for kind in 0..3 {
        let mut app = app_on(0, MenuScreen::Command);
        crate::dialogue::InputPrompts::register(&mut app);
        app.world_mut().resource_mut::<MenuOpen>().0 = false;
        match kind {
            0 => app.world_mut().resource_mut::<Dialogue>().open(Vec::new()),
            1 => app
                .world_mut()
                .resource_mut::<Choice>()
                .open(vec!["Igen".into()], 0, 1),
            _ => app.world_mut().resource_mut::<InputNumber>().open(2, 1),
        }
        app.add_systems(Update, close_prompts.before(menu_input));
        press_frame(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<MenuOpen>().0, "prompt {kind}");
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::Escape)
        );
        press_frame(&mut app, KeyCode::Escape);
        assert!(app.world().resource::<MenuOpen>().0);
    }
}
