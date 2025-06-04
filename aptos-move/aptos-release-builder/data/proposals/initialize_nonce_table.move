// Initialize the nonce table in nonce_validation.move
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::nonce_validation;
    use std::string::utf8;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve_multi_step_proposal(
            proposal_id,
            @0x1,
            {{ script_hash }},
        );
        nonce_validation::initialize_nonce_table(
            &framework_signer
        );
    }
}
