use super::*;
use crate::battle::model::testkit::build_party2;

fn fixture() -> (App, Vec<Entity>) {
    let mut battle = build_party2();
    let mut extra = build_party2().enemies.remove(0);
    extra.x = 220;
    battle.enemies.push(extra);
    battle.phase = Phase::Command;
    battle.menu = MenuLevel::Target;
    let bases = battle
        .enemies
        .iter()
        .map(|foe| battler_base(foe.x, foe.y))
        .collect::<Vec<_>>();
    let mut app = App::new();
    app.init_resource::<GameFrames>()
        .insert_resource(battle)
        .add_message::<BattlerFlash>();
    register(&mut app);
    let entities = bases
        .into_iter()
        .enumerate()
        .map(|(index, base)| {
            app.world_mut()
                .spawn((
                    Battler {
                        index,
                        base,
                        height: 48.0,
                    },
                    Sprite::default(),
                    Transform::default(),
                ))
                .id()
        })
        .collect();
    app.update();
    (app, entities)
}

fn frame(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<GameFrames>().frame = frame;
    app.update();
}

fn alpha(app: &App, entity: Entity) -> u8 {
    app.world().get::<SpriteFlash>(entity).unwrap().0[3]
}

#[test]
fn selection_waits_sixty_frames_and_finishes_on_the_original_enemy_after_cursor_changes() {
    let (mut app, enemies) = fixture();
    frame(&mut app, 59);
    assert_eq!(alpha(&app, enemies[0]), 0);
    assert_eq!(
        app.world().get::<Sprite>(enemies[0]).unwrap().color,
        Color::WHITE
    );
    frame(&mut app, 60);
    assert_eq!(alpha(&app, enemies[0]), 192);
    app.world_mut().resource_mut::<Battle>().cursor = 1;
    frame(&mut app, 63);
    assert_eq!(alpha(&app, enemies[0]), 156);
    assert_eq!(alpha(&app, enemies[1]), 0);
    frame(&mut app, 76);
    assert_eq!(alpha(&app, enemies[0]), 0);
    frame(&mut app, 120);
    assert_eq!(alpha(&app, enemies[1]), 192);
    app.world_mut().resource_mut::<Battle>().menu = MenuLevel::Command;
    frame(&mut app, 121);
    assert_eq!(alpha(&app, enemies[1]), 180);
    frame(&mut app, 136);
    assert_eq!(alpha(&app, enemies[1]), 0);
    app.world_mut().resource_mut::<Battle>().menu = MenuLevel::Target;
    frame(&mut app, 137);
    frame(&mut app, 195);
    assert_eq!(alpha(&app, enemies[1]), 0);
    frame(&mut app, 196);
    assert_eq!(alpha(&app, enemies[1]), 192);
}

#[test]
fn flash_timeline_matches_at_low_and_high_render_rates_without_losing_the_last_pulse() {
    for fps in [15, 30, 60, 144] {
        let (mut app, enemies) = fixture();
        for _ in 0..fps * 4 {
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / fps as f64);
            app.update();
            let now = app.world().resource::<GameFrames>().frame;
            let age = now % 60;
            let expected = if now >= 60 && age < 16 {
                (16 - age) * 12
            } else {
                0
            };
            assert_eq!(
                u32::from(alpha(&app, enemies[0])),
                expected,
                "{fps} FPS, frame {now}"
            );
            assert_eq!(alpha(&app, enemies[1]), 0);
        }
    }
}

#[test]
fn scene_transition_freezes_existing_flashes_without_a_catchup_jump() {
    let (mut app, enemies) = fixture();
    frame(&mut app, 60);
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 60, IVec2::ZERO);
    app.insert_resource(transition);
    frame(&mut app, 90);
    assert_eq!(alpha(&app, enemies[0]), 192);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    frame(&mut app, 91);
    assert_eq!(alpha(&app, enemies[0]), 180);
}

#[test]
fn landed_hits_blink_by_hiding_the_sprite_in_five_frame_blocks() {
    let (mut app, enemies) = fixture();
    let base = app.world().get::<Battler>(enemies[0]).unwrap().base;
    app.world_mut()
        .resource_mut::<Battle>()
        .pending_blinks
        .push((base.x, base.y));
    frame(&mut app, 1);
    for age in 0..=20 {
        frame(&mut app, age + 1);
        let hidden = (20 - age) % 10 >= 5;
        assert_eq!(
            app.world().get::<Sprite>(enemies[0]).unwrap().color.alpha(),
            if hidden { 0.0 } else { 1.0 }
        );
        assert_eq!(
            app.world().get::<Sprite>(enemies[1]).unwrap().color,
            Color::WHITE
        );
    }
}

