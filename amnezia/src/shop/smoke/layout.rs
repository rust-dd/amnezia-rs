use super::*;
use crate::shop::ShopRequest;
use crate::state::Party;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
pub(in crate::shop) struct Checks(pub Arc<AtomicUsize>);

pub(super) fn input(frame: u32) -> Vec<KeyCode> {
    match frame {
        790 | 980 => vec![KeyCode::Enter],
        800 => vec![KeyCode::PageDown],
        820 => vec![KeyCode::PageUp],
        825 => vec![KeyCode::ArrowUp],
        810 | 860 | 870 | 880 | 890 | 900 | 910 | 920 => vec![KeyCode::ArrowDown],
        840 => vec![KeyCode::ArrowDown, KeyCode::Enter],
        850 => vec![KeyCode::ArrowRight],
        835 | 930 | 940 | 990 | 1030 | 1070 | 1140 => vec![KeyCode::Escape],
        _ => vec![],
    }
}

fn open(world: &mut World, items: Vec<u32>, style: u32, buy: bool, sell: bool) {
    world.write_message(ShopRequest::OpenShop {
        items,
        shop_type: style,
        allow_buy: buy,
        allow_sell: sell,
    });
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        760 => {
            world.resource_mut::<Party>().restore(vec![1, 2, 3, 4]);
            world.insert_resource(crate::equipment::Equipment::default());
            world.insert_resource(crate::progression::Progression::default());
            world.insert_resource(crate::appearance::Appearance::default());
            let mut inventory = Inventory::default();
            inventory.add_gold(990_999);
            for id in 1..=20 {
                inventory.add_item(id, 1);
            }
            world.insert_resource(inventory);
            crate::events::message_boxes(
                &[amnezia_data::EventCommand {
                    code: 10130,
                    indent: 0,
                    string: "Ron".into(),
                    params: vec![6, 0, 0],
                }],
                &mut world.resource_mut::<crate::dialogue::Dialogue>().face,
            );
            open(world, (1..=20).collect(), 0, true, true);
        }
        950 => {
            world.resource_mut::<crate::dialogue::Dialogue>().face = default();
            world.insert_resource(Inventory::default());
            open(world, vec![7], 1, true, false);
        }
        1000 => open(world, vec![], 2, false, true),
        1040 => open(world, vec![], 2, true, true),
        1080 => open(world, vec![], 2, true, false),
        1110 => {
            world
                .query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>()
                .single_mut(world)
                .unwrap()
                .resolution
                .set(1001.0, 751.0);
        }
        1150 => {
            assert!(!world.resource::<ShopOpen>().0);
            let mut inventory = Inventory::default();
            inventory.add_gold(990_999);
            inventory.add_item(1, 1);
            world.insert_resource(inventory);
        }
        _ => {}
    }
    match frame {
        780 => Some("shop-portrait-command"),
        795 => Some("shop-equipped"),
        804 => Some("shop-page"),
        812 => Some("shop-scroll-first"),
        813 => Some("shop-scroll-half"),
        815 => Some("shop-scroll-done"),
        827 => Some("shop-scroll-up-first"),
        828 => Some("shop-scroll-up-half"),
        830 => Some("shop-scroll-up-done"),
        845 => Some("shop-sell-columns"),
        922 => Some("shop-sell-scroll-first"),
        923 => Some("shop-sell-scroll-half"),
        925 => Some("shop-sell-scroll-done"),
        935 => Some("shop-portrait-regreeting"),
        970 => Some("shop-disabled-buy"),
        1020 => Some("shop-empty-sell"),
        1060 => Some("shop-third-style"),
        1100 => Some("shop-empty-buy"),
        1120 => Some("shop-resized"),
        _ => None,
    }
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0.load(Ordering::Relaxed), 26);
    assert!(!world.resource::<ShopOpen>().0);
    info!("shop layout: 26 complete images / 1996800 reference pixels verified");
}
