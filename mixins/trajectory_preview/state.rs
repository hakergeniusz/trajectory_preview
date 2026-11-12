// The preview's shared state: the game-side simulation publishes each game
// tick, the frame injection reads and draws each rendered frame. One
// mutex-guarded Option, so a reader never blocks the writer beyond a copy.
// Copyright (c) 2026 hakergeniusz. MIT.
use horizon_client::gizmos::Line;
use std::sync::Mutex;

/// The flight line: white, two pixels, like a gizmo stroke.
pub const LINE_COLOR: u32 = 0xFFFF_FFFF;
pub const LINE_WIDTH: f32 = 2.0;
/// The impact marker's red, the same width.
pub const MARK_COLOR: u32 = 0xFFFF_3F3F;
/// The impact marker's half-size, in blocks.
pub const MARK_SIZE: f64 = 0.15;

/// One simulated flight: a position per step, and where it lands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    pub points: Vec<[f64; 3]>,
    pub impact: Option<[f64; 3]>,
}

static PREVIEW: Mutex<Option<Preview>> = Mutex::new(None);

/// The game tick publishes; a frame reads the latest.
pub fn publish(preview: Preview) {
    if let Ok(mut held) = PREVIEW.lock() {
        *held = Some(preview);
    }
}

/// Nothing in the hand (or nothing live): no lines this frame.
pub fn clear() {
    if let Ok(mut held) = PREVIEW.lock() {
        *held = None;
    }
}

/// The latest preview, if any.
pub fn held() -> Option<Preview> {
    PREVIEW.lock().ok().and_then(|held| held.clone())
}

/// A cube's wireframe around `at`: twelve edges, the impact marker.
pub fn mark(at: [f64; 3]) -> Vec<Line> {
    let s = MARK_SIZE;
    let corners = [
        [at[0] - s, at[1] - s, at[2] - s],
        [at[0] + s, at[1] - s, at[2] - s],
        [at[0] - s, at[1] + s, at[2] - s],
        [at[0] + s, at[1] + s, at[2] - s],
        [at[0] - s, at[1] - s, at[2] + s],
        [at[0] + s, at[1] - s, at[2] + s],
        [at[0] - s, at[1] + s, at[2] + s],
        [at[0] + s, at[1] + s, at[2] + s],
    ];
    let edges = [
        (0, 1),
        (1, 3),
        (3, 2),
        (2, 0),
        (4, 5),
        (5, 7),
        (7, 6),
        (6, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    edges
        .iter()
        .map(|(a, b)| Line {
            start: corners[*a],
            end: corners[*b],
            color: MARK_COLOR,
            width: LINE_WIDTH,
        })
        .collect()
}

/// The flight's lines: one segment per simulated step, plus the marker.
pub fn build_lines(preview: &Preview) -> Vec<Line> {
    let mut lines = Vec::with_capacity(
        preview
            .points
            .len()
            .saturating_sub(1)
            .saturating_add(usize::from(preview.impact.is_some()) * 12),
    );
    for pair in preview.points.windows(2) {
        lines.push(Line {
            start: pair[0],
            end: pair[1],
            color: LINE_COLOR,
            width: LINE_WIDTH,
        });
    }
    if let Some(at) = preview.impact {
        lines.extend(mark(at));
    }
    lines
}
