//! Game state the event interpreter reads and writes: switches and variables,
//! plus the active-page selector that respects RM2000 page conditions.

use amnezia_data::{Event, EventPage};
use bevy::prelude::*;
use std::collections::HashMap;

/// The game's on/off switches, keyed by 1-based id (default false).
#[derive(Resource, Default)]
pub struct Switches(HashMap<u32, bool>);

impl Switches {
    pub fn get(&self, id: u32) -> bool {
        self.0.get(&id).copied().unwrap_or(false)
    }
    pub fn set(&mut self, id: u32, value: bool) {
        self.0.insert(id, value);
    }
}

/// The game's integer variables, keyed by 1-based id (default 0).
#[derive(Resource, Default)]
pub struct Variables(HashMap<u32, i32>);

impl Variables {
    pub fn get(&self, id: u32) -> i32 {
        self.0.get(&id).copied().unwrap_or(0)
    }
    pub fn set(&mut self, id: u32, value: i32) {
        self.0.insert(id, value);
    }
}

fn condition_holds(page: &EventPage, switches: &Switches, variables: &Variables) -> bool {
    let c = &page.condition;
    if c.flags & 0x01 != 0 && !switches.get(c.switch_a) {
        return false;
    }
    if c.flags & 0x02 != 0 && !switches.get(c.switch_b) {
        return false;
    }
    if c.flags & 0x04 != 0 && variables.get(c.variable_id) < c.variable_value as i32 {
        return false;
    }
    true
}

/// The active page: the highest-index page whose condition holds (RM2000 rule).
/// Item/actor/timer conditions are treated as satisfied for now.
pub fn active_page<'a>(
    event: &'a Event,
    switches: &Switches,
    variables: &Variables,
) -> Option<&'a EventPage> {
    event.pages.iter().rev().find(|p| condition_holds(p, switches, variables))
}

#[cfg(test)]
mod tests {
    use super::*;
    use amnezia_data::EventCondition;

    fn page(condition: EventCondition) -> EventPage {
        EventPage {
            trigger: 0,
            graphic_name: String::new(),
            graphic_index: 0,
            layer: 0,
            condition,
            commands: vec![],
        }
    }

    fn event(pages: Vec<EventPage>) -> Event {
        Event { id: 1, x: 0, y: 0, name: String::new(), pages }
    }

    #[test]
    fn highest_satisfied_page_wins() {
        let mut switches = Switches::default();
        let variables = Variables::default();
        let ev = event(vec![
            page(EventCondition::default()),
            page(EventCondition { flags: 1, switch_a: 2, ..Default::default() }),
        ]);
        assert_eq!(active_page(&ev, &switches, &variables).map(|p| p.condition.flags), Some(0));
        switches.set(2, true);
        assert_eq!(active_page(&ev, &switches, &variables).map(|p| p.condition.flags), Some(1));
    }

    #[test]
    fn no_page_when_none_satisfied() {
        let ev = event(vec![page(EventCondition { flags: 1, switch_a: 5, ..Default::default() })]);
        assert!(active_page(&ev, &Switches::default(), &Variables::default()).is_none());
    }
}
