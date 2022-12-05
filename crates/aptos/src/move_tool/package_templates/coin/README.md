# Example coin package for Aptos blockchain

This module defines `UserCoin`, a coin on the Aptos blockchain.

Aptos standard library has a `coin.move` module, which describes how coins are defined. Think of it as ERC20.

To define a new coin, one needs to create a struct with name of the coin. It's `struct UserCoin {}` here.

Struct itself is not a coin. You need to use this type as an argument to `coin::Coin` type, ie. `Coin<UserCoin>`.

The module also defines a couple of entrypoints to handle this new coin:
* `initialize(coin_admin: &signer)` - registers coin and adds permissions to mint them to the `coin_admin` account.
      It can only be executed once.
* `mint(coin_admin: &signer, to_addr: address, amount: u64)` - mints an amount of `Coin<UserCoin>`
  to `to_addr` balance. Should be signed with the `coin_admin` account.
* `burn(user: &signer, amount: u64)` - burns an `amount` of `Coin<UserCoin>` from `user` balance.
