use super::{Dialogue, MessageOptions, MessagePosition, view};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Page {
    generation: u64,
    index: usize,
    battle: bool,
    prompt: (u8, u64),
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        latch
            .after(super::MessageUpdate)
            .before(crate::interpreter::InterpreterStep),
    );
}

pub(crate) fn latch(world: &mut World) {
    if world.contains_resource::<Dialogue>()
        && world.contains_resource::<MessagePosition>()
        && world.contains_resource::<MessageOptions>()
    {
        world.run_system_cached(update).unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    position: Res<MessagePosition>,
    options: Res<MessageOptions>,
    dialogue: Res<Dialogue>,
    prompts: view::prompts::Presentation,
    map: Option<Res<crate::world::MapData>>,
    screen: crate::world::MapScreen,
    battle: Option<Res<crate::battle::BattleActive>>,
    mut previous: Local<Option<Page>>,
    mut panels: Query<&mut Node, With<view::DialoguePanel>>,
) {
    if !dialogue.active && !prompts.active() {
        if !dialogue.lifecycle.message.visible() {
            *previous = None;
        }
        return;
    }
    let battle = battle.is_some_and(|battle| battle.0);
    let page = Page {
        generation: dialogue.generation,
        index: dialogue.index,
        battle,
        prompt: if dialogue.active {
            (0, 0)
        } else {
            prompts.key()
        },
    };
    if *previous == Some(page) {
        return;
    }
    *previous = Some(page);
    let hero_y = map
        .as_deref()
        .and_then(|map| screen.hero_y(map))
        .unwrap_or(128);
    let position = options.position(*position, hero_y, battle);
    let Ok(mut node) = panels.single_mut() else {
        return;
    };
    match position {
        MessagePosition::Top => {
            node.top = Val::Px(0.0);
            node.bottom = Val::Auto;
        }
        MessagePosition::Middle => {
            node.top = Val::Px(240.0);
            node.bottom = Val::Auto;
        }
        MessagePosition::Bottom => {
            node.top = Val::Auto;
            node.bottom = Val::Px(0.0);
        }
    }
}
