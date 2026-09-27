use crate::world::test_support::*;
use crate::world::{Character, EventSprite, MapData, MoveQueue, RouteStepper};
use amnezia_data::EventCommand;
use bevy::prelude::*;

mod inactive;
mod ordering;
mod routes;

fn counter() -> EventCommand {
    EventCommand {
        code: 10220,
        indent: 0,
        string: String::new(),
        params: vec![0, 1, 1, 1, 0, 1],
    }
}

#[test]
fn a_chain_of_characters_steps_out_of_the_way_once_each_in_one_map_update() {
    let mut app = app(
        (1..=3)
            .map(|id| event(id, id, vec![page(vec![command(1, 0)])]))
            .collect(),
        false,
    );
    app.update();
    for id in 1..=3 {
        let npc = entity(&mut app, id);
        let character = app.world().get::<EventSprite>(npc).unwrap();
        assert_eq!(character.tile(), (id as i32 + 1, 1));
        let queue = app.world().get::<MoveQueue>(npc).unwrap();
        let data = app.world().resource::<MapData>();
        let origin = data.tile_center(id as i32, 1).0;
        assert_eq!(queue.render_position(character, data).x - origin, 2.0);
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            0
        );
    }
}

#[test]
fn opposing_movers_do_not_recurse_or_tick_the_same_character_twice() {
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![command(1, 0)])]),
            event(2, 2, vec![page(vec![command(3, 0)])]),
        ],
        false,
    );
    app.update();
    for id in 1..=2 {
        let npc = entity(&mut app, id);
        assert_eq!(
            app.world().get::<EventSprite>(npc).unwrap().tile_x,
            id as i32
        );
        assert_eq!(
            app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
            1
        );
    }
}

#[test]
fn make_way_runs_parallel_scripts_before_testing_their_layer_and_again_on_normal_visit() {
    let mut other = page(vec![]);
    other.layer = 2;
    other.trigger = 4;
    other.commands = vec![counter()];
    let mut app = app(
        vec![
            event(1, 1, vec![page(vec![command(1, 0)])]),
            event(2, 2, vec![other]),
        ],
        false,
    );
    app.update();
    assert_eq!(app.world().resource::<crate::state::Variables>().get(1), 2);
    let npc = entity(&mut app, 2);
    assert_eq!(
        app.world().get::<RouteStepper>(npc).unwrap().stop_count(),
        1
    );
}

#[test]
fn target_tile_rejection_happens_after_make_way_but_source_rejection_happens_before_it() {
    for blocked_x in [1, 2] {
        let mut other = page(vec![]);
        other.layer = 2;
        other.trigger = 4;
        other.commands = vec![counter()];
        let mut app = app(
            vec![
                event(1, 1, vec![page(vec![command(1, 0)])]),
                event(2, 2, vec![other]),
            ],
            false,
        );
        let mut data = app.world_mut().resource_mut::<MapData>();
        let index = data.width as usize + blocked_x;
        data.upper[index] = 10001;
        data.passages_up[1] = 0;
        app.update();
        assert_eq!(
            app.world().resource::<crate::state::Variables>().get(1),
            if blocked_x == 1 { 1 } else { 2 }
        );
        let npc = entity(&mut app, 1);
        assert_eq!(app.world().get::<EventSprite>(npc).unwrap().tile_x, 1);
    }
}
