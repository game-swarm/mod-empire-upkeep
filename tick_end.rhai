// tick_end.rhai — empire-upkeep
// Executed at the end of each tick, after death_cleanup / resource recovery
// Typical use: cleanup temp state, record metrics, emit events

// Example: record metrics for this tick
// actions.record_metric(MOD_NAME, "metric_name", value);
