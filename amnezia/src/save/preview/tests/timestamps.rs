use super::*;
use std::path::PathBuf;
use std::time::Duration;

struct Slots(PathBuf);

impl Slots {
    fn new(tag: &str) -> Self {
        let directory = crate::save::tests::temp_slot(tag);
        std::fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn path(&self, number: u8) -> PathBuf {
        self.0.join(format!("slot{number}.ron"))
    }

    fn write(&self, number: u8, version: u32, saved_at: Option<u64>, modified: Duration) {
        let stamp = saved_at.map_or_else(String::new, |stamp| format!("saved_at:Some({stamp}),"));
        let text = format!(
            "(format_version:{version},{stamp}map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0)"
        );
        std::fs::write(self.path(number), text).unwrap();
        std::fs::File::open(self.path(number))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + modified))
            .unwrap();
    }

    fn selected(&self, data: &GameData) -> usize {
        let before = self.bytes();
        let entries = catalog(&self.path(1), data);
        assert_eq!(self.bytes(), before);
        latest(&entries)
    }

    fn bytes(&self) -> Vec<Option<Vec<u8>>> {
        (1..=15)
            .map(|number| std::fs::read(self.path(number)).ok())
            .collect()
    }
}

impl Drop for Slots {
    fn drop(&mut self) {
        for number in 1..=15 {
            let path = self.path(number);
            if path.is_file() {
                std::fs::remove_file(path).unwrap();
            }
        }
        std::fs::remove_dir(&self.0).unwrap();
    }
}

#[test]
fn saved_timestamps_select_the_latest_slot_even_when_every_file_time_is_reversed() {
    let slots = Slots::new("preview_saved_timestamps");
    let data = database();
    for number in 1..=15 {
        slots.write(
            number,
            15,
            Some(u64::from(number)),
            Duration::from_secs(100 - u64::from(number)),
        );
    }
    assert_eq!(slots.selected(&data), 14);
    std::fs::copy(slots.path(4), slots.path(1)).unwrap();
    assert_eq!(slots.selected(&data), 14);
}

#[test]
fn equal_saved_seconds_keep_the_first_valid_slot_regardless_of_file_times() {
    let slots = Slots::new("preview_timestamp_tie");
    let data = database();
    slots.write(4, 15, Some(20), Duration::from_secs(4));
    slots.write(15, 15, Some(20), Duration::from_secs(15));
    slots.write(
        2,
        crate::save::SAVE_FORMAT_VERSION + 1,
        Some(100),
        Duration::from_secs(100),
    );
    assert_eq!(slots.selected(&data), 3);
}

#[test]
fn legacy_saves_keep_their_precise_file_time_fallback_without_being_rewritten() {
    let slots = Slots::new("preview_legacy_timestamps");
    let data = database();
    for version in 0..=15 {
        slots.write(1, version, None, Duration::new(100, 0));
        slots.write(15, version, None, Duration::new(100, 500_000_000));
        assert_eq!(slots.selected(&data), 14, "legacy format {version}");
    }
}

#[test]
fn an_explicit_epoch_timestamp_is_not_mistaken_for_missing_metadata() {
    let slots = Slots::new("preview_epoch_timestamp");
    let data = database();
    slots.write(1, 15, Some(0), Duration::from_secs(100));
    slots.write(15, 15, Some(1), Duration::from_secs(1));
    assert_eq!(slots.selected(&data), 14);
}

#[test]
fn the_full_timestamp_range_can_be_compared_without_a_platform_clock_overflow() {
    let slots = Slots::new("preview_timestamp_range");
    let data = database();
    slots.write(1, 15, Some(1), Duration::from_secs(100));
    slots.write(15, 15, Some(u64::MAX), Duration::from_secs(1));
    assert_eq!(slots.selected(&data), 14);
}
