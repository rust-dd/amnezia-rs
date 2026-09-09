use super::*;
use crate::assets::{asset_root, load_ron};
use crate::tiles::{PASS_LEFT, PASS_RIGHT};
use amnezia_data::{EventCondition, Map};

struct Fixture {
    data: MapData,
    events: MapEvents,
    switches: Switches,
    variables: Variables,
    party: Party,
    inventory: Inventory,
    bodies: CollisionBodies,
}

impl Fixture {
    fn new() -> Self {
        Self {
            data: MapData::for_test(5, 5),
            events: MapEvents::default(),
            switches: Switches::default(),
            variables: Variables::default(),
            party: Party::default(),
            inventory: Inventory::default(),
            bodies: CollisionBodies::default(),
        }
    }

    fn collision(&self) -> MapCollision<'_> {
        MapCollision::new(
            &self.data,
            &self.events,
            (
                &self.switches,
                &self.variables,
                &self.party,
                &self.inventory,
            ),
            &self.bodies,
        )
    }

    fn enter(&self, mover: Mover) -> bool {
        self.collision()
            .can_move((1, 2), (2, 2), mover, None, false)
    }

    fn wall(&mut self) {
        self.data.upper[12] = 10001;
        self.data.passages_up[1] = 0;
    }
}

fn event(id: u32, layer: u32, tile: Option<u32>) -> Event {
    let map = load_ron::<Map>(&format!("{}/maps/map_0094.ron", asset_root()));
    let mut page = map.events[0].pages[0].clone();
    page.condition = EventCondition::default();
    page.layer = layer;
    page.graphic_name = tile.map_or_else(|| "Chara1".into(), |_| String::new());
    page.graphic_index = tile.unwrap_or(0);
    page.overlap_forbidden = false;
    Event {
        id,
        x: 2,
        y: 2,
        name: String::new(),
        pages: vec![page],
    }
}

fn npc(id: u32, layer: u32, tile: Option<u32>) -> Mover {
    Mover {
        id,
        layer,
        tile,
        through: false,
    }
}

#[test]
fn original_world_bridge_and_switched_staircase_use_their_tile_passages() {
    let chips = load_ron::<Vec<amnezia_data::Chipset>>(&format!("{}/chipsets.ron", asset_root()));
    for (id, event_id, position) in [(13, 67, (73, 86)), (19, 10, (6, 7))] {
        let map = load_ron::<Map>(&format!("{}/maps/map_{id:04}.ron", asset_root()));
        let chip = chips.iter().find(|chip| chip.id == map.chipset_id).unwrap();
        let mut f = Fixture::new();
        f.data = MapData::for_test(map.width as i32, map.height as i32);
        f.data.lower = map.lower;
        f.data.upper = map.upper;
        f.data.passages_down = chip.passages_down.clone();
        f.data.passages_up = chip.passages_up.clone();
        f.events.events = vec![
            map.events
                .into_iter()
                .find(|event| event.id == event_id)
                .unwrap(),
        ];
        if id == 13 {
            assert!(!f.data.passable(position.0, position.1));
            for bit in [1, 2, 4, 8] {
                assert!(f.collision().tile_passable(position, bit, 0));
            }
        } else {
            assert!(f.collision().tile_passable(position, PASS_LEFT, 0));
            f.switches.set(36, true);
            assert!(f.collision().tile_passable(position, PASS_RIGHT, 0));
            for bit in [1, 2, 8] {
                assert!(!f.collision().tile_passable(position, bit, 0));
            }
        }
    }
}

#[test]
fn either_overlap_flag_blocks_other_events_but_not_the_hero() {
    for self_forbidden in [false, true] {
        for other_forbidden in [false, true] {
            let mut f = Fixture::new();
            let mut moving = event(1, 1, None);
            moving.x = 1;
            moving.pages[0].overlap_forbidden = self_forbidden;
            let mut other = event(2, 0, None);
            other.pages[0].overlap_forbidden = other_forbidden;
            f.events.events = vec![moving, other];
            assert_eq!(
                f.enter(npc(1, 1, None)),
                !self_forbidden && !other_forbidden
            );
            assert!(f.enter(Mover::hero(false)));
            f.events.events[1].pages[0].condition.flags = 1;
            f.events.events[1].pages[0].condition.switch_a = 8;
            assert!(f.enter(npc(1, 1, None)));
        }
    }
}

