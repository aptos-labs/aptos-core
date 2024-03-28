spec aptos_framework::jwk_consensus_config {
    spec on_new_epoch() {
        include config_buffer::OnNewEpochAbortsIf<JWKConsensusConfig>;
    }
}
