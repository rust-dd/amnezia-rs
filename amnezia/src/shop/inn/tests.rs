use super::*;
use crate::audio::{AudioRequest, CurrentBgm, SystemMusic};
use crate::choice::Choice;
use crate::dialogue::Dialogue;
use crate::inputnumber::InputNumber;
use crate::shop::{ShopOpen, ShopOutcome, ShopRequest};
use crate::state::{Inventory, Party};
use crate::terms::Terms;
use crate::timing::GameFrames;
use crate::transitions::Transition;
use crate::vitals::Vitals;
use bevy::ecs::system::RunSystemOnce;

mod music;
mod timing;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<State>()
        .init_resource::<ShopOpen>()
        .init_resource::<ShopOutcome>()
        .init_resource::<Dialogue>()
        .init_resource::<Choice>()
        .init_resource::<InputNumber>()
        .init_resource::<Inventory>()
        .init_resource::<Party>()
        .init_resource::<Vitals>()
        .init_resource::<GameFrames>()
        .init_resource::<Transition>()
        .init_resource::<SystemMusic>()
        .insert_resource(crate::text::HeroName("Ron".into()))
        .init_resource::<crate::state::Variables>()
        .insert_resource(CurrentBgm::with_track("Field", 0.66, 1.25))
        .insert_resource(Terms(crate::assets::load_ron(&format!(
            "{}/terms.ron",
            crate::assets::asset_root()
        ))))
        .add_message::<ShopRequest>()
        .add_message::<AudioRequest>()
        .add_systems(Update, (flow::open, flow::accept, flow::advance).chain());
    app.update();
    app.world_mut().resource_mut::<Inventory>().add_gold(100);
    app.world_mut().resource_mut::<Vitals>().set(1, 2, 0);
    app.world_mut().resource_mut::<Vitals>().set(2, 3, 1);
    app
}

fn open(app: &mut App, cost: i32) {
    app.world_mut()
        .write_message(ShopRequest::ShowInn { cost, inn_type: 1 });
    app.update();
}

fn heard(app: &mut App) -> Vec<AudioRequest> {
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .collect()
}

#[test]
fn paid_inn_uses_four_typed_message_rows_and_the_remembered_face() {
    let mut app = app();
    crate::events::message_boxes(
        &[amnezia_data::EventCommand {
            code: 10130,
            indent: 0,
            string: "Chara1".into(),
            params: vec![2],
        }],
        &mut app.world_mut().resource_mut::<Dialogue>().face,
    );
    open(&mut app, 30);
    let dialogue = app.world().resource::<Dialogue>();
    assert!(dialogue.active);
    assert_eq!(dialogue.boxes[0].face.as_deref(), Some("Chara1"));
    assert_eq!(dialogue.boxes[0].face_index, 2);
    assert_eq!(
        dialogue.boxes[0].lines,
        ["Egy éjszaka  30GP !", "Bejegyezhetem mára?", "Igen", "Nem"]
    );
    assert!(!app.world().resource::<Choice>().active());
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    let choice = app.world().resource::<Choice>();
    assert_eq!(choice.options, ["Igen", "Nem"]);
    assert_eq!(choice.cancel_type, 5);
    assert!(choice.disabled.is_empty());
    assert!(heard(&mut app).is_empty());
}

#[test]
fn unaffordable_inn_disables_only_accept_without_changing_gold_or_health() {
    let mut app = app();
    open(&mut app, 200);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    assert_eq!(app.world().resource::<Choice>().disabled, [0]);
    assert_eq!(app.world().resource::<Inventory>().gold(), 100);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
    assert!(heard(&mut app).is_empty());
}

