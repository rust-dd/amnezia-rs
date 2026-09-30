use super::*;

pub(super) fn input(world: &mut World, frame: u32) {
    let mut key = None;
    if let Some(probe) = world.get_resource::<Probe>() {
        let stage = probe.stage;
        if stage == Stage::ApproachTower {
            if world.query::<&Player>().single(world).unwrap().tile() != (54, 35) {
                key = Some(KeyCode::ArrowUp);
            }
        } else if stage != Stage::Done && frame.is_multiple_of(15) {
            let choice = world.resource::<Choice>();
            if choice.active {
                if world.resource::<Dialogue>().prompt_input_ready()
                    && probe.generation == choice.generation
                {
                    let target = match stage {
                        Stage::Depart
                        | Stage::ReturnOff
                        | Stage::ReturnOn
                        | Stage::EnterOff
                        | Stage::EnterOn => 1,
                        _ => 0,
                    };
                    key = Some(
                        if matches!(
                            stage,
                            Stage::CancelOff | Stage::CancelOn | Stage::CancelTower
                        ) {
                            KeyCode::Escape
                        } else if choice.cursor != target {
                            KeyCode::ArrowDown
                        } else {
                            KeyCode::Enter
                        },
                    );
                }
            } else if world.resource::<Dialogue>().active {
                key = Some(KeyCode::Enter);
            } else if idle(world) {
                key = match stage {
                    Stage::Depart | Stage::ReturnOff | Stage::ReturnOn => Some(KeyCode::Enter),
                    Stage::LandTower | Stage::RevisitTower => Some(KeyCode::Enter),
                    Stage::CancelTower | Stage::LeaveTower | Stage::LeaveAgain
                        if world.resource::<MapData>().map_id == 249 =>
                    {
                        let hero = world.query::<&Player>().single(world).unwrap();
                        Some(if hero.dir == crate::tiles::DIR_UP {
                            KeyCode::Enter
                        } else {
                            KeyCode::ArrowUp
                        })
                    }
                    Stage::CancelOff
                    | Stage::MapOn
                    | Stage::CancelOn
                    | Stage::MapOff
                    | Stage::MapOnAgain
                    | Stage::HideMap
                    | Stage::EnterOff
                    | Stage::EnterOn => Some(KeyCode::Escape),
                    _ => None,
                };
            }
        }
    }
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    if let Some(key) = key {
        keys.press(key);
    }
}

pub(super) fn verify_options(stage: Stage, options: &[String]) {
    let expected = match stage {
        Stage::Depart | Stage::ReturnOff | Stage::ReturnOn => {
            vec!["Igen!", "Igen, de ne a kastélyhoz...", "Még nem..."]
        }
        Stage::CancelTower | Stage::LeaveTower | Stage::LeaveAgain => {
            vec!["Vissza a léghajóra...", "Még ne..."]
        }
        Stage::CancelOff | Stage::MapOn | Stage::MapOnAgain | Stage::EnterOff => {
            vec!["Térkép bekapcsolása", "Belépés a Draco belsejébe", "Mégse"]
        }
        Stage::CancelOn | Stage::MapOff | Stage::EnterOn | Stage::HideMap => {
            vec!["Térkép kikapcsolása", "Belépés a Draco belsejébe", "Mégse"]
        }
        _ => panic!("unexpected choice in {stage:?}: {options:?}"),
    };
    assert_eq!(options, expected, "original choice options in {stage:?}");
}
