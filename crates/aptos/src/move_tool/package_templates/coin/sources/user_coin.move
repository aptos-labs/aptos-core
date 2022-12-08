/// This module defines `UserCoin`, a coin on the Aptos blockchain.
///
/// Aptos standard library has a `coin.move` module, which describes how coins are defined. Think of it as ERC20.
/// To define a new coin, one needs to create a struct with name of the coin.
/// It's `struct UserCoin {}` here.
/// Struct itself is not a coin. You need to use this type as an argument to `coin::Coin` type, ie. `Coin<UserCoin>`.
///
/// The module also defines a couple of entrypoints to handle this new coin:
///    * `initialize(coin_admin: &signer)` - registers coin and adds permissions to mint them to the `coin_admin` account.
///      It can only be executed once.
///    * `mint(coin_admin: &signer, to_addr: address, amount: u64)` - mints an amount of `Coin<UserCoin>`
///      to `to_addr` balance. Should be signed with the `coin_admin` account.
///    * `burn(user: &signer, amount: u64)` - burns an `amount` of `Coin<UserCoin>` from `user` balance.
module coin_address::user_coin {
    use std::signer;
    use std::string::utf8;

    use aptos_framework::coin::{Self, MintCapability, BurnCapability};

    /// Signer account is not an admin of the coin.
    const ERR_NOT_ADMIN: u64 = 1;

    /// Invalid initialize() call, coin has already been initialized.
    const ERR_COIN_INITIALIZED: u64 = 2;

    /// Coin is not initialized, call initialize() beforehand.
    const ERR_COIN_NOT_INITIALIZED: u64 = 3;

    /// COIN struct is a parameter to be used as a generic, coin itself is a resource of type `Coin<COIN>`
    struct UserCoin {}

    /// Mint and burn functionality of coins in the Aptos ecosystem is accessed only by a specific accounts.
    /// Those accounts should have a special resources: `MintCapability<UserCoin>` and `BurnCapability<UserCoin>`
    /// respectively.
    ///
    /// This `Capabilities` resource stores those capability objects for later use.
    struct Capabilities has key { mint_cap: MintCapability<UserCoin>, burn_cap: BurnCapability<UserCoin> }

    /// Initializes the COIN struct as a Coin in the Aptos network.
    public entry fun initialize(coin_admin: &signer) {
        assert!(signer::address_of(coin_admin) == @coin_address, ERR_NOT_ADMIN);
        assert!(!coin::is_coin_initialized<UserCoin>(), ERR_COIN_INITIALIZED);

        let (burn_cap, freeze_cap, mint_cap) =
            coin::initialize<UserCoin>(
                coin_admin,
                utf8(b"UserCoin"),
                utf8(b"USER_COIN"),
                6,
                true
            );
        coin::destroy_freeze_cap(freeze_cap);

        let caps = Capabilities { mint_cap, burn_cap };
        move_to(coin_admin, caps);
    }

    /// Mints an `amount` of Coin<COIN> and deposits it to the address `to_addr`.
    public entry fun mint(coin_admin: &signer, to_addr: address, amount: u64) acquires Capabilities {
        assert!(signer::address_of(coin_admin) == @coin_address, ERR_NOT_ADMIN);
        assert!(coin::is_coin_initialized<UserCoin>(), ERR_COIN_NOT_INITIALIZED);

        let caps = borrow_global<Capabilities>(@coin_address);
        let coins = coin::mint(amount, &caps.mint_cap);
        coin::deposit(to_addr, coins);
    }

    /// Burns an `amount` of `Coin<COIN>` from user's balance.
    public entry fun burn(user: &signer, amount: u64) acquires Capabilities {
        assert!(coin::is_coin_initialized<UserCoin>(), ERR_COIN_NOT_INITIALIZED);

        let coin = coin::withdraw<UserCoin>(user, amount);
        let caps = borrow_global<Capabilities>(@coin_address);
        coin::burn(coin, &caps.burn_cap);
    }
}
