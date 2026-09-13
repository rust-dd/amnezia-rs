use bevy::prelude::Resource;
use std::path::{Path, PathBuf};

pub(crate) const COUNT: u8 = 15;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActiveSlot(u8);

impl Default for ActiveSlot {
    fn default() -> Self {
        Self::new(1).unwrap()
    }
}

impl ActiveSlot {
    pub(crate) fn new(number: u8) -> Option<Self> {
        (1..=COUNT).contains(&number).then_some(Self(number))
    }

    pub(crate) fn path(self, first_slot: &Path) -> PathBuf {
        if self.0 == 1 {
            first_slot.to_owned()
        } else {
            first_slot.with_file_name(format!("slot{}.ron", self.0))
        }
    }

    pub(crate) fn index(self) -> usize {
        usize::from(self.0 - 1)
    }
}
