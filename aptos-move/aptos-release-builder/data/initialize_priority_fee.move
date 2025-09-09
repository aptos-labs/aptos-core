script {
    use aptos_framework::aptos_governance;
    use aptos_framework::stake;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );
        stake::initialize_pending_transaction_fee(
            &framework_signer
        );
    }
}
