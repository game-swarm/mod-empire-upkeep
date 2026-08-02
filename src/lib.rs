use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use swarm_engine_api::prelude::{
    API_VERSION, ConfigFieldDescriptor, ConfigValidator, ConfigValueType,
    DESCRIPTOR_SCHEMA_VERSION, PlayerId, PluginDescriptor, RoomId, SystemDescriptor, TickPhase,
};
use swarm_engine_plugin_sdk::prelude::{
    Controller, DeathMark, Drone, NativeModRegisterContext, NativeModRegisterError, Position,
    Structure,
};
use swarm_engine_plugin_sdk::traits::SwarmPlugin;

#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerEnergyLedger {
    pub balances: BTreeMap<PlayerId, u32>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct UpkeepShortfalls {
    pub deficits: BTreeMap<PlayerId, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortfallPolicy {
    Degrade,
    Damage,
    Despawn,
}

#[derive(Resource, Debug, Clone)]
pub struct EmpireUpkeepConfig {
    pub base_upkeep: u32,
    pub room_soft_cap: u32,
    pub controller_passive_income: u32,
    pub controller_passive_income_rcl_bonus: u32,
    pub resource: String,
    pub repair_cap: u32,
    pub distance_decay_bp: u32,
    pub recycle_refund_base: u32,
    pub recycle_refund_min: u32,
    pub tutorial_recycle_refund_full_ticks: u64,
    pub onshortfall: ShortfallPolicy,
}

impl Default for EmpireUpkeepConfig {
    fn default() -> Self {
        Self {
            base_upkeep: 50,
            room_soft_cap: 10,
            controller_passive_income: 40,
            controller_passive_income_rcl_bonus: 5,
            resource: "Energy".to_string(),
            repair_cap: 3_500,
            distance_decay_bp: 500,
            recycle_refund_base: 5_000,
            recycle_refund_min: 1_000,
            tutorial_recycle_refund_full_ticks: 500,
            onshortfall: ShortfallPolicy::Degrade,
        }
    }
}

impl EmpireUpkeepConfig {
    fn upkeep_cost(&self, rooms: u32) -> u32 {
        if rooms == 0 || self.room_soft_cap == 0 {
            return 0;
        }

        let rooms = u64::from(rooms);
        let room_soft_cap = u64::from(self.room_soft_cap);
        let cost = u64::from(self.base_upkeep)
            .saturating_mul(rooms)
            .saturating_mul(room_soft_cap.saturating_add(rooms))
            / room_soft_cap;
        u32::try_from(cost).unwrap_or(u32::MAX)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EmpireUpkeepModPlugin;

impl Plugin for EmpireUpkeepModPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmpireUpkeepConfig>()
            .init_resource::<PlayerEnergyLedger>()
            .init_resource::<UpkeepShortfalls>()
            .add_systems(Update, empire_upkeep_system);
    }
}

impl SwarmPlugin for EmpireUpkeepModPlugin {
    fn descriptor() -> PluginDescriptor {
        plugin_descriptor(vec![SystemDescriptor {
            system_id: "empire-upkeep.update".to_string(),
            version: "0.1.0".to_string(),
            phase: TickPhase::Update,
            order: 0,
            reads: vec![
                "EmpireUpkeepConfig".to_string(),
                "PlayerEnergyLedger".to_string(),
                "Drone".to_string(),
                "Position".to_string(),
                "Structure".to_string(),
                "Controller".to_string(),
            ],
            writes: vec![
                "PlayerEnergyLedger".to_string(),
                "UpkeepShortfalls".to_string(),
                "Controller".to_string(),
                "Drone".to_string(),
                "DeathMark".to_string(),
            ],
            produces_buffers: Vec::new(),
            consumes_buffers: Vec::new(),
            deterministic_iteration: vec!["PlayerId".to_string()],
        }])
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmpireUpkeepNativeConfig {
    base_upkeep: u32,
    room_soft_cap: u32,
    controller_passive_income: u32,
    controller_passive_income_rcl_bonus: u32,
    resource: String,
    repair_cap: u32,
    distance_decay_bp: u32,
    recycle_refund_base: u32,
    recycle_refund_min: u32,
    tutorial_recycle_refund_full_ticks: u64,
}

#[derive(Debug, Clone)]
struct EngineEmpireUpkeepPlugin {
    config: EmpireUpkeepConfig,
}

impl Plugin for EngineEmpireUpkeepPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.config.clone());
    }
}

