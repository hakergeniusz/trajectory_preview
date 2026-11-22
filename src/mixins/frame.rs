// Every rendered frame: the published flight becomes depth-tested gizmo
// lines — one segment per simulated step, plus the impact marker. The
// simulation runs per game tick, so between ticks the same lines draw
// again.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_frame(lines: &mut Vec<horizon_client::gizmos::Line>) {
    if let Some(preview) = trajectory_preview::held() {
        lines.extend(trajectory_preview::build_lines(&preview).into_iter().map(
            |seg| horizon_client::gizmos::Line {
                start: seg.start,
                end: seg.end,
                color: seg.color,
                width: seg.width,
            },
        ));
    }
}
