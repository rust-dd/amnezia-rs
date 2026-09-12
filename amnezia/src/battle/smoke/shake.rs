use super::*;
use crate::battle::model::{Action, Command, Source};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource)]
struct Fixture {
    hp: i32,
    states: Vec<(u32, u32)>,
    defense: u32,
    attack: u32,
    rng: u64,
    queue: Vec<Action>,
    queue_at: usize,
    log: Vec<String>,
    checks: Arc<AtomicUsize>,
    restored: bool,
    underlay: Entity,
}

pub(in crate::battle) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 883 {
        let underlay = world
            .spawn((
                Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(320.0, 240.0)),
                Transform::from_xyz(0.0, 0.0, 90.0),
                crate::animation::overlay_layer(),
            ))
            .id();
        let mut battle = world.resource_mut::<Battle>();
        let fixture = Fixture {
            hp: battle.members[1].hp,
            states: battle.members[1].states.clone(),
            defense: battle.members[1].stats.defense,
            attack: battle.enemies[0].stats.attack,
            rng: battle.rng,
            queue: battle.queue.clone(),
            queue_at: battle.queue_at,
            log: battle.log.clone(),
            checks: Arc::new(AtomicUsize::new(0)),
            restored: false,
            underlay,
        };
        battle.members[1].states = vec![(7, 0)];
        battle.members[1].stats.defense = 0;
        battle.enemies[0].stats.attack = 30;
        battle.queue = vec![Action {
            source: Source::Enemy(0),
            kind: Command::Attack { target: 1 },
            agility: 1,
        }];
        battle.queue_at = 0;
        assert!(battle.resolve_next_with_items(|_| true));
        assert!(battle.members[1].hp > 0 && battle.members[1].hp < fixture.hp);
        assert!(battle.pending_shake);
        world.insert_resource(fixture);
    }
    if frame == 897 {
        let mut fixture = world.remove_resource::<Fixture>().unwrap();
        world.despawn(fixture.underlay);
        let mut battle = world.resource_mut::<Battle>();
        battle.members[1].hp = fixture.hp;
        battle.members[1].states = std::mem::take(&mut fixture.states);
        battle.members[1].stats.defense = fixture.defense;
        battle.enemies[0].stats.attack = fixture.attack;
        battle.rng = fixture.rng;
        battle.queue = std::mem::take(&mut fixture.queue);
        battle.queue_at = fixture.queue_at;
        battle.log = std::mem::take(&mut fixture.log);
        fixture.restored = true;
        world.insert_resource(fixture);
    }
    match frame {
        884 => Some("battle-shake-right"),
        888 => Some("battle-shake-left"),
        894 => Some("battle-shake-restored"),
        _ => None,
    }
}

pub(in crate::battle) fn checks(world: &World) -> Arc<AtomicUsize> {
    world.resource::<Fixture>().checks.clone()
}

pub(super) fn verify_finished(world: &World) {
    let fixture = world.resource::<Fixture>();
    assert!(fixture.restored);
    assert_eq!(fixture.checks.load(Ordering::Relaxed), 3);
}
