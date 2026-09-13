use super::*;
use amnezia_data::{EventCommand, Map};

#[derive(Resource, Default)]
struct Probe {
    before: Option<Vec<u8>>,
    saved: Option<crate::interpreter::saved::State>,
    slot_bytes: Vec<u8>,
    held: u32,
    checked: u8,
}

fn switch(id: i32, operation: i32) -> EventCommand {
    EventCommand {
        code: 10210,
        indent: 0,
        string: String::new(),
        params: vec![0, id, id, operation],
    }
}

fn start(world: &mut World, map_id: u32) {
    world.init_resource::<Probe>();
    let path = world.resource::<Fixture>().slot.path(world);
    world.resource_mut::<Probe>().before = std::fs::read(path).ok();
    let map = crate::assets::load_ron::<Map>(&format!(
        "{}/maps/map_{map_id:04}.ron",
        crate::assets::asset_root()
    ));
    let page = map
        .events
        .iter()
        .flat_map(|event| &event.pages)
        .find(|page| page.commands.iter().any(|command| command.code == 11910))
        .unwrap();
    assert_eq!(page.commands[1].code, 11910);
    assert!(page.commands[1].params.is_empty());
    world.resource_mut::<Switches>().set(9901, false);
    world.resource_mut::<Switches>().set(9902, false);
    let mut commands = vec![switch(9901, 2)];
    commands.extend(page.commands.clone());
    commands.push(switch(9902, 2));
    let mut running = world.resource_mut::<RunningEvent>();
    assert!(!running.active());
    running.start(0, commands);
}

pub(super) fn drive(world: &mut World, frame: u32) {
    let path = world.resource::<Fixture>().slot.path(world);
    match frame {
        300 => start(world, 2),
        490 => start(world, 260),
        301 | 491 => {
            assert!(world.resource::<EventSaveRequest>().0);
            assert_eq!(world.resource::<RunningEvent>().debug_id(), Some(0));
            assert!(world.resource::<Switches>().get(9901));
            assert!(!world.resource::<Switches>().get(9902));
            assert_eq!(std::fs::read(path).ok(), world.resource::<Probe>().before);
            world.resource_mut::<Probe>().checked |= if frame == 301 { 1 } else { 4 };
        }
        303 | 493 => {
            let saved = read_save(&path).unwrap();
            assert!(saved.foreground.is_some());
            assert!(saved.switches.contains(&(9901, true)));
            assert!(!saved.switches.contains(&(9902, true)));
            assert!(world.resource::<Switches>().get(9901));
            assert!(world.resource::<Switches>().get(9902));
            assert!(!world.resource::<EventSaveRequest>().0);
            assert!(!world.resource::<RunningEvent>().active());
            let mut probe = world.resource_mut::<Probe>();
            probe.saved = saved.foreground;
            probe.slot_bytes = std::fs::read(&path).unwrap();
            probe.held = 0;
            probe.checked |= if frame == 303 { 2 } else { 8 };
            info!("save boundary: original crystal suspended, file written before event tail");
        }
        _ => {}
    }
    check_restore(world, frame);
}

pub(super) fn verify_finished(world: &mut World) {
    assert_eq!(world.remove_resource::<Probe>().unwrap().checked, 63);
}

fn check_restore(world: &mut World, frame: u32) {
    if !(321..410).contains(&frame) && !(501..590).contains(&frame) {
        return;
    }
    let path = world.resource::<Fixture>().slot.path(world);
    assert_eq!(
        std::fs::read(path).unwrap(),
        world.resource::<Probe>().slot_bytes
    );
    assert!(!world.resource::<EventSaveRequest>().0);
    assert!(world.resource::<Switches>().get(9901));
    if world.resource::<Fade>().busy() {
        assert_eq!(
            world.resource::<RunningEvent>().snapshot(),
            world.resource::<Probe>().saved
        );
        assert!(!world.resource::<Switches>().get(9902));
        world.resource_mut::<Probe>().held += 1;
    } else {
        assert!(world.resource::<Probe>().held > 60);
        assert!(!world.resource::<RunningEvent>().active());
        assert!(world.resource::<Switches>().get(9902));
        let bit = if frame < 410 { 16 } else { 32 };
        if world.resource::<Probe>().checked & bit == 0 {
            info!(
                "saved interpreter: exact state through the entire load fade, original crystal tail resumed once without rewriting the slot"
            );
        }
        world.resource_mut::<Probe>().checked |= bit;
    }
}
