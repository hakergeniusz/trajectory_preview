//! Trajectory Preview, the native-route native mod: where the projectile in
//! hand will land, drawn with the game's own physics.
//!
//! The brain is pure standard library — no game types — so it compiles and
//! tests wherever. The policy table says, from the item in hand, what flies
//! and how: the same speed and pitch offset the game's own release paths
//! use (the bow's draw curve, the trident's ten-tick gate, the throwables'
//! 1.5, the potions' -20 degree lob). The simulation itself runs on the
//! game side (the tick mixin) with the game's own projectile tick against
//! the live world; the frame mixin joins the flight to the depth-tested
//! gizmo lines, and the HUD mixin paints the side-profile panel by the
//! crosshair — a length × height plot that stays readable at every aim
//! angle (the world-space arc collapses onto the view axis dead-ahead).
//!
//! Clean-room: written for Horizon from the game's own public behavior; it
//! copies nothing from any mod.
//!
//! Copyright (c) 2026 hakergeniusz. MIT.

use std::sync::Mutex;

use mod_api::{Hooks, RegistrationContext, Registrar};

/// What is in the hand right now and how it is being used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum What {
    /// A bow drawn for `charge` ticks (the use state's ticks).
    Bow {
        charge: i32,
    },
    /// A charged crossbow; a firework rocket among the loaded projectiles
    /// would fly as a rocket, which this port does not preview.
    Crossbow,
    /// A trident held for `charge` ticks (the release fires from ten).
    Trident {
        charge: u32,
    },
    Snowball,
    Egg,
    EnderPearl,
    SplashPotion,
    LingeringPotion,
    ExperienceBottle,
}

/// How one kind flies: the speed and the pitch offset its throw applies,
/// the numbers the game's own release paths hand `shoot_from_rotation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub speed: f32,
    pub offset: f32,
}

/// The held item that flies on a plain right click, no charge involved.
pub fn from_item(item: &str) -> Option<What> {
    match item {
        "minecraft:snowball" => Some(What::Snowball),
        "minecraft:egg" => Some(What::Egg),
        "minecraft:ender_pearl" => Some(What::EnderPearl),
        "minecraft:splash_potion" => Some(What::SplashPotion),
        "minecraft:lingering_potion" => Some(What::LingeringPotion),
        "minecraft:experience_bottle" => Some(What::ExperienceBottle),
        _ => None,
    }
}

/// The bow's draw curve, `item_use::bow_release_power`'s numbers: a full
/// draw takes twenty ticks, the power grows quadratically to one, and a
/// shot under a tenth of the draw does not fire.
pub fn bow_power(charge: i32) -> Option<f32> {
    if charge < 0 {
        return None;
    }
    let f = charge as f32 / 20.0;
    let power = ((f * f + f * 2.0) / 3.0).min(1.0);
    (power >= 0.1).then_some(power)
}

/// Whether and how the shot flies; None hides the preview.
pub fn plan(what: What) -> Option<Shot> {
    let shot = match what {
        What::Bow { charge } => Shot {
            speed: bow_power(charge)? * 3.0,
            offset: 0.0,
        },
        // TridentItem.releaseUsing: ten ticks to throw.
        What::Trident { charge } if charge < 10 => return None,
        What::Trident { .. } => Shot {
            speed: 2.5,
            offset: 0.0,
        },
        // CrossbowItem's shooting power: arrows 3.15.
        What::Crossbow => Shot {
            speed: 3.15,
            offset: 0.0,
        },
        // The plain throwables: 1.5, straight.
        What::Snowball | What::Egg | What::EnderPearl => Shot {
            speed: 1.5,
            offset: 0.0,
        },
        // The potions lob with a -20 degree pitch offset at 0.5.
        What::SplashPotion | What::LingeringPotion => Shot {
            speed: 0.5,
            offset: -20.0,
        },
        // The experience bottle lobs the same way at 0.7.
        What::ExperienceBottle => Shot {
            speed: 0.7,
            offset: -20.0,
        },
    };
    Some(shot)
}

// ── The published preview ─────────────────────────────────────────────────

