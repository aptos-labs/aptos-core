// Empty governance proposal to demonstrate functionality for including proposal in the release builder;
//
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::transaction_fee;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0000000000000000000000000000000000000000000000000000000000000001,
            {{ script_hash }},
        );
        transaction_fee::convert_to_aptos_fa_burn_ref(&framework_signer);
    }
}
