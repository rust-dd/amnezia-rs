use super::{Fade, PendingTeleport};
use crate::interpreter::{CommonEvents, RunningEvent};
use crate::state::Variables;
use crate::timing::{GameFrames, SceneFrames};
use crate::transitions::Transition;
use crate::world::{MapData, MapEvents};
use amnezia_data::{CommonEvent, EventCommand};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct Probe {
    scene: Option<u32>,
    started: u32,
    checks: u32,
    finished: bool,
    captured: bool,
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
    command(10220, vec![0, 4900 + index, 4900 + index, 1, 0, 1])
}

fn common(id: u32, commands: Vec<EventCommand>) -> CommonEvent {
    CommonEvent {
        id,
        name: "Transfer reservation probe".into(),
        trigger: 4,
        switch_flag: false,
        switch_id: 0,
        commands,
    }
}

pub(crate) fn configure(app: &mut App) {
    crate::timing::logical::post(app, || observe.after(super::TransferCommit));
}

fn prepare(world: &mut World) {
    assert!(!world.resource::<RunningEvent>().active());
    assert!(!world.resource::<Transition>().busy());
    assert_eq!(world.resource::<MapData>().map_id, 3);
    world.insert_resource(Probe::default());
    let mut variables = world.resource_mut::<Variables>();
    for id in 4900..4905 {
        variables.set(id, 0);
    }
    for (id, value) in [(4910, 3), (4911, 12), (4912, 6)] {
        variables.set(id, value);
    }
    world.resource_mut::<CommonEvents>().0.extend([
        common(
            900,
            vec![
                command(10810, vec![4, 7, 6]),
                increment(0),
                command(10830, vec![4910, 4911, 4912]),
                increment(1),
                command(11410, vec![1000]),
            ],
        ),
        common(901, vec![increment(2), command(11410, vec![1000])]),
    ]);
    let mut event = world.resource::<MapEvents>().events[0].clone();
    event.id = 900;
    event.pages.truncate(1);
    let page = &mut event.pages[0];
    page.condition = default();
    page.trigger = 4;
    page.commands = vec![
        command(10810, vec![4, 8, 8]),
        increment(3),
        command(11410, vec![1000]),
    ];
    world.resource_mut::<MapEvents>().events.push(event);
    world
        .resource_mut::<RunningEvent>()
        .start(7, vec![command(10810, vec![3, 15, 6]), increment(4)]);
}

#[allow(clippy::too_many_arguments)]
fn observe(
    probe: Option<ResMut<Probe>>,
    raw: Res<GameFrames>,
    scene: Res<SceneFrames>,
    variables: Res<Variables>,
    fade: Res<Fade>,
    pending: Res<PendingTeleport>,
    transition: Res<Transition>,
    map: Res<MapData>,
    hero: Query<&crate::player::Player>,
) {
    let Some(mut probe) = probe else {
        return;
    };
    if probe.finished {
        return;
    }
    let counts = std::array::from_fn::<_, 5, _>(|index| variables.get(4900 + index as u32));
    assert_eq!(&counts[..4], &[1; 4]);
    assert!(pending.0.is_none());
    if let Some(before) = probe.scene {
        assert_eq!(scene.frame, before + u32::from(counts[4] == 1));
    } else {
        assert!(fade.busy());
        assert!(transition.busy());
        assert_eq!(transition.age(), 0);
        assert_eq!(counts[4], 0);
        probe.scene = Some(scene.frame);
        probe.started = raw.frame;
    }
    probe.checks += 1;
    if counts[4] == 1 {
        assert!(!fade.busy());
        assert!(!transition.busy());
        assert!(raw.frame.wrapping_sub(probe.started) >= 70);
        assert_eq!(map.map_id, 3);
        let hero = hero.single().unwrap();
        assert_eq!((hero.tile_x, hero.tile_y), (15, 6));
        probe.finished = true;
    } else {
        assert!(raw.frame.wrapping_sub(probe.started) < 200);
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        prepare(world);
    }
    let mut probe = world.get_resource_mut::<Probe>()?;
    if probe.finished && !probe.captured {
        probe.captured = true;
        return Some("reserved-transfer-finished");
    }
    None
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.finished && probe.captured);
    assert!(probe.checks >= 70);
    info!(
        "reserved transfers: {} ordered states, all old-map owners, recall tail, final target and scene clock verified",
        probe.checks
    );
}
