// Empty governance proposal to demonstrate functionality for including proposal in the release builder;
//
script {
    use aptos_framework::aptos_governance;

    fun main(proposal_id: u64) {
        let _framework_signer = aptos_governance::resolve(proposal_id, @0x1);
    }
}