#[test]
fn through_characters_neither_block_nor_override_the_map() {
    let mut f = Fixture::new();
    let mut other = event(2, 1, None);
    other.pages[0].overlap_forbidden = true;
    f.events.events.push(other);
    assert!(!f.enter(Mover::hero(false)));
    f.bodies.events.insert(
        2,
        Mover {
            through: true,
            ..npc(2, 1, None)
        },
    );
    assert!(f.enter(Mover::hero(false)));
    assert!(f.enter(npc(1, 0, None)));
    f.wall();
    f.events.events[0].pages[0].layer = 0;
    f.events.events[0].pages[0].graphic_name.clear();
    f.events.events[0].pages[0].graphic_index = 2;
    f.bodies.events.insert(
        2,
        Mover {
            through: true,
            ..npc(2, 0, Some(2))
        },
    );
    assert!(!f.enter(Mover::hero(false)));
}

#[test]
fn below_tile_events_override_geometry_for_both_entry_and_departure() {
    let mut f = Fixture::new();
    f.wall();
    assert!(!f.enter(Mover::hero(false)));
    f.events.events.push(event(2, 0, Some(2)));
    assert!(f.enter(Mover::hero(false)));
    assert!(
        f.collision()
            .can_move((2, 2), (3, 2), Mover::hero(false), None, false)
    );
    f.data.passages_up[2] = PASS_RIGHT;
    assert!(!f.enter(Mover::hero(false)));
    assert!(
        f.collision()
            .can_move((2, 2), (3, 2), Mover::hero(false), None, false)
    );
    assert!(
        !f.collision()
            .can_move((2, 2), (1, 2), Mover::hero(false), None, false)
    );
    assert!(
        f.collision()
            .can_move((1, 2), (2, 2), Mover::hero(false), None, true)
    );
    f.data.passages_up[2] = 0;
    assert!(
        !f.collision()
            .can_move((1, 2), (2, 2), Mover::hero(false), None, true)
    );
}

#[test]
fn highest_tile_event_id_wins_even_when_blank_or_above_hero() {
    let mut f = Fixture::new();
    f.wall();
    f.events.events = vec![event(9, 0, Some(0)), event(2, 0, Some(2))];
    assert!(!f.enter(Mover::hero(false)));
    f.events.events[0].pages[0].graphic_index = 3;
    f.data.passages_up[3] = PASS_ALL | ABOVE_HERO_BIT;
    assert!(!f.enter(Mover::hero(false)));
    f.events.events[0].pages[0].graphic_name = "Chara1".into();
    assert!(f.enter(Mover::hero(false)));
    f.events.events[0].pages[0].graphic_name.clear();
    f.bodies.events.insert(
        9,
        Mover {
            through: true,
            ..npc(9, 0, Some(3))
        },
    );
    assert!(f.enter(Mover::hero(false)));
}

#[test]
fn live_route_graphic_replaces_the_page_tile_for_collision() {
    let mut f = Fixture::new();
    f.wall();
    f.events.events.push(event(2, 0, Some(2)));
    assert!(f.enter(Mover::hero(false)));
    let mut character = EventSprite {
        id: 2,
        tile_x: 2,
        tile_y: 2,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
        layer: 0,
    };
    let route = RouteStepper::default();
    f.bodies = CollisionBodies::from_events(std::iter::once((&character, Some(&route))));
    assert!(!f.enter(Mover::hero(false)));
    character.charset.clear();
    character.index = 2;
    f.bodies.update(&character, &route);
    assert!(f.enter(Mover::hero(false)));
}

