// RUN: publish
module 0x1::transaction_context {
    use std::option::{Self, Option};
    use std::string::String;

    struct EntryFunctionPayload has copy, drop {
        account_address: address,
        module_name: String,
        function_name: String,
        ty_args_names: vector<String>,
        args: vector<vector<u8>>,
    }

    struct MultisigPayload has copy, drop {
        multisig_address: address,
        entry_function_payload: Option<EntryFunctionPayload>,
    }

    native fun entry_function_payload_internal(): Option<EntryFunctionPayload>;
    native fun multisig_payload_internal(): Option<MultisigPayload>;
    native fun is_multisig_payload_txn_internal(): bool;
    native fun is_multisig_payload_txn_internal_for_test_only(): bool;

    fun entry_payload(): EntryFunctionPayload {
        option::destroy_some(entry_function_payload_internal())
    }

    fun multisig(): MultisigPayload {
        option::destroy_some(multisig_payload_internal())
    }

    public fun entry_payload_is_some(): bool {
        option::is_some(&entry_function_payload_internal())
    }

    public fun entry_payload_address(): address {
        entry_payload().account_address
    }

    public fun entry_payload_module(): String {
        entry_payload().module_name
    }

    public fun entry_payload_function(): String {
        entry_payload().function_name
    }

    public fun entry_payload_num_ty_args(): u64 {
        std::vector::length(&entry_payload().ty_args_names)
    }

    public fun entry_payload_ty_arg(i: u64): String {
        *std::vector::borrow(&entry_payload().ty_args_names, i)
    }

    public fun entry_payload_num_args(): u64 {
        std::vector::length(&entry_payload().args)
    }

    public fun entry_payload_arg(i: u64): vector<u8> {
        *std::vector::borrow(&entry_payload().args, i)
    }

    public fun multisig_is_some(): bool {
        option::is_some(&multisig_payload_internal())
    }

    public fun multisig_address(): address {
        multisig().multisig_address
    }

    public fun multisig_inner_is_some(): bool {
        option::is_some(&multisig().entry_function_payload)
    }

    public fun is_multisig_txn(): bool {
        is_multisig_payload_txn_internal()
    }

    public fun is_multisig_txn_for_test(): bool {
        is_multisig_payload_txn_internal_for_test_only()
    }

    // Fills the heap on both sides of the call, so the native allocates the
    // payload under pressure and the collector then relocates it.
    public fun entry_payload_survives_gc(rounds: u64): (String, vector<u8>) {
        churn(rounds);
        let payload = entry_payload();
        churn(rounds);
        (payload.module_name, *std::vector::borrow(&payload.args, 0))
    }

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + std::vector::length(&junk);
        };
    }
}

// RUN: execute 0x1::transaction_context::entry_payload_is_some
// CHECK: results: true

// RUN: execute 0x1::transaction_context::entry_payload_address
// CHECK: results: 0x1

// RUN: execute 0x1::transaction_context::entry_payload_module
// CHECK: results: "some_module"

// RUN: execute 0x1::transaction_context::entry_payload_function
// CHECK: results: "some_function"

// RUN: execute 0x1::transaction_context::entry_payload_num_ty_args
// CHECK: results: 2

// RUN: execute 0x1::transaction_context::entry_payload_ty_arg --args 0
// CHECK: results: "u64"

// RUN: execute 0x1::transaction_context::entry_payload_ty_arg --args 1
// CHECK: results: "0x1::string::String"

// RUN: execute 0x1::transaction_context::entry_payload_num_args
// CHECK: results: 2

// RUN: execute 0x1::transaction_context::entry_payload_arg --args 0
// CHECK: results: 0x010203

// RUN: execute 0x1::transaction_context::entry_payload_arg --args 1
// CHECK: results: 0x

// RUN: execute 0x1::transaction_context::multisig_is_some
// CHECK: results: true

// RUN: execute 0x1::transaction_context::multisig_address
// CHECK: results: 0x2

// RUN: execute 0x1::transaction_context::multisig_inner_is_some
// CHECK: results: false

// RUN: execute 0x1::transaction_context::is_multisig_txn
// CHECK: results: true

// RUN: execute 0x1::transaction_context::is_multisig_txn_for_test
// CHECK: results: true

// RUN: execute 0x1::transaction_context::entry_payload_survives_gc --args 0 --heap-size 1024
// CHECK: results: "some_module", 0x010203
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::transaction_context::entry_payload_survives_gc --args 2000 --heap-size 1024
// CHECK: results: "some_module", 0x010203
// CHECK-GC-COUNT: 58
