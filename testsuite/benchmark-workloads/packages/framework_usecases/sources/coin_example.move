module 0xABCD::coin_example {
    use std::signer;

    /// Caller is not the module publisher.
    const ENOT_AUTHORIZED: u64 = 1;

    struct ExampleCoin {}

    public entry fun initialize(sender: &signer) {
        assert!(signer::address_of(sender) == @publisher_address, ENOT_AUTHORIZED);
        aptos_framework::managed_coin::initialize<ExampleCoin>(
            sender,
            b"Example Coin",
            b"Example",
            8,
            false,
        );
    }

    public entry fun mint_p(user: &signer, admin: &signer, amount: u64) {
        aptos_framework::managed_coin::register<ExampleCoin>(user);
        aptos_framework::managed_coin::mint<ExampleCoin>(admin, signer::address_of(user), amount);
    }
}
