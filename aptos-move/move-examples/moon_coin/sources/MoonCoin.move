//:!:>moon
module MoonCoin::moon_coin {
    use std::signer;

    /// The signer is not the module's account.
    const ENOT_AUTHORIZED: u64 = 1;

    struct MoonCoin {}

    public entry fun initialize(sender: &signer) {
        assert!(signer::address_of(sender) == @MoonCoin, ENOT_AUTHORIZED);
        aptos_framework::managed_coin::initialize<MoonCoin>(
            sender,
            b"Moon Coin",
            b"MOON",
            6,
            false,
        );
    }
}
//<:!:moon
