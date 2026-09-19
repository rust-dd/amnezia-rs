use crate::animation::AnimationLibrary;
use crate::audio::{AudioRequest, SystemSounds, play_system_se};
use crate::gamedata::GameData;
use amnezia_data::SoundDef;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(in crate::menu) struct MenuSfx<'w> {
    audio: MessageWriter<'w, AudioRequest>,
    sounds: Option<Res<'w, SystemSounds>>,
    animations: Option<Res<'w, AnimationLibrary>>,
}

impl MenuSfx<'_> {
    pub(super) fn play(&mut self, pick: impl FnOnce(&SystemSounds) -> &SoundDef) {
        if let Some(sounds) = &self.sounds {
            play_system_se(&mut self.audio, pick(sounds));
        }
    }

    pub(super) fn cursor(&mut self) {
        self.play(|sounds| &sounds.cursor);
    }

    pub(super) fn decision(&mut self) {
        self.play(|sounds| &sounds.decision);
    }

    pub(super) fn cancel(&mut self) {
        self.play(|sounds| &sounds.cancel);
    }

    pub(super) fn buzzer(&mut self) {
        self.play(|sounds| &sounds.buzzer);
    }

    pub(super) fn skill(&mut self, skill_id: u32, data: &GameData) {
        let Some(skill) = data.skills.iter().find(|skill| skill.id == skill_id) else {
            return;
        };
        let Some(library) = &self.animations else {
            return;
        };
        let Some(animation) = library
            .0
            .iter()
            .find(|animation| animation.id == skill.animation_id)
        else {
            return;
        };
        let Some(timing) = animation
            .timings
            .iter()
            .find(|timing| !timing.se_name.is_empty() && timing.se_name != "(OFF)")
        else {
            return;
        };
        // Field casts use only the first sound, even when its timeline starts later.
        if timing.se_volume > 0
            && let Some(request) =
                AudioRequest::se(&timing.se_name, timing.se_volume, timing.se_tempo)
        {
            self.audio.write(request);
        }
    }
}
