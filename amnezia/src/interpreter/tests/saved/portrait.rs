use super::*;
use crate::events::MessageFace;

#[test]
fn an_active_foreground_portrait_survives_a_real_slot_without_repeating_the_save() {
    let (mut app, path) = app("active-portrait");
    app.world_mut().resource_mut::<RunningEvent>().start(
        0,
        vec![
            EventCommand {
                string: "Ron".into(),
                ..cmd(10130, 0, vec![6, 0, 0])
            },
            cmd(11910, 0, vec![]),
            EventCommand {
                string: "Saved portrait".into(),
                ..cmd(10110, 0, vec![])
            },
        ],
    );
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().boxes[0].face.as_deref(),
        Some("Ron")
    );
    let bytes = std::fs::read(&path).unwrap();
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
    load(&mut app);
    assert_eq!(
        app.world().resource::<Dialogue>().face.graphic(),
        Some(("Ron", 6))
    );
    assert!(!app.world().resource::<Dialogue>().active);
    resume(&mut app);
    let dialogue = app.world().resource::<Dialogue>();
    assert_eq!(dialogue.boxes[0].face.as_deref(), Some("Ron"));
    assert_eq!(dialogue.boxes[0].face_index, 6);
    app.world_mut().resource_mut::<Dialogue>().close();
    app.update();
    assert_eq!(
        app.world().resource::<Dialogue>().face,
        MessageFace::default()
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_file(path).unwrap();
}
