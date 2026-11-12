// Every rendered frame, while a world is visible: the published flight
// becomes depth-tested gizmo lines — one segment per simulated step, plus
// the impact marker. The simulation runs per game tick, so between ticks
// the same lines draw again.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_frame(
    game: &crate::game::Game<'_>,
    lines: &mut Vec<horizon_client::gizmos::Line>,
) {
    use crate::game::trajectory_state as state;
    if !game.native_on(state::ID) {
        return;
    }
    if let Some(preview) = state::held() {
        lines.extend(state::build_lines(&preview));
    }
}
