module poc::sender_internal {
    use velor_framework::transaction_context;

    public entry fun main(_owner:&signer) {
        let _sender_addr = transaction_context::sender();
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code=196609, location = velor_framework::transaction_context)]
    fun a(owner:&signer){
        main(owner);
    }
}
