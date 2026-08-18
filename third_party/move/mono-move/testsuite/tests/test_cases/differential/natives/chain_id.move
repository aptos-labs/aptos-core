// Differential test for `type_info::chain_id_internal` and
// `transaction_context::chain_id_internal`.
//
// Both natives return the chain id carried by the transaction-context
// extension, which both VMs seed with the same fixed value (TEST_CHAIN_ID = 4).

// RUN: publish
module 0x1::type_info {
    public native fun chain_id_internal(): u8;

    public fun get(): u8 {
        chain_id_internal()
    }
}
module 0x1::transaction_context {
    public native fun chain_id_internal(): u8;

    public fun get(): u8 {
        chain_id_internal()
    }
}

// RUN: execute 0x1::type_info::get
// CHECK: results: 4

// RUN: execute 0x1::transaction_context::get
// CHECK: results: 4
