// Every game tick, on the window's game only: simulate the projectile in
// hand and publish its flight for the frame injection to draw.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::trajectory_sim;
    if !game.native_on(crate::game::trajectory_preview::ID) {
        return;
    }
    if !game.native_window() {
        return;
    }
    trajectory_sim::preview(game);
}