/// A flight is simulated at most this many ticks (twenty seconds).
pub const MAX_TICKS: u32 = 400;
/// A flight past this depth stops (the void).
pub const VOID_Y: f64 = -64.0;
/// The spawn spot every player throw shares: the eye, a tenth of a block down.
pub const EYE_DROP: f64 = 0.1;
/// The ground track samples the terrain under the flight every this many
/// steps (a straight cast down, the game's own clip_ray).
pub const GROUND_EVERY: u32 = 4;

/// The flight line: white, two pixels, like a gizmo stroke.
pub const LINE_COLOR: u32 = 0xFFFF_FFFF;
pub const LINE_WIDTH: f32 = 2.0;
/// The impact marker's red, the same width.
pub const MARK_COLOR: u32 = 0xFFFF_3F3F;
/// The impact marker's half-size, in blocks.
pub const MARK_SIZE: f64 = 0.15;
/// The ground track's gray — the flight's shadow on the terrain below it.
pub const GROUND_COLOR: u32 = 0xFF6E_6E6E;
/// The arc draws every other step: a dashed line reads as the flight it is.
pub const DASH: usize = 2;

/// One simulated flight: a position per step, where it lands, and the
/// flight's shadow on the terrain (one point per few steps). The drop's
/// side view lives in the HUD panel (the projection below); the
/// world-space lines are the depth-anchored view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preview {
    pub points: Vec<[f64; 3]>,
    pub ground: Vec<[f64; 3]>,
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

/// One gizmo segment as the brain says it: the frame helper wraps these
/// into the game's own `Line` values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub color: u32,
    pub width: f32,
}

/// A cube's wireframe around `at`: twelve edges, the impact marker.
pub fn mark(at: [f64; 3]) -> Vec<Seg> {
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
        .map(|(a, b)| Seg {
            start: corners[*a],
            end: corners[*b],
            color: MARK_COLOR,
            width: LINE_WIDTH,
        })
        .collect()
}

