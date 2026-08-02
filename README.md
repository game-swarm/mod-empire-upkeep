# empire-upkeep

帝国规模维护费模组。drone 和房间越多，每 tick 消耗越大。

## 职责

- 每 tick 按玩家拥有的房间数计算维护费
- 计算公式：`upkeep = base_upkeep × rooms × (room_soft_cap + rooms) / room_soft_cap`
- Engine 集成路径通过 Resource Ledger 扣除；独立 crate 运行使用自身 `PlayerEnergyLedger`
- Engine native `register` 只安装解码后的 `EmpireUpkeepConfig` 和 config-only descriptor；Engine authoritative Resource Ledger/system 负责实际维护费流程
- 直接添加 `EmpireUpkeepModPlugin` 才启用 self-contained `PlayerEnergyLedger`、`UpkeepShortfalls` 和 `empire_upkeep_system`
- 独立 crate 的 self-contained ledger 资源不足时按内部 `onshortfall` resource policy 处理：degrade（降级 Controller）/ damage（伤害 drone）/ despawn（杀死最旧 drone）
- 构建 anti-snowball 经济曲线——维护费随帝国规模超线性增长

## 依赖

- bevy
- Engine 集成使用权威 Resource Ledger；独立 crate 保持自包含 ledger

## 配置

以下 `mod.toml` 配置是 Engine 集成契约：strict control plane 将 `world.toml [mods.empire-upkeep]` values/defaults 解码并写入 replay identity；`mods.lock` 不保存 gameplay config。production native register context 只提供 locked schema 的十个 versioned defaults，并完整映射为 `EmpireUpkeepConfig` Resource；`world.toml` override 仍由 engine-owned upkeep config 处理。`onshortfall` 不属于 mod schema，production native entry 固定为 `Degrade`；其他 policy 仅供独立 crate 直接配置其 self-contained ledger。独立 crate 的 config/ledger 不冒充 Engine Resource Ledger。

mod.toml:
```toml
[config]
base_upkeep = { type = "u32", default = 50 }
room_soft_cap = { type = "u32", default = 10 }
controller_passive_income = { type = "u32", default = 40 }
controller_passive_income_rcl_bonus = { type = "u32", default = 5 }
resource = { type = "string", default = "Energy" }
repair_cap = { type = "basis_points", default = 3500 }
distance_decay_bp = { type = "basis_points", default = 500 }
recycle_refund_base = { type = "basis_points", default = 5000 }
recycle_refund_min = { type = "basis_points", default = 1000 }
tutorial_recycle_refund_full_ticks = { type = "u64", default = 500 }
```

## 资源

- 维护费从玩家 Energy 储备中扣除
- 短fall 处理通过 Entity 操作（降级、伤害、杀死）

## Standalone Development

### Engine integration

The exported native `register` path is for the Engine generated bundle. It decodes all ten config fields, forces the production `onshortfall` policy to `Degrade`, inserts only `EmpireUpkeepConfig`, and publishes the Engine-facing descriptor without standalone systems or actions. The Engine authoritative Resource Ledger owns the runtime upkeep system, so this path does not install `PlayerEnergyLedger`, `UpkeepShortfalls`, or `empire_upkeep_system`. `repair_cap` and `distance_decay_bp` remain decoded for config/replay identity but are inactive migration metadata. While Engine pins canonical repository `https://github.com/game-swarm/mod-empire-upkeep.git` at published legacy revision `c4c1f9b545a03b83caef287dfeb634abb055ce9b`, its exact-source adapter harvests this descriptor in a disposable registration App; after this config-only entry is published, Engine will update the pin and remove that adapter transactionally.

### Standalone plugin

For a self-contained Bevy app, insert an `EmpireUpkeepConfig` and add `EmpireUpkeepModPlugin`. This direct path initializes `PlayerEnergyLedger` and `UpkeepShortfalls`, runs `empire_upkeep_system`, and exposes the standalone descriptor with its upkeep system entry. Standalone `onshortfall` policies (`Degrade`, `Damage`, and `Despawn`) remain available here.

This crate pins `swarm-engine-api` and `swarm-engine-plugin-sdk` to canonical source `https://github.com/game-swarm/engine-api.git`, exact version `0.1.0`, and identical full revision `0d97444af0c8f8c563bbe58837a4fdf8753630cf`. Cargo fetches both crates directly; no sibling API checkout is required.

```sh
cargo check
cargo test
```

To adopt a later API/SDK release, update both canonical URLs, both exact versions, and both full Git revisions in `Cargo.toml` together, then regenerate `Cargo.lock` and verify both packages resolve to the same commit.
