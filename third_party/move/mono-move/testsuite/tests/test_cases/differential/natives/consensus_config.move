// Differential test for `consensus_config::validator_txn_enabled_internal`.
//
// The native BCS-decodes an `OnChainConsensusConfig` from its argument and
// returns whether validator transactions are enabled. It reads no VM context,
// so both VMs agree on any input. Empty bytes fail to decode and fall back to
// the default config (validator txns disabled); the `enabled` blob is a V3
// config whose `vtxn` field is `V1`.

// RUN: publish
module 0x1::consensus_config {
    native fun validator_txn_enabled_internal(config_bytes: vector<u8>): bool;

    public fun disabled(): bool {
        validator_txn_enabled_internal(vector[])
    }

    public fun enabled(): bool {
        validator_txn_enabled_internal(x"02010000000000000000000100000000000000000000000000000000")
    }
}

// RUN: execute 0x1::consensus_config::disabled
// CHECK: results: false

// RUN: execute 0x1::consensus_config::enabled
// CHECK: results: true
