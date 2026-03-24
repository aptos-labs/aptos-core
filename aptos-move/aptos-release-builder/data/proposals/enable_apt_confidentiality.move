// Enable confidential transfers for APT.
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::confidential_asset;

    fun main(proposal_id: u64) {
        let framework = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );

        confidential_asset::set_confidentiality_for_apt(&framework, true);
    }
}
