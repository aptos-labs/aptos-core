spec aptos_std::type_info {
    // Move Prover natively supports `type_of` and `type_name`.

    spec type_of {
        pragma opaque;
        aborts_if [abstract] false;
    }

    spec chain_id_internal {
        // TODO: temporary mockup.
        pragma opaque;
    }
}
