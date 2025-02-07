script {
    use aptos_framework::aptos_governance;

    fun main(account: &signer){
        let framework_signer = aptos_governance::get_signer_testnet_only(account,
@0000000000000000000000000000000000000000000000000000000000000001);
        aptos_governance::update_governance_config(
            &framework_signer,
            30000000000000000, // 300M min_voting_threshold
            aptos_governance::get_required_proposer_stake(),
            aptos_governance::get_voting_duration_secs(),
        );
        aptos_governance::reconfigure(&framework_signer);
    }
}