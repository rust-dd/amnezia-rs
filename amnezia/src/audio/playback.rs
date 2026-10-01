use super::*;

#[cfg(test)]
mod tests;

pub(super) fn play_requests(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut requests: MessageReader<AudioRequest>,
    mut current: ResMut<CurrentBgm>,
    mut memorized: ResMut<MemorizedBgm>,
    mut sinks: Query<&mut AudioSink>,
) {
    for request in requests.read() {
        match request {
            AudioRequest::Sound {
                name,
                volume,
                speed,
            } => {
                // RPG_RT opens no channel for a 0-volume SE; skip the spawn.
                if *volume <= 0.0 {
                    continue;
                }
                if let Some(path) = resolve_audio("Sound", name, &["wav"]) {
                    commands.spawn((
                        AudioPlayer::new(asset_server.load(path)),
                        PlaybackSettings::DESPAWN
                            .with_volume(Volume::Linear(*volume))
                            .with_speed(*speed),
                    ));
                }
            }
            AudioRequest::Bgm {
                name,
                volume,
                speed,
                fade_in,
            } => start_bgm(
                &mut commands,
                &asset_server,
                &mut current,
                &mut sinks,
                name,
                *volume,
                *speed,
                *fade_in,
                false,
            ),
            AudioRequest::BgmOnce(track) => {
                stop_bgm(&mut commands, &mut current);
                start_bgm(
                    &mut commands,
                    &asset_server,
                    &mut current,
                    &mut sinks,
                    &track.name,
                    track.volume,
                    track.speed,
                    track.fade_in,
                    true,
                );
            }
            AudioRequest::FadeOutBgm { duration } => current.start_fade_out(*duration),
            AudioRequest::StopBgm => stop_bgm(&mut commands, &mut current),
            AudioRequest::MemorizeBgm => memorized.0 = current.track(),
            AudioRequest::PlayMemorizedBgm => match memorized.0.clone() {
                Some(track) => start_bgm(
                    &mut commands,
                    &asset_server,
                    &mut current,
                    &mut sinks,
                    &track.name,
                    track.volume,
                    track.speed,
                    track.fade_in,
                    false,
                ),
                None => stop_bgm(&mut commands, &mut current),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BgmAction {
    /// Same track, same params: a seamless replay — leave it (and any fade) alone.
    Ignore,
    /// Same track, changed volume/tempo: update pending or live playback without restarting.
    UpdateParams,
    /// A new track (or one that is fading out): stop the old and start fresh.
    Restart,
}

/// Apply [`CurrentBgm::action_for`] without restarting same-track parameter changes.
#[allow(clippy::too_many_arguments)]
fn start_bgm(
    commands: &mut Commands,
    asset_server: &AssetServer,
    current: &mut CurrentBgm,
    sinks: &mut Query<&mut AudioSink>,
    name: &str,
    volume: f32,
    speed: f32,
    fade_in: f32,
    once: bool,
) {
    current.fade_in = fade_in;
    let action = current.action_for(name, volume, speed);
    eprintln!("[BGM] request '{name}' vol={volume:.3} fade_in={fade_in}s -> {action:?}");
    match action {
        BgmAction::Ignore => {}
        BgmAction::UpdateParams => {
            current.volume = volume;
            current.speed = speed;
            current.fade = None;
            if let Some(entity) = current.entity {
                commands
                    .entity(entity)
                    .entry::<PlaybackSettings>()
                    .and_modify(move |mut settings| {
                        settings.volume = Volume::Linear(volume);
                        settings.speed = speed;
                    });
                if let Ok(mut sink) = sinks.get_mut(entity) {
                    sink.set_volume(Volume::Linear(volume));
                    sink.set_speed(speed);
                }
            }
        }
        BgmAction::Restart => {
            if let Some(entity) = current.entity.take() {
                commands.entity(entity).despawn();
            }
            current.name = name.to_string();
            current.volume = volume;
            current.speed = speed;
            current.fade = None;
            let initial = if fade_in > 0.0 { 0.0 } else { volume };
            match resolve_audio("Music", name, &["ogg", "wav"]) {
                Some(path) => {
                    let entity = commands
                        .spawn((
                            AudioPlayer::new(asset_server.load(path)),
                            (if once {
                                PlaybackSettings::ONCE
                            } else {
                                PlaybackSettings::LOOP
                            })
                            .with_volume(Volume::Linear(initial))
                            .with_speed(speed),
                        ))
                        .id();
                    current.entity = Some(entity);
                    if fade_in > 0.0 {
                        current.fade = Some(BgmFade::fade_in(volume, fade_in));
                    }
                }
                None => {
                    eprintln!("[BGM] '{name}' has NO playable file (MIDI only) — silent");
                    debug!("bgm '{name}' has no playable audio (MIDI only); skipping");
                }
            }
        }
    }
}

/// Fade-out stops playback; fade-in retains the track at its target gain.
pub(super) fn drive_bgm_fade(
    time: Res<Time>,
    step: Option<Res<crate::timing::logical::Step>>,
    mut commands: Commands,
    mut current: ResMut<CurrentBgm>,
    mut sinks: Query<&mut AudioSink>,
) {
    if step.is_some_and(|step| step.callback) {
        return;
    }
    let Some((volume, finished, stop)) = current.fade.as_mut().map(|fade| {
        let volume = fade.advance(time.delta_secs());
        (volume, fade.finished(), fade.stop_at_end)
    }) else {
        return;
    };
    if let Some(entity) = current.entity {
        commands
            .entity(entity)
            .entry::<PlaybackSettings>()
            .and_modify(move |mut settings| {
                settings.volume = Volume::Linear(volume);
            });
        if let Ok(mut sink) = sinks.get_mut(entity) {
            sink.set_volume(Volume::Linear(volume));
        }
    }
    if finished {
        current.fade = None;
        if stop {
            stop_bgm(&mut commands, &mut current);
        }
    }
}

fn stop_bgm(commands: &mut Commands, current: &mut CurrentBgm) {
    if let Some(entity) = current.entity.take() {
        commands.entity(entity).despawn();
    }
    current.name.clear();
    current.fade_in = 0.0;
    current.fade = None;
}
