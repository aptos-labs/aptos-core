

module SwapDeployer::TestCoinsV1 {
    use std::signer;
    use std::string::utf8;

    use aptos_framework::coin::{Self, MintCapability, FreezeCapability, BurnCapability};

    /// Represents test USDT coin.
    struct USDT {}

    /// Represents test BTC coin.
    struct BTC {}

    /// Storing mint/burn capabilities for `USDT` and `BTC` coins under user account.
    struct Caps<phantom CoinType> has key {
        mint: MintCapability<CoinType>,
        freeze: FreezeCapability<CoinType>,
        burn: BurnCapability<CoinType>,
    }

    /// Initializes `BTC` and `USDT` coins.
    public entry fun initialize(admin: &signer) {
        let (btc_b, btc_f, btc_m) =
            coin::initialize<BTC>(admin,
                utf8(b"Bitcoin"), utf8(b"BTC"), 8, true);
        let (usdt_b, usdt_f, usdt_m) =
            coin::initialize<USDT>(admin,
                utf8(b"Tether"), utf8(b"USDT"), 8, true);
        move_to(admin, Caps<BTC> { mint: btc_m, freeze: btc_f, burn: btc_b });
        move_to(admin, Caps<USDT> { mint: usdt_m, freeze: usdt_f, burn: usdt_b });
        register_coins_all(admin);
    }

    // only resource_account should call this
    public entry fun register_coins_all(account: &signer) {
        let account_addr = signer::address_of(account);
        if (!coin::is_account_registered<BTC>(account_addr)) {
            coin::register<BTC>(account);
        };
        if (!coin::is_account_registered<USDT>(account_addr)) {
            coin::register<USDT>(account);
        };
    }

    // Mints new coin `CoinType` on account `acc_addr`.
    public entry fun mint_coin<CoinType>(admin: &signer, acc_addr: address, amount: u64) acquires Caps {
        let admin_addr = signer::address_of(admin);
        let caps = borrow_global<Caps<CoinType>>(admin_addr);
        let coins = coin::mint<CoinType>(amount, &caps.mint);
        coin::deposit(acc_addr, coins);
    }
}