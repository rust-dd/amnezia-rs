use super::*;

#[test]
fn none_preserves_erasure_and_repeated_erase_is_skipped() {
    let mut state = Transition::default();
    assert!(state.start(Kind::None, true, 0, IVec2::ZERO));
    assert!(!state.erased);
    assert!(!state.busy());
    assert!(state.start(Kind::Cut, true, 0, IVec2::ZERO));
    assert!(state.busy());
    state.advance(1);
    assert!(state.erased);
    let serial = state.serial;
    assert!(state.start(Kind::Mosaic, true, 1, IVec2::ZERO));
    assert!(!state.busy());
    assert_eq!(state.serial, serial);
    assert!(state.start(Kind::None, false, 1, IVec2::ZERO));
    assert!(state.erased);
    state.clear();
    assert!(!state.erased);
    assert_eq!(state.serial, serial);
}

#[test]
fn every_transition_keeps_frame_zero_and_finishes_at_original_duration() {
    for kind in [Kind::Fade, Kind::Zoom, Kind::Mosaic, Kind::Cut] {
        let mut state = Transition::default();
        assert!(state.start(kind, true, u32::MAX - 2, IVec2::new(160, 120)));
        assert_eq!(state.frame, 0);
        assert!(!state.start(Kind::Fade, false, 0, IVec2::ZERO));
        for frame in 0..kind.frames() {
            state.advance((u32::MAX - 2).wrapping_add(frame));
            assert!(state.busy(), "{kind:?} at {frame}");
            assert_eq!(state.frame, frame);
        }
        state.advance((u32::MAX - 2).wrapping_add(kind.frames()));
        assert!(!state.busy());
        assert!(state.erased);
    }
}

#[test]
fn transition_duration_does_not_depend_on_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut frames = crate::timing::GameFrames::default();
        let mut state = Transition::default();
        state.start(Kind::Mosaic, true, 0, IVec2::new(160, 120));
        for _ in 0..fps {
            frames.advance(1.0 / fps as f64);
            state.advance(frames.frame);
            assert_eq!(state.busy(), frames.frame < 41, "{fps} FPS");
        }
    }
}

#[test]
fn fade_quantization_and_mosaic_reverse_match_the_reference() {
    let fade = Effect::new(Kind::Fade, true, false, IVec2::ZERO);
    for (frame, alpha) in [(0, 7), (1, 15), (16, 131), (31, 247), (32, 255), (34, 255)] {
        assert_eq!(fade.fade_alpha(frame), alpha);
    }
    let mut mosaic = Effect::new(Kind::Mosaic, true, false, IVec2::ZERO);
    for frame in 0..41 {
        let (size, offset) = mosaic.mosaic(frame);
        assert_eq!(size, frame + 1);
        assert!(offset < size);
    }
    mosaic.offsets = (0..41).collect();
    mosaic.erase = false;
    assert_eq!(mosaic.mosaic(0), (41, 40));
    assert_eq!(mosaic.mosaic(12), (29, 28));
    assert_eq!(mosaic.mosaic(40), (1, 0));
}

#[test]
fn zoom_keeps_original_hero_edge_correction_and_nonzero_last_crop() {
    for (center, frame, expected) in [
        (IVec2::new(160, 120), 0, IVec4::new(0, 0, 320, 240)),
        (IVec2::new(160, 120), 20, IVec4::new(80, 60, 160, 120)),
        (IVec2::new(160, 120), 40, IVec4::new(156, 117, 8, 6)),
        (IVec2::ZERO, 39, IVec4::new(0, 0, 8, 6)),
        (IVec2::new(320, 240), 39, IVec4::new(312, 234, 8, 6)),
        (IVec2::new(16, 200), 20, IVec4::new(0, 110, 160, 120)),
        (IVec2::new(300, 16), 39, IVec4::new(294, 14, 8, 6)),
    ] {
        let mut effect = Effect::new(Kind::Zoom, true, false, center);
        assert_eq!(effect.zoom_rect(frame), expected, "{center} frame {frame}");
        effect.erase = false;
        assert_eq!(effect.zoom_rect(40 - frame), expected);
    }
}
