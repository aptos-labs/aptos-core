spec aptos_framework::randomness_config {
    spec current {
        aborts_if false;
    }
    spec on_new_epoch() {
        pragma verify = false;
    }

    spec on_new_epoch_v2(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<RandomnessConfig>;
        aborts_if false;
    }
}