impl SwarmPlugin for EngineEmpireUpkeepPlugin {
    fn descriptor() -> PluginDescriptor {
        plugin_descriptor(Vec::new())
    }
}

pub fn register(context: &mut NativeModRegisterContext<'_>) -> Result<(), NativeModRegisterError> {
    let config = context.decode_config::<EmpireUpkeepNativeConfig>()?;
    context.install(EngineEmpireUpkeepPlugin {
        config: EmpireUpkeepConfig {
            base_upkeep: config.base_upkeep,
            room_soft_cap: config.room_soft_cap,
            controller_passive_income: config.controller_passive_income,
            controller_passive_income_rcl_bonus: config.controller_passive_income_rcl_bonus,
            resource: config.resource,
            repair_cap: config.repair_cap,
            distance_decay_bp: config.distance_decay_bp,
            recycle_refund_base: config.recycle_refund_base,
            recycle_refund_min: config.recycle_refund_min,
            tutorial_recycle_refund_full_ticks: config.tutorial_recycle_refund_full_ticks,
            onshortfall: ShortfallPolicy::Degrade,
        },
    })
}

fn plugin_descriptor(systems: Vec<SystemDescriptor>) -> PluginDescriptor {
    PluginDescriptor {
        id: "empire-upkeep".to_string(),
        version: "0.1.0".to_string(),
        api_version: API_VERSION.to_string(),
        dependencies: Vec::new(),
        config: vec![
            config_field("base_upkeep", ConfigValueType::U32, 50_u32.into(), None),
            config_field(
                "room_soft_cap",
                ConfigValueType::U32,
                10_u32.into(),
                Some(ConfigValidator::Positive),
            ),
            config_field(
                "controller_passive_income",
                ConfigValueType::U32,
                40_u32.into(),
                None,
            ),
            config_field(
                "controller_passive_income_rcl_bonus",
                ConfigValueType::U32,
                5_u32.into(),
                None,
            ),
            config_field(
                "resource",
                ConfigValueType::String,
                "Energy".into(),
                Some(ConfigValidator::NonEmptyString),
            ),
            config_field(
                "repair_cap",
                ConfigValueType::BasisPoints,
                3_500_u32.into(),
                Some(ConfigValidator::BasisPoints),
            ),
            config_field(
                "distance_decay_bp",
                ConfigValueType::BasisPoints,
                500_u32.into(),
                Some(ConfigValidator::BasisPoints),
            ),
            config_field(
                "recycle_refund_base",
                ConfigValueType::BasisPoints,
                5_000_u32.into(),
                Some(ConfigValidator::BasisPoints),
            ),
            config_field(
                "recycle_refund_min",
                ConfigValueType::BasisPoints,
                1_000_u32.into(),
                Some(ConfigValidator::BasisPoints),
            ),
            config_field(
                "tutorial_recycle_refund_full_ticks",
                ConfigValueType::U64,
                500_u64.into(),
                None,
            ),
        ],
        systems,
        actions: Vec::new(),
        descriptor_schema_version: DESCRIPTOR_SCHEMA_VERSION.to_string(),
    }
}

fn config_field(
    key: &str,
    value_type: ConfigValueType,
    default: serde_json::Value,
    validator: Option<ConfigValidator>,
) -> ConfigFieldDescriptor {
    ConfigFieldDescriptor {
        key: key.to_string(),
        value_type,
        default,
        required: false,
        validator,
    }
}

