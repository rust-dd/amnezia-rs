use super::{Phase, State};
use crate::audio::{AudioRequest, BgmTrack, CurrentBgm, SystemSounds};
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::interpreter::RunningEvent;
use crate::shop::{ShopOpen, ShopOutcome};
use crate::state::{Inventory, Party};
use crate::timing::SceneFrames;
use crate::transitions::Transition;
use crate::vitals::Vitals;
use bevy::audio::{AudioSink, AudioSinkPlayback, PlaybackMode};
use bevy::ecs::message::MessageCursor;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

mod animation;
pub(crate) use animation::reference as animation_reference;
mod clock;
mod fixtures;
mod observation;
mod pixels;
pub(crate) use pixels::snapshot;

pub(crate) fn configure(app: &mut App) {
    crate::timing::logical::pre(app, || observation::release_finished_input);
    app.add_systems(
        Update,
        observation::update.after(crate::interpreter::InterpreterStep),
    );
}

pub(crate) fn entry() -> Vec<amnezia_data::EventCommand> {
    vec![amnezia_data::EventCommand {
        code: 10810,
        indent: 0,
        string: String::new(),
        params: vec![52, 13, 6],
    }]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Gap,
    Typing,
    Ready,
    Rest,
    Exit,
    Done,
}

#[derive(Resource)]
struct Probe {
    step: Step,
    case: usize,
    age: u32,
    black: Entity,
    before: Option<BgmTrack>,
    memorized: Option<BgmTrack>,
    audio: MessageCursor<AudioRequest>,
    rest_scene: Option<(u32, u32)>,
    rest_ticks: u32,
    callbacks: u32,
    handoffs: u8,
    played: bool,
    ended: bool,
    asleep: bool,
    checked: u32,
    animation: u16,
    pixels: Arc<AtomicUsize>,
    clock: clock::Clock,
}

impl Probe {
    fn new(black: Entity, audio: MessageCursor<AudioRequest>) -> Self {
        Self {
            step: Step::Gap,
            case: 0,
            age: 0,
            black,
            before: None,
            memorized: None,
            audio,
            rest_scene: None,
            rest_ticks: 0,
            callbacks: 0,
            handoffs: 0,
            played: false,
            ended: false,
            asleep: false,
            checked: 0,
            animation: 0,
            pixels: Arc::default(),
            clock: default(),
        }
    }
}

pub(crate) fn input(world: &mut World, frame: u32) -> bool {
    if frame < 300 {
        return false;
    }
    let Some(probe) = world.get_resource::<Probe>() else {
        return true;
    };
    let pressed = match (probe.step, probe.age, probe.case) {
        (Step::Typing, 1 | 2, _) => vec![KeyCode::Enter],
        (Step::Ready, 20, 0 | 2 | 3) => vec![KeyCode::Enter],
        (Step::Ready, 20, 1) => vec![KeyCode::ArrowDown, KeyCode::Enter],
        (Step::Ready, 22, 0) | (Step::Ready, 20, 5) => vec![KeyCode::Escape],
        (Step::Rest, _, _) => vec![KeyCode::Escape, KeyCode::ArrowDown, KeyCode::Enter],
        _ => Vec::new(),
    };
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    for key in pressed {
        keys.press(key);
    }
    true
}

