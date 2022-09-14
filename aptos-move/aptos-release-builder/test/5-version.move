script {
    use aptos_framework::aptos_governance;
    use aptos_framework::version;

    fun main(proposal_id: u64) {
        let framework_signer = aptos_governance::resolve(proposal_id, @0000000000000000000000000000000000000000000000000000000000000001);

        version::set_version(framework_signer, 4);
    }
}
