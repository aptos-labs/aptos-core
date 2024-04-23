spec aptos_framework::randomness_config {
    spec module {
        use aptos_framework::chain_status;
        invariant[suspendable] chain_status::is_operating() ==> exists<RandomnessConfig>(
            @aptos_framework
        );
    }

    spec current {
        aborts_if false;
    }
}
