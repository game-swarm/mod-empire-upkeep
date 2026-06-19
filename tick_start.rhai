// tick_start.rhai — empire-upkeep
// Executed at the start of each tick, before combat/spawning
// Computes per-room upkeep costs, controller repair, and recycle refunds
//
// state:  read-only world snapshot (global view, no fog_of_war)
// actions: write buffer (deduct_resource, add_resource, spawn_npc, etc.)

// Iterate all players and their rooms
for player in state.players() {
    for room in player.rooms() {
        for drone in room.drones() {
            // Mod-specific logic here
        }
    }
}
