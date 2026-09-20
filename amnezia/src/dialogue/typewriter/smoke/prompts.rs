use super::*;

#[derive(Resource, Default)]
struct Probe(u8);

fn command(code: u32, indent: u32, string: &str, params: Vec<i32>) -> EventCommand {
    EventCommand {
        code,
        indent,
        string: string.into(),
        params,
    }
}

pub(super) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    match frame {
        750 => {
            world.init_resource::<Probe>();
            assert!(!world.resource::<RunningEvent>().active());
            world.resource_mut::<RunningEvent>().start(
                0,
                vec![
                    command(10140, 0, "", vec![1]),
                    command(20140, 0, "Tovább", vec![0]),
                    command(10210, 1, "", vec![0, 9004, 9004, 0]),
                    command(20141, 0, "", Vec::new()),
                ],
            );
        }
        780 => {
            assert!(world.resource::<crate::choice::Choice>().active());
            assert!(!world.resource::<Switches>().get(9004));
            world.resource_mut::<Probe>().0 |= 1;
            return Some("message-choice-input");
        }
        850 => {
            assert!(!world.resource::<crate::choice::Choice>().active());
            assert!(world.resource::<Switches>().get(9004));
            assert!(!world.resource::<RunningEvent>().active());
            world.resource_mut::<Probe>().0 |= 2;
        }
        900 => {
            world
                .resource_mut::<RunningEvent>()
                .start(0, vec![command(10150, 0, "", vec![2, 9005])]);
        }
        950 => {
            let number = world.resource::<crate::inputnumber::InputNumber>();
            assert!(number.active());
            assert_eq!(number.value, 1);
            world.resource_mut::<Probe>().0 |= 4;
            return Some("message-number-input");
        }
        980 => {
            assert!(!world.resource::<crate::inputnumber::InputNumber>().active());
            assert!(!world.resource::<RunningEvent>().active());
            assert_eq!(world.resource::<crate::state::Variables>().get(9005), 1);
            world.resource_mut::<Probe>().0 |= 8;
        }
        _ => {}
    }
    None
}

pub(super) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Probe>().0, 15);
    assert!(!world.resource::<crate::menu::MenuOpen>().0);
    info!("message input: ignored typing keys, cancel, choice and number completion verified");
}
