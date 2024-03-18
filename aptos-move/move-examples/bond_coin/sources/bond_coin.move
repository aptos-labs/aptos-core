module bond_coin::bond_coin {
    struct BondCoin {}

    fun init_module(sender: &signer) {
        aptos_framework::managed_coin::initialize<BondCoin>(
            sender,
            b"Bond Coin",
            b"BOND",
            0,
            false,
        );
    }
}
