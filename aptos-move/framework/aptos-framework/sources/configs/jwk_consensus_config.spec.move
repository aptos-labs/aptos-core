spec aptos_framework::jwk_consensus_config {
    spec initialize(framework: &signer, config: JWKConsensusConfig) {
        pragma opaque;
        include config_buffer::InitializeResource<JWKConsensusConfig>;
    }

    spec set_for_next_epoch(framework: &signer, config: JWKConsensusConfig) {
        pragma opaque;
        include config_buffer::SetForNextEpoch<JWKConsensusConfig> { new_config: config };
    }

    spec on_new_epoch(framework: &signer) {
        pragma opaque;
        include config_buffer::OnNewEpochApply<JWKConsensusConfig>;
    }

    spec new_off {
        pragma opaque;
        aborts_if false;
        ensures result == JWKConsensusConfig { variant: copyable_any::pack(ConfigOff {}) };
    }

    spec new_oidc_provider(name: String, config_url: String): OIDCProvider {
        pragma opaque;
        aborts_if false;
        ensures result == OIDCProvider { name, config_url };
    }
}
