# Trajectory Preview

A Horizon native mod that draws where the projectile in hand will land,
with the game's own physics — the port of maDU59's
[ProjectilesTrajectoryPreview](https://github.com/maDU59/ProjectilesTrajectoryPreview)
idea to Horizon, as an independent clean-room reimplementation (no original
code; MIT).

While you hold it, it previews:

- a **bow** being drawn (the line grows with the draw power, nothing under
  the release threshold),
- a **charged crossbow** (arrows; a loaded firework rocket is not previewed),
- a **trident** past its ten-tick charge (a riptide trident previews only
  when it would actually throw),
- the plain throwables: **snowball, egg, ender pearl, splash/lingering
  potion, experience bottle** (the potions show their -20° lob).

The white line is the flight, one segment per simulated tick; the red box
marks the impact. Both are depth-tested — they hide behind the terrain
they will actually hit.

## How it works

- Every game tick the mod simulates one flight of the held projectile with
  the game's own projectile tick against the live world — the same
  `ProjWorld`/`ArrowScene` the creature tick builds, so blocks, water,
  entities and the world border behave exactly as they will for the real
  throw. The release spread (the triangle noise the real throw applies) is
  the one thing a preview cannot know, so the sim runs it at zero and draws
  the flight's center.
- Every frame the published flight joins the game's depth-tested gizmo
  lines. The game tick runs at 20 Hz, the frame as fast as the GPU allows;
  between ticks the same lines draw again.

## Building

The mod ships as a source bundle (`.hnmod`): `python3 -I tools/package.py`
from this directory packages `mixins/trajectory_preview/` into
`target/trajectory_preview-0.1.0.hnmod`. The launcher compiles it against
the pinned Horizon revision (`compatibility.json`).

## License

MIT — see LICENSE. The upstream idea is maDU59's ProjectilesTrajectoryPreview
(MIT); this implementation is independent and contains none of its code.
