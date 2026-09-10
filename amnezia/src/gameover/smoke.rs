use super::*;
use amnezia_data::EventCommand;

#[derive(Resource, Default)]
struct Trace {
    shots: u8,
    ready_at: Option<u32>,
    returned_at: Option<u32>,
    confirmed: bool,
    defeated: bool,
    defeat_confirmed: bool,
    new_game: bool,
    resumed: bool,
}

pub(crate) fn entry(battle: bool) -> Vec<EventCommand> {
    let commands = if battle {
        vec![
            (10810, "", vec![3, 15, 12]),
            (10710, "Cave1", vec![0, 2, 1, 0, 0, 0]),
        ]
    } else {
        vec![
            (10810, "", vec![3, 15, 12]),
            (11010, "", vec![17]),
            (12420, "", vec![]),
        ]
    };
    commands
        .into_iter()
        .map(|(code, text, params)| EventCommand {
            code,
            indent: 0,
            string: text.into(),
            params,
        })
        .collect()
}

pub(crate) fn input(world: &mut World, frame: u32, battle: bool) -> Option<KeyCode> {
    let trace = world.get_resource::<Trace>()?;
    if trace.defeated && !trace.defeat_confirmed {
        world.resource_mut::<Trace>().defeat_confirmed = true;
        return Some(KeyCode::Enter);
    }
    if trace.ready_at.is_some_and(|ready| frame >= ready + 20) && !trace.confirmed {
        world.resource_mut::<Trace>().confirmed = true;
        return Some(KeyCode::Enter);
    }
    if !battle
        && !trace.new_game
        && trace
            .returned_at
            .is_some_and(|returned| frame >= returned + 15)
    {
        world.resource_mut::<Trace>().new_game = true;
        crate::title::smoke::select_new_game(world);
        return Some(KeyCode::Enter);
    }
    (trace.new_game
        && frame.is_multiple_of(15)
        && world.resource::<crate::dialogue::Dialogue>().active)
        .then_some(KeyCode::Enter)
}

pub(crate) fn drive(world: &mut World, frame: u32, battle: bool) -> Option<&'static str> {
    if frame == 150 {
        world.insert_resource(Trace::default());
    }
    if frame < 150 {
        return None;
    }
    if battle
        && !world.resource::<Trace>().defeated
        && world.resource::<crate::battle::BattleActive>().0
        && !world.resource::<crate::battle::BattleFlow>().busy()
    {
        crate::battle::smoke::defeat(world);
        world.resource_mut::<Trace>().defeated = true;
    }
    let age = world.resource::<crate::transitions::Transition>().age();
    let stage = world.resource::<GameOverFlow>().0;
    let ready_title = crate::title::smoke::ready(world);
    let resumed = world.resource::<Trace>().new_game
        && world.resource::<crate::world::MapData>().map_id == 3
        && !world
            .resource::<crate::interpreter::RunningEvent>()
            .active()
        && !world.resource::<TitleActive>().0
        && !world.resource::<crate::teleport::Fade>().busy();
    let mut trace = world.resource_mut::<Trace>();
    if ready_title && trace.confirmed && trace.returned_at.is_none() {
        trace.returned_at = Some(frame);
        trace.shots |= 8;
        return Some("gameover-title");
    }
    if resumed && !trace.resumed {
        trace.resumed = true;
        return Some("gameover-new-game");
    }
    let shot = match stage {
        Stage::Revealing if age >= 40 => Some((0, "gameover-fade-in")),
        Stage::Showing => Some((1, "gameover-screen")),
        Stage::Leaving if age >= 40 => Some((2, "gameover-fade-out")),
        _ => None,
    };
    if let Some((index, label)) = shot
        && trace.shots & (1 << index) == 0
    {
        trace.shots |= 1 << index;
        if index == 1 {
            trace.ready_at = Some(frame);
        }
        return Some(label);
    }
    None
}

pub(crate) fn verify_finished(world: &World, battle: bool) {
    let trace = world.resource::<Trace>();
    assert_eq!(trace.shots, 0b1111);
    assert!(!world.resource::<GameOverActive>().0);
    if battle {
        assert!(trace.defeated && crate::title::smoke::ready(world));
    } else {
        assert!(trace.resumed);
    }
    info!("Game Over and title handoffs completed, battle defeat = {battle}");
}

pub(crate) fn snapshot(world: &World, label: &str) -> Option<Image> {
    if label != "gameover-screen" {
        return None;
    }
    let handle = world
        .resource::<AssetServer>()
        .load(crate::assets::resolve_png("GameOver", "GameOver"));
    Some(
        world
            .resource::<Assets<Image>>()
            .get(&handle)
            .expect("loaded Game Over graphic")
            .clone(),
    )
}

pub(crate) fn verify_image(source: &Image, output: &Image) {
    assert_eq!((source.width(), source.height()), (320, 240));
    for y in 0..240 {
        for x in 0..320 {
            let expected = source.get_color_at(x, y).unwrap().to_srgba().to_u8_array();
            let actual = output
                .get_color_at(
                    (2 * x + 1) * output.width() / 640,
                    (2 * y + 1) * output.height() / 480,
                )
                .unwrap()
                .to_srgba()
                .to_u8_array();
            assert!(
                actual[..3]
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "Game Over ({x},{y}): {actual:?}, expected {expected:?}"
            );
        }
    }
    info!("all 76800 Game Over pixels match the original graphic");
}
