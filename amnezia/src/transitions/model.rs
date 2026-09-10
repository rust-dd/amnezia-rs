use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Fade = 0,
    Zoom = 16,
    Mosaic = 17,
    Cut = 19,
    None = 20,
}

impl Kind {
    pub(super) fn frames(self) -> u32 {
        match self {
            Self::Fade => 35,
            Self::Zoom | Self::Mosaic => 41,
            Self::Cut => 1,
            Self::None => 0,
        }
    }
}

pub(super) struct Effect {
    pub kind: Kind,
    pub erase: bool,
    pub from_erased: bool,
    pub duration: u32,
    pub center: IVec2,
    pub offsets: Vec<u32>,
}

impl Effect {
    pub fn new(kind: Kind, erase: bool, from_erased: bool, center: IVec2) -> Self {
        let mut rng = crate::interpreter::EventRng::default();
        Self {
            kind,
            erase,
            from_erased,
            duration: kind.frames(),
            center: center.clamp(IVec2::ZERO, IVec2::new(320, 240)),
            offsets: if kind == Kind::Mosaic {
                (0..kind.frames())
                    .map(|frame| (rng.next_u64() % (frame + 1) as u64) as u32)
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    pub fn fade_alpha(&self, frame: u32) -> u32 {
        ((frame + 1) * 255 / self.duration.saturating_sub(2).max(1)).min(255)
    }

    pub fn mosaic(&self, frame: u32) -> (u32, u32) {
        let index = if self.erase {
            frame
        } else {
            self.duration - frame - 1
        };
        (index + 1, self.offsets[index as usize])
    }

    pub fn zoom_rect(&self, frame: u32) -> IVec4 {
        let last = self.duration as i32 - 1;
        let step = if self.erase {
            frame as i32
        } else {
            last - frame as i32
        }
        .min(last - 1);
        let axis = |length: i32, center: i32| {
            let low = length / 4;
            let high = length * 3 / 4;
            let mut position = center.clamp(low, high) * step / last;
            let size = length * (last - step) / last;
            let edge = if center < low {
                last * center / low - last
            } else if center > high {
                last * (center - high) / (length - high)
            } else {
                0
            };
            if edge != 0 && step > 0 {
                let fixed_position = position * edge.abs() / step;
                let fixed_size = length * (last - edge.abs()) / last;
                position += if step < edge.abs() {
                    i32::from(edge > 0) * (length - size) - position
                } else if edge > 0 {
                    length - fixed_position - fixed_size
                } else {
                    -fixed_position
                };
            }
            (position, size)
        };
        let (x, width) = axis(320, self.center.x);
        let (y, height) = axis(240, self.center.y);
        IVec4::new(x, y, width, height)
    }
}
