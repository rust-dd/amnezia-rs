use super::*;

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
    let expected = *world
        .resource_mut::<Probe>()
        .rest_scene
        .get_or_insert(scene);
    assert_eq!(
        scene, expected,
        "inn must not advance scene time during rest"
    );
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    let (mut playing, mut ended) = (false, false);
    for (settings, sink) in world.query::<(&PlaybackSettings, &AudioSink)>().iter(world) {
        if matches!(settings.mode, PlaybackMode::Once) {
            playing |= !sink.empty() && !sink.position().is_zero();
            ended |= sink.empty();
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
            "event must finish in the terminal fade frame"
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
