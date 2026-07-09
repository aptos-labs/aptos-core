// Differential test for `state_storage::get_state_storage_usage_only_at_epoch_beginning`.
//
// Both VMs are seeded with the same fixed usage (items=100, bytes=2000) via
// their state-storage native extension.

// RUN: publish
module 0x1::state_storage {
    struct Usage has copy, drop, store {
        items: u64,
        bytes: u64,
    }

    public native fun get_state_storage_usage_only_at_epoch_beginning(): Usage;

    public fun items(): u64 {
        get_state_storage_usage_only_at_epoch_beginning().items
    }

    public fun bytes(): u64 {
        get_state_storage_usage_only_at_epoch_beginning().bytes
    }
}

// RUN: execute 0x1::state_storage::items
// CHECK: results: 100

// RUN: execute 0x1::state_storage::bytes
// CHECK: results: 2000
