//! The inn's background music: play `system.inn_music` while the guest rests and
//! resume the map BGM on checkout. Kept out of [`super`] so the shop file stays
//! small; it watches the parent's private [`Screen`] state directly.

use crate::audio::{AudioRequest, BgmTrack, CurrentBgm, SystemMusic};
use bevy::prelude::*;

use super::Screen;

/// The map BGM playing when an inn rest begins, remembered so it resumes when the
/// guest checks out. Its own slot — not the interpreter's `MemorizeBGM` nor the
/// battle's — so an inn visit never clobbers an event- or fight-scoped memory.
#[derive(Resource, Default)]
struct InnBgm(Option<BgmTrack>);

/// Register the inn-music resource and driver on the shop plugin's app.
pub(super) fn register(app: &mut App) {
    app.init_resource::<InnBgm>()
        .add_systems(Update, drive_inn_music);
}

/// Play the inn's overnight jingle while the guest rests and resume the map BGM on
/// checkout. Watching the `done` flag (set the moment Yes is confirmed) keeps a
/// bare Yes/No cancel from touching the music: only an actual rest swaps it.
fn drive_inn_music(
    screen: Res<Screen>,
    music: Option<Res<SystemMusic>>,
    overrides: Option<Res<crate::system_bgm::SystemBgm>>,
    current_bgm: Res<CurrentBgm>,
    mut inn_bgm: ResMut<InnBgm>,
    mut audio: MessageWriter<AudioRequest>,
    mut resting: Local<bool>,
) {
    let now_resting = matches!(*screen, Screen::Inn { done: true, .. });
    if now_resting == *resting {
        return;
    }
    *resting = now_resting;
    let Some(music) = music else {
        return;
    };
    if now_resting {
        // Remember the map BGM and switch to the inn jingle for the sleep.
        inn_bgm.0 = current_bgm.track();
        audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
            overrides.as_deref(),
            2,
            &music.inn,
        )));
    } else {
        // Checked out: bring the map BGM back (silence if the map was silent).
        let resume = inn_bgm.0.take();
        audio.write(resume.map_or(AudioRequest::StopBgm, |track| track.replay()));
    }
}
