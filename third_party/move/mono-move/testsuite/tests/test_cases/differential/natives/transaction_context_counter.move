// The transaction-context counter natives and the `_for_test_only` natives. The counter is
// `reserved_byte << 120 | timestamp_us << 56 | transaction_index << 24 |
// session_counter << 16 | local_counter`, with reserved byte 0, transaction index 5 and
// session counter 2 in the test context. A nonzero timestamp and u64::MAX (which must be
// widened before the shift) land in bits 56..119. Both counter natives advance the same
// local counter: 65535 calls succeed and the next one aborts. The test transaction is not
// orderless.

// RUN: publish
module 0x1::transaction_context {
    native fun monotonically_increasing_counter_internal(timestamp_us: u64): u128;
    native fun monotonically_increasing_counter_internal_for_test_only(): u128;
    native fun is_orderless_txn_internal(): bool;
    native fun is_orderless_txn_internal_for_test_only(): bool;
    native fun is_multisig_payload_txn_internal_for_test_only(): bool;

    public fun counter_at(timestamp_us: u64): u128 {
        monotonically_increasing_counter_internal(timestamp_us)
    }

    // The test-only counter returns the local counter, which the other
    // counter native also advances.
    public fun shared_local_counter(): (u128, u128, u128) {
        let first = monotonically_increasing_counter_internal(0);
        let second = monotonically_increasing_counter_internal_for_test_only();
        let third = monotonically_increasing_counter_internal(0);
        (first, second, third)
    }

    public fun exhaust_counter(calls: u64): u128 {
        let last = 0;
        let done = 0;
        while (done < calls) {
            last = monotonically_increasing_counter_internal(0);
            done = done + 1;
        };
        last
    }

    public fun exhaust_test_only_counter(calls: u64): u128 {
        let last = 0;
        let done = 0;
        while (done < calls) {
            last = monotonically_increasing_counter_internal_for_test_only();
            done = done + 1;
        };
        last
    }

    public fun orderless_and_multisig(): (bool, bool, bool) {
        (
            is_orderless_txn_internal_for_test_only(),
            is_orderless_txn_internal(),
            is_multisig_payload_txn_internal_for_test_only(),
        )
    }
}

// 1 << 56 | 5 << 24 | 2 << 16 | 1
// RUN: execute 0x1::transaction_context::counter_at --args 1
// CHECK: results: 72057594121945089

// (2^64 - 1) << 56 | 5 << 24 | 2 << 16 | 1
// RUN: execute 0x1::transaction_context::counter_at --args 18446744073709551615
// CHECK: results: 1329227995784915872831749466326433793

// RUN: execute 0x1::transaction_context::shared_local_counter
// CHECK: results: 84017153, 2, 84017155

// RUN: execute 0x1::transaction_context::exhaust_counter --args 65535
// CHECK: results: 84082687

// RUN: execute 0x1::transaction_context::exhaust_counter --args 65536
// CHECK: aborted: code 196610 (Monotonically increasing counter has reached maximum value (too many calls in this session)) in 0x1::transaction_context

// RUN: execute 0x1::transaction_context::exhaust_test_only_counter --args 65535
// CHECK: results: 65535

// RUN: execute 0x1::transaction_context::exhaust_test_only_counter --args 65536
// CHECK: aborted: code 196610 (Monotonically increasing counter has reached maximum value (too many calls in this session)) in 0x1::transaction_context

// RUN: execute 0x1::transaction_context::orderless_and_multisig
// CHECK: results: false, false, true
