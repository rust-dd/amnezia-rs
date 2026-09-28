use crate::interpreter::{CommonEvents, RunningEvent};
use crate::state::{Switches, Variables};
use crate::transitions::Transition;
use crate::world::MapEvents;
use amnezia_data::{CommonEvent, EventCommand};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Probe {
    completed: u8,
    checks: u32,
    started: bool,
    inn: bool,
    owner: u32,
    callbacks: u8,
    barriers: [u8; 3],
}

pub(crate) fn configure(app: &mut App) {
    crate::timing::logical::post(app, || observe.after(crate::teleport::TransferCommit));
}

fn observe(world: &mut World) {
    if !world.resource::<crate::timing::logical::Step>().callback {
        return;
    }
    let Some(probe) = world.get_resource::<Probe>() else {
        return;
    };
    let owner = probe.owner;
    if probe.callbacks & (1 << owner) != 0 {
        return;
    }
    let inn = probe.inn;
    let values = std::array::from_fn::<_, 8, _>(|index| {
        world.resource::<Variables>().get(4800 + index as u32)
    });
    if values[7] == 0 {
        if inn {
            return;
        }
        let phase = match &values[4..] {
            [1, 0, 0, 0] => 1,
            [1, 1, 0, 0] => 2,
            [1, 1, 1, 0] => 4,
            other => panic!("unexpected transition callback: {other:?}"),
        };
        assert!(world.resource::<Transition>().busy());
        let mut probe = world.resource_mut::<Probe>();
        assert_eq!(probe.barriers[owner as usize], phase - 1);
        probe.barriers[owner as usize] |= phase;
        return;
    }
    assert_eq!(
        values, [1; 8],
        "the callback must complete exactly the interrupted map visit"
    );
    assert!(!world.resource::<Transition>().busy());
    world.resource_mut::<Probe>().callbacks |= 1 << owner;
    world.resource_mut::<Switches>().set(4800, false);
}

fn command(code: u32, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        params,
        string: String::new(),
    }
}

fn increment(index: i32) -> EventCommand {
    command(10220, vec![0, 4800 + index, 4800 + index, 1, 0, 1])
}

fn commands(inn: bool) -> Vec<EventCommand> {
    if inn {
        return vec![
            command(10210, vec![0, 4801, 4801, 1]),
            command(10730, vec![0, 0, 1]),
            increment(4),
            increment(5),
            increment(6),
            increment(7),
        ];
    }
    vec![
        command(10210, vec![0, 4801, 4801, 1]),
        command(11010, vec![0]),
        increment(4),
        command(11010, vec![17]),
        increment(5),
        command(11020, vec![20]),
        increment(6),
        command(11020, vec![0]),
        increment(7),
    ]
}

fn common(id: u32, gate: u32, commands: Vec<EventCommand>) -> CommonEvent {
    CommonEvent {
        id,
        name: "Async continuation probe".into(),
        trigger: 4,
        switch_flag: true,
        switch_id: gate,
        commands,
    }
}

fn prepare(world: &mut World, case: u32) {
    let inn = world.resource::<Probe>().inn;
    if inn {
        world
            .resource_mut::<crate::system_bgm::SystemBgm>()
            .change("(OFF)", &[2, 0, 100, 100, 50]);
        world.resource_mut::<crate::vitals::Vitals>().set(1, 2, 0);
    }
    assert!(!world.resource::<crate::dialogue::Dialogue>().busy());
    assert!(!world.resource::<RunningEvent>().active());
    assert!(!world.resource::<Transition>().busy());
    world.resource_mut::<Probe>().started = false;
    world.resource_mut::<Probe>().owner = case;
    for id in 4800..4808 {
        world.resource_mut::<Variables>().set(id, 0);
    }
    for id in [4800, 4801] {
        world.resource_mut::<Switches>().set(id, true);
    }
    let mut common_events = world.resource_mut::<CommonEvents>();
    common_events
        .0
        .retain(|event| !(900..903).contains(&event.id));
    common_events.0.extend([
        common(900, 4800, vec![increment(0)]),
        common(902, 4800, vec![increment(1)]),
    ]);
    if case == 1 {
        common_events.0.push(common(901, 4801, commands(inn)));
    }
    let template = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0003.ron",
        crate::assets::asset_root()
    ))
    .events[0]
        .clone();
    let mut map_events = world.resource_mut::<MapEvents>();
    map_events
        .events
        .retain(|event| !(900..903).contains(&event.id));
    for (id, index) in [(900, 2), (902, 3)] {
        let mut event = template.clone();
        event.id = id;
        event.pages.truncate(1);
        let page = &mut event.pages[0];
        page.condition = default();
        page.condition.flags = 1;
        page.condition.switch_a = 4800;
        page.trigger = 4;
        page.commands = vec![increment(index)];
        map_events.events.push(event);
    }
    if case == 2 {
        let mut event = template;
        event.id = 901;
        event.pages.truncate(1);
        let page = &mut event.pages[0];
        page.condition = default();
        page.condition.flags = 1;
        page.condition.switch_a = 4801;
        page.trigger = 4;
        page.commands = commands(inn);
        map_events.events.push(event);
    }
    if case == 0 {
        world.resource_mut::<RunningEvent>().start(7, commands(inn));
    }
}

