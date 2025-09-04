//:!:>moon
module MoonCoin::moon_coin {
    struct MoonCoin {}

    fun init_module(sender: &signer) {
        velor_framework::managed_coin::initialize<MoonCoin>(
            sender,
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
    }
}
//<:!:moon
