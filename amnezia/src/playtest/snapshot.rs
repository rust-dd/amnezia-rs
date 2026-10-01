use super::Controller;
use crate::state::{Inventory, Party, Switches, Variables};
use crate::world::collision::{CollisionBodies, MapCollision, Mover};
use crate::world::{EventSprite, MapData, MapEvents, MoveQueue, RouteStepper};
use bevy::prelude::*;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct Snapshot {
    id: u64,
    frame: u32,
    map: u32,
    width: i32,
    height: i32,
    player: Option<(i32, i32, u32, bool)>,
    title: bool,
    transition: bool,
    erased: bool,
    running_event: Option<u32>,
    dialogue: bool,
    dialogue_ready: bool,
    text: Vec<String>,
    choice: bool,
    options: Vec<String>,
    choice_cursor: usize,
    number: Option<(i64, usize, u32)>,
    menu: bool,
    menu_state: String,
    shop: bool,
    shop_state: String,
    save_allowed: bool,
    battle: bool,
    battle_phase: String,
    battle_menu: u8,
    battle_cursor: usize,
    battle_turn: usize,
    battle_round: u32,
    battle_log: Vec<String>,
    fighters: Vec<(u32, String, i32, i32, i32, i32)>,
    battle_states: Vec<(u32, Vec<u32>)>,
    enemies: Vec<(String, i32, i32)>,
    party: Vec<u32>,
    levels: Vec<(u32, u32)>,
    experience: Vec<(u32, u32)>,
    vitals: Vec<(u32, (i32, i32))>,
    conditions: Vec<(u32, Vec<u32>)>,
    skills: Vec<(u32, Vec<u32>)>,
    equipment: Vec<(u32, [u32; 5])>,
    gold: i32,
    items: Vec<(u32, u32)>,
    switches: Vec<(u32, bool)>,
    variables: Vec<(u32, i32)>,
    timer: Option<f32>,
    events: Vec<EventView>,
    edges: Vec<u8>,
}

#[derive(Serialize)]
struct EventView {
    id: u32,
    name: String,
    x: i32,
    y: i32,
    page: usize,
    trigger: u32,
    layer: u32,
    graphic: String,
    commands: Vec<amnezia_data::EventCommand>,
}

pub(super) fn log_message(world: &mut World) {
    let dialogue = world.resource::<crate::dialogue::Dialogue>();
    let message = if dialogue.active && dialogue.ready_to_advance() {
        dialogue
            .boxes
            .get(dialogue.index)
            .map(|page| page.lines.join("\n"))
            .unwrap_or_default()
    } else {
        return;
    };
    let map = world.get_resource::<MapData>().map_or(0, |map| map.map_id);
    let frame = world.resource::<crate::timing::GameFrames>().frame;
    let mut controller = world.resource_mut::<Controller>();
    if message != controller.last_message {
        super::storage::report(super::storage::append(
            &controller.directory.join("dialogue.log"),
            &format!("[{frame} map {map}]\n{message}\n"),
        ));
        controller.last_message = message;
    }
}

