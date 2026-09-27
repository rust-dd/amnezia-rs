use super::super::{frame::Frame, opcodes::*, scenes::Scene};
use super::{Exec, Flow};
use crate::battle::BattleRequest;
use crate::shop::ShopRequest;
use amnezia_data::EventCommand;

pub(super) fn execute(frame: &mut Frame, command: &EventCommand, x: &mut Exec) -> Flow {
    match command.code {
        ENEMY_ENCOUNTER => {
            let troop_id = super::super::commands::operate_value(
                0,
                command.params.first().copied().unwrap_or(0),
                command.params.get(1).copied().unwrap_or(0),
                &x.variables,
            )
            .max(0) as u32;
            let request = BattleRequest {
                troop_id,
                background: command.string.clone(),
                allow_escape: command.params.get(3).copied().unwrap_or(0) != 0,
                first_strike: command.params.get(5).copied().unwrap_or(0) != 0,
                defeat_ends_game: command.params.get(4).copied().unwrap_or(0) == 0,
            };
            frame.scene_request = Some(x.subsystems.scenes.replace(Scene::Battle(request)));
            frame.battle_outcome = None;
            frame.battle_pending = true;
            frame.ip += 1;
        }
        OPEN_SHOP => {
            let (allow_buy, allow_sell) = match command.params.first().copied().unwrap_or(0) {
                1 => (true, false),
                2 => (false, true),
                _ => (true, true),
            };
            let shop_type = command.params.get(1).copied().unwrap_or(0).max(0) as u32;
            let items = command
                .params
                .iter()
                .skip(4)
                .filter(|&&p| p >= 0)
                .map(|&p| p as u32)
                .collect();
            let request = ShopRequest::OpenShop {
                items,
                allow_buy,
                allow_sell,
                shop_type,
            };
            frame.scene_request = Some(x.subsystems.scenes.replace(Scene::Shop(request)));
            frame.shop_transacted = None;
            frame.shop_pending = true;
        }
        SHOW_INN => {
            let cost = command.params.get(1).copied().unwrap_or(0);
            let inn_type = command.params.first().copied().unwrap_or(0).max(0) as u32;
            x.subsystems
                .merchant
                .writer
                .write(ShopRequest::ShowInn { cost, inn_type });
            frame.shop_pending = true;
        }
        OPEN_SAVE_MENU => {
            x.subsystems.scenes.replace(Scene::Save);
            frame.ip += 1;
        }
        _ => unreachable!(),
    }
    Flow::Yield
}

pub(super) fn game_over(frame: &mut Frame, x: &mut Exec) -> Flow {
    x.subsystems.scenes.replace(Scene::GameOver);
    frame.ip += 1;
    Flow::Yield
}
