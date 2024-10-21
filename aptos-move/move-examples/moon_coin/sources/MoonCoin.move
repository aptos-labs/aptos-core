//:!:>moon
module MoonCoin::moon_coin {
    struct MoonCoin {}

    fun init_module(sender: &signer) {
        aptos_framework::managed_coin::initialize<MoonCoin>(
            sender,
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
    }



    public fun test_moon_coin(caller:&signer){

    }
}
//<:!:moon
