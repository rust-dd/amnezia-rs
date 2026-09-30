const COMPLETED_SCENES: [u32; 8] = [300, 312, 313, 315, 316, 321, 322, 324];

pub(super) fn switches(phase: &[u32]) -> Vec<(u32, bool)> {
    COMPLETED_SCENES
        .iter()
        .chain(phase)
        .map(|&id| (id, true))
        .collect()
}
