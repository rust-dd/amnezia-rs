use super::*;
use amnezia_data::{EventCondition, EventPage};

fn cmd(code: u32) -> EventCommand {
    EventCommand {
        code,
        indent: 0,
        string: String::new(),
        params: Vec::new(),
    }
}

fn page(trigger: u32, condition: EventCondition) -> EventPage {
    EventPage {
        trigger,
        graphic_name: String::new(),
        graphic_index: 0,
        direction: 2,
        pattern: 1,
        animation_type: 0,
        translucent: false,
        overlap_forbidden: false,
        move_type: 0,
        move_frequency: 3,
        move_speed: 3,
        move_route: Default::default(),
        layer: 0,
        condition,
        commands: vec![cmd(10210)],
    }
}

fn event(id: u32, pages: Vec<EventPage>) -> amnezia_data::Event {
    amnezia_data::Event {
        id,
        x: 0,
        y: 0,
        name: String::new(),
        pages,
    }
}

fn common_event(id: u32, trigger: u32, switch_id: u32) -> CommonEvent {
    CommonEvent {
        id,
        name: String::new(),
        trigger,
        switch_flag: switch_id != 0,
        switch_id,
        commands: vec![cmd(10210)],
    }
}

/// Run the reconciler with fresh empty state except the given switches.
fn reconcile_with(
    pool: &mut ParallelPool,
    commons: &CommonEvents,
    map: Option<&MapEvents>,
    switches: &Switches,
) {
    let (variables, party, inventory) =
        (Variables::default(), Party::default(), Inventory::default());
    reconcile(pool, commons, map, switches, &variables, &party, &inventory);
}

#[test]
fn parallel_common_event_is_retained_across_switch_pauses() {
    let commons = CommonEvents(vec![common_event(1, 4, 5)]);
    let mut pool = ParallelPool::default();
    let mut switches = Switches::default();

    reconcile_with(&mut pool, &commons, None, &switches);
    assert_eq!(
        pool.count(),
        1,
        "a parallel common event retains its interpreter while gated off"
    );

    switches.set(5, true);
    reconcile_with(&mut pool, &commons, None, &switches);
    assert_eq!(pool.count(), 1);
    assert!(matches!(pool.frames[0].source, ParallelSource::Common(1)));

    switches.set(5, false);
    reconcile_with(&mut pool, &commons, None, &switches);
    assert_eq!(
        pool.count(),
        1,
        "turning its switch off must not discard its execution state"
    );
}

#[test]
fn unconditional_common_event_and_call_only_are_handled() {
    let commons = CommonEvents(vec![common_event(1, 4, 0), common_event(2, 5, 0)]);
    let mut pool = ParallelPool::default();
    reconcile_with(&mut pool, &commons, None, &Switches::default());
    assert_eq!(
        pool.count(),
        1,
        "only the unconditional parallel event runs"
    );
    assert!(matches!(pool.frames[0].source, ParallelSource::Common(1)));
}

#[test]
fn only_trigger_four_map_pages_join_the_pool() {
    let map = MapEvents {
        events: vec![
            event(1, vec![page(4, EventCondition::default())]),
            event(2, vec![page(0, EventCondition::default())]),
        ],
    };
    let mut pool = ParallelPool::default();
    reconcile_with(
        &mut pool,
        &CommonEvents::default(),
        Some(&map),
        &Switches::default(),
    );
    assert_eq!(
        pool.count(),
        1,
        "the trigger-4 page runs; the plain event does not"
    );
    assert!(matches!(
        pool.frames[0].source,
        ParallelSource::MapPage(1, 0)
    ));
}

#[test]
fn a_promoted_page_resets_the_frame_to_the_new_page() {
    let map = MapEvents {
        events: vec![event(
            7,
            vec![
                page(4, EventCondition::default()),
                page(
                    4,
                    EventCondition {
                        flags: 0x01,
                        switch_a: 5,
                        ..Default::default()
                    },
                ),
            ],
        )],
    };
    let mut pool = ParallelPool::default();
    let mut switches = Switches::default();

    reconcile_with(&mut pool, &CommonEvents::default(), Some(&map), &switches);
    assert!(matches!(
        pool.frames[0].source,
        ParallelSource::MapPage(7, 0)
    ));

    switches.set(5, true);
    reconcile_with(&mut pool, &CommonEvents::default(), Some(&map), &switches);
    assert_eq!(pool.count(), 1, "still one frame for the event");
    assert!(
        matches!(pool.frames[0].source, ParallelSource::MapPage(7, 1)),
        "the frame is keyed to the newly active page"
    );
}

#[test]
fn tracked_refresh_observes_switch_changes_between_calls_without_a_render() {
    let mut world = World::new();
    world.init_resource::<ParallelPool>();
    world.init_resource::<Switches>();
    world.init_resource::<Variables>();
    world.init_resource::<Party>();
    world.init_resource::<Inventory>();
    world.insert_resource(MapEvents {
        events: vec![event(
            7,
            vec![
                page(4, EventCondition::default()),
                page(
                    4,
                    EventCondition {
                        flags: 1,
                        switch_a: 5,
                        ..default()
                    },
                ),
            ],
        )],
    });
    world.run_system_cached(refresh_map_pages).unwrap();
    let original = world
        .resource::<ParallelPool>()
        .owner(ParallelSource::MapPage(7, 0))
        .unwrap();
    world.run_system_cached(refresh_map_pages).unwrap();
    assert!(original.current(world.resource::<ParallelPool>()));
    world.resource_mut::<Switches>().set(5, true);
    world.run_system_cached(refresh_map_pages).unwrap();
    assert!(!original.current(world.resource::<ParallelPool>()));
    let next = world
        .resource::<ParallelPool>()
        .owner(ParallelSource::MapPage(7, 1))
        .unwrap();
    world.resource_mut::<Switches>().set(5, false);
    world.run_system_cached(refresh_map_pages).unwrap();
    assert!(!next.current(world.resource::<ParallelPool>()));
}

#[test]
fn reconcile_is_idempotent_and_preserves_running_frames() {
    let map = MapEvents {
        events: vec![event(1, vec![page(4, EventCondition::default())])],
    };
    let mut pool = ParallelPool::default();
    reconcile_with(
        &mut pool,
        &CommonEvents::default(),
        Some(&map),
        &Switches::default(),
    );
    pool.frames[0].frame.start(1, vec![cmd(10210)]);
    reconcile_with(
        &mut pool,
        &CommonEvents::default(),
        Some(&map),
        &Switches::default(),
    );
    assert_eq!(
        pool.count(),
        1,
        "a still-desired frame is neither dropped nor duplicated"
    );
    assert!(
        pool.frames[0].frame.active(),
        "and its running state is preserved"
    );
}
