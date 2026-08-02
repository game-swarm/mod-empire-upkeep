use bevy::prelude::App;
use serde_json::json;
use swarm_engine_api::prelude::{RoomId, WorldMode};
use swarm_engine_plugin_sdk::prelude::{
    BodyPartRegistry, DeathMark, Drone, InstalledPluginDescriptors, NativeModConfig,
    NativeModInstallExpectation, NativeModRegisterContext, NativeModRegisterError, Position,
};
use swarm_mod_empire_upkeep::{
    EmpireUpkeepConfig, EmpireUpkeepModPlugin, PlayerEnergyLedger, ShortfallPolicy,
    UpkeepShortfalls, register,
};

#[test]
fn native_register_maps_every_canonical_config_value_into_the_runtime_resource() {
    let mut app = App::new();
    let mut context = NativeModRegisterContext::new(
        &mut app,
        "empire-upkeep",
        WorldMode::Default,
        NativeModConfig::from_defaults(json!({
            "base_upkeep": 999,
            "room_soft_cap": 77,
            "controller_passive_income": 88,
            "controller_passive_income_rcl_bonus": 9,
            "resource": "Crystal",
            "repair_cap": 1234,
            "distance_decay_bp": 4321,
            "recycle_refund_base": 6789,
            "recycle_refund_min": 456,
            "tutorial_recycle_refund_full_ticks": 42
        })),
        NativeModInstallExpectation::enabled("0.1.0"),
    );

    register(&mut context).expect("register empire-upkeep");

    let descriptor = app
        .world()
        .resource::<InstalledPluginDescriptors>()
        .get("empire-upkeep")
        .expect("installed descriptor");
    assert_eq!(descriptor.version, "0.1.0");
    assert!(descriptor.systems.is_empty());
    assert!(descriptor.actions.is_empty());
    assert_eq!(descriptor.config.len(), 10);
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

    let config = app.world().resource::<EmpireUpkeepConfig>();
    assert_eq!(config.base_upkeep, 999);
    assert_eq!(config.room_soft_cap, 77);
    assert_eq!(config.controller_passive_income, 88);
    assert_eq!(config.controller_passive_income_rcl_bonus, 9);
    assert_eq!(config.resource, "Crystal");
    assert_eq!(config.repair_cap, 1234);
    assert_eq!(config.distance_decay_bp, 4321);
    assert_eq!(config.recycle_refund_base, 6789);
    assert_eq!(config.recycle_refund_min, 456);
    assert_eq!(config.tutorial_recycle_refund_full_ticks, 42);
    assert_eq!(config.onshortfall, ShortfallPolicy::Degrade);

    assert!(app.world().get_resource::<PlayerEnergyLedger>().is_none());
    assert!(app.world().get_resource::<UpkeepShortfalls>().is_none());

    app.update();

    assert!(app.world().get_resource::<PlayerEnergyLedger>().is_none());
    assert!(app.world().get_resource::<UpkeepShortfalls>().is_none());
}

#[test]
fn native_register_rejects_unknown_fields_without_installing_the_plugin() {
    let mut app = App::new();
    let error = {
        let mut context = NativeModRegisterContext::new(
            &mut app,
            "empire-upkeep",
            WorldMode::Default,
            NativeModConfig::from_defaults(json!({
                "base_upkeep": 50,
                "room_soft_cap": 10,
                "controller_passive_income": 40,
                "controller_passive_income_rcl_bonus": 5,
                "resource": "Energy",
                "repair_cap": 3500,
                "distance_decay_bp": 500,
                "recycle_refund_base": 5000,
                "recycle_refund_min": 1000,
                "tutorial_recycle_refund_full_ticks": 500,
                "unexpected": true
            })),
            NativeModInstallExpectation::enabled("0.1.0"),
        );

        register(&mut context).expect_err("unknown config field must fail registration")
    };

    assert!(matches!(
        error,
        NativeModRegisterError::InvalidConfig { .. }
    ));
    assert!(app.world().get_resource::<EmpireUpkeepConfig>().is_none());
    assert!(
        app.world()
            .get_resource::<InstalledPluginDescriptors>()
            .is_none()
    );
}

#[test]
fn app_update_charges_configured_base_and_room_soft_cap_upkeep() {
    let mut app = standalone_app(EmpireUpkeepConfig {
        base_upkeep: 25,
        room_soft_cap: 2,
        ..EmpireUpkeepConfig::default()
    });

    app.world_mut()
        .resource_mut::<PlayerEnergyLedger>()
        .balances
        .insert(1, 1_000);
    for room in [1, 2, 3] {
        app.world_mut().spawn((
            Drone::new(1, Vec::new(), &BodyPartRegistry::default()),
            Position {
                x: 5,
                y: 7,
                room: RoomId(room),
            },
        ));
    }

    app.update();

    assert_eq!(
        app.world()
            .resource::<PlayerEnergyLedger>()
            .balances
            .get(&1),
        Some(&813)
    );
}

#[test]
fn app_update_damage_shortfall_reduces_drone_hits() {
    let mut app = standalone_app(EmpireUpkeepConfig {
        onshortfall: ShortfallPolicy::Damage,
        ..EmpireUpkeepConfig::default()
    });
    app.world_mut()
        .resource_mut::<PlayerEnergyLedger>()
        .balances
        .insert(1, 50);
    let drone = app
        .world_mut()
        .spawn((
            Drone::new(1, Vec::new(), &BodyPartRegistry::default()),
            Position {
                x: 5,
                y: 7,
                room: RoomId(1),
            },
        ))
        .id();

    app.update();

    assert_eq!(app.world().entity(drone).get::<Drone>().unwrap().hits, 95);
}

#[test]
fn app_update_lethal_damage_shortfall_marks_the_drone_for_death() {
    let mut app = standalone_app(EmpireUpkeepConfig {
        onshortfall: ShortfallPolicy::Damage,
        ..EmpireUpkeepConfig::default()
    });
    let mut damaged_drone = Drone::new(1, Vec::new(), &BodyPartRegistry::default());
    damaged_drone.hits = 5;
    let drone = app
        .world_mut()
        .spawn((
            damaged_drone,
            Position {
                x: 5,
                y: 7,
                room: RoomId(1),
            },
        ))
        .id();

    app.update();

    let entity = app.world().entity(drone);
    assert_eq!(entity.get::<Drone>().unwrap().hits, 0);
    assert!(entity.contains::<DeathMark>());
}

fn standalone_app(config: EmpireUpkeepConfig) -> App {
    let mut app = App::new();
    app.insert_resource(config);
    app.add_plugins(EmpireUpkeepModPlugin);
    app
}
