use bevy::prelude::App;
use serde_json::json;
use swarm_engine_api::prelude::{RoomId, WorldMode};
use swarm_engine_plugin_sdk::prelude::{
    BodyPartRegistry, Drone, InstalledPluginDescriptors, NativeModConfig,
    NativeModInstallExpectation, NativeModRegisterContext, Position,
};
use swarm_mod_empire_upkeep::{EmpireUpkeepConfig, PlayerEnergyLedger, ShortfallPolicy, register};

#[test]
fn native_register_preserves_mod_local_defaults_and_upkeep_behavior() {
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

    let config = app.world().resource::<EmpireUpkeepConfig>();
    assert_eq!(config.drone_cost, 2);
    assert_eq!(config.room_base, 10);
    assert_eq!(config.room_superlinear, 1);
    assert_eq!(config.onshortfall, ShortfallPolicy::Degrade);

    app.world_mut()
        .resource_mut::<PlayerEnergyLedger>()
        .balances
        .insert(1, 100);
    app.world_mut().spawn((
        Drone::new(1, Vec::new(), &BodyPartRegistry::default()),
        Position {
            x: 5,
            y: 7,
            room: RoomId(3),
        },
    ));
    app.update();

    assert_eq!(
        app.world()
            .resource::<PlayerEnergyLedger>()
            .balances
            .get(&1),
        Some(&87)
    );
}
