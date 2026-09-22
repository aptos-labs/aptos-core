spec aptos_framework::randomness_config {
    spec initialize(framework: &signer, config: RandomnessConfig) {
        pragma opaque;
        include config_buffer::InitializeResource<RandomnessConfig>;
    }

    spec set_for_next_epoch(framework: &signer, new_config: RandomnessConfig) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<RandomnessConfig>;
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<RandomnessConfig>;
    }

    spec current {
        aborts_if false;
        ensures exists<RandomnessConfig>(@aptos_framework) ==>
            result == global<RandomnessConfig>(@aptos_framework);
        ensures !exists<RandomnessConfig>(@aptos_framework) ==>
            result == RandomnessConfig { variant: copyable_any::pack(ConfigOff {}) };
    }

    spec new_off {
        pragma opaque;
        aborts_if false;
        ensures result == RandomnessConfig { variant: copyable_any::pack(ConfigOff {}) };
    }

    spec new_v1(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): RandomnessConfig {
        pragma opaque;
        aborts_if false;
        ensures result == RandomnessConfig {
            variant: copyable_any::pack(ConfigV1 { secrecy_threshold, reconstruction_threshold })
        };
    }

    spec new_v2(
        secrecy_threshold: FixedPoint64,
        reconstruction_threshold: FixedPoint64,
        fast_path_secrecy_threshold: FixedPoint64,
    ): RandomnessConfig {
        pragma opaque;
        aborts_if false;
        ensures result == RandomnessConfig {
            variant: copyable_any::pack(ConfigV2 {
                secrecy_threshold,
                reconstruction_threshold,
                fast_path_secrecy_threshold,
            })
        };
    }
}
