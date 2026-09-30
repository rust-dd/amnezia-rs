use super::{Character, EventSprite, MapData, MoveQueue, RouteStepper};
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::state::{Inventory, Switches, Variables};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod pixels;
pub(crate) use pixels::snapshot;

const HERO: (i32, i32) = (11, 7);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Approaching,
    Blocked,
    Collecting,
    WalkingAway,
    Done,
}

#[derive(Resource, Default)]
struct Probe {
    stage: Stage,
    blocked: usize,
    pictures: Arc<AtomicUsize>,
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        params,
        indent: 0,
        string: String::new(),
    }
}

pub(crate) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.init_resource::<Probe>();
    world.resource_mut::<Switches>().load(vec![(106, true)]);
    vec![
        command(10810, vec![42, HERO.0, HERO.1]),
        command(11330, vec![10001, 8, 0, 0, 14]),
        // The original upper-layer ghost must stop at the book's overlap restriction.
        command(11330, vec![11, 8, 0, 0, 1, 1, 1, 1, 1]),
        command(11410, vec![100_000]),
    ]
}

pub(crate) fn input(world: &mut World, frame: u32) -> bool {
    if world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.stage == Stage::WalkingAway)
    {
        let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::ArrowLeft);
        return true;
    }
    let collecting = world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.stage == Stage::Collecting);
    if !collecting
        || world.resource::<Dialogue>().busy()
        || world.resource::<RunningEvent>().active()
    {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if frame.is_multiple_of(15) {
        keys.press(KeyCode::Enter);
    }
    true
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let current = world.get_resource::<Probe>()?.stage;
    if current == Stage::Done
        || world.resource::<MapData>().map_id != 42
        || world.resource::<crate::teleport::Fade>().busy()
    {
        return None;
    }
    assert!(
        frame < 1800,
        "original ghost/book probe stalled in {current:?}"
    );
    let (ghost, queue, route) = world
        .query::<(&EventSprite, &MoveQueue, &RouteStepper)>()
        .iter(world)
        .find(|(actor, _, _)| actor.id == 11)
        .unwrap();
    let (tile, moving, pending) = (ghost.tile(), queue.busy(), route.pending());
    assert_eq!(ghost.layer, 2);
    match current {
        Stage::Approaching if tile == (10, 8) && !moving => {
            assert!(pending);
            world.resource_mut::<Probe>().stage = Stage::Blocked;
        }
        Stage::Blocked => {
            assert_eq!(tile, (10, 8));
            assert!(!moving && pending);
            let book = world
                .query::<&EventSprite>()
                .iter(world)
                .find(|actor| actor.id == 10)
                .unwrap();
            assert_eq!((book.tile(), book.layer, book.index), ((11, 8), 1, 95));
            let mut probe = world.resource_mut::<Probe>();
            probe.blocked += 1;
            if probe.blocked == 45 {
                probe.stage = Stage::Collecting;
                world.insert_resource(RunningEvent::default());
                return Some("overlap-original-book-blocked");
            }
        }
        Stage::Collecting
            if world.resource::<Switches>().get(107)
                && !world.resource::<RunningEvent>().active()
                && !world.resource::<Dialogue>().busy()
                && tile == (11, 8)
                && !moving
                && !pending =>
        {
            world.resource_mut::<Probe>().stage = Stage::WalkingAway;
        }
        Stage::WalkingAway => {
            let (hero, queue) = world
                .query::<(&crate::player::Player, &MoveQueue)>()
                .single(world)
                .unwrap();
            if hero.tile() == (10, 7) && !queue.busy() {
                world.resource_mut::<Probe>().stage = Stage::Done;
                return Some("overlap-original-book-collected");
            }
        }
        _ => {}
    }
    None
}

pub(crate) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.stage == Stage::Done)
}

pub(crate) fn verify_finished(world: &mut World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.stage, Stage::Done);
    assert_eq!(probe.blocked, 45);
    assert_eq!(probe.pictures.load(Ordering::Relaxed), 2);
    assert!(world.resource::<Switches>().get(107));
    assert_eq!(world.resource::<Variables>().get(5), 1);
    assert_eq!(world.resource::<Inventory>().count(169), 1);
    let book = world
        .query::<&EventSprite>()
        .iter(world)
        .find(|actor| actor.id == 10)
        .unwrap();
    assert!(book.charset.is_empty());
    assert_eq!((book.layer, book.index), (0, 0));
    info!(
        "original map-42 ghost/book: 45 blocked updates across different layers, original interaction/reward, page removal, resumed route and walking away verified"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_approaches_the_original_book_from_floor_not_inside_the_rock() {
        let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
            "{}/maps/map_0042.ron",
            crate::assets::asset_root()
        ));
        let chip = crate::assets::load_ron::<Vec<amnezia_data::Chipset>>(&format!(
            "{}/chipsets.ron",
            crate::assets::asset_root()
        ))
        .into_iter()
        .find(|chip| chip.id == map.chipset_id)
        .unwrap();
        let index = (HERO.1 * map.width as i32 + HERO.0) as usize;
        assert!(
            crate::tiles::passable(
                map.lower[index],
                map.upper[index],
                &chip.passages_down,
                &chip.passages_up,
                crate::tiles::PASS_ALL
            ),
            "Ron must start on walkable floor"
        );
        assert_eq!(HERO.0.abs_diff(11) + HERO.1.abs_diff(8), 1);
        assert!(
            map.events
                .iter()
                .all(|event| (event.x as i32, event.y as i32) != HERO)
        );
        for y in (HERO.1 - 1)..=HERO.1 {
            for x in (HERO.0 - 1)..=(HERO.0 + 1) {
                let index = (y * map.width as i32 + x) as usize;
                assert!(!crate::tiles::above_hero_lower(
                    map.lower[index],
                    &chip.passages_down
                ));
                assert!(!crate::tiles::above_hero(
                    map.upper[index],
                    &chip.passages_up
                ));
            }
        }
    }
}
