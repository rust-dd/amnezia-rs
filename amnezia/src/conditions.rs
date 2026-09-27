use crate::assets::{asset_root, load_ron};
use crate::gamedata::GameData;
use crate::progression::Progression;
use crate::state::Party;
use crate::vitals::Vitals;
use amnezia_data::StateDef;
use bevy::prelude::*;
use std::sync::OnceLock;

#[cfg(test)]
mod movement_tests;

pub fn definitions() -> &'static [StateDef] {
    static STATES: OnceLock<Vec<StateDef>> = OnceLock::new();
    STATES.get_or_init(|| load_ron(&format!("{}/states.ron", asset_root())))
}

pub fn names(vitals: &Vitals, actor_id: u32) -> String {
    vitals
        .states(actor_id)
        .into_iter()
        .map(|id| {
            if id == 1 {
                "Ájult".to_string()
            } else {
                definitions()
                    .iter()
                    .find(|s| s.id == id)
                    .map_or_else(|| format!("#{id}"), |s| crate::i18n::tr(&s.name))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Resource, Default)]
pub struct FieldSteps {
    pub count: u64,
    pending: bool,
}

impl FieldSteps {
    pub(crate) fn record(&mut self) {
        self.pending = true;
    }
}

pub struct ConditionsPlugin;

impl Plugin for ConditionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldSteps>()
            .add_message::<crate::screenfx::ScreenEffect>();
        crate::player::update::character(app, || {
            step.after(crate::player::PlayerInput)
                .after(crate::vehicles::VehicleInput)
                .before(crate::player::PlayerStep)
                .before(crate::dialogue::MessageUpdate)
        });
    }
}

fn step(
    actors: Res<GameData>,
    progression: Res<Progression>,
    party: Res<Party>,
    mut steps: ResMut<FieldSteps>,
    mut vitals: ResMut<Vitals>,
    mut effects: MessageWriter<crate::screenfx::ScreenEffect>,
) {
    if !std::mem::take(&mut steps.pending) {
        return;
    }
    steps.count = steps.count.wrapping_add(1);
    let mut damaged = false;
    for id in party.snapshot() {
        let Some(actor) = actors.actor(id) else {
            continue;
        };
        let level = progression.level(actor).max(1) as usize - 1;
        let full = (
            actor.curves.max_hp.get(level).copied().unwrap_or(actor.hp) as i32,
            actor.curves.max_sp.get(level).copied().unwrap_or(actor.sp) as i32,
        );
        damaged |= apply_step(&mut vitals, id, steps.count, full);
    }
    if damaged {
        effects.write(crate::screenfx::ScreenEffect::Flash {
            r: 31,
            g: 10,
            b: 10,
            intensity: 20,
            secs: 0.1,
        });
    }
}

fn apply_step(vitals: &mut Vitals, actor: u32, step: u64, full: (i32, i32)) -> bool {
    let (mut hp, sp) = vitals.get_stored(actor).unwrap_or(full);
    if hp <= 0 {
        return false;
    }
    let mut changed = false;
    let mut damaged = false;
    for state_id in vitals.states(actor) {
        let Some(state) = definitions().iter().find(|s| s.id == state_id) else {
            continue;
        };
        if state.hp_change_map_steps == 0
            || state.hp_change_map_val == 0
            || !step.is_multiple_of(state.hp_change_map_steps as u64)
        {
            continue;
        }
        let amount = state.hp_change_map_val as i32;
        match state.hp_change_type {
            0 => {
                hp = hp.saturating_sub(amount).clamp(1, full.0.max(1));
                changed = true;
                damaged = true;
            }
            1 => {
                hp = hp.saturating_add(amount).clamp(1, full.0.max(1));
                changed = true;
            }
            _ => {}
        }
    }
    if changed {
        vitals.set(actor, hp, sp);
    }
    damaged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_poison_drains_every_four_steps_but_never_kills_on_the_map() {
        let mut vitals = Vitals::default();
        vitals.set(1, 2, 5);
        vitals.set_states(1, vec![2]);
        apply_step(&mut vitals, 1, 3, (50, 10));
        assert_eq!(vitals.get_stored(1), Some((2, 5)));
        apply_step(&mut vitals, 1, 4, (50, 10));
        assert_eq!(vitals.get_stored(1), Some((1, 5)));
        apply_step(&mut vitals, 1, 8, (50, 10));
        assert_eq!(vitals.get_stored(1), Some((1, 5)));
        assert_eq!(names(&vitals, 1), "Méreg");
    }
}
