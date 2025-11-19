script {
    use aptos_framework::aptos_governance;
    use aptos_framework::system_addresses;

    fun three_min_voting(core_resources: &signer) {
        let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let min_voting_threshold = aptos_governance::get_min_voting_threshold();
        let required_proposer_stake = aptos_governance::get_required_proposer_stake();

        aptos_governance::update_governance_config(
            &core_signer,
            min_voting_threshold,
            required_proposer_stake,
            180, // 3min
        );
    }
}