/// The flight's lines: the dashed arc (one segment every [`DASH`] steps),
/// the ground track, and the marker.
pub fn build_lines(preview: &Preview) -> Vec<Seg> {
    let mut lines = Vec::with_capacity(
        (preview.points.len() / DASH)
            .saturating_add(preview.ground.len().saturating_sub(1))
            .saturating_add(usize::from(preview.impact.is_some()) * 12),
    );
    for (i, pair) in preview.points.windows(2).enumerate() {
        if i % DASH != 0 {
            continue;
        }
        lines.push(Seg {
            start: pair[0],
            end: pair[1],
            color: LINE_COLOR,
            width: LINE_WIDTH,
        });
    }
    for pair in preview.ground.windows(2) {
        lines.push(Seg {
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

// ── The side profile panel ────────────────────────────────────────────────
//
// The world-space arc is depth-tested geometry: aim dead-ahead and the whole
// flight lies on the view axis, one screen pixel wide — nothing left to
// read. The panel is the same flight in screen space, a side profile: every
// point projected onto the plane that holds the aim's horizontal forward and
// up, plotted right of the crosshair with the eye level through the
// crosshair's height. Length across, drop down — readable at every aim
// angle, because the projection follows only where the shot flies, not
// where the camera looks.

/// The panel's size, in GUI pixels.
pub const PANEL_W: f32 = 110.0;
pub const PANEL_H: f32 = 70.0;
/// The crosshair → panel gap.
pub const PANEL_GAP: f32 = 26.0;
/// The inner padding between the frame and the plot.
pub const PANEL_PAD: f32 = 5.0;
/// The eye level sits this far from the panel's top: a third above it for
/// the ascent, the rest below for the drop (drops outrun ascents).
pub const EYE_FRAC: f32 = 0.3;
/// The most GUI units one block may take: a short lob does not get a giant
/// arc, so the plot's scale stays honest across shots.
pub const SCALE_CAP: f64 = 8.0;
/// The flight's dash spacing and the solid lines' spacing, in GUI units.
pub const DASH_SPACING: f32 = 3.0;
pub const LINE_SPACING: f32 = 1.5;
/// One drawn dot's size.
pub const DOT: f32 = 1.0;

/// The panel's colors, `argb` for fills — but the label's, which is `rgb`
/// for `native_text`.
pub const PANEL_BG: u32 = 0x7800_0000;
pub const PANEL_FLIGHT: u32 = 0xE6FF_FFFF;
pub const PANEL_GROUND: u32 = 0x8C6E_6E6E;
pub const PANEL_EYE: u32 = 0x30FF_FFFF;
pub const PANEL_IMPACT: u32 = 0xFFFF_3F3F;
pub const PANEL_LABEL: u32 = 0xFF_FFFF;

/// The panel's rectangle on the HUD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    pub x0: f32,
    pub y0: f32,
    pub w: f32,
    pub h: f32,
}

/// Where the panel sits on an HUD of `width` × `height`: right of the
/// crosshair, vertically centered on it, clamped inside the screen — a
/// narrow window slides the panel left rather than off the edge.
pub fn panel(width: f32, height: f32) -> Panel {
    let w = PANEL_W.min(width - 8.0).max(40.0);
    let h = PANEL_H.min(height - 8.0).max(30.0);
    Panel {
        x0: ((width / 2.0) + PANEL_GAP).min(width - w - 4.0).max(4.0),
        y0: ((height / 2.0) - h / 2.0).max(4.0).min(height - h - 4.0),
        w,
        h,
    }
}

/// The eye level's y in GUI pixels: [`EYE_FRAC`] down the panel.
pub fn eye_y(panel: &Panel) -> f32 {
    panel.y0 + EYE_FRAC * panel.h
}

/// The aim's horizontal forward for `yaw_deg`: yaw 0 faces +z, yaw 90
/// faces -x — the sim's own convention (`shoot_from_rotation`'s).
pub fn forward_unit(yaw_deg: f64) -> [f64; 2] {
    let yr = yaw_deg.to_radians();
    [-yr.sin(), yr.cos()]
}

/// A world point in the profile plane: `[forward, height]` — forward from
/// the eye along the aim's horizontal, height above the eye.
pub fn profile(yaw_deg: f64, eye: [f64; 3], at: [f64; 3]) -> [f64; 2] {
    let f = forward_unit(yaw_deg);
    [
        (at[0] - eye[0]) * f[0] + (at[2] - eye[2]) * f[1],
        at[1] - eye[1],
    ]
}

/// The plot's scale, GUI units per block, for a panel and its
/// already-projected flight and ground: fitting the shot's length across,
/// its rise above the eye line and its fall below it (the ground track's
/// hollows count — the drop must fit), then capped by [`SCALE_CAP`].
pub fn scale_for(panel: &Panel, flight: &[[f64; 2]], ground: &[[f64; 2]]) -> f64 {
    let level = eye_y(panel);
    let plot_w = f64::from(panel.w - 2.0 * PANEL_PAD).max(1.0);
    let above = f64::from(level - panel.y0 - PANEL_PAD).max(1.0);
    let below = f64::from(panel.y0 + panel.h - level - PANEL_PAD).max(1.0);
    let span = flight
        .iter()
        .chain(ground)
        .map(|p| p[0])
        .fold(1.0_f64, f64::max);
    let rise = flight
        .iter()
        .chain(ground)
        .map(|p| p[1])
        .fold(0.0_f64, f64::max);
    let fall = flight
        .iter()
        .chain(ground)
        .map(|p| p[1])
        .fold(0.0_f64, f64::min);
    let by_width = plot_w / span.max(1.0);
    let by_rise = if rise > 0.0 {
        above / rise
    } else {
        f64::INFINITY
    };
    let by_fall = if fall < 0.0 {
        below / -fall
    } else {
        f64::INFINITY
    };
    by_width.min(by_rise).min(by_fall).min(SCALE_CAP)
}

/// A profile point's place in the panel, in GUI pixels: the eye line
/// anchors the heights (up is toward the top), the pad anchors the length.
pub fn to_panel(panel: &Panel, scale: f64, at: [f64; 2]) -> [f32; 2] {
    [
        panel.x0 + PANEL_PAD + (at[0] * scale) as f32,
        eye_y(panel) - (at[1] * scale) as f32,
    ]
}

/// A polyline resampled into dots every `spacing` units — the HUD batch has
/// no lines, only fills, so a stroke is a walk of small squares. The first
/// and the last point always draw.
pub fn stroke(points: &[[f32; 2]], spacing: f32) -> Vec<[f32; 2]> {
    let spacing = spacing.max(0.5);
    let Some(first) = points.first().copied() else {
        return Vec::new();
    };
    let mut dots = vec![first];
    let mut want = spacing;
    let mut walked = 0.0;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if len <= 1.0e-6 {
            continue;
        }
        while want <= walked + len {
            let t = (want - walked) / len;
            dots.push([a[0] + d[0] * t, a[1] + d[1] * t]);
            want += spacing;
        }
        walked += len;
    }
    let Some(last) = points.last().copied() else {
        return dots;
    };
    let tail = dots.last().copied().unwrap_or(first);
    let d = [last[0] - tail[0], last[1] - tail[1]];
    if d[0] * d[0] + d[1] * d[1] > 0.25 {
        dots.push(last);
    }
    dots
}

/// The flight's distance label: whole meters from ten up, one decimal
/// under it.
pub fn range_label(meters: f64) -> String {
    if meters >= 10.0 {
        format!("{} m", meters.round() as i64)
    } else {
        format!("{:.1} m", meters)
    }
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// The mod registers nothing: the preview has no toggle — installing it
    /// is asking for it, and what the hand holds decides the rest.
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {}

    /// Implemented as a no-op so the game always calls the
    /// `native_client_tick_pre` dispatch the tick mixin lives in: the game
    /// skips dispatches no mod listens to, and the mixin must run.
    fn client_tick_pre(&self, _: &mut dyn mod_api::ClientHost, _: &mut mod_api::ClientTickPreEvent) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bow_does_not_fire_below_a_tenth_of_a_draw() {
        assert_eq!(bow_power(0), None);
        assert_eq!(bow_power(2), None);
        let f = 3.0_f32 / 20.0;
        assert_eq!(bow_power(3), Some((f * f + f * 2.0) / 3.0));
    }

    #[test]
    fn the_bow_caps_at_a_full_draw() {
        assert_eq!(bow_power(20), Some(1.0));
        assert_eq!(bow_power(40), Some(1.0));
        assert_eq!(bow_power(-1), None);
        assert_eq!(plan(What::Bow { charge: 20 }).map(|s| s.speed), Some(3.0));
        assert_eq!(plan(What::Bow { charge: 2 }), None);
    }

    #[test]
    fn the_trident_needs_ten_ticks() {
        assert_eq!(plan(What::Trident { charge: 9 }), None);
        let shot = plan(What::Trident { charge: 10 }).expect("ten ticks throw");
        assert_eq!(shot.speed, 2.5);
        assert_eq!(shot.offset, 0.0);
    }

    #[test]
    fn throwables_and_potions_match_the_release_paths() {
        for what in [What::Snowball, What::Egg, What::EnderPearl] {
            let shot = plan(what).expect("a plain throwable always flies");
            assert_eq!((shot.speed, shot.offset), (1.5, 0.0));
        }
        let splash = plan(What::SplashPotion).expect("the splash flies");
        assert_eq!((splash.speed, splash.offset), (0.5, -20.0));
        let lingering = plan(What::LingeringPotion).expect("the lingering flies");
        assert_eq!((lingering.speed, lingering.offset), (0.5, -20.0));
        let bottle = plan(What::ExperienceBottle).expect("the bottle flies");
        assert_eq!((bottle.speed, bottle.offset), (0.7, -20.0));
        let crossbow = plan(What::Crossbow).expect("a charged crossbow fires");
        assert_eq!((crossbow.speed, crossbow.offset), (3.15, 0.0));
    }

    #[test]
    fn held_items_map_to_what_fly() {
        assert_eq!(from_item("minecraft:snowball"), Some(What::Snowball));
        assert_eq!(from_item("minecraft:ender_pearl"), Some(What::EnderPearl));
        assert_eq!(
            from_item("minecraft:experience_bottle"),
            Some(What::ExperienceBottle)
        );
        assert_eq!(
            from_item("minecraft:bow"),
            None,
            "the bow charges, it is not a plain throw"
        );
        assert_eq!(from_item("minecraft:diamond_sword"), None);
    }

    #[test]
    fn the_lines_are_arc_ground_and_marker() {
        let preview = Preview {
            points: vec![[0.0, 64.0, 0.0], [0.0, 64.0, 1.0], [0.0, 63.5, 2.0]],
            ground: vec![[0.0, 60.0, 0.5], [0.0, 60.0, 1.5]],
            impact: Some([0.0, 63.0, 3.0]),
        };
        let lines = build_lines(&preview);
        // One dashed arc segment (the second window skips on the dash), one
        // ground segment, and the impact marker's twelve edges.
        assert_eq!(lines.len(), 1 + 1 + 12);
        assert_eq!(lines[0].color, LINE_COLOR);
        assert_eq!(lines[1].color, GROUND_COLOR);
        assert_eq!(lines[2].color, MARK_COLOR);
    }

    #[test]
    fn the_panel_sits_right_of_the_crosshair() {
        let p = panel(426.67, 240.0);
        assert!(p.x0 > 213.0, "right of the crosshair");
        assert!(p.y0 < 120.0 && p.y0 + p.h > 120.0, "the crosshair inside");
        // A narrow window slides the panel left but keeps it on screen.
        let p = panel(320.0, 240.0);
        assert!(p.x0 >= 4.0 && p.x0 + p.w <= 316.0, "{p:?}");
    }

    #[test]
    fn the_profile_uses_the_sim_s_yaw() {
        // Yaw 0 faces +z, yaw 90 faces -x; height is plain y.
        let eye = [10.0, 64.0, -5.0];
        assert_eq!(profile(0.0, eye, [10.0, 64.0, 15.0]), [20.0, 0.0]);
        assert_eq!(profile(90.0, eye, [0.0, 66.0, -5.0]), [10.0, 2.0]);
        assert_eq!(forward_unit(0.0), [0.0, 1.0]);
    }

    #[test]
    fn the_scale_fits_length_rise_and_fall_then_caps() {
        let p = panel(426.67, 240.0);
        // A twenty-block level flight: the width decides (plot 100 / 20).
        let scale = scale_for(&p, &[[0.0, 0.0], [20.0, 0.0]], &[]);
        assert!((scale - 5.0).abs() < 0.01, "{scale}");
        // A ten-block drop from the same shot: the fall budget decides.
        let scale = scale_for(&p, &[[0.0, 0.0], [20.0, -10.0]], &[]);
        assert!((scale - 4.4).abs() < 0.01, "{scale}");
        // A hundred-block shot: the width again, uncapped.
        let scale = scale_for(&p, &[[0.0, 0.0], [100.0, 0.0]], &[]);
        assert!((scale - 1.0).abs() < 0.01, "{scale}");
        // A two-block lob: the cap, not the fit, decides.
        let scale = scale_for(&p, &[[0.0, 0.0], [2.0, 2.0]], &[]);
        assert!((scale - SCALE_CAP).abs() < 0.01, "{scale}");
    }

    #[test]
    fn the_panel_maps_around_the_eye_line() {
        let p = panel(426.67, 240.0);
        let at = to_panel(&p, 2.0, [10.0, 3.0]);
        assert!((at[0] - (p.x0 + PANEL_PAD + 20.0)).abs() < 0.01, "{at:?}");
        assert!((at[1] - (p.y0 + EYE_FRAC * p.h - 6.0)).abs() < 0.01, "{at:?}");
        assert!((eye_y(&p) - (p.y0 + EYE_FRAC * p.h)).abs() < 0.01);
    }

    #[test]
    fn the_stroke_walks_the_line_end_to_end() {
        let dots = stroke(&[[0.0, 0.0], [10.0, 0.0]], DASH_SPACING);
        assert_eq!(dots.len(), 5, "{dots:?}");
        assert_eq!(dots[0], [0.0, 0.0]);
        assert_eq!(dots[4], [10.0, 0.0]);
        assert!(stroke(&[], DASH_SPACING).is_empty());
        assert_eq!(stroke(&[[5.0, 5.0]], DASH_SPACING), vec![[5.0, 5.0]]);
    }

    #[test]
    fn the_label_wholes_from_ten_one_decimal_under() {
        assert_eq!(range_label(23.4), "23 m");
        assert_eq!(range_label(9.83), "9.8 m");
        assert_eq!(range_label(0.0), "0.0 m");
    }
}
