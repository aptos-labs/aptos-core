spec aptos_framework::jwk_consensus_config {

    spec module {
        use aptos_framework::chain_status;
        invariant[suspendable] chain_status::is_operating() ==> exists<JWKConsensusConfig>(
            @aptos_framework
        );
    }

}
