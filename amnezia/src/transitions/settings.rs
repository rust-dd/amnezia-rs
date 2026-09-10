use super::{Kind, Transition};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Resource)]
pub(crate) struct Defaults(pub [i32; 6]);

impl Default for Defaults {
    fn default() -> Self {
        let system = crate::assets::load_ron::<amnezia_data::SystemDef>(&format!(
            "{}/system.ron",
            crate::assets::asset_root()
        ));
        Self(system.transitions)
    }
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Settings(BTreeMap<usize, i32>);

impl Settings {
    pub(crate) fn change(&mut self, params: &[i32], defaults: &Defaults) {
        let [slot, value, ..] = params else {
            return;
        };
        let slot = *slot as usize;
        let Some(default) = defaults.0.get(slot) else {
            return;
        };
        if value < &0 || value == default {
            self.0.remove(&slot);
        } else {
            self.0.insert(slot, *value);
        }
    }

    pub(crate) fn get(&self, slot: usize, defaults: &Defaults) -> Kind {
        Kind::from_id(
            self.0
                .get(&slot)
                .copied()
                .unwrap_or(defaults.0[slot])
                .clamp(0, 20),
        )
    }
}

#[derive(SystemParam)]
pub(crate) struct TransitionIo<'w> {
    pub state: ResMut<'w, Transition>,
    pub settings: ResMut<'w, Settings>,
    pub defaults: Res<'w, Defaults>,
    pub frames: Res<'w, crate::timing::GameFrames>,
}

impl TransitionIo<'_> {
    pub(crate) fn kind(&self, slot: usize) -> Kind {
        self.settings.get(slot, &self.defaults)
    }

    pub(crate) fn event(&mut self, id: i32, erase: bool, center: IVec2) -> bool {
        let kind = if id == -1 {
            self.kind(usize::from(!erase))
        } else {
            Kind::from_id(id)
        };
        let (kind, duration) = if erase && kind == Kind::None {
            (Kind::Cut, 30)
        } else {
            (kind, kind.frames())
        };
        if !self
            .state
            .start_for(kind, erase, self.frames.frame, center, duration)
        {
            return false;
        }
        self.state.event_erased = erase;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_transition_overrides_keep_database_fallbacks_and_round_trip() {
        let defaults = Defaults([0, 0, 16, 17, 17, 16]);
        let mut settings = Settings::default();
        for (slot, kind) in [
            Kind::Fade,
            Kind::Fade,
            Kind::Zoom,
            Kind::Mosaic,
            Kind::Mosaic,
            Kind::Zoom,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(settings.get(slot, &defaults), kind);
            settings.change(&[slot as i32, 19], &defaults);
            assert_eq!(settings.get(slot, &defaults), Kind::Cut);
        }
        settings.change(&[0, 20], &defaults);
        settings.change(&[1, 0], &defaults);
        settings.change(&[2, -1], &defaults);
        settings.change(&[3, 99], &defaults);
        settings.change(&[-1, 17], &defaults);
        settings.change(&[6, 17], &defaults);
        assert_eq!(settings.get(0, &defaults), Kind::None);
        assert_eq!(settings.get(1, &defaults), Kind::Fade);
        assert_eq!(settings.get(2, &defaults), Kind::Zoom);
        assert_eq!(settings.get(3, &defaults), Kind::None);
        assert_eq!(settings.0.len(), 4);
        assert_eq!(
            ron::from_str::<Settings>(&ron::to_string(&settings).unwrap()).unwrap(),
            settings
        );
    }
}
