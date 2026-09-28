use super::*;
use crate::animation::saved as animation;
use crate::screenfx::saved as screen;
use crate::transitions::Transition;

#[derive(Resource, Default)]
struct Probe {
    held: u32,
    screen: Option<screen::ScreenState>,
    shown: bool,
    captured: bool,
    pixels: Arc<AtomicUsize>,
}

pub(crate) fn configure(app: &mut App) {
    crate::timing::logical::post(app, || observe.after(crate::teleport::TransferCommit));
}

fn named(code: u32, name: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        string: name.into(),
        ..command(code, 1, params)
    }
}

fn prepare(world: &mut World) {
    assert!(!world.resource::<RunningEvent>().active());
    assert!(!world.resource::<Dialogue>().busy());
    assert_eq!(world.resource::<MapData>().map_id, 13);
    assert_eq!(
        world
            .query::<&crate::player::Player>()
            .single(world)
            .unwrap()
            .tile_x,
        60
    );
    world.insert_resource(Probe::default());
    world.resource_mut::<Variables>().set(4916, 0);
    world.resource_mut::<CommonEvents>().0.push(CommonEvent {
        id: 904,
        name: "Destination presentation probe".into(),
        trigger: 4,
        switch_flag: false,
        switch_id: 0,
        commands: vec![
            command(10220, 0, vec![0, 4915, 4915, 0, 6, 10001, 1]),
            command(12010, 0, vec![1, 4915, 0, 62, 0]),
            named(
                11110,
                "Cross",
                vec![49, 0, 96, 60, 0, 800, 0, 1, 100, 100, 100, 100, 0, 0],
            ),
            command(11210, 1, vec![62, 10001, 0, 0]),
            command(11030, 1, vec![70, 90, 110, 50, 0, 0]),
            command(11040, 1, vec![31, 10, 5, 20, 100, 0]),
            named(11720, "Morning1", vec![1, 1, 1, -1, 1, 1]),
            named(11510, "(OFF)", vec![0, 100, 100, 50]),
            command(10220, 1, vec![0, 4916, 4916, 0, 0, 1]),
            command(11410, 1, vec![1000]),
            command(22011, 0, vec![]),
        ],
    });
    world
        .resource_mut::<crate::transitions::Settings>()
        .change(&[1, 0], &crate::transitions::Defaults([0; 6]));
    world.resource_mut::<Transition>().hold_black();
    world
        .resource_mut::<crate::teleport::PendingTeleport>()
        .reserve((13, 62, 60), false);
}

fn observe(world: &mut World) {
    if !world.contains_resource::<Probe>() || world.resource::<Probe>().shown {
        return;
    }
    if world.resource::<Variables>().get(4916) == 0 {
        return;
    }
    let pictures = world
        .run_system_cached(|capture: crate::picture::saved::Capture| capture.snapshot())
        .unwrap();
    let picture = pictures.iter().find(|picture| picture.id == 49).unwrap();
    assert_eq!(
        (picture.visual.x, picture.visual.y, picture.visual.zoom),
        (96.0, 60.0, 800.0)
    );
    assert!(
        world
            .resource::<crate::audio::CurrentBgm>()
            .track()
            .is_none()
    );
    let image = world
        .resource::<crate::panorama::BackgroundImage>()
        .image()
        .unwrap();
    assert_eq!(
        image.path().unwrap().path().file_name().unwrap(),
        "Morning1.png"
    );
    if world.resource::<crate::teleport::Fade>().busy() {
        assert!(
            world.resource::<Transition>().busy()
                || crate::timing::logical::callback_pending(world)
        );
        assert_eq!(animation::snapshot(world).cast.unwrap().elapsed, 0);
        let current = screen::snapshot(world);
        let mut probe = world.resource_mut::<Probe>();
        if let Some(before) = &probe.screen {
            assert_eq!(&current, before);
        } else {
            assert_eq!(current.tone.tone(), [70.0, 90.0, 110.0, 50.0]);
            probe.screen = Some(current);
        }
        probe.held += 1;
    } else {
        assert!(!world.resource::<Transition>().busy());
        assert!(world.resource::<Assets<Image>>().contains(image));
        world.resource_mut::<Probe>().shown = true;
    }
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 1100 {
        prepare(world);
    }
    if frame == 1180 {
        assert!(world.resource::<Probe>().shown);
        world.resource_mut::<Probe>().captured = true;
        return Some("normal-transfer-destination-graphics");
    }
    None
}

pub(crate) struct Snapshot {
    pixels: Vec<(u32, u32, [u8; 4])>,
    checked: Arc<AtomicUsize>,
}

pub(super) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    if label != "normal-transfer-destination-graphics" {
        return None;
    }
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Cross"));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let key = source.get_color_at(0, 0).unwrap().to_srgba().to_u8_array();
    let mut pixels = Vec::new();
    for y in 0..source.height() {
        for x in 0..source.width() {
            let color = source.get_color_at(x, y).unwrap().to_srgba().to_u8_array();
            if color[3] != 255 || color[..3] == key[..3] {
                continue;
            }
            for dy in 0..8 {
                for dx in 0..8 {
                    pixels.push((
                        96 - source.width() / 2 * 8 + x * 8 + dx,
                        60 - source.height() / 2 * 8 + y * 8 + dy,
                        color,
                    ));
                }
            }
        }
    }
    assert_eq!(pixels.len(), 320);
    Some(Snapshot {
        pixels,
        checked: world.resource::<Probe>().pixels.clone(),
    })
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.pixels {
            let actual = crate::display::smoke::pixel_at(image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "destination picture ({x}, {y}): expected {expected:?}, got {actual:?}"
            );
        }
        self.checked.fetch_add(1, Ordering::Relaxed);
        info!("destination presentation: 320 original picture pixels verified");
    }
}

pub(super) fn verify_finished(world: &World) {
    let probe = world.resource::<Probe>();
    assert!(probe.shown && probe.captured);
    assert!(probe.held >= 35);
    assert_eq!(probe.pixels.load(Ordering::Relaxed), 1);
    info!(
        "destination presentation: {} unaged pre-render states and the loaded background verified",
        probe.held
    );
}
