module poc::generate_unique_address {
    use velor_framework::transaction_context;

    public entry fun main(_owner: &signer) {
        let _addr1 = transaction_context::generate_auid_address();
        let _addr2 = transaction_context::generate_auid_address();
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
