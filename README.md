# Swarm Mod: empire-upkeep

Empire maintenance fee system — per-room upkeep, controller repair, and recycle refund for Swarm
int
int
int
int
int
int

## Directory Structure

```
mods/empire-upkeep/
├── Cargo.toml        # Static Bevy Plugin crate
├── mod.toml          # Mod metadata + configurable parameters
├── src/lib.rs        # `impl Plugin` entry point
└── README.md
```

## Configuration

See `mod.toml` for all configurable parameters. Server operators can override via:

```bash
swarm mod config empire-upkeep <key> <value>
```

Or in `world.toml`:

```toml
[mods.empire-upkeep.config]
# key = value
```

## Engine API

Mods are statically compiled Bevy Plugin crates. Enable this mod with the
`mod_empire_upkeep` Cargo feature, or with `vanilla_mods`.

## Publishing

```bash
git tag v0.1.0
git push --tags
swarm mod pack
```
