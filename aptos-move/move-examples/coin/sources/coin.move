/// This module provides the foundation for typesafe Coins.
module coin_example::coin {
    /// Core data structures

    struct Coin<phantom CoinType> {
        /// Amount of coin this address has.
        value: u128,
    }

    struct CoinData has store {
        balance: u128,
        frozen: bool,
    }

    struct CoinStore has key {
        // Probably should be Aaron unordered map to co-locate balances in a single slot
        balances: aptos_std::table::Table<std::string::String, CoinData>,
        deposit_event: aptos_framework::event::EventHandle<DepositEvent>,
        withdraw_event: aptos_framework::event::EventHandle<WithdrawEvent>,
    }

    /// Information about a specific coin type. Stored on the creator of the coin's account.
    struct CoinInfo<phantom CoinType> has key {
        name: std::string::String,
        /// Symbol of the coin, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: std::string::String,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
        /// Amount of this coin type in existence.
        supply: u128,
    }

    /// Event emitted when some amount of a coin is deposited into an account.
    struct DepositEvent has drop, store {
        amount: u128,
    }

    /// Event emitted when some amount of a coin is withdrawn from an account.
    struct WithdrawEvent has drop, store {
        amount: u128,
    }

    /// Capability required to mint coins.
    struct MintCapability<phantom CoinType> has copy, store {}

    /// Capability required to freeze a coin store.
    struct FreezeCapability<phantom CoinType> has copy, store {}

    /// Capability required to burn coins.
    struct BurnCapability<phantom CoinType> has copy, store {}


    fun mint_internal<T>(amount: u128): Coin<T> {
        Coin { value: amount }
    }

    fun burn_internal<T>(coin: Coin<T>): u128 {
        let Coin { value: amount} = coin;
        amount
    }

    public entry fun mint<T>(amount: u128, _: &MintCapability<T>): Coin<T> acquires CoinInfo {
        let coin_info = borrow_global_mut<CoinInfo<T>>(@coin_example);
        coin_info.supply = coin_info.supply + amount;
        mint_internal<T>(amount)
    }

    public entry fun burn<T>(coin: Coin<T>, _: &BurnCapability<T>) acquires CoinInfo {
        let amount = burn_internal(coin);
        let coin_info = borrow_global_mut<CoinInfo<T>>(@coin_example);
        coin_info.supply = coin_info.supply - amount;
    }

    /*public entry fun freeze_coin<T>(addr: &address, cap: &FreezeCapability<T>) {

    }*/

    public fun withdraw<T>(from: &signer, amount: u128): Coin<T> acquires CoinStore {
        let typename = std::type_info::type_name<T>();
        let coin_balance = borrow_global_mut<CoinStore>(std::signer::address_of(from));
        let coin_data = aptos_std::table::borrow_mut(&mut coin_balance.balances, typename);
        coin_data.balance = coin_data.balance - amount;
        aptos_framework::event::emit_event(&mut coin_balance.withdraw_event, WithdrawEvent { amount });
        Coin { value: amount }
    }

    public fun deposit<T>(to: &address, coin: Coin<T>): u128 acquires CoinStore {
        let typename = std::type_info::type_name<T>();
        let coin_balance = borrow_global_mut<CoinStore>(*to);
        let coin_data = aptos_std::table::borrow_mut(&mut coin_balance.balances, typename);
        let Coin { value: amount } = coin;
        aptos_framework::event::emit_event(&mut coin_balance.deposit_event, DepositEvent { amount });
        coin_data.balance = coin_data.balance + amount;
        amount
    }

    public fun merge<T>(a: Coin<T>, b: Coin<T>): Coin<T> {
        join(&mut a, b);
        a
    }

    public fun join<T>(a: &mut Coin<T>, b: Coin<T>) {
        let Coin { value: b_amount} = b;
        a.value = a.value + b_amount;
    }

    public fun split<T>(a: &mut Coin<T>, amount: u128): Coin<T> {
        a.value = a.value - amount;
        Coin { value: amount }
    }

    /// A helper function that returns the address of CoinType.
    fun coin_address<CoinType>(): address {
        let type_info = std::type_info::type_of<CoinType>();
        std::type_info::account_address(&type_info)
    }

    #[view]
    /// Returns the balance of `owner` for provided `CoinType`.
    public fun balance<T>(owner: address): u128 acquires CoinStore {
        let typename = std::type_info::type_name<T>();
        let coin_balance = borrow_global_mut<CoinStore>(owner);
        let coin_data = aptos_std::table::borrow_mut(&mut coin_balance.balances, typename);
        coin_data.balance
    }

    /// Returns the name of the coin.
    public fun name<CoinType>(): std::string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(@coin_example).name
    }

    /// Returns the symbol of the coin, usually a shorter version of the name.
    public fun symbol<CoinType>(): std::string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(@coin_example).symbol
    }

    /// Returns the number of decimals used to get its user representation.
    /// For example, if `decimals` equals `2`, a balance of `505` coins should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    public fun decimals<CoinType>(): u8 acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(@coin_example).decimals
    }

    /// Returns the amount of coin in existence.
    public fun supply<CoinType>(): u128 acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(@coin_example).supply
    }

    /// Transfers `amount` of coins `CoinType` from `from` to `to`.
    public entry fun transfer<CoinType>(
        from: &signer,
        to: address,
        amount: u128,
    ) acquires CoinStore {
        let coin = withdraw<CoinType>(from, amount);
        deposit(&to, coin);
    }

    /// Returns the `value` passed in `coin`.
    public fun value<CoinType>(coin: &Coin<CoinType>): u128 {
        coin.value
    }

    #[test]
    fun coin_test() {}
}
