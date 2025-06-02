spec aptos_std::type_info {

    spec native fun spec_is_struct<T>(): bool;

    spec type_of<T>(): TypeInfo {
        // Move Prover natively supports this function.
        // This function will abort if `T` is not a struct type.
    }

    spec type_name<T>(): String {
        // Move Prover natively supports this function.
    }

    spec chain_id(): u8 {
        aborts_if !features::spec_is_enabled(features::APTOS_STD_CHAIN_ID_NATIVES);
        ensures result == spec_chain_id_internal();
    }

    spec chain_id_internal(): u8 {
        pragma opaque;
        aborts_if false;
        ensures result == spec_chain_id_internal();
    }

    // The chain ID is modeled as an uninterpreted function.
    spec fun spec_chain_id_internal(): u8;

    spec fun spec_size_of_val<T>(val_ref: T): u64 {
        len(std::bcs::serialize(val_ref))
    }

    spec size_of_val<T>(val_ref: &T): u64 {
        ensures result == spec_size_of_val<T>(val_ref);
    }
}
