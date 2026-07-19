# empire-upkeep

帝国规模维护费模组。drone 和房间越多，每 tick 消耗越大。

## 职责

- 每 tick 按玩家拥有的 drone 数和房间数计算维护费
- 计算公式：`cost = drone_count × drone_cost + room_count × room_base + room_count² × room_superlinear`
- Engine 集成路径通过 Resource Ledger 扣除；独立 crate 运行使用自身 `PlayerEnergyLedger`
- 资源不足时按 onshortfall 策略处理：degrade（降级 Controller）/ damage（伤害 drone）/ despawn（杀死最旧 drone）
- 构建 anti-snowball 经济曲线——维护费随帝国规模超线性增长

## 依赖

- bevy
- Engine 集成使用权威 Resource Ledger；独立 crate 保持自包含 ledger

## 配置

以下 `mod.toml` 配置是 Engine 集成契约：strict mod control plane 将 `mods.lock` 基础值解码到权威 `WorldConfig.empire_upkeep`，显式 `world.toml` 字段优先。独立 crate 的 `EmpireUpkeepConfig` 与 `PlayerEnergyLedger` 用于隔离运行，不冒充 Engine Resource Ledger。

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

This crate depends on `swarm-engine-api` and `swarm-engine-plugin-sdk` version `0.1.0`. Until those crates are published to a registry, place the API repository beside `mods/` so the paths in `Cargo.toml` resolve:

```sh
mkdir -p swarm/mods
git clone <this-mod-repository-url> swarm/mods/empire-upkeep
# Fetch the reviewed API/SDK v0.1.0 source:
git clone --branch v0.1.0 https://github.com/game-swarm/engine-api swarm/engine-api
cd swarm/mods/empire-upkeep
cargo check
cargo test
```

To adopt a later API/SDK release, update both exact versions in `Cargo.toml` and the immutable API commit in `.github/workflows/ci.yml` together.
