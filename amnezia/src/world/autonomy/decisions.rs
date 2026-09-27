use super::*;

#[derive(Debug, PartialEq)]
pub(super) enum Decision {
    Move(u32),
    Cycle(u32),
    Idle(u32),
}

impl AutoMove {
    pub(super) fn decide(
        &mut self,
        direction: u32,
        hero_delta: (i32, i32),
        visible: bool,
        maximum: u32,
    ) -> Decision {
        match self.move_type {
            1 => match next_rand(&mut self.rng) % 10 {
                0..=2 => Decision::Move(direction),
                3..=4 => Decision::Move((direction + 3) % 4),
                5..=6 => Decision::Move((direction + 1) % 4),
                7 => Decision::Move(reverse(direction)),
                _ => Decision::Idle(next_rand(&mut self.rng) % (maximum + 1)),
            },
            2 | 3 => {
                let primary = if self.move_type == 2 {
                    DIR_DOWN
                } else {
                    DIR_RIGHT
                };
                Decision::Cycle(if direction == reverse(primary) {
                    direction
                } else {
                    primary
                })
            }
            4 | 5 => {
                let direction = if !visible {
                    next_rand(&mut self.rng) % 4
                } else {
                    match next_rand(&mut self.rng) % 10 {
                        0 => direction,
                        1 => next_rand(&mut self.rng) % 4,
                        _ if self.move_type == 4 => toward(hero_delta),
                        _ => reverse(toward(hero_delta)),
                    }
                };
                Decision::Move(direction)
            }
            _ => unreachable!("stationary and custom movement have no autonomous decision"),
        }
    }
}

fn toward((dx, dy): (i32, i32)) -> u32 {
    if dx.abs() > dy.abs() {
        if dx > 0 { DIR_RIGHT } else { DIR_LEFT }
    } else if dy < 0 {
        DIR_UP
    } else {
        DIR_DOWN
    }
}

pub(super) fn visible((x, y): (i32, i32)) -> bool {
    (-32..=352).contains(&x) && (-32..=272).contains(&y)
}

#[cfg(test)]
mod tests;
