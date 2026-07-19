//! Game state the event interpreter reads and writes: switches and variables,
//! plus the active-page selector that respects RM2000 page conditions.

use amnezia_data::{Event, EventPage};
use bevy::prelude::*;
use std::collections::HashMap;

/// RM2000 caps the active party at four members (EasyRPG `Game_Party::AddActor`).
const MAX_PARTY: usize = 4;
/// The RM2000 gold ceiling (EasyRPG `Game_Constants::MaxGoldValue`).
const MAX_GOLD: i32 = 999_999;
/// The default RM2000 per-item stack limit (EasyRPG `Game_Constants::MaxItemCount`).
const MAX_ITEM_COUNT: u32 = 99;
/// The RM2000 variable bounds: every write clamps into `[MIN_VAR, MAX_VAR]`
/// (EasyRPG `Game_Variables::min_2k`/`max_2k`, the RPG2000 six-nines range).
const MIN_VAR: i32 = -999_999;
const MAX_VAR: i32 = 999_999;

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
    /// Every set switch as `(id, value)` pairs, for a save snapshot.
    pub fn entries(&self) -> Vec<(u32, bool)> {
        self.0.iter().map(|(&id, &value)| (id, value)).collect()
    }
    /// Replace all switches with a restored snapshot.
    pub fn load(&mut self, entries: Vec<(u32, bool)>) {
        self.0 = entries.into_iter().collect();
    }
}

/// The game's integer variables, keyed by 1-based id (default 0).
#[derive(Resource, Default)]
pub struct Variables(HashMap<u32, i32>);

impl Variables {
    pub fn get(&self, id: u32) -> i32 {
        self.0.get(&id).copied().unwrap_or(0)
    }
    /// Store `value` in variable `id`, clamped to the RM2000 range
    /// `[MIN_VAR, MAX_VAR]` — RPG_RT clamps every variable write, so an add/mul
    /// that overshoots saturates at the six-nines bound rather than wrapping.
    pub fn set(&mut self, id: u32, value: i32) {
        self.0.insert(id, value.clamp(MIN_VAR, MAX_VAR));
    }
    /// Every set variable as `(id, value)` pairs, for a save snapshot.
    pub fn entries(&self) -> Vec<(u32, i32)> {
        self.0.iter().map(|(&id, &value)| (id, value)).collect()
    }
    /// Replace all variables with a restored snapshot.
    pub fn load(&mut self, entries: Vec<(u32, i32)>) {
        self.0 = entries.into_iter().collect();
    }
}

/// The current party roster: actor ids in join order. Starts with the hero
/// (actor 1); `ChangePartyMembers` adds and removes members.
#[derive(Resource)]
pub struct Party {
    members: Vec<u32>,
}

impl Default for Party {
    fn default() -> Self {
        Self { members: vec![1] }
    }
}

impl Party {
    /// Add `actor_id` to the roster unless it is already present or the party is
    /// full — RM2000 refuses a fifth member (`Game_Party::AddActor`). The
    /// actor-exists check lives at the call site, which holds the actor database.
    pub fn add(&mut self, actor_id: u32) {
        if !self.members.contains(&actor_id) && self.members.len() < MAX_PARTY {
            self.members.push(actor_id);
        }
    }
    pub fn remove(&mut self, actor_id: u32) {
        self.members.retain(|&id| id != actor_id);
    }
}

impl Party {
    pub fn has(&self, actor_id: u32) -> bool {
        self.members.contains(&actor_id)
    }
    /// The roster as actor ids in join order, for a save snapshot.
    pub fn snapshot(&self) -> Vec<u32> {
        self.members.clone()
    }
    /// Replace the roster with a restored snapshot.
    pub fn restore(&mut self, members: Vec<u32>) {
        self.members = members;
    }
}

/// The party's inventory: item counts keyed by item id, plus gold.
#[derive(Resource, Default)]
pub struct Inventory {
    items: HashMap<u32, u32>,
    gold: i32,
}

impl Inventory {
    /// Add `count` of an item, capping the stack at [`MAX_ITEM_COUNT`] (RM2000
    /// `Game_Party::AddItem` clamps to the per-item maximum, 99 by default).
    pub fn add_item(&mut self, item_id: u32, count: u32) {
        let owned = self.items.entry(item_id).or_insert(0);
        *owned = owned.saturating_add(count).min(MAX_ITEM_COUNT);
    }
    pub fn remove_item(&mut self, item_id: u32, count: u32) {
        if let Some(owned) = self.items.get_mut(&item_id) {
            *owned = owned.saturating_sub(count);
        }
    }
    /// Add gold, clamping the total to `[0, MAX_GOLD]` (RM2000
    /// `Game_Party::GainGold`).
    pub fn add_gold(&mut self, amount: i32) {
        self.gold = self.gold.saturating_add(amount).clamp(0, MAX_GOLD);
    }
    /// Remove gold, clamping the total to `[0, MAX_GOLD]` (RM2000
    /// `Game_Party::LoseGold`).
    pub fn remove_gold(&mut self, amount: i32) {
        self.gold = self.gold.saturating_sub(amount).clamp(0, MAX_GOLD);
    }
}

impl Inventory {
    pub fn count(&self, item_id: u32) -> u32 {
        self.items.get(&item_id).copied().unwrap_or(0)
    }
    pub fn has(&self, item_id: u32) -> bool {
        self.count(item_id) > 0
    }
    pub fn gold(&self) -> i32 {
        self.gold
    }
    /// Item `(id, count)` pairs plus gold, for a save snapshot.
    pub fn snapshot(&self) -> (Vec<(u32, u32)>, i32) {
        (
            self.items.iter().map(|(&id, &count)| (id, count)).collect(),
            self.gold,
        )
    }
    /// Replace items and gold with a restored snapshot.
    pub fn restore(&mut self, items: Vec<(u32, u32)>, gold: i32) {
        self.items = items.into_iter().collect();
        self.gold = gold;
    }
}

