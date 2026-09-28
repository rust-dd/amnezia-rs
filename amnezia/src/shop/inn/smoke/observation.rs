use super::*;
use crate::shop::inn::Completion;

pub(super) fn release_finished_input(
    probe: Option<Res<Probe>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    if probe
        .is_some_and(|probe| probe.step == Step::Rest && probe.handoffs & (1 << probe.case) != 0)
    {
        // A render may contain more logical updates after the terminal inn frame.
        keys.reset_all();
    }
}

pub(super) fn update(world: &mut World) {
    let Some(probe) = world.get_resource::<Probe>() else {
        return;
    };
    if probe.step != Step::Rest || probe.handoffs & (1 << probe.case) != 0 {
        return;
    }
    let phase = &world.resource::<State>().phase;
    fixtures::verify_vitals(world, matches!(phase, Phase::FadeIn | Phase::Idle));
    let finished = matches!(phase, Phase::Idle);
    let scene = world.resource::<SceneFrames>().frame;
    let raw = world.resource::<crate::timing::GameFrames>().frame;
    let callback = world.resource::<crate::timing::logical::Step>().callback;
    let paused = world.resource::<crate::timing::SceneWait>().0;
    let mut probe = world.resource_mut::<Probe>();
    if let Some((before_raw, before_scene)) = probe.rest_scene.replace((raw, scene)) {
        let delta = if callback || paused {
            0
        } else {
            raw.wrapping_sub(before_raw)
        };
        assert_eq!(
            scene,
            before_scene.wrapping_add(delta),
            "only inn transitions freeze scene time"
        );
        if callback {
            assert_eq!(raw, before_raw);
        }
        probe.rest_ticks += delta;
    }
    probe.callbacks += u32::from(callback);
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    let mut playing = false;
    let ended = world.resource::<State>().completed == Some(Completion::PlaybackStopped);
    for (settings, sink) in world.query::<(&PlaybackSettings, &AudioSink)>().iter(world) {
        if matches!(settings.mode, PlaybackMode::Once) {
            playing |= !sink.empty() && !sink.position().is_zero();
        }
    }
    let mut probe = world.resource_mut::<Probe>();
    probe.played |= playing;
    probe.ended |= ended;
    if finished {
        probe.handoffs |= 1 << probe.case;
        let case = probe.case;
        assert!(
            !world.resource::<RunningEvent>().active(),
            "event must finish in the callback before the fresh player update"
        );
        assert!(world.resource::<crate::state::Switches>().get(9031));
        assert_eq!(
            world.resource::<crate::state::Variables>().get(9032),
            case as i32 + 1
        );
        assert!(!world.resource::<ShopOpen>().0);
    }
}

#[cfg(test)]
mod tests;
