// Differential test for `consensus_config::validator_txn_enabled_internal`.
//
// The native BCS-decodes an `OnChainConsensusConfig` from its argument and
// returns whether validator transactions are enabled. It reads no VM context,
// so both VMs agree on any input. Empty bytes fail to decode and fall back to
// the default config (validator txns disabled); the `enabled` blob is a V3
// config whose `vtxn` field is `V1`.
//
// Bytes: 02 = V3; 01 = DAG algorithm (8-byte window, anchor election 00); then
// 00 = vtxn `V0`, or 01 = `V1` and two u64 limits. `vtxn_off` decodes to `V0`, but
// the default config also gives `false`, so that step cannot tell a decode from the
// fallback. The `enabled` blob plus a trailing byte fails to decode and falls back.

// RUN: publish
module 0x1::consensus_config {
    native fun validator_txn_enabled_internal(config_bytes: vector<u8>): bool;

    public fun disabled(): bool {
        validator_txn_enabled_internal(vector[])
    }

    public fun enabled(): bool {
        validator_txn_enabled_internal(x"02010000000000000000000100000000000000000000000000000000")
    }

    public fun vtxn_off(): bool {
        validator_txn_enabled_internal(x"020100000000000000000000")
    }

    public fun enabled_trailing_byte(): bool {
        validator_txn_enabled_internal(x"0201000000000000000000010000000000000000000000000000000000")
    }
}

// RUN: execute 0x1::consensus_config::disabled
// CHECK: results: false

// RUN: execute 0x1::consensus_config::enabled
// CHECK: results: true

// RUN: execute 0x1::consensus_config::vtxn_off
// CHECK: results: false

// RUN: execute 0x1::consensus_config::enabled_trailing_byte
// CHECK: results: false