pub(crate) fn drive(world: &mut World, frame: u32, inn: bool) -> Option<&'static str> {
    if frame == 300 {
        world.init_resource::<Probe>();
        world.resource_mut::<Probe>().inn = inn;
    }
    if !(300..900).contains(&frame) {
        return None;
    }
    let case = (frame - 300) / 200;
    let age = (frame - 300) % 200;
    if age == 0 {
        prepare(world, case);
        return None;
    }
    if world.resource::<Probe>().completed & (1 << case) != 0 {
        return None;
    }
    assert!(age < 170, "asynchronous owner {case} failed to resume");
    let vars = world.resource::<Variables>();
    let counts = std::array::from_fn::<_, 8, _>(|index| vars.get(4800 + index as u32));
    let finished = counts[7] != 0;
    let expected = if finished || case == 0 {
        [1, 1, 1, 1]
    } else if case == 1 {
        [1, 0, 0, 0]
    } else {
        [1, 1, 1, 0]
    };
    assert_eq!(&counts[..4], &expected, "owner={case}, age={age}");
    if inn {
        assert_eq!(&counts[4..7], &[i32::from(finished); 3]);
    } else {
        assert!(matches!(
            &counts[4..7],
            [0, 0, 0] | [1, 0, 0] | [1, 1, 0] | [1, 1, 1]
        ));
    }
    assert_eq!(counts[7], i32::from(finished));
    if finished {
        assert!(!world.resource::<Transition>().busy());
        assert_ne!(world.resource::<Probe>().callbacks & (1 << case), 0);
    } else {
        assert!(
            world.resource::<Transition>().busy()
                || crate::timing::logical::callback_pending(world)
        );
    }
    if !finished {
        world.resource_mut::<Probe>().started = true;
    }
    world.resource_mut::<Probe>().checks += 1;
    if finished {
        assert!(world.resource::<Probe>().started);
        assert!(!world.resource::<Transition>().erased());
        assert!(!world.resource::<Transition>().event_erased);
        world.resource_mut::<Switches>().set(4800, false);
        world.resource_mut::<Probe>().completed |= 1 << case;
        if inn {
            assert_eq!(
                world.resource::<crate::vitals::Vitals>().get_stored(1),
                None
            );
            assert!(!world.resource::<crate::shop::inn::State>().active());
            return Some(match case {
                0 => "async-inn-foreground",
                1 => "async-inn-common",
                _ => "async-inn-map",
            });
        }
        return Some(match case {
            0 => "async-transition-foreground",
            1 => "async-transition-common",
            _ => "async-transition-map",
        });
    }
    None
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert_eq!(probe.completed, 7);
    assert_eq!(probe.callbacks, 7);
    assert!(probe.checks >= 210);
    if probe.inn {
        info!(
            "async inns: {} ordered states across foreground, common and map owners; healing and disabled gates verified",
            probe.checks
        );
        return;
    }
    assert_eq!(probe.barriers, [7; 3]);
    info!(
        "async transitions: {} ordered states across foreground, common and map owners; redundant erase, no-effect update barriers and disabled gates verified",
        probe.checks
    );
}