#[test]
fn animation_flashes_reach_only_the_matching_enemy_with_original_eight_bit_strength() {
    let (mut app, enemies) = fixture();
    app.world_mut().write_message(BattlerFlash {
        pos: Vec2::new(0.0, 80.0),
        rgb: [248; 3],
        power: 31,
        age: 0,
    });
    frame(&mut app, 1);
    assert_eq!(alpha(&app, enemies[0]), 0);
    let base = app.world().get::<Battler>(enemies[0]).unwrap().base;
    app.world_mut().write_message(BattlerFlash {
        pos: base,
        rgb: [248, 152, 72],
        power: 31,
        age: 0,
    });
    frame(&mut app, 2);
    assert_eq!(
        app.world().get::<SpriteFlash>(enemies[0]).unwrap().0,
        [248, 152, 72, 248]
    );
    assert_eq!(alpha(&app, enemies[1]), 0);
    frame(&mut app, 5);
    assert_eq!(alpha(&app, enemies[0]), 200);
    frame(&mut app, 13);
    assert_eq!(alpha(&app, enemies[0]), 0);
}

#[test]
fn action_start_flashes_only_the_acting_enemy_for_ten_logical_frames() {
    use crate::battle::model::{Action, Command, Source};
    for fps in [15, 30, 60, 144] {
        let (mut app, enemies) = fixture();
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.phase = Phase::Resolve;
        battle.queue = vec![Action {
            source: Source::Enemy(0),
            kind: Command::Defend,
            agility: 1,
        }];
        battle.resolve_next();
        frame(&mut app, 1);
        assert_eq!(
            app.world().get::<SpriteFlash>(enemies[0]).unwrap().0,
            [248, 248, 248, 80]
        );
        for _ in 0..fps {
            app.world_mut()
                .resource_mut::<GameFrames>()
                .advance(1.0 / fps as f64);
            app.update();
            let age = app.world().resource::<GameFrames>().frame - 1;
            assert_eq!(
                u32::from(alpha(&app, enemies[0])),
                10_u32.saturating_sub(age) * 8
            );
            assert_eq!(alpha(&app, enemies[1]), 0);
        }
    }
}

#[test]
fn silent_cancelled_actions_do_not_flash_but_deliberate_ai_noops_do() {
    use crate::battle::model::{Action, Command, Source};
    for (kind, expected) in [
        (Command::Nothing, 0),
        (Command::DoNothing, 80),
        (Command::Observe, 80),
        (
            Command::Skill {
                skill_id: 999,
                target: 0,
            },
            0,
        ),
        (
            Command::Item {
                item_id: 999,
                target: 0,
            },
            0,
        ),
    ] {
        let (mut app, enemies) = fixture();
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.phase = Phase::Resolve;
        battle.queue = vec![Action {
            source: Source::Enemy(0),
            kind,
            agility: 1,
        }];
        battle.resolve_next_with_items(|_| false);
        frame(&mut app, 1);
        assert_eq!(alpha(&app, enemies[0]), expected);
        assert_eq!(alpha(&app, enemies[1]), 0);
    }
}

#[test]
fn an_incapacitated_battler_only_flashes_for_a_visible_starting_state_message() {
    use crate::battle::model::{Action, Command, Source};
    for message in ["", " vár az ébredésre"] {
        let (mut app, enemies) = fixture();
        let mut battle = app.world_mut().resource_mut::<Battle>();
        battle.phase = Phase::Resolve;
        battle.states =
            crate::assets::load_ron(&format!("{}/states.ron", crate::assets::asset_root()));
        let state = battle
            .states
            .iter_mut()
            .find(|state| state.id == 7)
            .unwrap();
        state.hold_turn = 99;
        state.auto_release_prob = 0;
        state.message_affected = message.into();
        battle.enemies[0].states = vec![(7, 0)];
        battle.queue = vec![Action {
            source: Source::Enemy(0),
            kind: Command::Nothing,
            agility: 1,
        }];
        battle.resolve_next();
        frame(&mut app, 1);
        assert_eq!(
            alpha(&app, enemies[0]),
            if message.is_empty() { 0 } else { 80 }
        );
    }
}
