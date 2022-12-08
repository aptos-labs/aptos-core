spec aptos_std::type_info {
    // Move Prover natively supports `type_of` and `type_name`.

    spec native fun spec_is_struct<T>(): bool;

    // The chain ID is modeled as an uninterpreted function.
    spec fun spec_chain_id_internal(): u8;

    spec chain_id_internal(): u8 {
        pragma opaque;
        aborts_if false;
        ensures result == spec_chain_id_internal();
    }
}