pub fn empire_upkeep_system(
    mut commands: Commands,
    config: Res<EmpireUpkeepConfig>,
    mut ledger: ResMut<PlayerEnergyLedger>,
    mut shortfalls: ResMut<UpkeepShortfalls>,
    mut drones: Query<(Entity, &mut Drone, &Position)>,
    rooms_from_structures: Query<(&Structure, &Position)>,
    mut controllers: Query<&mut Controller>,
) {
    let mut rooms: BTreeMap<PlayerId, BTreeSet<RoomId>> = BTreeMap::new();

    for (_, drone, position) in &drones {
        rooms.entry(drone.owner).or_default().insert(position.room);
    }
    for (structure, position) in &rooms_from_structures {
        if let Some(owner) = structure.owner {
            rooms.entry(owner).or_default().insert(position.room);
        }
    }
    for controller in &controllers {
        if let Some(owner) = controller.owner {
            rooms.entry(owner).or_default();
        }
    }

    let players: BTreeSet<PlayerId> = rooms.keys().copied().collect();
    for player in players {
        let rooms_owned = rooms
            .get(&player)
            .map_or(0, |set| u32::try_from(set.len()).unwrap_or(u32::MAX));
        let cost = config.upkeep_cost(rooms_owned);
        let balance = ledger.balances.entry(player).or_default();
        if *balance >= cost {
            *balance -= cost;
            shortfalls.deficits.remove(&player);
            continue;
        }

        let deficit = cost.saturating_sub(*balance);
        *balance = 0;
        shortfalls.deficits.insert(player, deficit);
        match config.onshortfall {
            ShortfallPolicy::Degrade => {
                for mut controller in &mut controllers {
                    if controller.owner == Some(player) {
                        controller.downgrade_timer =
                            controller.downgrade_timer.saturating_sub(deficit);
                        if controller.downgrade_timer == 0 && controller.level > 0 {
                            controller.level -= 1;
                        }
                    }
                }
            }
            ShortfallPolicy::Damage => {
                for (entity, mut drone, _) in &mut drones {
                    if drone.owner == player {
                        drone.hits = drone.hits.saturating_sub(deficit.max(1));
                        if drone.hits == 0 {
                            commands.entity(entity).insert(DeathMark);
                        }
                    }
                }
            }
            ShortfallPolicy::Despawn => {
                let mut owned: Vec<_> = drones
                    .iter()
                    .filter(|(_, drone, _)| drone.owner == player)
                    .map(|(entity, drone, _)| (drone.age, entity))
                    .collect();
                owned.sort_by_key(|(age, entity)| (std::cmp::Reverse(*age), entity.to_bits()));
                if let Some((_, entity)) = owned.first() {
                    commands.entity(*entity).insert(DeathMark);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_upkeep_policy_degrades_on_shortfall() {
        let config = EmpireUpkeepConfig::default();

        assert_eq!(config.base_upkeep, 50);
        assert_eq!(config.room_soft_cap, 10);
        assert_eq!(config.onshortfall, ShortfallPolicy::Degrade);
    }

    #[test]
    fn ledgers_start_empty() {
        assert!(PlayerEnergyLedger::default().balances.is_empty());
        assert!(UpkeepShortfalls::default().deficits.is_empty());
    }

    #[test]
    fn descriptor_is_valid_and_identifies_empire_upkeep() {
        let descriptor = EmpireUpkeepModPlugin::descriptor();
        swarm_engine_api::validation::assert_valid_descriptor(&descriptor);
        assert_eq!(descriptor.id, "empire-upkeep");
        assert_eq!(descriptor.config.len(), 10);
        assert_eq!(descriptor.systems.len(), 1);
        assert_eq!(
            descriptor
                .config
                .iter()
                .map(|field| field.key.as_str())
                .collect::<Vec<_>>(),
            [
                "base_upkeep",
                "room_soft_cap",
                "controller_passive_income",
                "controller_passive_income_rcl_bonus",
                "resource",
                "repair_cap",
                "distance_decay_bp",
                "recycle_refund_base",
                "recycle_refund_min",
                "tutorial_recycle_refund_full_ticks",
            ]
        );
        assert!(descriptor.dependencies.is_empty());
        assert_eq!(
            descriptor.systems[0].writes,
            [
                "PlayerEnergyLedger",
                "UpkeepShortfalls",
                "Controller",
                "Drone",
                "DeathMark",
            ]
        );
    }
}
