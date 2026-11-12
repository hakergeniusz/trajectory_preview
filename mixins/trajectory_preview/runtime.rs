// Trajectory Preview's brain: pure standard library, no game types, so it
// compiles and tests wherever — as this crate's library and, injected, as a
// module of the game. The policy table says, from the item in hand, what
// flies and how: the same speed and pitch offset the game's own release
// paths use (the bow's draw curve, the trident's ten-tick gate, the
// throwables' 1.5, the potions' -20 degree lob). The simulation itself runs
// on the game side (`sim.rs`) with the game's own projectile tick; this
// file only decides whether a shot is live and with which parameters.
//
// Clean-room: written for Horizon from the game's own public behavior; it
// copies nothing from any mod.
//
// Copyright (c) 2026 hakergeniusz. MIT.

/// The mod's registry id: the Options > Mods row and the injection gates.
pub const ID: &str = "trajectory_preview";

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

#[cfg(test)]
mod tests {
    use super::{What, bow_power, from_item, plan};

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
}
