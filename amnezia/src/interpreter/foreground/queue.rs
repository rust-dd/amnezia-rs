use crate::state::{Inventory, Party, Switches, Variables, active_page_index};
use crate::world::MapEvents;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    page: usize,
    waiting: bool,
    paused: bool,
    #[serde(default)]
    decision: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(in crate::interpreter) struct Queue {
    map_id: Option<u32>,
    events: BTreeMap<u32, Entry>,
}

impl Queue {
    pub(in crate::interpreter) fn empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(in crate::interpreter) fn paused(&self, id: u32) -> bool {
        self.events.get(&id).is_some_and(|entry| entry.paused)
    }

    pub(in crate::interpreter) fn waiting(&self) -> bool {
        self.events.values().any(|entry| entry.waiting)
    }

    #[cfg(test)]
    pub(super) fn waiting_ids(&self) -> Vec<u32> {
        self.events
            .iter()
            .filter_map(|(&id, entry)| entry.waiting.then_some(id))
            .collect()
    }

    pub(super) fn schedule(&mut self, map_id: u32, id: u32, page: usize, decision: bool) -> bool {
        self.map_id = Some(map_id);
        let entry = self.events.entry(id).or_insert(Entry {
            page,
            waiting: false,
            paused: false,
            decision,
        });
        entry.decision = decision;
        if entry.waiting {
            return false;
        }
        entry.waiting = true;
        entry.paused = true;
        true
    }

    pub(super) fn take_next(&mut self) -> Option<(u32, usize, bool)> {
        self.events.iter_mut().find_map(|(&id, entry)| {
            if !entry.waiting {
                return None;
            }
            entry.waiting = false;
            Some((id, entry.page, entry.decision))
        })
    }

    pub(in crate::interpreter) fn unpause(&mut self, id: u32) {
        if let Some(entry) = self.events.get_mut(&id) {
            entry.paused = false;
            if !entry.waiting {
                self.events.remove(&id);
            }
        }
        if self.events.is_empty() {
            self.map_id = None;
        }
    }

    pub(super) fn refresh(
        &mut self,
        map_id: u32,
        events: &MapEvents,
        switches: &Switches,
        variables: &Variables,
        party: &Party,
        inventory: &Inventory,
    ) {
        if self.map_id.is_some_and(|id| id != map_id) {
            *self = Self::default();
            return;
        }
        self.events.retain(|id, entry| {
            events
                .events
                .iter()
                .find(|event| event.id == *id)
                .is_some_and(|event| {
                    active_page_index(event, switches, variables, party, inventory)
                        == Some(entry.page)
                })
        });
        if self.events.is_empty() {
            self.map_id = None;
        }
    }

    pub(in crate::interpreter) fn valid(&self) -> bool {
        self.events.is_empty()
            || (self.map_id.is_some()
                && self
                    .events
                    .iter()
                    .all(|(&id, entry)| id > 0 && (entry.waiting || entry.paused)))
    }

    pub(in crate::interpreter) fn valid_map(&self, id: u32, map: &amnezia_data::Map) -> bool {
        self.map_id.is_none_or(|saved| saved == id)
            && self.events.iter().all(|(&id, entry)| {
                map.events
                    .iter()
                    .find(|event| event.id == id)
                    .is_some_and(|event| entry.page < event.pages.len())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::RunningEvent;

    #[test]
    fn finishing_an_event_keeps_a_second_scheduled_run_without_repausing_it() {
        let mut queue = Queue::default();
        queue.schedule(3, 1, 0, false);
        assert_eq!(queue.take_next(), Some((1, 0, false)));
        assert!(queue.paused(1));
        queue.schedule(3, 1, 0, false);
        queue.unpause(1);
        assert!(queue.waiting());
        assert!(!queue.paused(1));
        assert_eq!(queue.take_next(), Some((1, 0, false)));
        queue.unpause(1);
        assert_eq!(queue, Queue::default());
    }

    #[test]
    fn a_queue_without_an_active_interpreter_is_persistent_state() {
        let mut running = RunningEvent::default();
        running.queue.schedule(3, 1, 0, false);
        assert!(!running.active());
        let saved = running.snapshot().unwrap();
        assert!(saved.valid());
        let serialized = ron::to_string(&saved).unwrap();
        let restored = ron::from_str::<crate::interpreter::saved::State>(&serialized).unwrap();
        assert_eq!(restored, saved);
    }

    #[test]
    fn malformed_queue_entries_are_not_valid_save_state() {
        let mut queue = Queue::default();
        queue.schedule(3, 1, 0, false);
        queue.map_id = None;
        assert!(!queue.valid());
        queue.map_id = Some(3);
        queue.events.get_mut(&1).unwrap().waiting = false;
        queue.events.get_mut(&1).unwrap().paused = false;
        assert!(!queue.valid());
    }
}
