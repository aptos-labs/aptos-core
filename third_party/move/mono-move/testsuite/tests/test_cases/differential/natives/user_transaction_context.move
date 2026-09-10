// RUN: publish
module 0x1::transaction_context {
    native fun sender_internal(): address;
    native fun gas_payer_internal(): address;
    native fun max_gas_amount_internal(): u64;
    native fun gas_unit_price_internal(): u64;
    native fun is_encrypted_txn_internal(): bool;
    native fun is_orderless_txn_internal(): bool;
    native fun secondary_signers_internal(): vector<address>;
    native fun get_txn_hash(): vector<u8>;
    native fun get_script_hash(): vector<u8>;

    public fun sender(): address {
        sender_internal()
    }

    public fun gas_payer(): address {
        gas_payer_internal()
    }

    public fun max_gas_amount(): u64 {
        max_gas_amount_internal()
    }

    public fun gas_unit_price(): u64 {
        gas_unit_price_internal()
    }

    public fun is_encrypted_txn(): bool {
        is_encrypted_txn_internal()
    }

    public fun is_orderless_txn(): bool {
        is_orderless_txn_internal()
    }

    // `vector<address>` is not a renderable return type, so return the count.
    public fun num_secondary_signers(): u64 {
        let signers = secondary_signers_internal();
        std::vector::length(&signers)
    }

    public fun txn_hash(): vector<u8> {
        get_txn_hash()
    }

    public fun script_hash(): vector<u8> {
        get_script_hash()
    }
}

// RUN: execute 0x1::transaction_context::sender
// CHECK: results: 0x0

// RUN: execute 0x1::transaction_context::gas_payer
// CHECK: results: 0x0

// RUN: execute 0x1::transaction_context::max_gas_amount
// CHECK: results: 0

// RUN: execute 0x1::transaction_context::gas_unit_price
// CHECK: results: 0

// RUN: execute 0x1::transaction_context::is_encrypted_txn
// CHECK: results: false

// RUN: execute 0x1::transaction_context::is_orderless_txn
// CHECK: results: false

// RUN: execute 0x1::transaction_context::num_secondary_signers
// CHECK: results: 0

// RUN: execute 0x1::transaction_context::txn_hash
// CHECK: results: 0x0707070707070707070707070707070707070707070707070707070707070707

// RUN: execute 0x1::transaction_context::script_hash
// CHECK: results: 0x01
