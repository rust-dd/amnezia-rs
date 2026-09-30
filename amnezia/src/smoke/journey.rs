use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::player::Player;
use crate::state::{Party, Switches};
use crate::world::{Character, EventSprite, MapData, MoveQueue};
use amnezia_data::EventCommand;
use bevy::prelude::*;
use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[cfg(test)]
mod history_tests;
mod murder;

#[derive(Resource, Default)]
struct Probe {
    flown: BTreeSet<(i32, i32)>,
    interior_required: bool,
    interior_checked: bool,
    staged: bool,
    walking: bool,
    done: bool,
    pixels: Arc<AtomicUsize>,
}

pub(super) const CAST: [u32; 7] = [22, 26, 32, 33, 34, 35, 36];

fn entry_tile(map: i32) -> (i32, i32) {
    match map {
        125 => (9, 12),
        126 => (9, 20),
        127 => (11, 8),
        129 => (10, 5),
        _ => panic!("fortress arrival starts on maps 125, 126, 127 or 129"),
    }
}

pub(super) fn entry(world: &mut World) -> Vec<EventCommand> {
    world.insert_resource(Probe::default());
    if murder::enabled() {
        return murder::entry(world);
    }
    let map = std::env::args()
        .find_map(|arg| arg.strip_prefix("--smoke-map=").map(str::to_owned))
        .map_or(125, |id| id.parse::<i32>().unwrap());
    let (x, y) = entry_tile(map);
    world.resource_mut::<Probe>().interior_required = map == 127;
    world
        .resource_mut::<Switches>()
        .load(super::airship_history::switches(&[329]));
    world.resource_mut::<Party>().restore(vec![1, 2]);
    world.resource_mut::<crate::state::Variables>().set(1, 9);
    vec![EventCommand {
        code: 10810,
        indent: 0,
        string: String::new(),
        params: vec![map, x, y],
    }]
}

pub(super) fn pixels(world: &World) -> Arc<AtomicUsize> {
    world.resource::<Probe>().pixels.clone()
}

pub(super) fn held_input(world: &mut World) -> bool {
    if murder::input(world) {
        return true;
    }
    if !world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.walking)
    {
        return false;
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::ArrowUp);
    true
}

pub(super) fn drive(world: &mut World) -> Option<&'static str> {
    if let Some(label) = murder::drive(world) {
        return Some(label);
    }
    let probe = world.get_resource::<Probe>()?;
    if probe.done {
        return None;
    }
    let map = world.resource::<MapData>().map_id;
    if map == 127 && !world.resource::<crate::teleport::Fade>().busy() {
        let alens = world
            .query::<(&EventSprite, &InheritedVisibility)>()
            .iter(world)
            .filter(|(actor, visible)| {
                visible.get() && actor.charset == "Chara4" && actor.index == 0
            })
            .map(|(actor, _)| (actor.id, actor.tile()))
            .collect::<Vec<_>>();
        assert_eq!(alens, [(36, (11, 5))], "only the current Alen may remain");
        let hero = world.query::<&Player>().single(world).unwrap();
        assert_eq!(hero.tile(), entry_tile(127));
        assert!(
            world
                .resource::<MapData>()
                .passable(hero.tile_x, hero.tile_y)
        );
        if !world.resource::<Probe>().interior_checked
            && world.resource::<Dialogue>().ready_to_advance()
        {
            assert_eq!(world.resource::<RunningEvent>().debug_id(), Some(37));
            world.resource_mut::<Probe>().interior_checked = true;
            return Some("airship-interior-arrival");
        }
    }
    if map == 13 {
        let vehicles = world.resource::<crate::vehicles::Vehicles>();
        let tile = vehicles.save.vehicles[2].tile();
        if vehicles.save.riding == Some(2) && !vehicles.save.boarding {
            let hero = world
                .query::<(&Player, &InheritedVisibility)>()
                .single(world)
                .unwrap();
            assert!(
                !hero.1.get(),
                "the rider must not duplicate the airship sprite"
            );
            assert_eq!(hero.0.tile(), tile);
            let sprites = world
                .query::<(&crate::vehicles::VehicleSprite, &InheritedVisibility)>()
                .iter(world)
                .filter(|(id, visible)| id.0 == 2 && visible.get())
                .count();
            assert_eq!(sprites, 1);
            world.resource_mut::<Probe>().flown.insert(tile);
        }
    }
    if map != 132 || world.resource::<crate::teleport::Fade>().busy() {
        return None;
    }
    let dialogue = world.resource::<Dialogue>();
    let text = dialogue
        .boxes
        .iter()
        .flat_map(|page| &page.lines)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if !world.resource::<Probe>().staged
        && dialogue.ready_to_advance()
        && text.contains("se jött fel a hajóra")
    {
        verify_cast(world);
        world.resource_mut::<Probe>().staged = true;
        return Some("airship-fortress-cast");
    }
    if world.resource::<Switches>().get(327)
        && world.resource::<Switches>().get(627)
        && !world.resource::<RunningEvent>().active()
        && !world.resource::<Dialogue>().busy()
        && !world.resource::<crate::transitions::Transition>().busy()
    {
        let (hero, queue) = world
            .query::<(&Player, &MoveQueue)>()
            .single(world)
            .unwrap();
        if queue.busy() {
            return None;
        }
        if !world.resource::<Probe>().walking {
            assert_eq!(hero.tile(), (8, 22));
            world.resource_mut::<Probe>().walking = true;
        } else if hero.tile() == (8, 21) {
            let mut probe = world.resource_mut::<Probe>();
            probe.walking = false;
            probe.done = true;
            return Some("airship-fortress-continued");
        }
    }
    None
}

