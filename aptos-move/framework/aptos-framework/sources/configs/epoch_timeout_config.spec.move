spec aptos_framework::epoch_timeout_config {
    spec on_new_epoch(framework: &signer) {
        requires @aptos_framework == std::signer::address_of(framework);
        include config_buffer::OnNewEpochRequirement<EpochTimeoutConfig>;
        aborts_if false;
    }

    spec new_with_grace_period(grace_period_secs: u64): EpochTimeoutConfig {
        aborts_if grace_period_secs == 0;
    }
}
