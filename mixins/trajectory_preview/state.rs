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
/// The ground track's gray — the flight's shadow on the terrain below it.
pub const GROUND_COLOR: u32 = 0xFF6E_6E6E;
/// The plumb comb's faint white: short verticals from the aim ray down to
/// the flight. Aiming at the target puts the whole arc in the view's plane,
/// so it projects to the vertical line under the crosshair no matter what —
/// the comb is what makes the drop readable there: each tick hangs below
/// where the aim says the shot should be, and it lengthens as gravity wins.
pub const TICK_COLOR: u32 = 0x66FF_FFFF;
/// A plumb tick every this many blocks of forward travel.
pub const TICK_EVERY: f64 = 2.0;
/// The most plumb ticks one flight draws (a 400-tick flight capped).
pub const MAX_TICKS: usize = 64;
/// The arc draws every other step: a dashed line reads as the flight it is.
pub const DASH: usize = 2;

/// One simulated flight: a position per step, where it lands, the flight's
/// shadow on the terrain (one point per few steps) and the plumb comb
/// (aim-ray point, flight point) pairs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    pub points: Vec<[f64; 3]>,
    pub ground: Vec<[f64; 3]>,
    pub ticks: Vec<([f64; 3], [f64; 3])>,
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

/// The flight's lines: the dashed arc (one segment every [`DASH`] steps),
/// the plumb comb, the ground track, and the marker.
pub fn build_lines(preview: &Preview) -> Vec<Line> {
    let mut lines = Vec::with_capacity(
        (preview.points.len() / DASH)
            .saturating_add(preview.ticks.len())
            .saturating_add(preview.ground.len().saturating_sub(1))
            .saturating_add(usize::from(preview.impact.is_some()) * 12),
    );
    for (i, pair) in preview.points.windows(2).enumerate() {
        if i % DASH != 0 {
            continue;
        }
        lines.push(Line {
            start: pair[0],
            end: pair[1],
            color: LINE_COLOR,
            width: LINE_WIDTH,
        });
    }
    for (aim, flight) in &preview.ticks {
        lines.push(Line {
            start: *aim,
            end: *flight,
            color: TICK_COLOR,
            width: LINE_WIDTH,
        });
    }
    for pair in preview.ground.windows(2) {
        lines.push(Line {
            start: pair[0],
            end: pair[1],
            color: GROUND_COLOR,
            width: LINE_WIDTH,
        });
    }
    if let Some(at) = preview.impact {
        lines.extend(mark(at));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_comb_hangs_between_aim_and_flight() {
        let preview = Preview {
            points: vec![[0.0, 64.0, 0.0], [0.0, 64.0, 1.0], [0.0, 63.5, 2.0]],
            ground: vec![],
            ticks: vec![([0.0, 64.0, 2.0], [0.0, 63.5, 2.0])],
            impact: Some([0.0, 63.0, 3.0]),
        };
        let lines = build_lines(&preview);
        // One dashed arc segment (the second window skips on the dash), one
        // plumb tick, and the impact marker's twelve edges.
        assert_eq!(lines.len(), 1 + 1 + 12);
        let tick = &lines[1];
        assert_eq!(tick.start, [0.0, 64.0, 2.0]);
        assert_eq!(tick.end, [0.0, 63.5, 2.0]);
        assert_eq!(tick.color, TICK_COLOR);
    }
}
