use super::model::Battle;
use super::{BattleActive, BattleData, BattleOutcome, BattleResult, MapBgm};
use crate::audio::AudioRequest;
use crate::transitions::TransitionIo;
use bevy::prelude::*;

pub(crate) mod smoke;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BattleFlowSet;

#[derive(Default, PartialEq, Eq, Debug)]
enum Stage {
    #[default]
    Map,
    Enter,
    MapErase,
    BattleShow,
    Battle,
    Leave,
    BattleErase,
    MapShow,
}

#[derive(Resource, Default)]
pub(crate) struct BattleFlow {
    stage: Stage,
    prepared: Option<Battle>,
    outcome: Option<BattleOutcome>,
    defeat_ends_game: bool,
}

impl BattleFlow {
    pub(super) fn enter(&mut self, battle: Battle, defeat_ends_game: bool) {
        self.prepared = Some(battle);
        self.outcome = None;
        self.defeat_ends_game = defeat_ends_game;
        self.stage = Stage::Enter;
    }

    pub(super) fn leave(&mut self, outcome: BattleOutcome) {
        self.outcome = Some(outcome);
        self.stage = Stage::Leave;
    }

    pub(crate) fn busy(&self) -> bool {
        !matches!(self.stage, Stage::Map | Stage::Battle)
    }
}

pub(super) fn playing(
    flow: Option<Res<BattleFlow>>,
    step: Option<Res<crate::timing::logical::Step>>,
) -> bool {
    !flow.is_some_and(|flow| flow.busy()) && !step.is_some_and(|step| step.callback)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn drive(
    mut flow: ResMut<BattleFlow>,
    mut transition: TransitionIo,
    mut battle: ResMut<Battle>,
    mut active: ResMut<BattleActive>,
    mut result: ResMut<BattleResult>,
    map_bgm: Res<MapBgm>,
    battle_data: Res<BattleData>,
    system_bgm: Option<Res<crate::system_bgm::SystemBgm>>,
    mut audio: MessageWriter<AudioRequest>,
    map: Option<Res<crate::world::MapData>>,
    screen: crate::world::MapScreen,
    vehicles: Option<Res<crate::vehicles::Vehicles>>,
    mut gameover: ResMut<crate::gameover::GameOverActive>,
    mut gameover_flow: ResMut<crate::gameover::GameOverFlow>,
) {
    if transition.state.busy() {
        return;
    }
    let hero_center = match (map.as_deref(), vehicles.as_deref()) {
        (Some(map), Some(vehicles)) => {
            let (x, y) = screen.character(10001, 0, map, vehicles, (0, 0));
            IVec2::new(x, y - 8)
        }
        _ => IVec2::new(160, 120),
    };
    let now = transition.frames.frame;
    match flow.stage {
        Stage::Map | Stage::Battle => {}
        Stage::Enter => {
            transition.begin_battle(hero_center);
            flow.stage = Stage::MapErase;
        }
        Stage::MapErase => {
            *battle = flow.prepared.take().expect("prepared battle at entry");
            audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
                system_bgm.as_deref(),
                0,
                &battle_data.system.battle_music,
            )));
            let kind = transition.kind(3);
            transition
                .state
                .start(kind, false, now, IVec2::new(160, 120));
            flow.stage = Stage::BattleShow;
        }
        Stage::BattleShow => flow.stage = Stage::Battle,
        Stage::Leave => {
            let kind = transition.kind(4);
            transition
                .state
                .start(kind, true, now, IVec2::new(160, 120));
            flow.stage = Stage::BattleErase;
        }
        Stage::BattleErase => {
            *battle = Battle::default();
            if flow.defeat_ends_game && flow.outcome == Some(BattleOutcome::Defeat) {
                gameover.0 = true;
                gameover_flow.prepare_from_battle();
                result.0 = flow.outcome.take();
                active.0 = false;
                flow.stage = Stage::Map;
                return;
            }
            audio.write(map_bgm.restore());
            if !transition.state.event_erased {
                let kind = transition.kind(5);
                transition.state.start(kind, false, now, hero_center);
            }
            flow.stage = Stage::MapShow;
        }
        Stage::MapShow => {
            result.0 = flow.outcome.take();
            active.0 = false;
            flow.stage = Stage::Map;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<BattleFlow>()
        .init_resource::<crate::gameover::GameOverActive>()
        .init_resource::<crate::gameover::GameOverFlow>()
        .init_resource::<crate::transitions::Transition>()
        .init_resource::<crate::transitions::Settings>()
        .init_resource::<crate::transitions::Defaults>()
        .init_resource::<crate::timing::GameFrames>()
        .add_systems(
            Update,
            drive
                .in_set(BattleFlowSet)
                .after(super::systems::start_on_request)
                .before(super::events::drive)
                .before(super::input::command_input)
                .before(super::systems::resolve_tick)
                .before(super::systems::abort_expired_battle)
                .before(super::systems::outcome_input),
        );
}