fn condition_holds(
    page: &EventPage,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
) -> bool {
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
    if c.flags & 0x08 != 0 && !inventory.has(c.item_id) {
        return false;
    }
    if c.flags & 0x10 != 0 && !party.has(c.actor_id) {
        return false;
    }
    true
}

/// The active page: the highest-index page whose condition holds (RM2000 rule).
/// Timer conditions (flag bit 5) are still treated as satisfied.
pub fn active_page<'a>(
    event: &'a Event,
    switches: &Switches,
    variables: &Variables,
    party: &Party,
    inventory: &Inventory,
) -> Option<&'a EventPage> {
    event
        .pages
        .iter()
        .rev()
        .find(|p| condition_holds(p, switches, variables, party, inventory))
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
            direction: 2,
            pattern: 1,
            move_type: 0,
            move_frequency: 3,
            move_speed: 3,
            layer: 0,
            condition,
            commands: vec![],
        }
    }

    fn event(pages: Vec<EventPage>) -> Event {
        Event {
            id: 1,
            x: 0,
            y: 0,
            name: String::new(),
            pages,
        }
    }

    #[test]
    fn highest_satisfied_page_wins() {
        let mut sw = Switches::default();
        let (var, party, inv) = (Variables::default(), Party::default(), Inventory::default());
        let ev = event(vec![
            page(EventCondition::default()),
            page(EventCondition {
                flags: 1,
                switch_a: 2,
                ..Default::default()
            }),
        ]);
        assert_eq!(
            active_page(&ev, &sw, &var, &party, &inv).map(|p| p.condition.flags),
            Some(0)
        );
        sw.set(2, true);
        assert_eq!(
            active_page(&ev, &sw, &var, &party, &inv).map(|p| p.condition.flags),
            Some(1)
        );
    }

    #[test]
    fn no_page_when_none_satisfied() {
        let ev = event(vec![page(EventCondition {
            flags: 1,
            switch_a: 5,
            ..Default::default()
        })]);
        let (sw, var, party, inv) = (
            Switches::default(),
            Variables::default(),
            Party::default(),
            Inventory::default(),
        );
        assert!(active_page(&ev, &sw, &var, &party, &inv).is_none());
    }

    #[test]
    fn item_and_actor_gated_pages() {
        let (sw, var) = (Switches::default(), Variables::default());
        let ev = event(vec![
            page(EventCondition::default()),
            page(EventCondition {
                flags: 0x08,
                item_id: 5,
                ..Default::default()
            }),
            page(EventCondition {
                flags: 0x10,
                actor_id: 3,
                ..Default::default()
            }),
        ]);
        let (mut party, mut inv) = (Party::default(), Inventory::default());
        let flags = |party: &Party, inv: &Inventory| {
            active_page(&ev, &sw, &var, party, inv).map(|p| p.condition.flags)
        };
        assert_eq!(flags(&party, &inv), Some(0)); // no item, actor 3 absent
        inv.add_item(5, 1);
        assert_eq!(flags(&party, &inv), Some(0x08)); // has item 5
        party.add(3);
        assert_eq!(flags(&party, &inv), Some(0x10)); // actor 3 joined (higher page)
    }

    #[test]
    fn variable_writes_clamp_to_the_rm2000_range() {
        let mut var = Variables::default();
        var.set(1, 5_000_000);
        assert_eq!(var.get(1), MAX_VAR, "a write above the ceiling saturates");
        var.set(2, -5_000_000);
        assert_eq!(var.get(2), MIN_VAR, "a write below the floor saturates");
        var.set(3, 12_345);
        assert_eq!(var.get(3), 12_345, "an in-range write is stored verbatim");
    }

    #[test]
    fn party_starts_with_hero_and_changes() {
        let mut party = Party::default();
        assert!(party.has(1) && !party.has(2));
        party.add(2);
        party.add(2); // idempotent
        assert!(party.has(2));
        party.remove(2);
        assert!(!party.has(2));
    }

    #[test]
    fn inventory_items_and_gold() {
        let mut inv = Inventory::default();
        inv.add_item(181, 2);
        assert_eq!(inv.count(181), 2);
        inv.remove_item(181, 5); // saturates at 0
        assert_eq!(inv.count(181), 0);
        inv.add_gold(100);
        inv.remove_gold(30);
        assert_eq!(inv.gold(), 70);
    }

    #[test]
    fn party_refuses_a_fifth_member() {
        let mut party = Party::default(); // starts with the hero (actor 1)
        party.add(2);
        party.add(3);
        party.add(4);
        assert_eq!(party.snapshot(), vec![1, 2, 3, 4]);
        party.add(5); // the party is full — RM2000 caps at four
        assert_eq!(party.snapshot(), vec![1, 2, 3, 4], "fifth member refused");
    }

    #[test]
    fn gold_and_item_counts_are_clamped_to_their_maxima() {
        let mut inv = Inventory::default();
        inv.add_gold(2_000_000); // above the 999_999 ceiling
        assert_eq!(inv.gold(), MAX_GOLD);
        inv.remove_gold(5_000_000); // never below zero
        assert_eq!(inv.gold(), 0);

        inv.add_item(7, 250); // above the 99 stack cap
        assert_eq!(inv.count(7), MAX_ITEM_COUNT);
        inv.add_item(7, 50); // stays capped
        assert_eq!(inv.count(7), MAX_ITEM_COUNT);
    }
}
