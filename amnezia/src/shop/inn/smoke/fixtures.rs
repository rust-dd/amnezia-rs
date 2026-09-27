use super::*;
use amnezia_data::{EventCommand, Map};

pub(super) const COUNT: usize = 6;
pub(super) const LABELS: [[&str; 2]; COUNT] = [
    ["inn-disabled-a", "inn-disabled-b"],
    ["inn-top-a", "inn-top-b"],
    ["inn-paid-a", "inn-paid-b"],
    ["inn-transparent-a", "inn-transparent-b"],
    ["inn-free-a", "inn-free-b"],
    ["inn-first-style-a", "inn-first-style-b"],
];

pub(super) struct Case {
    pub cost: i32,
    pub gold: i32,
    pub top: u32,
    pub face: bool,
    pub transparent: bool,
    pub stay: bool,
}

pub(super) fn case(index: usize) -> Case {
    Case {
        cost: [200, 30, 50, 200, 0, 30][index],
        gold: [199, 100, 100, 200, 0, 999_999][index],
        top: [160, 0, 160, 80, 160, 0][index],
        face: matches!(index, 1..=3),
        transparent: index == 3,
        stay: matches!(index, 2..=4),
    }
}

fn command(code: u32, indent: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: String::new(),
        params,
    }
}

fn commands(index: usize) -> Vec<EventCommand> {
    let cost = case(index).cost;
    let inn = if matches!(index, 4 | 5) {
        command(10730, 0, vec![0, cost, 1])
    } else {
        let mut found = None;
        for map in [2, 52, 119] {
            let map = crate::assets::load_ron::<Map>(&format!(
                "{}/maps/map_{map:04}.ron",
                crate::assets::asset_root()
            ));
            found = found.or_else(|| {
                map.events
                    .into_iter()
                    .flat_map(|event| event.pages)
                    .flat_map(|page| page.commands)
                    .find(|command| command.code == 10730 && command.params[1] == cost)
            });
        }
        let inn = found.expect("original inn command with the requested price");
        assert_eq!(inn.params[0], 1);
        inn
    };
    vec![
        EventCommand {
            string: if case(index).face {
                "Ron".into()
            } else {
                String::new()
            },
            ..command(10130, 0, vec![6, 0, 0])
        },
        inn,
        command(20730, 0, vec![]),
        command(10210, 1, vec![0, 9031, 9031, 0]),
        command(20731, 0, vec![]),
        command(10210, 1, vec![0, 9031, 9031, 1]),
        command(20732, 0, vec![]),
        command(10220, 0, vec![0, 9032, 9032, 1, 0, 1]),
    ]
}

pub(super) fn prepare(world: &mut World, index: usize) {
    let fixture = case(index);
    assert!(!world.resource::<RunningEvent>().active());
    world.resource_mut::<Party>().restore(vec![1, 2]);
    let mut inventory = Inventory::default();
    inventory.add_gold(fixture.gold);
    world.insert_resource(inventory);
    let mut vitals = Vitals::default();
    for id in 1..=3 {
        vitals.set(id, if id == 1 { 0 } else { id as i32 }, 0);
        vitals.set_states(id, vec![2, 3]);
    }
    world.insert_resource(vitals);
    world
        .resource_mut::<crate::dialogue::MessageOptions>()
        .fixed = true;
    *world.resource_mut::<crate::dialogue::MessagePosition>() = match fixture.top {
        0 => crate::dialogue::MessagePosition::Top,
        80 => crate::dialogue::MessagePosition::Middle,
        _ => crate::dialogue::MessagePosition::Bottom,
    };
    world
        .resource_mut::<crate::dialogue::MessageTransparent>()
        .0 = fixture.transparent;
    world.resource_mut::<crate::system_bgm::SystemBgm>().change(
        if index == 2 { "Inn" } else { "(OFF)" },
        &[2, 0, 90, 100, 50],
    );
    if index == 4 {
        world.write_message(AudioRequest::StopBgm);
        world.resource_mut::<Transition>().hold_black();
        world.resource_mut::<Transition>().event_erased = true;
    }
    world
        .resource_mut::<crate::state::Switches>()
        .set(9031, !fixture.stay);
    world
        .resource_mut::<RunningEvent>()
        .start(0, commands(index));
}

pub(super) fn verify_vitals(world: &World, healed: bool) {
    let vitals = world.resource::<Vitals>();
    for id in 1..=2 {
        assert_eq!(
            vitals.get_stored(id),
            if healed {
                None
            } else {
                Some((if id == 1 { 0 } else { 2 }, 0))
            },
            "inn case {}, age {}, steps {}, healed {healed}",
            world.resource::<Probe>().case,
            world.resource::<Probe>().age,
            world.resource::<crate::conditions::FieldSteps>().count,
        );
        assert_eq!(vitals.states(id).is_empty(), healed);
    }
    assert_eq!(vitals.get_stored(3), Some((3, 0)));
    assert_eq!(vitals.states(3), [2, 3]);
}
