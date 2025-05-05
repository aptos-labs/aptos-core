module poc::secondary_signers_internal {
    use aptos_framework::transaction_context;

    public entry fun main(_owner:&signer) {
        let _secondary_signers_vec = transaction_context::secondary_signers();
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code=196609, location = aptos_framework::transaction_context)]
    fun a(owner:&signer){
        main(owner);
    }
}