fn verify_cast(world: &mut World) {
    for (id, charset, index, x, y) in [
        (22, "Chara1", 1, 9, 22),
        (26, "Chara3", 2, 6, 20),
        (32, "Chara1", 2, 10, 23),
        (33, "Chara4", 4, 12, 21),
        (34, "Chara4", 2, 12, 20),
        (35, "Chara4", 0, 7, 21),
        (36, "Chara2", 2, 9, 19),
    ] {
        let actors = world
            .query::<&EventSprite>()
            .iter(world)
            .filter(|event| event.id == id)
            .collect::<Vec<_>>();
        assert_eq!(actors.len(), 1);
        let actor = actors[0];
        assert_eq!(
            (
                actor.charset.as_str(),
                actor.index,
                actor.tile_x,
                actor.tile_y
            ),
            (charset, index, x, y)
        );
    }
    let hero = world
        .query::<(&Player, &InheritedVisibility)>()
        .single(world)
        .unwrap();
    assert_eq!(hero.0.tile(), (8, 22));
    assert!(hero.1.get());
}

pub(super) fn ready(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.done)
}

pub(super) fn verify_finished(world: &mut World) {
    murder::verify_finished(world);
    let probe = world.resource::<Probe>();
    assert!(probe.done && probe.staged);
    if probe.interior_required {
        assert!(probe.interior_checked);
        super::cast_pixels::verify_finished(world, &["airship-interior-arrival"]);
    }
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 1);
    for x in 28..=55 {
        assert!(
            probe.flown.contains(&(x, 100)),
            "missing flight tile {x},100"
        );
    }
    assert!(probe.flown.contains(&(28, 101)));
    let vehicles = world.resource::<crate::vehicles::Vehicles>();
    assert_eq!(vehicles.save.riding, None);
    assert_eq!(vehicles.save.vehicles[2].definition.map_id, 13);
    assert_eq!(vehicles.save.vehicles[2].tile(), (28, 101));
    assert_eq!(world.resource::<Party>().snapshot(), [1, 2]);
    for id in [327, 627] {
        assert!(world.resource::<Switches>().get(id), "switch {id}");
    }
    assert!(!world.resource::<Switches>().get(626));
    for actor in world
        .query::<&EventSprite>()
        .iter(world)
        .filter(|actor| CAST.contains(&actor.id))
    {
        assert!(
            actor.charset.is_empty(),
            "departed actor {} still visible",
            actor.id
        );
    }
    info!(
        "full fortress arrival: original boarding, all 29 flight tiles, docking, seven staged actors, departures, party and returned controls verified"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_four_airship_rooms_share_the_complete_original_fortress_arrival() {
        let mut reference = None;
        for id in [125, 126, 127, 129] {
            let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
                "{}/maps/map_{id:04}.ron",
                crate::assets::asset_root()
            ));
            let page = map
                .events
                .iter()
                .flat_map(|event| &event.pages)
                .find(|page| page.trigger == 3 && page.condition.switch_a == 329)
                .unwrap();
            assert_eq!(page.commands.len(), 34);
            assert!(
                page.commands
                    .iter()
                    .any(|command| command.code == 10810 && command.params == [132, 8, 22])
            );
            let mut choreography = page.commands.clone();
            assert_eq!(
                choreography[3].string,
                if id == 126 {
                    "Stark"
                } else {
                    "Stark hangja valahonnan"
                }
            );
            choreography[3].string.clear();
            if let Some(expected) = &reference {
                assert_eq!(&choreography, expected);
            } else {
                reference = Some(choreography);
            }
        }
    }
}
