use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Case {
    label: &'static str,
    kind: Kind,
    flashes: bool,
    erase: bool,
    from_erased: bool,
    frame: u32,
    center: IVec2,
}

const CASES: [Case; 12] = [
    Case {
        label: "transition-fade-out",
        flashes: false,
        kind: Kind::Fade,
        erase: true,
        from_erased: false,
        frame: 16,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-fade-in",
        flashes: false,
        kind: Kind::Fade,
        erase: false,
        from_erased: true,
        frame: 16,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-crossfade",
        flashes: false,
        kind: Kind::Fade,
        erase: false,
        from_erased: false,
        frame: 16,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-mosaic-out",
        flashes: false,
        kind: Kind::Mosaic,
        erase: true,
        from_erased: false,
        frame: 20,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-mosaic-in",
        flashes: false,
        kind: Kind::Mosaic,
        erase: false,
        from_erased: true,
        frame: 12,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-zoom-out",
        flashes: false,
        kind: Kind::Zoom,
        erase: true,
        from_erased: false,
        frame: 20,
        center: IVec2::new(16, 200),
    },
    Case {
        label: "transition-zoom-in",
        flashes: false,
        kind: Kind::Zoom,
        erase: false,
        from_erased: true,
        frame: 0,
        center: IVec2::new(300, 16),
    },
    Case {
        label: "transition-cut-out",
        flashes: false,
        kind: Kind::Cut,
        erase: true,
        from_erased: false,
        frame: 0,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-cut-in",
        flashes: false,
        kind: Kind::Cut,
        erase: false,
        from_erased: true,
        frame: 0,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-battle-flash-peak",
        kind: Kind::Zoom,
        flashes: true,
        erase: true,
        from_erased: false,
        frame: 0,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-battle-flash-decay",
        kind: Kind::Zoom,
        flashes: true,
        erase: true,
        from_erased: false,
        frame: 15,
        center: IVec2::ZERO,
    },
    Case {
        label: "transition-battle-zoom",
        kind: Kind::Zoom,
        flashes: true,
        erase: true,
        from_erased: false,
        frame: 40,
        center: IVec2::new(16, 200),
    },
];

#[derive(Resource)]
struct Fixture {
    sprite: Entity,
    images: [Handle<Image>; 2],
    complete: Arc<AtomicUsize>,
}

pub(crate) struct Snapshot {
    index: usize,
    complete: Arc<AtomicUsize>,
}

fn color(x: u32, y: u32, variant: usize) -> [u8; 3] {
    if (50..70).contains(&x) && (50..60).contains(&y) {
        return [220, 50, 90];
    }
    let first = [x as u8, y as u8, (x + y) as u8];
    if variant == 0 {
        first
    } else {
        first.map(|channel| 255 - channel)
    }
}

