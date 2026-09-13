use super::*;
use crate::save::slots::ActiveSlot;

#[derive(Resource)]
struct Probe {
    first_bytes: Vec<u8>,
    loaded: bool,
}

pub(super) fn drive(world: &mut World, frame: u32) {
    if frame == 485 {
        assert_eq!(*world.resource::<ActiveSlot>(), ActiveSlot::default());
        let first = world.resource::<Fixture>().slot.path(world);
        world.insert_resource(Probe {
            first_bytes: std::fs::read(first).unwrap(),
            loaded: false,
        });
        world.insert_resource(ActiveSlot::new(15).unwrap());
    }
    if let Some(probe) = world.get_resource::<Probe>() {
        let first = world.resource::<Fixture>().slot.path(world);
        assert_eq!(std::fs::read(first).unwrap(), probe.first_bytes);
    }
    if frame == 700 {
        assert_eq!(
            *world.resource::<ActiveSlot>(),
            ActiveSlot::new(15).unwrap()
        );
        assert_eq!(world.resource::<LoadOutcome>().0, Some(true));
        let first = world.resource::<Fixture>().slot.path(world);
        assert_eq!(
            read_save(&first).unwrap().format_version,
            SAVE_FORMAT_VERSION
        );
        let selected = world.resource::<Fixture>().slot.selected_path(world);
        assert_ne!(selected, first);
        assert_eq!(read_save(&selected).unwrap().format_version, 1);
        world.resource_mut::<Probe>().loaded = true;
        info!("saved slots: slot 15 restored independently while slot 1 stayed unchanged");
    }
}

pub(super) fn verify_finished(world: &mut World) {
    let probe = world.remove_resource::<Probe>().unwrap();
    assert!(probe.loaded);
    let first = world.resource::<Fixture>().slot.path(world);
    assert_eq!(std::fs::read(first).unwrap(), probe.first_bytes);
    let selected = world.resource::<Fixture>().slot.selected_path(world);
    std::fs::remove_file(selected).unwrap();
    world.insert_resource(ActiveSlot::default());
}
