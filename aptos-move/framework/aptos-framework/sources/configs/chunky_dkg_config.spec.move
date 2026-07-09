spec aptos_framework::chunky_dkg_config {
    spec initialize(framework: &signer, config: ChunkyDKGConfig) {
        pragma opaque;
        include config_buffer::InitializeResource<ChunkyDKGConfig>;
    }

    spec set_for_next_epoch(framework: &signer, new_config: ChunkyDKGConfig) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<ChunkyDKGConfig>;
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<ChunkyDKGConfig>;
    }

    spec new_off {
        pragma opaque;
        aborts_if false;
        ensures result == ChunkyDKGConfig { variant: copyable_any::pack(ConfigOff {}) };
    }

    spec new_v1(secrecy_threshold: FixedPoint64, reconstruction_threshold: FixedPoint64): ChunkyDKGConfig {
        pragma opaque;
        aborts_if false;
        ensures result == ChunkyDKGConfig {
            variant: copyable_any::pack(ConfigV1 { secrecy_threshold, reconstruction_threshold })
        };
    }

    spec new_shadow_v1(
        secrecy_threshold: FixedPoint64,
        reconstruction_threshold: FixedPoint64,
        grace_period_secs: u64,
    ): ChunkyDKGConfig {
        pragma opaque;
        aborts_if false;
        ensures result == ChunkyDKGConfig {
            variant: copyable_any::pack(ConfigShadowV1 {
                secrecy_threshold,
                reconstruction_threshold,
                grace_period_secs,
            })
        };
    }

    spec current {
        aborts_if false;
        ensures exists<ChunkyDKGConfig>(@aptos_framework) ==>
            result == global<ChunkyDKGConfig>(@aptos_framework);
        ensures !exists<ChunkyDKGConfig>(@aptos_framework) ==>
            result == ChunkyDKGConfig { variant: copyable_any::pack(ConfigOff {}) };
    }
}
