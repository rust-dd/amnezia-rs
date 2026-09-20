use super::NumberState;

pub(super) fn navigate(number: &mut NumberState, [down, up, right, left]: [bool; 4]) -> bool {
    let previous = number.count;
    if right && number.count < number.max {
        number.count += 1;
    } else if left && number.count > 1 {
        number.count -= 1;
    } else if up && number.count < number.max {
        number.count = number.count.saturating_add(10).min(number.max);
    } else if down && number.count > 1 {
        number.count = number.count.saturating_sub(10).max(1);
    }
    number.count != previous
}
