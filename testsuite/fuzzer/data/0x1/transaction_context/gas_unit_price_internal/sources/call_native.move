module poc::gas_unit_price_internal {
    use aptos_framework::transaction_context;

    public entry fun main(_owner:&signer) {
        let _price = transaction_context::gas_unit_price();
    }

    #[test(owner=@0x123)]
    #[expected_failure(abort_code=196609, location = aptos_framework::transaction_context)]
    fun a(owner:&signer){
        main(owner);
    }
}
