use super::*;
use crate::audio::{
    AudioRequest, BgmTrack,
    saved::{Capture, MusicState},
};
use bevy::audio::{AudioSink, AudioSinkPlayback};
use bevy::ecs::system::RunSystemOnce;

mod boundary;
pub(crate) mod message;

#[derive(Resource)]
struct Fixture {
    slot: super::smoke_slot::Slot,
    map_music: Option<amnezia_data::MapInfoDef>,
    checks: u8,
}

pub(crate) fn configure(app: &mut App) {
    let slot = super::smoke_slot::Slot::new(app, "save-music");
    app.insert_resource(Fixture {
        slot,
        map_music: None,
        checks: 0,
    });
}

fn house() -> BgmTrack {
    BgmTrack {
        name: "House".into(),
        volume: 0.25,
        speed: 1.1,
        fade_in: 1.5,
    }
}

fn elven() -> BgmTrack {
    BgmTrack {
        name: "Elven".into(),
        volume: 0.4,
        speed: 0.8,
        fade_in: 0.7,
    }
}

fn theme() -> BgmTrack {
    BgmTrack {
        name: "Theme".into(),
        volume: 1.0,
        speed: 1.0,
        fade_in: 0.0,
    }
}

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    let path = world.resource::<Fixture>().slot.path(world);
    let message_capture = message::drive(world, frame);
    boundary::drive(world, frame);
    match frame {
        260 => {
            assert_eq!(world.resource::<MapData>().map_id, 3);
            let mut maps = world.resource_mut::<crate::map_bgm::MapInfoData>();
            let map = maps.0.iter_mut().find(|map| map.id == 3).unwrap();
            let original = map.clone();
            map.music_type = 2;
            map.music = amnezia_data::MusicDef {
                name: "Theme".into(),
                volume: 100,
                tempo: 100,
                balance: 50,
                fadein: 0,
            };
            world.resource_mut::<Fixture>().map_music = Some(original);
        }
        270 => {
            world.write_message(elven().replay());
            world.write_message(AudioRequest::MemorizeBgm);
            world.write_message(house().replay());
        }
        303 => {
            let game = read_save(&path).unwrap();
            assert_eq!(game.format_version, SAVE_FORMAT_VERSION);
            assert_eq!(
                game.music,
                Some(MusicState {
                    current: Some(house()),
                    memorized: Some(elven())
                })
            );
        }
        310 | 493 => {
            if frame == 493 {
                assert_eq!(read_save(&path).unwrap().music, Some(MusicState::default()));
            }
            world.write_message(theme().replay());
            world.write_message(AudioRequest::MemorizeBgm);
        }
        320 | 500 | 610 => world.resource_mut::<LoadRequest>().0 = true,
        410 => {
            verify(world, Some(house()), Some(elven()));
            world.resource_mut::<Fixture>().checks |= 1;
            return Some("save-music-restored");
        }
        420 => {
            world.write_message(AudioRequest::PlayMemorizedBgm);
        }
        470 => {
            verify(world, Some(elven()), Some(elven()));
            world.resource_mut::<Fixture>().checks |= 2;
            return Some("save-music-memorized");
        }
        480 => {
            world.write_message(AudioRequest::StopBgm);
            world.write_message(AudioRequest::MemorizeBgm);
        }
        590 => {
            verify(world, None, None);
            world.resource_mut::<Fixture>().checks |= 4;
            return Some("save-music-silence");
        }
        600 => {
            let mut game = read_save(&path).unwrap();
            game.format_version = 1;
            game.foreground = None;
            game.music = None;
            game.message = default();
            game.camera = None;
            write_save(&path, &game).unwrap();
        }
        700 => {
            verify(world, Some(theme()), None);
            assert_eq!(read_save(&path).unwrap().format_version, 1);
            world.resource_mut::<Fixture>().checks |= 8;
            return Some("save-music-legacy");
        }
        _ => {}
    }
    message_capture
}

fn verify(world: &mut World, current: Option<BgmTrack>, memorized: Option<BgmTrack>) {
    assert_eq!(world.resource::<MapData>().map_id, 3);
    assert!(!world.resource::<Fade>().busy());
    let hero = world.query::<&Player>().single(world).unwrap();
    assert_eq!((hero.tile_x, hero.tile_y), (15, 12));
    let music = world
        .run_system_once(|capture: Capture| capture.snapshot())
        .unwrap()
        .unwrap();
    assert_eq!(
        music,
        MusicState {
            current: current.clone(),
            memorized
        }
    );
    let handles = ["House", "Elven", "Theme"].map(|name| {
        world
            .resource::<AssetServer>()
            .load::<AudioSource>(format!("audio/Music/{name}.ogg"))
    });
    if let Some(track) = current {
        let expected = world
            .resource::<AssetServer>()
            .load::<AudioSource>(format!("audio/Music/{}.ogg", track.name));
        let sinks = world
            .query::<(&AudioPlayer<AudioSource>, &AudioSink)>()
            .iter(world)
            .filter(|(player, _)| player.0 == expected)
            .collect::<Vec<_>>();
        assert_eq!(
            sinks.len(),
            1,
            "the restored track must have an actual decoded audio sink"
        );
        assert!((sinks[0].1.speed() - track.speed).abs() < 1e-6);
    } else {
        assert!(
            world
                .query::<&AudioPlayer<AudioSource>>()
                .iter(world)
                .all(|player| !handles.contains(&player.0))
        );
    }
    info!("saved music: map, current/remembered track, fade-in and actual playback verified");
}

pub(crate) fn verify_finished(world: &mut World) {
    boundary::verify_finished(world);
    message::verify_finished(world);
    let fixture = world.remove_resource::<Fixture>().unwrap();
    assert_eq!(fixture.checks, 15);
    fixture.slot.finish(world);
    let original = fixture.map_music.unwrap();
    let map_id = original.id;
    *world
        .resource_mut::<crate::map_bgm::MapInfoData>()
        .0
        .iter_mut()
        .find(|map| map.id == map_id)
        .unwrap() = original;
}
