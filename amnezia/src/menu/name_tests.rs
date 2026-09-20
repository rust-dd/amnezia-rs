use super::*;
use crate::equipment::Equipment;
use crate::progression::Progression;
use crate::state::Party;
use crate::terms::Terms;
use crate::text::HeroName;
use crate::vitals::Vitals;

#[test]
fn renamed_hero_appears_in_every_menu_and_target_view() {
    let mut data = testkit::data();
    let mut tiffany = testkit::actor();
    tiffany.id = 2;
    tiffany.name = "Tiffany".into();
    data.actors.push(tiffany);
    let mut party = Party::default();
    party.restore(vec![2, 1, 999]);
    let progression = Progression::default();
    let vitals = Vitals::default();
    let equipment = Equipment::default();
    let terms = Terms::default();
    for renamed in ["Áron", "", "Tiffany"] {
        let hero_name = HeroName(renamed.into());
        let rows = render::members(&hero_name, &data, &party, &progression, &vitals);
        assert_eq!(rows[0].name, "Tiffany");
        assert_eq!(rows[1].name, renamed);
        assert_eq!(rows[2].name, "#999");
        for screen in [
            MenuScreen::ItemTarget {
                item_id: testkit::ITEM_HERB,
                cursor: 1,
            },
            MenuScreen::SkillList {
                member: 1,
                cursor: 0,
            },
            MenuScreen::SkillTarget {
                member: 1,
                skill_id: 1,
                cursor: 1,
            },
            MenuScreen::Equip {
                member: 1,
                slot: 0,
                picking: None,
            },
            MenuScreen::Equip {
                member: 1,
                slot: 0,
                picking: Some(0),
            },
            MenuScreen::Status { member: 1 },
        ] {
            let view = render::content(
                &hero_name,
                screen,
                &data,
                &party,
                &progression,
                &vitals,
                &equipment,
                &terms,
            );
            let text = if matches!(
                screen,
                MenuScreen::ItemTarget { .. } | MenuScreen::SkillTarget { .. }
            ) {
                view::target::party_text(
                    &rows,
                    &terms,
                    &crate::font::bitmap::BitmapFont::from_id(0),
                )
                .runs
                .into_iter()
                .map(|run| run.text)
                .collect::<Vec<_>>()
                .join("\n")
            } else if matches!(screen, MenuScreen::SkillList { .. }) {
                view::skill_list::status(
                    &rows[1],
                    &terms,
                    &crate::font::bitmap::BitmapFont::from_id(0),
                )
                .runs
                .into_iter()
                .map(|run| run.text)
                .collect::<Vec<_>>()
                .join("\n")
            } else if matches!(screen, MenuScreen::Equip { .. }) {
                view::equipment::status(
                    renamed,
                    [1; 4],
                    None,
                    &terms,
                    &crate::font::bitmap::BitmapFont::from_id(0),
                )
                .runs
                .into_iter()
                .map(|run| run.text)
                .collect::<Vec<_>>()
                .join("\n")
            } else {
                view.text
            };
            assert!(!text.contains("Ron"), "{screen:?}: {text}");
            assert!(text.contains(renamed), "{screen:?}: {text}");
        }
        assert_eq!(data.actor(1).unwrap().name, "Ron");
    }
}
