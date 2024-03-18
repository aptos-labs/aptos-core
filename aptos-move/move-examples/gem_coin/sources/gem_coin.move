module gem_coin::gem_coin {
    struct GemCoin {}

    fun init_module(sender: &signer) {
        aptos_framework::managed_coin::initialize<GemCoin>(
            sender,
            b"Gem Coin",
            b"Gem",
            8,
            false,
        );
    }
}
