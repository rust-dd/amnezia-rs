use super::*;
use crate::save::slots::{ActiveSlot, COUNT};

fn fixture(tag: &str) -> (App, PathBuf) {
    let directory = temp_slot(tag);
    std::fs::create_dir(&directory).unwrap();
    let first = directory.join("slot1.ron");
    let mut app = save_app(first.clone());
    app.add_plugins(crate::gamedata::GameDataPlugin)
        .init_resource::<RunningEvent>();
    let mut map = MapData::for_test(20, 15);
    map.map_id = 2;
    app.insert_resource(map);
    app.world_mut().spawn(Player {
        tile_x: 3,
        tile_y: 4,
        dir: 2,
        frame: 1,
        charset: "Chara1".into(),
        index: 0,
    });
    (app, first)
}

#[test]
fn slot_numbers_are_bounded_and_the_first_path_keeps_legacy_names() {
    let first = std::path::Path::new("/saves/custom-legacy-slot.ron");
    for number in 0..=u8::MAX {
        let slot = ActiveSlot::new(number);
        assert_eq!(slot.is_some(), (1..=15).contains(&number));
        if let Some(slot) = slot {
            assert_eq!(slot.path(first).parent(), first.parent());
        }
    }
    assert_eq!(ActiveSlot::default().path(first), first);
    assert_eq!(
        ActiveSlot::new(15).unwrap().path(first),
        first.with_file_name("slot15.ron")
    );
}

#[test]
fn failed_selected_slot_writes_preserve_both_the_destination_and_neighbor() {
    let (mut app, first) = fixture("failed_selected_slot");
    saved_slot(&first, 1);
    let first_bytes = std::fs::read(&first).unwrap();
    let slot = ActiveSlot::new(2).unwrap();
    let selected = slot.path(&first);
    std::fs::create_dir(&selected).unwrap();
    let marker = selected.join("untouched");
    std::fs::write(&marker, "preserved").unwrap();
    app.insert_resource(slot);
    app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
    app.update();
    assert!(!app.world().resource::<EventSaveRequest>().0);
    assert_eq!(std::fs::read(&first).unwrap(), first_bytes);
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "preserved");
    assert_eq!(
        std::fs::read_dir(first.parent().unwrap()).unwrap().count(),
        2
    );
    std::fs::remove_file(marker).unwrap();
    std::fs::remove_dir(selected).unwrap();
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_dir(first.parent().unwrap()).unwrap();
}

fn saved_slot(first: &std::path::Path, number: u8) -> PathBuf {
    let path = ActiveSlot::new(number).unwrap().path(first);
    std::fs::write(&path, format!("(map_id:2,x:3,y:4,dir:2,switches:[],variables:[(77,{number})],party:[1],items:[],gold:0)")).unwrap();
    path
}

#[test]
fn loading_uses_the_selected_slot_instead_of_the_first_file() {
    let (mut app, first) = fixture("selected_save_slot");
    saved_slot(&first, 1);
    let selected = saved_slot(&first, 15);
    app.insert_resource(ActiveSlot::new(15).unwrap());
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert_eq!(app.world().resource::<Variables>().get(77), 15);
    std::fs::remove_file(selected).unwrap();
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_dir(first.parent().unwrap()).unwrap();
}

#[test]
fn an_empty_selected_slot_does_not_fall_back_to_an_existing_first_slot() {
    let (mut app, first) = fixture("missing_selected_slot");
    saved_slot(&first, 1);
    let bytes = std::fs::read(&first).unwrap();
    app.insert_resource(ActiveSlot::new(2).unwrap());
    app.world_mut().resource_mut::<Switches>().set(888, true);
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(false));
    assert!(app.world().resource::<Switches>().get(888));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert_eq!(std::fs::read(&first).unwrap(), bytes);
    std::fs::remove_file(&first).unwrap();
    std::fs::remove_dir(first.parent().unwrap()).unwrap();
}

#[test]
fn every_selected_slot_round_trips_without_overwriting_its_neighbors() {
    let (mut app, first) = fixture("fifteen_save_slots");
    let mut written = Vec::new();
    for number in 1..=COUNT {
        let slot = ActiveSlot::new(number).unwrap();
        app.insert_resource(slot);
        app.world_mut()
            .resource_mut::<Variables>()
            .set(77, number as i32);
        app.world_mut().resource_mut::<EventSaveRequest>().0 = true;
        app.update();
        let path = slot.path(&first);
        assert!(path.is_file(), "slot {number} must have its own file");
        written.push((path, std::fs::read(slot.path(&first)).unwrap()));
        for (path, bytes) in &written {
            assert_eq!(std::fs::read(path).unwrap(), *bytes);
        }
    }
    for number in (1..=COUNT).rev() {
        app.insert_resource(ActiveSlot::new(number).unwrap());
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        assert_eq!(app.world().resource::<Variables>().get(77), number as i32);
    }
    for (path, bytes) in written {
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_file(path).unwrap();
    }
    std::fs::remove_dir(first.parent().unwrap()).unwrap();
}
