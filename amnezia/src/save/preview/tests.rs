use super::*;
use bevy::prelude::*;

fn database() -> GameData {
    let mut app = App::new();
    app.add_plugins(crate::gamedata::GameDataPlugin);
    app.world_mut().remove_resource::<GameData>().unwrap()
}

#[test]
fn previews_use_the_saved_leader_vitals_name_and_portrait_order_without_writing() {
    let path = super::super::tests::temp_slot("preview_party");
    let data = database();
    for (party, name, hp) in [("1,2,3,4", "Álmos", 17), ("2,1,4,3", "Tiffany", 38)] {
        let original = format!(
            "(format_version:15,map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[{party}],items:[],gold:0,hero_name:\"Álmos\",vitals:[(1,(17,12))])"
        );
        std::fs::write(&path, &original).unwrap();
        let entry = read(&path, &data);
        let Contents::Party(preview) = entry.contents else {
            panic!("missing saved party")
        };
        assert_eq!(preview.name, name);
        assert_eq!(preview.hp, hp);
        let ids = party
            .split(',')
            .map(|id| id.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(preview.level, data.actor(ids[0]).unwrap().level);
        assert_eq!(
            preview.faces,
            ids.iter()
                .map(|&id| {
                    let actor = data.actor(id).unwrap();
                    (actor.face_name.clone(), actor.face_index)
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn previews_distinguish_empty_corrupt_future_and_repairable_legacy_slots() {
    let path = super::super::tests::temp_slot("preview_status");
    let data = database();
    assert_eq!(read(&path, &data).contents, Contents::Empty);
    for (version, hp, expected) in [(16, 63, false), (15, 2301, false), (0, 2301, true)] {
        let original = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,vitals:[(1,({hp},0))])"
        );
        std::fs::write(&path, &original).unwrap();
        let contents = read(&path, &data).contents;
        if expected {
            let Contents::Party(party) = contents else {
                panic!("legacy repair failed")
            };
            assert_eq!((party.name.as_str(), party.hp, party.level), ("Ron", 63, 2));
        } else {
            assert_eq!(contents, Contents::Corrupt);
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    std::fs::write(&path, "broken").unwrap();
    assert_eq!(read(&path, &data).contents, Contents::Corrupt);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn most_recent_file_selects_the_initial_row_and_ties_keep_the_first() {
    let mut entries = vec![
        Entry {
            contents: Contents::Empty,
            modified: None
        };
        15
    ];
    assert_eq!(latest(&entries), 0);
    let older = SystemTime::UNIX_EPOCH;
    let newer = older + std::time::Duration::from_secs(1);
    for index in [0, 6, 14] {
        entries[index].contents = Contents::Party(PartyPreview {
            name: "Ron".into(),
            level: 2,
            hp: 63,
            faces: Vec::new(),
        });
    }
    entries[14].modified = Some(newer);
    entries[0].modified = Some(older);
    assert_eq!(latest(&entries), 14);
    entries[6].modified = Some(newer);
    assert_eq!(latest(&entries), 6);
    entries[3] = Entry {
        contents: Contents::Corrupt,
        modified: Some(newer + std::time::Duration::from_secs(1)),
    };
    assert_eq!(latest(&entries), 6);
}
