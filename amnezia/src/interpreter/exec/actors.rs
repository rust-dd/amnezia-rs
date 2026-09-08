use super::super::commands::{actor_targets, operate_value};
use super::super::frame::Frame;
use super::super::opcodes::{CHANGE_CONDITION, CHANGE_EQUIPMENT, CHANGE_SKILLS, FULL_HEAL};
use super::{Exec, Flow};
use amnezia_data::EventCommand;

pub(super) fn execute(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    if let [mode, actor, ..] = command.params.as_slice() {
        for id in actor_targets(*mode, *actor, &x.variables, &x.party) {
            let edits = &mut x.subsystems.actor_edits;
            let Some(def) = edits.game_data.actor(id) else {
                continue;
            };
            match (command.code, command.params.as_slice()) {
                (CHANGE_CONDITION, [_, _, remove, state, ..]) => {
                    let level = edits.progression.level(def).max(1) as usize - 1;
                    let full = (
                        def.curves.max_hp.get(level).copied().unwrap_or(def.hp) as i32,
                        def.curves.max_sp.get(level).copied().unwrap_or(def.sp) as i32,
                    );
                    x.subsystems.vitals.change_condition(
                        id,
                        (*state).max(0) as u32,
                        *remove == 0,
                        full,
                    );
                }
                (CHANGE_SKILLS, [_, _, remove, operand_type, operand, ..]) => {
                    let skill = operate_value(0, *operand_type, *operand, &x.variables);
                    if skill > 0 && edits.game_data.skills.iter().any(|s| s.id == skill as u32) {
                        edits
                            .progression
                            .change_skill(def, skill as u32, *remove == 0);
                    }
                }
                (CHANGE_EQUIPMENT, [_, _, operation, operand_type, operand, ..]) => {
                    let (slot, item_id) = match operation {
                        0 => {
                            let item_id = operate_value(0, *operand_type, *operand, &x.variables);
                            let Some(item) = edits.game_data.item(item_id as u32) else {
                                continue;
                            };
                            if !(1..=5).contains(&item.item_type)
                                || (def.two_weapons && item.item_type == 2)
                            {
                                continue;
                            }
                            let slots = edits.equipment.slots(def);
                            let second_weapon = def.two_weapons
                                && item.item_type == 1
                                && !item.two_handed
                                && slots[1] == 0
                                && edits
                                    .game_data
                                    .item(slots[0])
                                    .is_some_and(|w| !w.two_handed);
                            let slot = if second_weapon {
                                1
                            } else {
                                item.item_type as usize - 1
                            };
                            (slot, item.id)
                        }
                        1 => (*operand_type as usize, 0),
                        _ => continue,
                    };
                    let slots = if slot == 5 { 0..5 } else { slot..slot + 1 };
                    for slot in slots {
                        edits.equipment.equip_from_event(
                            def,
                            slot,
                            item_id,
                            &edits.game_data.items,
                            &mut x.inventory,
                        );
                    }
                }
                (FULL_HEAL, _) => x.subsystems.vitals.heal(id),
                _ => {}
            }
        }
    }
    frame.ip += 1;
    Flow::Advance
}
