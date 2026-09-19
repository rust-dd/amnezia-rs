use super::*;
use crate::font::bitmap::{CRITICAL, DEFAULT, Run};
use crate::menu::testkit;
use crate::terms::Terms;

#[test]
fn target_windows_have_original_sizes_portraits_and_clipped_party_contents() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|root| spawn(root, &Handle::default()));
        });
    app.update();
    let world = app.world_mut();
    let root = world
        .query::<(Entity, &Part)>()
        .iter(world)
        .find(|(_, part)| matches!(part, Part::Root))
        .unwrap()
        .0;
    let rectangles = world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .map(|entity| {
            let node = world.get::<Node>(entity).unwrap();
            (node.left, node.top, node.width, node.height)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rectangles,
        [
            (Val::Px(0.0), Val::Px(0.0), Val::Px(408.0), Val::Px(96.0)),
            (Val::Px(0.0), Val::Px(96.0), Val::Px(408.0), Val::Px(96.0)),
            (Val::Px(408.0), Val::Px(0.0), Val::Px(552.0), Val::Px(720.0)),
        ]
    );
    for (part, node, parent) in world.query::<(&Part, &Node, &ChildOf)>().iter(world) {
        if let Part::Face(index) = part {
            assert_eq!(
                (node.left, node.top, node.width, node.height),
                (
                    Val::Px(0.0),
                    Val::Px(*index as f32 * 174.0),
                    Val::Px(144.0),
                    Val::Px(144.0)
                )
            );
            assert_eq!(
                world.get::<Node>(parent.parent()).unwrap().overflow,
                Overflow::clip()
            );
        }
    }
    assert_eq!(world.query::<&PixelText>().iter(world).count(), 3);
    assert_eq!(world.query::<&Text>().iter(world).count(), 0);
}

#[test]
fn target_heading_and_count_use_the_database_labels_and_separate_value_alignment() {
    let font = BitmapFont::from_id(0);
    let terms = Terms(crate::assets::load_ron(&format!(
        "{}/terms.ron",
        crate::assets::asset_root()
    )));
    let mut selection = model::Selection {
        key: model::Key::Item(105),
        name: "Gyógyital".into(),
        value: 12,
        selected: 0,
        whole_party: false,
    };
    assert_eq!(
        text::name(&selection).runs,
        [Run::new("Gyógyital", 0, 2, DEFAULT)]
    );
    assert_eq!(
        text::value(&selection, &terms, &font).runs,
        [
            Run::new("Tárgyak", 0, 2, 1),
            Run::new("12", 108, 2, DEFAULT)
        ]
    );
    selection.key = model::Key::Skill(0, 9);
    selection.value = 5;
    assert_eq!(
        text::value(&selection, &terms, &font).runs,
        [Run::new("Ár", 0, 2, 1), Run::new("5", 114, 2, DEFAULT)]
    );
}

#[test]
fn party_bitmap_keeps_names_vitals_and_critical_colors_in_original_columns() {
    let mut vitals = crate::vitals::Vitals::default();
    vitals.set(1, 7, 5);
    let members = render::members(
        &crate::text::HeroName("Áron".into()),
        &testkit::data(),
        &crate::state::Party::default(),
        &crate::progression::Progression::default(),
        &vitals,
    );
    let terms = Terms(crate::assets::load_ron(&format!(
        "{}/terms.ron",
        crate::assets::asset_root()
    )));
    let pixels = party_text(&members, &terms, &BitmapFont::from_id(0));
    assert_eq!(pixels.size, UVec2::new(168, 224));
    assert!(pixels.runs.contains(&Run::new("Áron", 56, 2, DEFAULT)));
    assert!(pixels.runs.contains(&Run::new("Sz", 56, 18, 1)));
    assert!(pixels.runs.contains(&Run::new("7", 138, 18, CRITICAL)));
    assert!(pixels.runs.contains(&Run::new("63", 156, 18, DEFAULT)));
    assert!(pixels.runs.contains(&Run::new("5", 138, 34, CRITICAL)));
    assert!(
        pixels
            .runs
            .iter()
            .all(|run| !run.text.contains("▶") && !run.text.contains("Esc"))
    );
}
