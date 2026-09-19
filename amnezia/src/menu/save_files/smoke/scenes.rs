use super::*;
use crate::menu::save_files::scene::Stage;
use crate::transitions::Transition;

#[derive(Resource, Default)]
pub(super) struct Checks(u8);

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 375 {
        let mut files = world.resource_mut::<SaveFiles>();
        files.navigation.cursors[0] = 5;
        files.navigation.arrow = 34;
    }
    let transition = world.resource::<Transition>();
    if !transition.busy() || transition.age() != 1 {
        return None;
    }
    let files = world.resource::<SaveFiles>();
    let (bit, label) = match files.stage {
        Stage::ShowingList if frame < 380 => (1, "save-slots-fade"),
        Stage::LeavingList if frame < 410 => (2, "save-slots-cancel-fade"),
        Stage::ShowingList if (800..850).contains(&frame) => (4, "save-crystal-fade"),
        Stage::ShowingList if (900..950).contains(&frame) => (8, "save-crystal-erased-fade"),
        _ => return None,
    };
    if world.resource::<Checks>().0 & bit != 0 {
        return None;
    }
    if files.stage == Stage::ShowingList {
        assert_eq!(files.navigation.cursors[files.navigation.index], 1);
        assert_eq!(files.navigation.arrows, [false; 2]);
    } else {
        assert_eq!(files.navigation.cursors[0], 11);
        assert_eq!(files.navigation.arrow, 0);
        assert_eq!(files.navigation.arrows, [false, true]);
    }
    assert!(files.active());
    assert!(files.blocks_io());
    world.resource_mut::<Checks>().0 |= bit;
    Some(label)
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0, 15);
    info!("save scene fades: manual entry/cancel and visible/erased crystal entry verified");
}
