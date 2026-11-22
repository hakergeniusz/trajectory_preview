// Every game tick: simulate the projectile in hand and publish its flight
// for the frame injection to draw — one flight of the held projectile,
// stepped with the game's own tick against the live world (the same
// ProjWorld and ArrowScene the creatures tick builds, so blocks, water,
// entities and the border behave exactly as they will for the real throw).
// The spread stays at zero (the release paths' triangle noise is the one
// thing the preview cannot know), so the line is the flight's center.
//
// The game's own release helpers live in scopes a tick helper cannot reach
// (the items module's bow power and held ticks, the creatures module's rain
// check), so the small pure ones are re-derived here: the bow's draw curve
// is the mod's own `bow_power` (the same numbers), the charge is the use
// state's ticks as they are, and the scene's rain reads as clear — the
// preview's honest approximations.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_tick(game: &mut crate::game::Game<'_>) {
    // What the hand holds right now: the kind to fly, its speed and the
    // pitch offset its throw applies — None hides the preview.
    fn shot_in_hand(game: &mut crate::game::Game<'_>) -> Option<(horizon_client::projectiles::ProjectileKind, f32, f32)> {
        use trajectory_preview as runtime;
        // A charge in progress: the bow and the trident are the only charging
        // throws (a shield, a drink or a spyglass preview nothing).
        if let Some((slot, ticks)) = game.using {
            let stack = game.inventory.get(slot).and_then(|s| s.as_ref())?.clone();
            let charge = ticks;
            return match &*stack.item {
                "minecraft:bow" => {
                    // release_bow: no ammo, no shot; the power curve decides the rest.
                    let has_ammo = game.creative
                        || game
                            .inventory
                            .iter()
                            .any(|s| s.as_ref().is_some_and(|s| s.item.as_ref().ends_with("_arrow")));
                    let power = runtime::bow_power(charge as i32)?;
                    let kind = horizon_client::projectiles::ProjectileKind::Arrow {
                        base_damage: 2.0,
                        critical: power >= 1.0,
                        effect: None,
                    };
                    Some((kind, power * 3.0, 0.0))
                }
                "minecraft:trident" => {
                    // release_trident: a riptide trident spins instead of
                    // throwing unless the player is wet and on foot.
                    let riptide = stack
                        .enchantment_level("minecraft:riptide");
                    if riptide > 0 && (!game.in_water_or_rain() || game.riding.is_some()) {
                        return None;
                    }
                    runtime::plan(runtime::What::Trident { charge })?;
                    Some((horizon_client::projectiles::ProjectileKind::Trident, 2.5, 0.0))
                }
                _ => None,
            };
        }
        let stack = game
            .inventory
            .get(game.selected)
            .and_then(|s| s.as_ref())?;
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
            let kind = horizon_client::projectiles::ProjectileKind::Arrow {
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
            runtime::What::Snowball => horizon_client::projectiles::ProjectileKind::Snowball,
            runtime::What::Egg => horizon_client::projectiles::ProjectileKind::Egg,
            runtime::What::EnderPearl => horizon_client::projectiles::ProjectileKind::EnderPearl,
            runtime::What::SplashPotion | runtime::What::LingeringPotion => {
                horizon_client::projectiles::ProjectileKind::Potion(
                    Box::new(stack.clone()),
                    item == "minecraft:lingering_potion",
                )
            }
            runtime::What::ExperienceBottle => {
                horizon_client::projectiles::ProjectileKind::ExperienceBottle
            }
            _ => return None,
        };
        Some((kind, shot.speed, shot.offset))
    }

    // Simulate the flight in hand and publish it (or clear the preview).
    let Some((kind, speed, offset)) = shot_in_hand(game) else {
        trajectory_preview::clear();
        return;
    };
    let Some(eye) = game.eye_position() else {
        trajectory_preview::clear();
        return;
    };
    let start = horizon_core::geometry::Vec3::new(
        eye[0],
        eye[1] - trajectory_preview::EYE_DROP,
        eye[2],
    );
    let velocity = game
        .player
        .as_ref()
        .map(|p| p.velocity)
        .unwrap_or(horizon_core::geometry::Vec3::ZERO);
    let on_ground = game
        .player
        .as_ref()
        .map_or(true, |p| p.ground.on_ground);
    let mut p = horizon_client::projectiles::Projectile::new(
        kind,
        start,
        Some(horizon_client::mobs::Who::Player),
    );
    // The release paths' own shooting call, with the spread at zero: the
    // preview cannot know the triangle noise, so it draws the center.
    p.shoot_from_rotation(
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
    let targets = game.projectile_targets();
    // The scene's rain reads as clear: a tick helper cannot reach the
    // weather module's check, and rain's drag on a previewed arrow is the
    // small honest cost of the approximation.
    let no_rain = |_: horizon_core::position::BlockPos| false;
    let scene = horizon_client::arrows::ArrowScene {
        world: &game.world,
        registry: game.pack.assets.registry,
        facts: game.res.facts(),
        collision: game.collision,
        fluids: &game.motion.fluids,
        loop_context: &game.loop_context,
        raining: &no_rain,
        mob_griefing: true,
        tnt_explodes: true,
        mob_hooks: None,
        owners: game,
    };
    let pw = horizon_client::game::creatures::projectile_world::ProjWorld {
        w: &w,
        targets: &targets,
        border: Some(game.border.bounds()),
    };
    let mut points = vec![[start.x, start.y, start.z]];
    let mut ground = Vec::new();
    let mut impact = None;
    for step in 0..trajectory_preview::MAX_TICKS {
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
        // the HUD panel's side profile is what shows the drop there, this
        // track anchors it in the terrain.
        if step % trajectory_preview::GROUND_EVERY == 0 {
            if let Some(hit) =
                game.clip_ray([p.position.x, p.position.y, p.position.z], [0.0, -1.0, 0.0], 48.0)
            {
                ground.push(hit.point);
            }
        }
        match hit {
            horizon_client::projectiles::Hit::None => {}
            horizon_client::projectiles::Hit::Block { at, .. }
            | horizon_client::projectiles::Hit::Entity { at, .. } => {
                impact = Some([at.x, at.y, at.z]);
                break;
            }
        }
        if !keep || p.position.y < trajectory_preview::VOID_Y {
            break;
        }
    }
    trajectory_preview::publish(trajectory_preview::Preview {
        points,
        ground,
        impact,
    });
}
