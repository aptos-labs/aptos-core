// Differential test for `hot_state_config::validate_internal`.
//
// `Config` mirrors the BCS layout of the Rust `OnChainHotStateConfig`. A valid
// config has 16 entries in each per-shard vector and no zero item cap.

// RUN: publish
module 0x1::hot_state_config {
    use std::bcs;
    use std::vector;

    enum Config has drop {
        V1 {
            max_items_per_shard: vector<u64>,
            max_bytes_per_shard: vector<u64>,
            refresh_interval_versions: u64,
            max_promotion_bytes_per_block: u64,
            defer_cold_writes_to_eviction: bool,
        }
    }

    native fun validate_internal(config: vector<u8>): bool;

    fun validate(max_items_per_shard: vector<u64>, max_bytes_per_shard: vector<u64>): bool {
        let config = Config::V1 {
            max_items_per_shard,
            max_bytes_per_shard,
            refresh_interval_versions: 1000,
            max_promotion_bytes_per_block: 1024,
            defer_cold_writes_to_eviction: false,
        };
        validate_internal(bcs::to_bytes(&config))
    }

    fun repeat(value: u64, n: u64): vector<u64> {
        let v = vector[];
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, value);
            i = i + 1;
        };
        v
    }

    public fun valid(): bool {
        validate(repeat(100, 16), repeat(4096, 16))
    }

    public fun wrong_items_len(): bool {
        validate(repeat(100, 15), repeat(4096, 16))
    }

    public fun wrong_bytes_len(): bool {
        validate(repeat(100, 16), repeat(4096, 17))
    }

    public fun zero_items(): bool {
        let items = repeat(100, 16);
        *vector::borrow_mut(&mut items, 7) = 0;
        validate(items, repeat(4096, 16))
    }

    public fun malformed(): bool {
        validate_internal(vector[])
    }
}

// RUN: execute 0x1::hot_state_config::valid
// CHECK: results: true

// RUN: execute 0x1::hot_state_config::wrong_items_len
// CHECK: results: false

// RUN: execute 0x1::hot_state_config::wrong_bytes_len
// CHECK: results: false

// RUN: execute 0x1::hot_state_config::zero_items
// CHECK: results: false

// RUN: execute 0x1::hot_state_config::malformed
// CHECK: results: false
