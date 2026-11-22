// The side profile panel: the published flight re-projected into screen
// space right of the crosshair — a length × height plot that stays readable
// at every aim angle (the world-space arc is depth-tested geometry and
// collapses onto the view axis when the shot flies dead-ahead). The arc and
// the ground track stay as they are; this is the second, always-readable
// view, with the eye level through the crosshair's height.
//
// The host is the hook that fires after each vanilla HUD layer's own draw:
// only the crosshair's pass paints, which is where the old hud hook lived.
fn trajectory_hud(
    game: &crate::game::Game<'_>,
    layer: &str,
    draw: &mut HudBatch,
) {
    use trajectory_preview as view;
    if layer != "minecraft:crosshair" {
        return;
    }
    // The panel draws over whatever is under it: keep it to bare playing —
    // over an inventory it would be a sticker on someone else's screen.
    if game.screen != crate::game::Screen::Playing
        || game.book.is_some()
        || game.sign_edit.is_some()
        || game.link_prompt.is_some()
        || game.command_prompt.is_some()
        || game.code_of_conduct.is_some()
    {
        return;
    }
    let Some(preview) = view::held() else {
        return;
    };
    let Some(eye) = game.eye_position() else {
        return;
    };
    let frame = view::panel(draw.width, draw.height);
    let yaw = f64::from(game.yaw);
    let flight: Vec<[f64; 2]> = preview
        .points
        .iter()
        .map(|&at| view::profile(yaw, eye, at))
        .collect();
    let ground: Vec<[f64; 2]> = preview
        .ground
        .iter()
        .map(|&at| view::profile(yaw, eye, at))
        .collect();
    let scale = view::scale_for(&frame, &flight, &ground);
    let mapped = |at: [f64; 2]| view::to_panel(&frame, scale, at);
    // The box and the eye level through it.
    game.native_fill(
        draw,
        [frame.x0, frame.y0, frame.w, frame.h],
        view::PANEL_BG,
    );
    let level = view::eye_y(&frame);
    for at in view::stroke(
        &[
            [frame.x0 + view::PANEL_PAD, level],
            [frame.x0 + frame.w - view::PANEL_PAD, level],
        ],
        view::LINE_SPACING,
    ) {
        game.native_fill(
            draw,
            [at[0], at[1], view::DOT, view::DOT],
            view::PANEL_EYE,
        );
    }
    // The ground track under the flight, then the dashed flight over it.
    let ground_px: Vec<[f32; 2]> = ground.iter().map(|&at| mapped(at)).collect();
    for at in view::stroke(&ground_px, view::LINE_SPACING) {
        game.native_fill(
            draw,
            [at[0], at[1], view::DOT, view::DOT],
            view::PANEL_GROUND,
        );
    }
    let flight_px: Vec<[f32; 2]> = flight.iter().map(|&at| mapped(at)).collect();
    for at in view::stroke(&flight_px, view::DASH_SPACING) {
        game.native_fill(
            draw,
            [at[0], at[1], view::DOT, view::DOT],
            view::PANEL_FLIGHT,
        );
    }
    // Where the flight ends — the impact when there is one — the marker and
    // the range label.
    let end = preview
        .impact
        .map(|at| view::profile(yaw, eye, at))
        .unwrap_or_else(|| *flight.last().unwrap_or(&[0.0, 0.0]));
    let end_px = mapped(end);
    game.native_fill(
        draw,
        [end_px[0] - 1.5, end_px[1] - 1.5, 3.0, 3.0],
        view::PANEL_IMPACT,
    );
    let label = view::range_label(end[0].max(0.0));
    game.native_text(
        draw,
        &label,
        [end_px[0] + 3.0, end_px[1] - 9.0],
        view::PANEL_LABEL,
        true,
    );
}
