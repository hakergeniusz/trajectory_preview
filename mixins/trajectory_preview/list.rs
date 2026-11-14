// The mod's row in Options > Mods: the mod's identity and what it does.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_preview_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "trajectory_preview".into(),
        title: "Trajectory Preview 0.2.1".into(),
        description: "A dashed flight arc with ground track and tick marks for the aimed projectile — bows, tridents and thrown items.".into(),
        keys: vec![],
    });
}
