// The mod's row in Options > Mods: the mod's identity and what it does.
// Copyright (c) 2026 hakergeniusz. MIT.
fn trajectory_preview_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "trajectory_preview".into(),
        title: "Trajectory Preview 0.3.0".into(),
        description: "A dashed flight arc with ground track, plus a side profile panel by the crosshair — distance and drop readable at every aim angle.".into(),
        keys: vec![],
    });
}
