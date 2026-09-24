use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Selection {
    index: Option<usize>,
    revision: u64,
}

#[derive(Clone, Copy)]
pub(in crate::interpreter) struct PageOwner {
    id: u32,
    revision: u64,
}

impl PageOwner {
    pub(in crate::interpreter) fn current(self, pool: &ParallelPool) -> bool {
        self.matches(&pool.pages)
    }

    fn matches(self, pages: &BTreeMap<u32, Selection>) -> bool {
        pages
            .get(&self.id)
            .is_some_and(|page| page.revision == self.revision)
    }
}

impl ParallelPool {
    pub(super) fn owner(&self, source: ParallelSource) -> Option<PageOwner> {
        let ParallelSource::MapPage(id, _) = source else {
            return None;
        };
        Some(PageOwner {
            id,
            revision: self.pages.get(&id)?.revision,
        })
    }

    pub(in crate::interpreter) fn discard_changed_pages(&mut self, exec: &Exec) {
        self.refresh_pages(
            exec.subsystems.flow.map_events.as_deref(),
            &exec.switches,
            &exec.variables,
            &exec.party,
            &exec.inventory,
        );
    }

    pub(super) fn refresh_pages(
        &mut self,
        events: Option<&MapEvents>,
        switches: &Switches,
        variables: &Variables,
        party: &Party,
        inventory: &Inventory,
    ) {
        let ids = events
            .into_iter()
            .flat_map(|events| events.events.iter().map(|event| event.id))
            .collect::<BTreeSet<_>>();
        self.pages.retain(|id, _| ids.contains(id));
        if let Some(events) = events {
            for event in &events.events {
                let index = active_page_index(event, switches, variables, party, inventory);
                let selection = self
                    .pages
                    .entry(event.id)
                    .or_insert(Selection { index, revision: 0 });
                if selection.index != index {
                    // Losing every page pauses future updates without clearing the current burst.
                    if index.is_some() {
                        selection.revision = selection.revision.wrapping_add(1);
                    }
                    selection.index = index;
                }
            }
        }
        let pages = &self.pages;
        self.frames
            .retain(|entry| entry.owner.is_none_or(|owner| owner.matches(pages)));
    }
}
