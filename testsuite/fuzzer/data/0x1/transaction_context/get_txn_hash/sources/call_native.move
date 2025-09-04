module poc::get_txn_hash {
    use velor_framework::transaction_context;

    public entry fun main(_owner: &signer) {
        let _hash_bytes = transaction_context::get_transaction_hash();
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
