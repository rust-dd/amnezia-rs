# Changelog

## Unreleased — campaign acceptance checkpoint (2026-10-01)

### Gameplay corrections

- Preserve RPG Maker's omitted learning and event-condition IDs, restoring Ron's starting X-Csapás and the concert donation page change.
- Prevent Draco's dream from restarting after the briefing and clear the staircase so the captain remains reachable after saving and returning.
- Avoid refreshing unchanged map/parallel event pages while preserving story-state invalidation.

### Playtest tooling

- Add visible and offscreen campaign control with isolated ordinary key input, diagnostic state and native captures.
- Maintain 60 logical ticks per wall-clock second during visible commands, independently of render speed and desktop focus.
- Report diagnostic write errors without terminating the game, retain history only for captures, and preserve command IDs across restarts.
- Handle foreground forced movement, successive choices, battle recovery and Stark's Draco techniques in the Python client.

### Acceptance and repository documentation

- Record the bandit branch through the happy Ron/Tiffany epilogue and VÉGE, fifteen save slots, optional tower/dragon events, and strongest-weapon acquisition checks.
- Preserve original and edited checkpoints, native captures, battle observations and SHA-256 manifests. Final fights used raised levels; Wyvern/Efreet redemption used granted EP.
- Add compatibility status, contribution instructions, evidence verification and macOS CI for formatting, workspace tests, Clippy and release compilation.

Natural final-combat balance, other branches, complete English localization and
native Windows/Linux validation remain separate acceptance work; see
[Compatibility status](docs/STATUS.md).