#[test]
fn replacement_clears_the_old_choice_or_number_and_types_a_fresh_question() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<Choice>()
        .open(vec!["Old".into()], 5, 2);
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    app.world_mut().resource_mut::<InputNumber>().open(4, 7);
    open(&mut app, 30);
    assert!(!app.world().resource::<Choice>().active());
    assert!(app.world().resource::<Choice>().result.is_none());
    assert!(!app.world().resource::<InputNumber>().active());
    assert_eq!(app.world().resource::<Inventory>().gold(), 100);
    assert!(app.world().resource::<State>().prompting());
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    assert_eq!(app.world().resource::<Choice>().options, ["Igen", "Nem"]);
}

#[test]
fn affordability_is_captured_when_the_original_inn_question_opens() {
    let mut app = app();
    open(&mut app, 30);
    app.world_mut().resource_mut::<Inventory>().remove_gold(100);
    crate::dialogue::testing::finish_prompt_text(app.world_mut());
    assert!(app.world().resource::<Choice>().disabled.is_empty());
    assert_eq!(app.world().resource::<State>().gold, 100);
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    app.world_mut().resource_mut::<Dialogue>().close();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(app.world().resource::<State>().resting());
    assert_eq!(app.world().resource::<Inventory>().gold(), 0);
}

#[test]
fn no_and_escape_leave_without_payment_heal_or_audio_changes() {
    for result in [1, 4] {
        let mut app = app();
        open(&mut app, 30);
        app.world_mut().resource_mut::<Choice>().result = Some(result);
        app.world_mut().resource_mut::<Dialogue>().close();
        app.world_mut().run_system_once(flow::advance).unwrap();
        assert!(!app.world().resource::<ShopOpen>().0);
        assert!(!app.world().resource::<ShopOutcome>().transacted);
        assert_eq!(app.world().resource::<Inventory>().gold(), 100);
        assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
        assert!(heard(&mut app).is_empty());
    }
}

#[test]
fn accepted_stay_charges_before_rest_but_heals_only_party_at_the_show_transition() {
    let mut app = app();
    open(&mut app, 30);
    app.world_mut().resource_mut::<Choice>().result = Some(0);
    app.world_mut().resource_mut::<Dialogue>().close();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(app.world().resource::<Inventory>().gold(), 70);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), Some((2, 0)));
    assert!(!app.world().resource::<ShopOutcome>().transacted);
    assert_eq!(
        heard(&mut app),
        [AudioRequest::FadeOutBgm { duration: 0.8 }]
    );
    assert!(app.world().resource::<Transition>().busy());
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), None);
    assert_eq!(app.world().resource::<Vitals>().get_stored(2), Some((3, 1)));
    assert!(app.world().resource::<ShopOutcome>().transacted);
    assert!(app.world().resource::<ShopOpen>().0);
    assert!(!app.world().resource::<Transition>().event_erased);
    assert!(app.world().resource::<Transition>().busy());
    assert_eq!(
        heard(&mut app),
        [
            AudioRequest::StopBgm,
            AudioRequest::Bgm {
                name: "Field".into(),
                volume: 0.66,
                speed: 1.25,
                fade_in: 0.0
            }
        ]
    );
    app.world_mut().resource_mut::<Transition>().clear();
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(!app.world().resource::<ShopOpen>().0);
    assert_eq!(app.world().resource::<Inventory>().gold(), 70);
}

#[test]
fn free_stay_skips_prompt_and_prior_erasure_never_suppresses_the_return_fade() {
    let mut app = app();
    app.world_mut().resource_mut::<Transition>().hold_black();
    app.world_mut().resource_mut::<Transition>().event_erased = true;
    open(&mut app, 0);
    assert!(!app.world().resource::<Dialogue>().active);
    assert!(!app.world().resource::<Choice>().active());
    assert_eq!(app.world().resource::<Inventory>().gold(), 100);
    app.world_mut().run_system_once(flow::advance).unwrap();
    assert!(matches!(
        app.world().resource::<State>().phase,
        Phase::FadeIn
    ));
    assert!(app.world().resource::<Transition>().busy());
    assert!(!app.world().resource::<Transition>().event_erased);
    assert_eq!(app.world().resource::<Vitals>().get_stored(1), None);
}
