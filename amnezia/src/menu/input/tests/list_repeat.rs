use super::*;
use crate::timing::GameFrames;

fn screen(skill: bool, cursor: usize) -> MenuScreen {
    if skill {
        MenuScreen::SkillList { member: 0, cursor }
    } else {
        MenuScreen::ItemList { cursor }
    }
}

fn prepared(skill: bool) -> App {
    let mut app = app_on(0, screen(skill, 0));
    let mut data = app.world_mut().resource_mut::<GameData>();
    data.items = (100..132)
        .map(|id| {
            let mut item = testkit::herb();
            item.id = id;
            item.scope = 1;
            item
        })
        .collect();
    data.skills = (100..132)
        .map(|id| {
            let mut skill = testkit::heal_skill(id, "Gyógyítás", 1, 10);
            skill.scope = 2;
            skill
        })
        .collect();
    data.actors[0].learnings = (100..132)
        .map(|skill_id| amnezia_data::Learning { level: 1, skill_id })
        .collect();
    for id in 100..132 {
        app.world_mut().resource_mut::<Inventory>().add_item(id, 1);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    app
}

fn tick(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn held_before_opening(skill: bool) {
    let mut app = prepared(skill);
    app.world_mut().resource_mut::<MenuOpen>().0 = false;
    for frame in 1..=28 {
        if frame == 11 {
            app.world_mut().resource_mut::<MenuOpen>().0 = true;
            app.world_mut().resource_mut::<MenuState>().screen = screen(skill, 0);
        }
        tick(&mut app, frame);
        if frame >= 11 {
            let cursor = if frame < 24 {
                0
            } else if frame < 28 {
                2
            } else {
                4
            };
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(skill, cursor),
                "frame {frame}"
            );
        }
    }
}

#[test]
fn item_list_keeps_the_hold_phase_from_before_it_opened() {
    held_before_opening(false);
}

#[test]
fn skill_list_keeps_the_hold_phase_from_before_it_opened() {
    held_before_opening(true);
}

#[test]
fn a_transition_keeps_hold_time_but_discards_the_paused_repeats() {
    for skill in [false, true] {
        let mut app = prepared(skill);
        for frame in 1..=32 {
            if frame == 10 {
                let mut transition = crate::transitions::Transition::default();
                transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
                app.insert_resource(transition);
            } else if frame == 30 {
                app.world_mut()
                    .resource_mut::<crate::transitions::Transition>()
                    .clear();
            }
            tick(&mut app, frame);
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(skill, if frame == 32 { 4 } else { 2 }),
                "skill={skill}, frame={frame}"
            );
        }
    }
}

#[test]
fn a_fixed_target_keeps_hold_time_without_replaying_hidden_list_moves() {
    for skill in [false, true] {
        let mut app = prepared(skill);
        for frame in 1..=32 {
            if frame == 10 {
                app.world_mut().resource_mut::<MenuState>().screen = if skill {
                    MenuScreen::SkillTarget {
                        member: 0,
                        skill_id: 100,
                        cursor: 0,
                    }
                } else {
                    MenuScreen::ItemTarget {
                        item_id: 100,
                        cursor: 0,
                    }
                };
            } else if frame == 30 {
                app.world_mut().resource_mut::<MenuState>().screen = screen(skill, 2);
            }
            tick(&mut app, frame);
            if frame >= 30 {
                assert_eq!(
                    app.world().resource::<MenuState>().screen,
                    screen(skill, if frame == 32 { 4 } else { 2 }),
                    "skill={skill}, frame={frame}"
                );
            }
        }
    }
}

#[test]
fn switching_between_item_and_skill_lists_keeps_the_shared_hold_phase() {
    for first_skill in [false, true] {
        let mut app = prepared(first_skill);
        for frame in 1..=28 {
            if frame == 10 {
                app.world_mut().resource_mut::<MenuState>().screen = screen(!first_skill, 0);
            }
            tick(&mut app, frame);
            if frame >= 10 {
                let cursor = if frame < 24 {
                    0
                } else if frame < 28 {
                    2
                } else {
                    4
                };
                assert_eq!(
                    app.world().resource::<MenuState>().screen,
                    screen(!first_skill, cursor)
                );
            }
        }
    }
}

#[test]
fn a_file_selector_does_not_reset_the_list_hold_or_queue_its_ignored_moves() {
    for skill in [false, true] {
        let mut app = prepared(skill);
        for frame in 1..=32 {
            if frame == 10 {
                app.world_mut().resource_mut::<SaveFiles>().request();
            } else if frame == 30 {
                app.insert_resource(SaveFiles::default());
            }
            tick(&mut app, frame);
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(skill, if frame == 32 { 4 } else { 2 })
            );
        }
    }
}

#[test]
fn rewinding_the_clock_does_not_catch_up_the_list_or_keep_the_old_repeat_phase() {
    for skill in [false, true] {
        let mut app = prepared(skill);
        for frame in 1..=24 {
            tick(&mut app, frame);
        }
        assert_eq!(app.world().resource::<MenuState>().screen, screen(skill, 4));
        for frame in 0..=24 {
            tick(&mut app, frame);
            assert_eq!(
                app.world().resource::<MenuState>().screen,
                screen(skill, if frame == 24 { 6 } else { 4 })
            );
        }
    }
}
