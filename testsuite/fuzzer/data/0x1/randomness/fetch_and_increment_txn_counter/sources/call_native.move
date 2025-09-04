module poc::fetch_and_increment_txn_counter {
    use velor_framework::randomness;


    #[lint::allow_unsafe_randomness]
    public entry fun main(_owner: &signer) {
        let _a = randomness::u8_integer();
    }

    #[test(owner=@0x1)]
    fun a(owner:&signer){
        randomness::initialize_for_testing(owner);
        main(owner);
    }
}
