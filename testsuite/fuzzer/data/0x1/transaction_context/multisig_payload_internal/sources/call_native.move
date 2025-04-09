module poc::multisig_payload_internal {
    use aptos_framework::transaction_context;

    public entry fun main(_owner:&signer) {
        let _payload_opt = transaction_context::multisig_payload();
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code=196609, location = aptos_framework::transaction_context)]
    fun a(owner:&signer){
        main(owner);
    }
}