fn music(world: &mut World) -> crate::audio::saved::MusicState {
    world
        .run_system_once(|capture: crate::audio::saved::Capture| capture.snapshot().unwrap())
        .unwrap()
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 300 {
        assert_eq!(world.resource::<crate::world::MapData>().map_id, 52);
        let black = world
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                GlobalZIndex(99),
            ))
            .id();
        let mut audio = MessageCursor::default();
        audio.clear(world.resource::<Messages<AudioRequest>>());
        world.insert_resource(Probe::new(black, audio));
        world.write_message(AudioRequest::play_bgm("Elven", &[125, 70, 120, 50]));
        world.write_message(AudioRequest::MemorizeBgm);
        world.write_message(AudioRequest::play_bgm("House", &[250, 80, 100, 50]));
        return None;
    }
    if frame < 300 {
        return None;
    }
    let (step, age, index, time) = {
        let mut probe = world.resource_mut::<Probe>();
        let step = probe.step;
        let time = probe
            .clock
            .step(std::time::Instant::now(), step == Step::Rest);
        let state = (step, probe.age, probe.case, time);
        probe.age += 1;
        state
    };
    world.insert_resource(time);
    verify_sounds(world, step, age, index);
    let animation = animation::capture(world, step, index);
    match step {
        Step::Gap if age == 12 => {
            let saved = music(world);
            let mut probe = world.resource_mut::<Probe>();
            probe.before = if index == 4 { None } else { saved.current };
            probe.memorized = saved.memorized;
            probe.rest_scene = None;
            fixtures::prepare(world, index);
            move_to(
                world,
                if fixtures::case(index).cost == 0 {
                    Step::Rest
                } else {
                    Step::Typing
                },
            );
        }
        Step::Typing => {
            assert!(age < 180, "inn question never finished typing");
            if world.resource::<Choice>().active()
                && world.resource::<Dialogue>().prompt_input_ready()
            {
                assert_eq!(
                    world.resource::<Choice>().disabled,
                    if index == 0 { vec![0] } else { vec![] }
                );
                move_to(world, Step::Ready);
            }
        }
        Step::Ready => {
            if age == 1 || age == 12 {
                return Some(fixtures::LABELS[index][usize::from(age == 12)]);
            }
            if index == 2 && age == 15 {
                let black = world.resource::<Probe>().black;
                *world.get_mut::<Visibility>(black).unwrap() = Visibility::Hidden;
            }
            if index == 2 && age == 18 {
                return Some("inn-original-map");
            }
            if index == 0 && age == 20 {
                assert!(world.resource::<Choice>().active());
                assert!(!world.resource::<ShopOpen>().0);
                assert!(world.resource::<State>().prompting());
                assert_eq!(world.resource::<Inventory>().gold(), 199);
                fixtures::verify_vitals(world, false);
            }
            if age == if index == 0 { 22 } else { 20 } {
                if fixtures::case(index).stay {
                    let black = world.resource::<Probe>().black;
                    *world.get_mut::<Visibility>(black).unwrap() = Visibility::Hidden;
                    move_to(world, Step::Rest);
                } else {
                    move_to(world, Step::Exit);
                }
            }
        }
        Step::Rest => {
            if matches!(world.resource::<State>().phase, Phase::Closing) {
                assert!(!world.resource::<Transition>().busy());
                assert!(!world.resource::<ShopOpen>().0);
                assert!(world.resource::<RunningEvent>().active());
                fixtures::verify_vitals(world, false);
                return None;
            }
            assert!(!world.resource::<crate::menu::MenuOpen>().0);
            let inn = world.resource::<State>();
            let healed = matches!(inn.phase, Phase::FadeIn | Phase::Idle);
            fixtures::verify_vitals(world, healed);
            assert_eq!(world.resource::<ShopOutcome>().transacted, healed);
            if matches!(world.resource::<State>().phase, Phase::Resting { .. }) {
                assert!(world.resource::<Transition>().erased());
                if !world.resource::<Probe>().asleep && world.resource::<Probe>().played {
                    world.resource_mut::<Probe>().asleep = true;
                    return Some("inn-asleep");
                }
            }
            if !world.resource::<ShopOpen>().0 {
                assert_ne!(world.resource::<Probe>().handoffs & (1 << index), 0);
                assert!(!world.resource::<Transition>().erased());
                assert!(!world.resource::<Transition>().event_erased);
                assert_eq!(
                    world.resource::<CurrentBgm>().track(),
                    world.resource::<Probe>().before
                );
                assert_eq!(music(world).memorized, world.resource::<Probe>().memorized);
                move_to(world, Step::Exit);
                return Some(match index {
                    2 => "inn-paid-return",
                    3 => "inn-silent-return",
                    _ => "inn-free-return",
                });
            }
        }
        Step::Exit if age >= 3 && !world.resource::<Dialogue>().busy() => {
            assert!(!world.resource::<RunningEvent>().active());
            let case = fixtures::case(index);
            assert_eq!(
                world.resource::<crate::state::Switches>().get(9031),
                case.stay
            );
            assert_eq!(
                world.resource::<crate::state::Variables>().get(9032),
                index as i32 + 1
            );
            assert_eq!(
                world.resource::<Inventory>().gold(),
                case.gold - if case.stay { case.cost } else { 0 }
            );
            fixtures::verify_vitals(world, case.stay);
            assert_eq!(music(world).memorized, world.resource::<Probe>().memorized);
            let black = world.resource::<Probe>().black;
            *world.get_mut::<Visibility>(black).unwrap() = Visibility::Visible;
            world.resource_mut::<Probe>().case += 1;
            if index + 1 == fixtures::COUNT {
                world.entity_mut(black).despawn();
                move_to(world, Step::Done);
                return Some("inn-final");
            }
            move_to(world, Step::Gap);
        }
        _ => {}
    }
    world.resource_mut::<Probe>().checked += 1;
    animation
}

fn move_to(world: &mut World, step: Step) {
    let mut probe = world.resource_mut::<Probe>();
    probe.step = step;
    probe.age = 0;
}

fn verify_sounds(world: &mut World, step: Step, age: u32, index: usize) {
    let sounds = world.resource::<SystemSounds>();
    let expected = match (step, age, index) {
        (Step::Ready, 20, 0) => vec![&sounds.buzzer],
        (Step::Ready, 20, 1) => vec![&sounds.cursor, &sounds.decision],
        (Step::Ready, 20, 2 | 3) => vec![&sounds.decision],
        (Step::Ready, 22, 0) | (Step::Ready, 20, 5) => vec![&sounds.cancel],
        _ => vec![],
    }
    .into_iter()
    .filter_map(|sound| AudioRequest::se(&sound.name, sound.volume, sound.tempo))
    .collect::<Vec<_>>();
    world.resource_scope(|world, mut probe: Mut<Probe>| {
        let actual = probe
            .audio
            .read(world.resource::<Messages<AudioRequest>>())
            .filter(|request| matches!(request, AudioRequest::Sound { .. }))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "inn audio, case {index}, age {age}");
    });
}

pub(crate) fn finished(world: &World) -> bool {
    world
        .get_resource::<Probe>()
        .is_some_and(|probe| probe.step == Step::Done)
}

pub(crate) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(finished(world));
    assert!(
        probe.played && probe.ended,
        "Inn must decode, play and reach its natural end, not the timeout"
    );
    assert_eq!(probe.handoffs, (1 << 2) | (1 << 3) | (1 << 4));
    assert_eq!(probe.callbacks, 6);
    assert!(probe.rest_ticks > 0);
    assert_eq!(probe.animation, (1 << 14) - 1);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 25);
    assert_eq!(world.resource::<crate::state::Variables>().get(9032), 6);
    info!(
        "inn: six original/compatibility cases, {} state checks, 1920000 reference pixels including fourteen animated frames, one decoded jingle, six exact branch handoffs, {} callback visits and {} resting scene ticks",
        probe.checked, probe.callbacks, probe.rest_ticks
    );
}
