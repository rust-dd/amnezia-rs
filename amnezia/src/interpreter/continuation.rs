use super::frame::MAX_STEPS_PER_FRAME;
use super::parallel::ParallelSource;
use bevy::prelude::*;

pub(crate) mod smoke;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AsyncOp {
    Transition {
        kind: i32,
        erase: bool,
        center: IVec2,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Owner {
    Parallel(ParallelSource),
    Foreground,
}

impl Owner {
    pub(super) fn interpreter(source: Option<ParallelSource>) -> Self {
        source.map_or(Self::Foreground, Self::Parallel)
    }
}

struct Pending {
    owner: Owner,
    remaining: usize,
}

#[derive(Default, Clone, Copy)]
enum Pass {
    #[default]
    Fresh,
    Waiting,
    Resume(Owner),
}

/// An asynchronous operation resumes the interrupted map visit, not a new frame.
#[derive(Resource, Default)]
pub(crate) struct Continuation {
    pending: Option<Pending>,
    pass: Pass,
}

impl Continuation {
    pub(super) fn resume_owner(&self) -> Option<Owner> {
        match self.pass {
            Pass::Resume(owner) => Some(owner),
            _ => None,
        }
    }

    pub(crate) fn waiting(&self) -> bool {
        matches!(self.pass, Pass::Waiting)
    }

    pub(crate) fn characters_paused(&self, frame_waiting: bool) -> bool {
        match self.pass {
            Pass::Fresh => frame_waiting,
            Pass::Waiting | Pass::Resume(Owner::Foreground) => true,
            Pass::Resume(Owner::Parallel(_)) => false,
        }
    }

    pub(crate) fn tail_paused(&self, frame_waiting: bool) -> bool {
        match self.pass {
            Pass::Fresh => frame_waiting,
            Pass::Waiting => true,
            Pass::Resume(_) => false,
        }
    }
}

pub(super) fn begin(world: &mut World) {
    let busy = world.resource::<crate::transitions::Transition>().busy();
    let mut state = world.resource_mut::<Continuation>();
    state.pass = state.pending.as_ref().map_or(Pass::Fresh, |pending| {
        if busy {
            Pass::Waiting
        } else {
            Pass::Resume(pending.owner)
        }
    });
}

pub(super) fn budget(world: &mut World, owner: Owner) -> usize {
    let mut state = world.resource_mut::<Continuation>();
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.owner == owner)
        && !state.waiting()
    {
        state.pending.take().unwrap().remaining
    } else {
        MAX_STEPS_PER_FRAME
    }
}

pub(super) fn complete_parallel(world: &mut World) {
    let mut state = world.resource_mut::<Continuation>();
    if matches!(state.pass, Pass::Resume(Owner::Parallel(_))) {
        state.pending = None;
    }
}

pub(super) fn suspend(world: &mut World, owner: Owner, remaining: usize, op: AsyncOp) -> bool {
    world.run_system_cached_with(apply, op).unwrap();
    if !world.resource::<crate::transitions::Transition>().busy() {
        return false;
    }
    let mut state = world.resource_mut::<Continuation>();
    state.pending = Some(Pending { owner, remaining });
    state.pass = Pass::Waiting;
    true
}

fn apply(In(op): In<AsyncOp>, mut transitions: crate::transitions::TransitionIo) {
    match op {
        AsyncOp::Transition {
            kind,
            erase,
            center,
        } => {
            transitions.event(kind, erase, center);
        }
    }
}