pub(super) fn write(world: &mut World, directory: &Path, id: u64, capture: bool) {
    let player = world
        .query::<(&crate::player::Player, &MoveQueue)>()
        .single(world)
        .ok()
        .map(|(player, motion)| (player.tile_x, player.tile_y, player.dir, motion.busy()));
    let sprites = world
        .query::<&EventSprite>()
        .iter(world)
        .map(|sprite| (sprite.id, sprite.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let bodies = CollisionBodies::from_events(
        world
            .query::<(&EventSprite, Option<&RouteStepper>)>()
            .iter(world),
    );
    let party = world.resource::<Party>();
    let inventory = world.resource::<Inventory>();
    let switches = world.resource::<Switches>();
    let variables = world.resource::<Variables>();
    let data = world.get_resource::<MapData>();
    let events = world.resource::<MapEvents>();
    let mut edges = Vec::new();
    if let Some(data) = data {
        let collision = MapCollision::new(
            data,
            events,
            (switches, variables, party, inventory),
            &bodies,
        );
        for y in 0..data.height {
            for x in 0..data.width {
                let mut mask = 0;
                for (bit, dx, dy) in [(1, 0, -1), (2, 1, 0), (4, 0, 1), (8, -1, 0)] {
                    if collision.can_move((x, y), (x + dx, y + dy), Mover::hero(false), None, false)
                    {
                        mask |= bit;
                    }
                }
                edges.push(mask);
            }
        }
    }
    let events = events
        .events
        .iter()
        .filter_map(|event| {
            let page =
                crate::state::active_page_index(event, switches, variables, party, inventory)?;
            let active = &event.pages[page];
            let sprite = sprites.get(&event.id);
            Some(EventView {
                id: event.id,
                name: event.name.clone(),
                x: sprite.map_or(event.x as i32, |sprite| sprite.tile_x),
                y: sprite.map_or(event.y as i32, |sprite| sprite.tile_y),
                page,
                trigger: active.trigger,
                layer: sprite.map_or(active.layer, |sprite| sprite.layer),
                graphic: sprite.map_or_else(
                    || active.graphic_name.clone(),
                    |sprite| sprite.charset.clone(),
                ),
                commands: active.commands.clone(),
            })
        })
        .collect();
    let dialogue = world.resource::<crate::dialogue::Dialogue>();
    let choice = world.resource::<crate::choice::Choice>();
    let number = world.resource::<crate::inputnumber::InputNumber>();
    let battle = world.resource::<crate::battle::Battle>();
    let progression = world.resource::<crate::progression::Progression>();
    let gamedata = world.resource::<crate::gamedata::GameData>();
    let transition = world.resource::<crate::transitions::Transition>();
    let (items, gold) = inventory.snapshot();
    let snapshot = Snapshot {
        id,
        frame: world.resource::<crate::timing::GameFrames>().frame,
        map: data.map_or(0, |map| map.map_id),
        width: data.map_or(0, |map| map.width),
        height: data.map_or(0, |map| map.height),
        player,
        title: world.resource::<crate::title::TitleActive>().0,
        transition: transition.busy(),
        erased: transition.erased(),
        running_event: world
            .resource::<crate::interpreter::RunningEvent>()
            .debug_id(),
        dialogue: dialogue.active,
        dialogue_ready: dialogue.ready_to_advance(),
        text: dialogue
            .boxes
            .get(dialogue.index)
            .map(|page| page.lines.clone())
            .unwrap_or_default(),
        choice: choice.active,
        options: choice.options.clone(),
        choice_cursor: choice.cursor,
        number: number
            .active
            .then(|| (number.value, number.cursor(), number.digits)),
        menu: world.resource::<crate::menu::MenuOpen>().0,
        menu_state: crate::menu::playtest_state(world),
        shop: world.resource::<crate::shop::ShopOpen>().0,
        shop_state: crate::shop::playtest_state(world),
        save_allowed: world.resource::<crate::save::SaveAccess>().0,
        battle: world.resource::<crate::battle::BattleActive>().0,
        battle_phase: format!("{:?}", battle.phase),
        battle_menu: battle.menu as u8,
        battle_cursor: battle.cursor,
        battle_turn: battle.turn,
        battle_round: battle.round,
        battle_log: battle.log.clone(),
        fighters: battle
            .members
            .iter()
            .map(|actor| {
                (
                    actor.actor_id,
                    actor.name.clone(),
                    actor.hp,
                    actor.max_hp,
                    actor.sp,
                    actor.max_sp,
                )
            })
            .collect(),
        battle_states: battle
            .members
            .iter()
            .map(|actor| {
                (
                    actor.actor_id,
                    actor.states.iter().map(|(id, _)| *id).collect(),
                )
            })
            .collect(),
        enemies: battle
            .enemies
            .iter()
            .map(|enemy| (enemy.name.clone(), enemy.hp, enemy.max_hp))
            .collect(),
        party: party.snapshot(),
        levels: gamedata
            .actors
            .iter()
            .map(|actor| (actor.id, progression.level(actor)))
            .collect(),
        experience: gamedata
            .actors
            .iter()
            .map(|actor| (actor.id, progression.total(actor)))
            .collect(),
        vitals: world.resource::<crate::vitals::Vitals>().entries(),
        conditions: world
            .resource::<crate::vitals::Vitals>()
            .condition_entries(),
        skills: gamedata
            .actors
            .iter()
            .map(|actor| (actor.id, progression.known_skill_ids(actor)))
            .collect(),
        equipment: world.resource::<crate::equipment::Equipment>().entries(),
        gold,
        items,
        switches: switches.entries(),
        variables: variables.entries(),
        timer: world
            .resource::<crate::timer::GameClock>()
            .running
            .then(|| world.resource::<crate::timer::GameClock>().remaining),
        events,
        edges,
    };
    match ron::ser::to_string_pretty(&snapshot, ron::ser::PrettyConfig::default()) {
        Ok(text) => super::storage::report(super::storage::snapshot(directory, id, &text, capture)),
        Err(error) => warn!("playtest snapshot serialization failed: {error}"),
    }
}