fn setup(world: &mut World) {
    world.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    let images = [0, 1].map(|variant| {
        let pixels = (0..240)
            .flat_map(|y| {
                (0..320).flat_map(move |x| {
                    let [r, g, b] = if variant == 0 {
                        [x as u8, y as u8, (x + y) as u8]
                    } else {
                        [255 - x as u8, 255 - y as u8, 255 - (x + y) as u8]
                    };
                    [r, g, b, 255]
                })
            })
            .collect::<Vec<_>>();
        world.resource_mut::<Assets<Image>>().add(Image::new(
            Extent3d {
                width: 320,
                height: 240,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
    });
    let sprite = world
        .spawn((
            Sprite::from_image(images[0].clone()),
            Transform::from_xyz(0.0, 0.0, 900.0),
            crate::animation::overlay_layer(),
        ))
        .id();
    let camera = world
        .query_filtered::<Entity, With<crate::battle::HudCamera>>()
        .single(world)
        .unwrap();
    world.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(150.0),
            top: Val::Px(150.0),
            width: Val::Px(60.0),
            height: Val::Px(30.0),
            ..default()
        },
        BackgroundColor(Color::srgb_u8(220, 50, 90)),
        UiTargetCamera(camera),
        GlobalZIndex(1100),
    ));
    world.insert_resource(Fixture {
        sprite,
        images,
        complete: Arc::new(AtomicUsize::new(0)),
    });
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 240 {
        setup(world);
    }
    for (index, case) in CASES.iter().enumerate() {
        let start = 260 + index as u32 * 80;
        if frame == start - 10 {
            world.resource_mut::<Transition>().clear();
            world.resource_mut::<crate::timing::GameFrames>().frame = 0;
            let fixture = world.resource::<Fixture>();
            let (sprite, image) = (fixture.sprite, fixture.images[0].clone());
            world.get_mut::<Sprite>(sprite).unwrap().image = image;
        }
        if frame == start {
            let fixture = world.resource::<Fixture>();
            let (sprite, image) = (
                fixture.sprite,
                fixture.images[usize::from(!case.erase)].clone(),
            );
            world.get_mut::<Sprite>(sprite).unwrap().image = image;
            let mut state = world.resource_mut::<Transition>();
            state.erased = case.from_erased;
            assert!(state.start(case.kind, case.erase, 0, case.center));
            if case.flashes {
                state.prepend_battle_flashes();
            }
            if case.kind == Kind::Mosaic {
                state.effect.as_mut().unwrap().offsets = (0..41).map(|i| i * 7 % (i + 1)).collect();
            }
        }
        if frame == start + 10 {
            let state = world.resource::<Transition>();
            assert!(world.resource::<snapshots::Capture>().ready(state.serial));
            world.resource_mut::<crate::timing::GameFrames>().frame = case.frame;
            let fixture = world.resource::<Fixture>();
            let (sprite, image) = (
                fixture.sprite,
                fixture.images[usize::from(case.erase)].clone(),
            );
            world.get_mut::<Sprite>(sprite).unwrap().image = image;
        }
        if frame == start + 30 {
            return Some(case.label);
        }
    }
    None
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let index = CASES.iter().position(|case| case.label == label)?;
    Some(Snapshot {
        index,
        complete: world.resource::<Fixture>().complete.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        let case = &CASES[self.index];
        let crop = match case.label {
            "transition-zoom-out" | "transition-battle-zoom" => [0, 110, 160, 120],
            "transition-zoom-in" => [294, 14, 8, 6],
            _ => [0, 0, 320, 240],
        };
        for y in 0..240 {
            for x in 0..320 {
                let (sx, sy) = if case.kind == Kind::Mosaic {
                    let step = if case.erase {
                        case.frame
                    } else {
                        40 - case.frame
                    };
                    let size = step as i32 + 1;
                    let offset = (step * 7 % (step + 1)) as i32;
                    let sample = |pixel: i32| {
                        ((pixel + offset + size / 2) / size * size - size / 2).max(0) as u32
                    };
                    (sample(x as i32).min(319), sample(y as i32).min(239))
                } else if case.kind == Kind::Zoom {
                    let sample = |pixel: u32, origin: i32, length: i32, dest: i32| {
                        let scale = ((length as f64 / dest as f64) * 65536.0).trunc() / 65536.0;
                        let offset = (origin as f64 * dest as f64 / length as f64).trunc();
                        (((offset + pixel as f64 + 0.5) * scale).ceil() - 1.0).max(0.0) as u32
                    };
                    (
                        sample(x, crop[0], crop[2], 320).min(319),
                        sample(y, crop[1], crop[3], 240).min(239),
                    )
                } else {
                    (x, y)
                };
                let expected = if case.flashes && case.frame < 20 {
                    let alpha = if case.frame == 0 { 248 } else { 123 };
                    color(x, y, 0).map(|channel| {
                        ((channel as u32 * (255 - alpha) + 248 * alpha + 127) / 255) as u8
                    })
                } else if case.kind == Kind::Fade {
                    let first = if case.from_erased {
                        [0; 3]
                    } else {
                        color(x, y, 0)
                    };
                    let second = if case.erase { [0; 3] } else { color(x, y, 1) };
                    std::array::from_fn(|i| {
                        ((first[i] as u32 * 124 + second[i] as u32 * 131 + 127) / 255) as u8
                    })
                } else {
                    color(sx, sy, usize::from(!case.erase))
                };
                let actual = image
                    .get_color_at(
                        (2 * x + 1) * image.width() / 640,
                        (2 * y + 1) * image.height() / 480,
                    )
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                assert!(
                    actual[..3]
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| a.abs_diff(b) <= 1),
                    "{} ({x},{y}): {actual:?}, expected {expected:?}",
                    case.label
                );
            }
        }
        self.complete.fetch_add(1, Ordering::SeqCst);
        info!(
            "{}: all 76800 transition pixels match the frozen scene reference",
            case.label
        );
    }
}

pub(crate) fn verify_finished(world: &World) {
    assert_eq!(
        world.resource::<Fixture>().complete.load(Ordering::SeqCst),
        CASES.len()
    );
}
