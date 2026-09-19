use super::*;
use crate::audio::{BgmTrack, CurrentBgm};
use crate::title::TitleActive;

#[derive(Resource)]
struct Music(Option<BgmTrack>);

pub(crate) fn configure(app: &mut App) {
    super::configure(app);
    let mut files = Vec::new();
    for number in [12, 14, 15] {
        let (name, hp) = if number == 15 {
            ("Áron", 23)
        } else {
            ("Álmos", 17)
        };
        let bytes = if number == 12 {
            b"broken save".to_vec()
        } else {
            format!("(format_version:15,saved_at:Some({number}),map_id:3,x:15,y:12,dir:2,switches:[],variables:[],party:[1,2,3,4],items:[],gold:17,hero_name:\"{name}\",vitals:[(1,({hp},12))],charset:\"Chara1\")").into_bytes()
        };
        let target = path(app.world(), number);
        std::fs::write(&target, &bytes).unwrap();
        std::fs::File::open(&target)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(100 - u64::from(number)),
            ))
            .unwrap();
        files.push((number, bytes));
    }
    app.world_mut().resource_mut::<Fixture>().files = files;
}

pub(crate) fn input(frame: u32) -> Option<KeyCode> {
    match frame {
        90 | 170 | 200 | 260 | 300 => Some(KeyCode::Enter),
        140 => Some(KeyCode::PageUp),
        180 => Some(KeyCode::ArrowUp),
        220 => Some(KeyCode::Escape),
        _ => None,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let files = world.resource::<SaveFiles>();
    if files.active() && world.resource::<crate::transitions::Transition>().busy() {
        assert!(
            files.entries.is_some(),
            "the load fade must capture a populated file list"
        );
    }
    if frame < 120
        && files.active()
        && world.resource::<crate::transitions::Transition>().age() == 1
        && world.resource::<Fixture>().checks & 16 == 0
    {
        world.resource_mut::<Fixture>().checks |= 16;
        return Some("load-slots-fade");
    }
    if frame == 60 {
        assert!(!path(world, 1).exists());
        crate::title::smoke::assert_continue(world);
        world.insert_resource(Music(world.resource::<CurrentBgm>().track()));
    }
    if (61..=300).contains(&frame) {
        assert_eq!(
            world.resource::<CurrentBgm>().track(),
            world.resource::<Music>().0
        );
    }
    if matches!(frame, 172 | 202) {
        assert!(world.resource::<TitleActive>().0);
        assert!(world.resource::<SaveFiles>().active());
        assert!(world.resource::<SaveFiles>().decision().is_none());
        assert!(!world.resource::<crate::save::LoadRequest>().0);
        world.resource_mut::<Fixture>().checks |= if frame == 172 { 1 } else { 2 };
    }
    if frame == 240 {
        crate::title::smoke::assert_continue(world);
        assert!(!world.resource::<SaveFiles>().active());
        assert!(!world.resource::<MenuOpen>().0);
        world.resource_mut::<Fixture>().checks |= 4;
    }
    if frame > 300 && !world.resource::<TitleActive>().0 {
        world.resource_mut::<MenuOpen>().0 = true;
    }
    if frame == 400 {
        assert!(!world.resource::<TitleActive>().0);
        assert!(!world.resource::<crate::teleport::Fade>().busy());
        assert!(!world.resource::<SaveFiles>().active());
        assert_eq!(world.resource::<crate::world::MapData>().map_id, 3);
        assert_eq!(
            *world.resource::<ActiveSlot>(),
            ActiveSlot::new(15).unwrap()
        );
        assert_eq!(world.resource::<crate::text::HeroName>().0, "Áron");
        assert_eq!(
            world.resource::<crate::vitals::Vitals>().get_stored(1),
            Some((23, 12))
        );
        assert_eq!(
            world.resource::<crate::state::Party>().snapshot(),
            [1, 2, 3, 4]
        );
        world.resource_mut::<Fixture>().checks |= 8;
    }
    match frame {
        120 => Some("load-slots-bottom"),
        155 => Some("load-slots-corrupt"),
        210 => Some("load-slots-empty"),
        240 => Some("title-load-return"),
        280 => Some("load-slots-reopened"),
        400 => Some("load-slots-restored"),
        _ => None,
    }
}

pub(crate) fn verify_finished(world: &mut World) {
    assert_eq!(world.resource::<Pixels>().0.load(Ordering::Relaxed), 5);
    assert_eq!(world.resource::<Fixture>().checks, 31);
    crate::title::smoke::verify_load_finished(world);
    assert!(!path(world, 1).exists());
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 3);
    for (number, original) in fixture.files {
        let target = path(world, number);
        assert_eq!(std::fs::read(&target).unwrap(), original);
        std::fs::remove_file(target).unwrap();
    }
    std::fs::remove_dir(fixture.directory).unwrap();
    world.resource_mut::<SaveLocation>().0 = fixture.original;
    *world.resource_mut::<ActiveSlot>() = fixture.original_slot;
    info!(
        "load selector: disabled slots, seamless title cancellation and a read-only slot 15 load verified"
    );
}