#[test]
fn self_conflicting_tiles_block_same_layer_occupants_only_while_walking() {
    let mut f = Fixture::new();
    f.data.passages_up[2] = PASS_LEFT;
    let moving = npc(1, 0, Some(2));
    assert!(
        !f.collision()
            .can_move((1, 2), (2, 2), moving, Some((2, 2)), false)
    );
    assert!(
        f.collision()
            .can_move((1, 2), (2, 2), moving, Some((2, 2)), true)
    );
    f.bodies.hero_through = true;
    assert!(
        f.collision()
            .can_move((1, 2), (2, 2), moving, Some((2, 2)), false)
    );
    f.events.events.push(event(2, 1, None));
    assert!(!f.enter(moving));
    f.data.passages_up[2] = PASS_RIGHT;
    assert!(f.enter(moving));
    f.events.events[0] = event(1, 0, Some(2));
    f.events.events[0].x = 1;
    f.data.passages_up[2] = 0;
    assert!(
        f.enter(moving),
        "the moving tile must not collide with its own graphic"
    );
}

#[test]
fn diagonal_steps_need_one_complete_orthogonal_path_but_jumps_do_not() {
    let mut f = Fixture::new();
    f.data.passages_up[1] = 0;
    f.data.upper[11] = 10001;
    f.data.upper[7] = 10001;
    assert!(
        !f.collision()
            .can_move((1, 1), (2, 2), Mover::hero(false), None, false)
    );
    assert!(
        f.collision()
            .can_move((1, 1), (2, 2), Mover::hero(false), None, true)
    );
    f.data.upper[7] = 10000;
    assert!(
        f.collision()
            .can_move((1, 1), (2, 2), Mover::hero(false), None, false)
    );
    f.events.events.push(event(2, 1, None));
    assert!(
        !f.collision()
            .can_move((1, 1), (2, 2), Mover::hero(false), None, false)
    );
}

#[test]
fn through_ignores_obstacles_but_preserves_map_bounds_and_looping() {
    let mut f = Fixture::new();
    f.wall();
    f.events.events.push(event(2, 1, None));
    assert!(f.enter(Mover::hero(true)));
    for jumping in [false, true] {
        assert!(
            !f.collision()
                .can_move((0, 2), (-1, 2), Mover::hero(true), None, jumping)
        );
    }
    f.data.scroll_type = 2;
    assert!(
        f.collision()
            .can_move((0, 2), (-1, 2), Mover::hero(true), None, false)
    );
    f.data.upper[14] = 10002;
    f.data.passages_up[2] = PASS_RIGHT;
    assert!(
        f.collision()
            .can_move((0, 2), (-1, 2), Mover::hero(false), None, false)
    );
    f.data.passages_up[2] = PASS_LEFT;
    assert!(
        !f.collision()
            .can_move((0, 2), (-1, 2), Mover::hero(false), None, false)
    );
}

#[test]
fn parked_airships_block_npcs_but_leave_the_heros_boarding_tile_open() {
    let mut f = Fixture::new();
    let mut vehicles = crate::vehicles::Vehicles::default();
    vehicles.set_location(2, 0, 2, 2);
    f.bodies.include_vehicles(Some(&vehicles), 0);
    assert!(f.enter(Mover::hero(false)));
    assert!(!f.enter(npc(1, 1, None)));
    assert!(f.enter(npc(1, 0, None)));
    vehicles.save.riding = Some(2);
    f.bodies.include_vehicles(Some(&vehicles), 0);
    assert!(
        f.collision()
            .can_move((1, 2), (2, 2), npc(1, 1, None), Some((2, 2)), false)
    );
}

#[test]
fn ships_block_ground_characters_but_not_other_layers_or_maps() {
    let mut f = Fixture::new();
    let mut vehicles = crate::vehicles::Vehicles::default();
    vehicles.set_location(0, 0, 2, 2);
    f.bodies.include_vehicles(Some(&vehicles), 0);
    assert!(!f.enter(Mover::hero(false)));
    assert!(!f.enter(npc(1, 1, None)));
    assert!(f.enter(npc(1, 0, None)));
    assert!(f.enter(Mover::hero(true)));
    vehicles.set_location(0, 13, 2, 2);
    f.bodies.include_vehicles(Some(&vehicles), 0);
    assert!(f.enter(Mover::hero(false)));
}
