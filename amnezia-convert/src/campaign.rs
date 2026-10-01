use amnezia_data::Map;

/// The completed briefing must retire the earlier dream and clear the staircase
/// before the player can leave to save and return to the captain. Switch 404 is
/// set only by the captain; the original dream's switch 317 is never set.
pub(crate) fn repair_draco_briefing_pages(map_id: u32, map: &mut Map) {
    let (event_id, old_switch) = match map_id {
        125 => (23, 317),
        126 => (51, 404),
        _ => return,
    };
    let Some(event) = map.events.iter_mut().find(|event| event.id == event_id) else {
        return;
    };
    if let Some(page) = event.pages.iter_mut().find(|page| {
        page.condition.flags == 1
            && page.condition.switch_a == old_switch
            && page.trigger == 0
            && page.graphic_name.is_empty()
            && page.commands.iter().all(|command| command.code == 0)
    }) {
        page.condition.switch_a = 405;
    }
}
