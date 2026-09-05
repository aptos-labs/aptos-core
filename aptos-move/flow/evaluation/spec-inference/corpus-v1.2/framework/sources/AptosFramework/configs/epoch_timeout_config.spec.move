spec aptos_framework::epoch_timeout_config {
    spec initialize(framework: &signer) {
        pragma opaque;
        include config_buffer::InitializeResource<EpochTimeoutConfig> {
            config: EpochTimeoutConfig { force_end_grace_period_secs: std::option::spec_none() }
        };
    }

    spec set_for_next_epoch(framework: &signer, new_config: EpochTimeoutConfig) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<EpochTimeoutConfig>;
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<EpochTimeoutConfig>;
    }

    spec new_disabled {
        pragma opaque;
        aborts_if false;
        ensures result == EpochTimeoutConfig { force_end_grace_period_secs: std::option::spec_none() };
    }

    spec new_with_grace_period(grace_period_secs: u64): EpochTimeoutConfig {
        pragma opaque;
        aborts_if grace_period_secs == 0;
        ensures result == EpochTimeoutConfig {
            force_end_grace_period_secs: std::option::spec_some(grace_period_secs)
        };
    }

    spec force_end_grace_period_secs {
        aborts_if false;
        ensures exists<EpochTimeoutConfig>(@aptos_framework) ==>
            result == global<EpochTimeoutConfig>(@aptos_framework).force_end_grace_period_secs;
        ensures !exists<EpochTimeoutConfig>(@aptos_framework) ==>
            result == std::option::spec_none<u64>();
    }
}
