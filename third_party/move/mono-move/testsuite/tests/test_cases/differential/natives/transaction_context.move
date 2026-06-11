// Differential test for `transaction_context::generate_unique_address` and
// `monotonically_increasing_counter_internal`.
//
// Both VMs seed the transaction-context extension with the same fixed inputs
// (transaction hash, session counter, transaction index), so the derived
// addresses and counters match.

// RUN: publish
module 0x1::transaction_context {
    public native fun generate_unique_address(): address;
    native fun monotonically_increasing_counter_internal(timestamp_us: u64): u128;

    public fun gen(): address {
        generate_unique_address()
    }

    // Successive calls within a transaction must differ (the AUID counter
    // increments each call).
    public fun two_differ(): bool {
        generate_unique_address() != generate_unique_address()
    }

    // With timestamp 0 the counter is `transaction_index << 24 | session_counter
    // << 16 | local_counter`, and the first call sets local_counter to 1.
    public fun counter(): u128 {
        monotonically_increasing_counter_internal(0)
    }

    // The per-session local counter increments each call.
    public fun counters_increase(): bool {
        monotonically_increasing_counter_internal(0) < monotonically_increasing_counter_internal(0)
    }
}

// RUN: execute 0x1::transaction_context::gen
// CHECK: results: 0xfed7af230c8570d8202056f471c0a73b3dabb935969297cd6294fa507aa196a8

// RUN: execute 0x1::transaction_context::two_differ
// CHECK: results: true

// RUN: execute 0x1::transaction_context::counter
// CHECK: results: 84017153

// RUN: execute 0x1::transaction_context::counters_increase
// CHECK: results: true
