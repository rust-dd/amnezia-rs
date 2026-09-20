use bevy::prelude::*;
use std::sync::{Arc, Mutex};

const LABELS: [&str; 6] = [
    "ui-layer-message-base",
    "ui-layer-message-effect",
    "ui-layer-choice-base",
    "ui-layer-choice-effect",
    "ui-layer-number-base",
    "ui-layer-number-effect",
];
const RECT: (u32, u32, u32, u32) = (0, 160, 320, 80);
const COVER: [u8; 3] = [180, 48, 16];

#[derive(Component)]
struct EffectPlane;

#[derive(Default)]
struct Results {
    baseline: [Option<Vec<[u8; 4]>>; 3],
    checked: usize,
}

#[derive(Resource, Default)]
struct Captures(Arc<Mutex<Results>>);

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if matches!(frame, 320 | 620 | 920) {
        // Hold cursor and arrow phases while changing only the effect layer.
        world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    }
    if frame == 260 {
        world.init_resource::<Captures>();
        world
            .resource_mut::<crate::dialogue::MessageOptions>()
            .apply(&[0, 2, 0, 0]);
        *world.resource_mut::<crate::dialogue::MessagePosition>() =
            crate::dialogue::MessagePosition::Bottom;
        world
            .resource_mut::<crate::dialogue::Dialogue>()
            .open(vec![crate::events::MessageBox {
                face: None,
                face_index: 0,
                lines: vec!["Árvíztűrő.".into()],
            }]);
    }
    if frame == 560 {
        world
            .resource_mut::<crate::choice::Choice>()
            .open(vec!["Igen".into(), "Nem".into()], 0, 2);
    }
    if frame == 860 {
        world
            .resource_mut::<crate::inputnumber::InputNumber>()
            .open(3, 4502);
    }
    if matches!(frame, 360 | 660 | 960) {
        world.spawn((
            Sprite::from_color(
                Color::srgb_u8(COVER[0], COVER[1], COVER[2]),
                Vec2::new(320.0, 240.0),
            ),
            Transform::from_xyz(0.0, 0.0, 800.0),
            crate::animation::overlay_layer(),
            EffectPlane,
        ));
    }
    if matches!(frame, 420 | 720 | 1020) {
        world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
        world.resource_mut::<crate::dialogue::Dialogue>().close();
        world.resource_mut::<crate::choice::Choice>().active = false;
        world
            .resource_mut::<crate::inputnumber::InputNumber>()
            .active = false;
        let planes = world
            .query_filtered::<Entity, With<EffectPlane>>()
            .iter(world)
            .collect::<Vec<_>>();
        for entity in planes {
            world.despawn(entity);
        }
    }
    [320, 400, 620, 700, 920, 1000]
        .iter()
        .position(|&at| at == frame)
        .map(|index| LABELS[index])
}

pub(super) struct Snapshot {
    index: usize,
    results: Arc<Mutex<Results>>,
}

pub(super) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let index = LABELS.iter().position(|&value| value == label)?;
    assert!(match index / 2 {
        0 => world.resource::<crate::dialogue::Dialogue>().active,
        1 => world.resource::<crate::choice::Choice>().active,
        _ => world.resource::<crate::inputnumber::InputNumber>().active,
    });
    Some(Snapshot {
        index,
        results: world.resource::<Captures>().0.clone(),
    })
}

impl Snapshot {
    pub(super) fn verify(&self, image: &Image) {
        let (left, top, width, height) = RECT;
        let pixels = (top..top + height)
            .flat_map(|y| (left..left + width).map(move |x| (x, y)))
            .map(|(x, y)| crate::display::smoke::pixel_at(image, x, y))
            .collect::<Vec<_>>();
        let mut results = self.results.lock().unwrap();
        if self.index.is_multiple_of(2) {
            assert!(
                pixels.iter().any(|color| color[2] > color[0]),
                "the baseline samples must contain the actual blue window"
            );
            results.baseline[self.index / 2] = Some(pixels);
        } else {
            let outside = crate::display::smoke::pixel_at(image, 20, 20);
            assert_eq!(
                outside[..3],
                COVER,
                "the effect plane must remain visible outside the UI"
            );
            let baseline = results.baseline[self.index / 2]
                .as_ref()
                .expect("baseline capture completed first");
            for (index, (actual, expected)) in pixels.iter().zip(baseline).enumerate() {
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(&a, &b)| a.abs_diff(b) <= 1),
                    "{} sample {}: {actual:?}, expected {expected:?}",
                    LABELS[self.index],
                    index
                );
            }
            info!(
                "UI layer: {} unchanged window pixels above a visible effect verified",
                pixels.len()
            );
        }
        results.checked += 1;
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Captures>().0.lock().unwrap().checked,
        LABELS.len()
    );
}
