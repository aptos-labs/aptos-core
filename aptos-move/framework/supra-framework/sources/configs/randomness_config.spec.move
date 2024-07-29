spec supra_framework::randomness_config {
    spec current {
        aborts_if false;
    }

    spec on_new_epoch(framework: &signer) {
        requires @supra_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<RandomnessConfig>;
        aborts_if false;
    }
}
