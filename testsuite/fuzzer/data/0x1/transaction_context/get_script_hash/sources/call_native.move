module poc::get_script_hash {
    use aptos_framework::transaction_context;

    public entry fun main(_owner: &signer) {
        let _script_hash = transaction_context::get_script_hash();
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
