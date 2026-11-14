// The game-side preview: one simulated flight of the projectile in hand,
// stepped with the game's own projectile tick against the live world — the
// same ProjWorld and ArrowScene the creatures tick builds, so blocks, water,
// entities and the border behave exactly as they will for the real throw.
// The spread stays at zero (the release paths' triangle noise is the one
// thing the preview cannot know), so the line is the flight's center.
// Published for the frame injection to draw.
// Copyright (c) 2026 hakergeniusz. MIT.
use crate::game::trajectory_state as state;
use horizon_client::projectiles::{Hit, Projectile, ProjectileKind};
use horizon_core::geometry::Vec3;

/// A flight is simulated at most this many ticks (twenty seconds).
const MAX_TICKS: u32 = 400;
/// A flight past this depth stops (the void).
const VOID_Y: f64 = -64.0;
/// The spawn spot every player throw shares: the eye, a tenth of a block down.
const EYE_DROP: f64 = 0.1;
/// The ground track samples the terrain under the flight every this many
/// steps (a straight cast down, [`crate::game::Game::clip_ray`]).
const GROUND_EVERY: u32 = 4;

/// What the hand holds right now: the kind to fly, its speed and the pitch
/// offset its throw applies — None hides the preview.
fn shot_in_hand(game: &mut crate::game::Game<'_>) -> Option<(ProjectileKind, f32, f32)> {
    use crate::game::trajectory_preview as runtime;
    // A charge in progress: the bow and the trident are the only charging
    // throws (a shield, a drink or a spyglass preview nothing).
    if let Some((slot, ticks)) = game.using {
        let stack = game.stack_in(slot)?.clone();
        let charge = game.ticks_held(&stack, ticks);
        return match &*stack.item {
            "minecraft:bow" => {
                // release_bow: no ammo, no shot; the power curve decides the rest.
                let has_ammo = game.get_projectile(&stack).is_some();
                let power =
                    crate::game::item_use::bow_release_power(has_ammo, charge as i32, false)?;
                let kind = ProjectileKind::Arrow {
                    base_damage: 2.0,
                    critical: power >= 1.0,
                    effect: None,
                };
                Some((kind, power * 3.0, 0.0))
            }
            "minecraft:trident" => {
                // release_trident: a riptide trident spins instead of
                // throwing unless the player is wet and on foot.
                let riptide = stack.enchantment_level("minecraft:riptide");
                if riptide > 0 && (!game.in_water_or_rain() || game.riding.is_some()) {
                    return None;
                }
                runtime::plan(runtime::What::Trident { charge })?;
                Some((ProjectileKind::Trident, 2.5, 0.0))
            }
            _ => None,
        };
    }
    let stack = game.held()?;
    let item = &*stack.item;
    // A charged crossbow fires on the next right click; a firework rocket
    // among the load flies as a rocket, which no line previews here.
    if item == "minecraft:crossbow" {
        let mut stack = stack.clone();
        let charged = &stack.data_mut().charged;
        if charged.is_empty() || charged.iter().any(|s| &*s.item == "minecraft:firework_rocket") {
            return None;
        }
        runtime::plan(runtime::What::Crossbow)?;
        let kind = ProjectileKind::Arrow {
            base_damage: 2.0,
            critical: false,
            effect: None,
        };
        return Some((kind, 3.15, 0.0));
    }
    // The plain throwables fly on a plain right click.
    let what = runtime::from_item(item)?;
    let shot = runtime::plan(what)?;
    let kind = match what {
        runtime::What::Snowball => ProjectileKind::Snowball,
        runtime::What::Egg => ProjectileKind::Egg,
        runtime::What::EnderPearl => ProjectileKind::EnderPearl,
        runtime::What::SplashPotion | runtime::What::LingeringPotion => {
            ProjectileKind::Potion(Box::new(stack.clone()), item == "minecraft:lingering_potion")
        }
        runtime::What::ExperienceBottle => ProjectileKind::ExperienceBottle,
        _ => return None,
    };
    Some((kind, shot.speed, shot.offset))
}

/// Simulate the flight in hand and publish it (or clear the preview).
pub fn preview(game: &mut crate::game::Game<'_>) {
    let Some((kind, speed, offset)) = shot_in_hand(game) else {
        state::clear();
        return;
    };
    let Some(eye) = game.eye_position() else {
        state::clear();
        return;
    };
    let start = Vec3::new(eye[0], eye[1] - EYE_DROP, eye[2]);
    let velocity = game.known_player_movement();
    let on_ground = game
        .player
        .as_ref()
        .map_or(true, |p| p.ground.on_ground);
    let mut p = Projectile::new(kind, start, Some(horizon_client::mobs::Who::Player));
    // The release paths' own shooting call, with the spread at zero: the
    // preview cannot know the triangle noise, so it draws the center.
    p.shoot_from_rotation(
        &mut game.world.random,
        game.pitch,
        game.yaw,
        offset,
        speed,
        0.0,
        velocity,
        on_ground,
    );
    // From here the game is only read: the world exactly as the creatures
    // tick sees it.
    let game = &*game;
    let w = game.mob_world_base(&[]);
    let rain = game.arrow_rain();
    let scene = game.arrow_scene(&rain);
    let pw = crate::game::creatures::ProjWorld {
        w: &w,
        targets: game.projectile_targets(),
        border: Some(game.border.bounds()),
    };
    let mut points = vec![[start.x, start.y, start.z]];
    let mut ground = Vec::new();
    let mut impact = None;
    for step in 0..MAX_TICKS {
        // Arrows and tridents tick through the arrow path; the thrown kinds
        // through the projectile's own — the dispatch the creatures tick makes.
        let (keep, hit) = if p.arrow.is_some() {
            horizon_client::arrows::tick(&mut p, &scene, &pw.targets)
        } else {
            p.tick(&pw)
        };
        points.push([p.position.x, p.position.y, p.position.z]);
        // The flight's shadow: every few steps, the ground under the flight
        // (a straight cast down; open sky adds nothing). Aiming dead-ahead
        // the arc lies on the view axis and reads as a line on screen —
        // the HUD panel's side profile (the runtime's projection) is what
        // shows the drop there, this track anchors it in the terrain.
        if step % GROUND_EVERY == 0 {
            if let Some(hit) = game
                .clip_ray([p.position.x, p.position.y, p.position.z], [0.0, -1.0, 0.0], 48.0)
            {
                ground.push(hit.point);
            }
        }
        match hit {
            Hit::None => {}
            Hit::Block { at, .. } | Hit::Entity { at, .. } => {
                impact = Some([at.x, at.y, at.z]);
                break;
            }
        }
        if !keep || p.position.y < VOID_Y {
            break;
        }
    }
    state::publish(state::Preview {
        points,
        ground,
        impact,
    });
}
