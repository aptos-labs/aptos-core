/// This module provides the foundation for typesafe Coins.
module coin_example::coin {
    use aptos_framework::event;
    use aptos_framework::smart_table;
    use aptos_std::type_info;

    /// Core data structures

    const EFROZEN_ACCOUNT: u64 = 0;
    const EREVOKE_UNFROZEN_ACCOUNT: u64 = 1;

    /// Ephemeral representation of a coin, allowing transfer of value
    struct Coin<phantom CoinType> {
        value: u128,
    }

    struct Approval<phantom CoinType> has drop {
        acct: address,
        amount: u128,
    }

    /// If a coin allows it, a permanent version of coin
    struct StorableCoin<phantom CoinType> has store {
        value: u128,
    }

    /// Keep track of the balance of an account
    struct BalanceData has store {
        balance: u128,
        frozen: bool,
    }

    /// Balances of all coins are stored together in a map. This allows many balances to be saved
    /// in the same slot. This might hamper parallizeabilty, but paralizing different txs distributing
    /// different coins to the same account are probably not relevant.
    struct CoinStore has key {
        balances: smart_table::SmartTable<std::string::String, BalanceData>,
        deposit_event: event::EventHandle<DepositEvent>,
        withdraw_event: event::EventHandle<WithdrawEvent>,
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
        /// If true coin can be converted into storable coins
        storable: bool,
    }

    struct Events has key {
        mint_event: event::EventHandle<MintEvent>,
        burn_event: event::EventHandle<BurnEvent>,
    }

    /// Event emitted if new coins are minted
    struct MintEvent has drop, store {
        amount: u128,
        coin_type: std::string::String,
    }

    /// Event emitted if coins are burned
    struct BurnEvent has drop, store {
        amount: u128,
        coin_type: std::string::String,
    }

    /// Event emitted when some amount of a coin is deposited into an account.
    struct DepositEvent has drop, store {
        amount: u128,
        coin_type: std::string::String,
    }

    /// Event emitted when some amount of a coin is withdrawn from an account.
    struct WithdrawEvent has drop, store {
        amount: u128,
        coin_type: std::string::String,
    }

    /// Capability required to mint coins.
    struct MintCapability<phantom CoinType> has copy, store {}

    /// Capability required to freeze a coin store.
    struct FreezeCapability<phantom CoinType> has copy, store {}

    /// Capability required to burn coins.
    struct BurnCapability<phantom CoinType> has copy, store {}


    inline fun mint_internal<T>(amount: u128): Coin<T> {
        Coin { value: amount }
    }

    inline fun burn_internal<T>(coin: Coin<T>): u128 {
        let Coin { value: amount} = coin;
        amount
    }

    inline fun get_coin_data<T>(addr: address): &mut BalanceData acquires CoinStore {
        let typename = std::type_info::type_name<T>();
        let coin_store = borrow_global_mut<CoinStore>(addr);
        if (!smart_table::contains(&mut coin_store.balances, typename)) {
            smart_table::add(&mut coin_store.balances, typename,
                BalanceData {
                    balance: 0,
                    frozen: false,
                }
            );
        };
        smart_table::borrow_mut(&mut coin_store.balances, typename)
    }

    /// A helper function that returns the address of CoinType.
    inline fun coin_address<CoinType>(): address {
        let type_info = type_info::type_of<CoinType>();
        type_info::account_address(&type_info)
    }

    public fun create_coin<T>(acc: &signer, name: std::string::String, symbol: std::string::String,
                              decimals: u8, storable: bool): (MintCapability<T>, BurnCapability<T>, FreezeCapability<T>) {
        assert!(coin_address<T>() == std::signer::address_of(acc),
            std::error::invalid_argument(0),
        );
        move_to(acc, CoinInfo<T> { name, symbol, decimals, supply: 0, storable});
        (MintCapability {}, BurnCapability {}, FreezeCapability {})
    }

    public fun convert_to_storable<T>(a: Coin<T>): StorableCoin<T> acquires CoinInfo {
        assert!(borrow_global<CoinInfo<T>>(coin_address<T>()).storable, 0);
        StorableCoin { value: burn_internal(a)}
    }

    public fun convert_from_storable<T>(a: StorableCoin<T>): Coin<T> acquires CoinInfo {
        assert!(borrow_global<CoinInfo<T>>(coin_address<T>()).storable, 0);
        let StorableCoin { value: amount } = a;
        mint_internal(amount)
    }

    public fun mint<T>(amount: u128, _: &MintCapability<T>): Coin<T> acquires CoinInfo, Events {
        let coin_info = borrow_global_mut<CoinInfo<T>>(@coin_example);
        coin_info.supply = coin_info.supply + amount;
        event::emit_event(&mut borrow_global_mut<Events>(@coin_example).mint_event,
            MintEvent {
                amount,
                coin_type: type_info::type_name<T>(),
            });
        mint_internal<T>(amount)
    }

    public fun burn<T>(coin: Coin<T>, _: &BurnCapability<T>) acquires CoinInfo, Events {
        let amount = burn_internal(coin);
        let coin_info = borrow_global_mut<CoinInfo<T>>(@coin_example);
        event::emit_event(&mut borrow_global_mut<Events>(@coin_example).burn_event,
            BurnEvent {
                amount,
                coin_type: type_info::type_name<T>(),
            });
        coin_info.supply = coin_info.supply - amount;
    }

    public fun freeze_address<T>(addr: address, _: &FreezeCapability<T>) acquires CoinStore {
        let coin_data = get_coin_data<T>(addr);
        coin_data.frozen = true;
    }

    public fun revoke_from<T>(addr: address, _: &FreezeCapability<T>): Coin<T> acquires CoinStore {
        let coin_data = get_coin_data<T>(addr);
        assert!(coin_data.frozen, EREVOKE_UNFROZEN_ACCOUNT);
        let amount = coin_data.balance;
        coin_data.balance = 0;
        Coin { value: amount}
    }

    public fun unfreeze_address<T>(addr: address, _: &FreezeCapability<T>) acquires CoinStore {
        let coin_data = get_coin_data<T>(addr);
        coin_data.frozen = false;
    }

    /// Withdraw coin from a signer's account balance
    public fun withdraw<T>(from: &signer, amount: u128): Coin<T> acquires CoinStore {
        withdraw_from(approve<T>(from, amount))
    }

    /// Give an approval to withdraw at most amount from signers address
    public fun approve<T>(from: &signer, amount: u128): Approval<T> {
        Approval { acct: std::signer::address_of(from), amount}
    }

    /// Withdraw with approval from an account
    public fun withdraw_from<T>(approval: Approval<T>): Coin<T> acquires CoinStore {
        let Approval { acct: addr, amount } = approval;
        let coin_data = get_coin_data<T>(addr);
        assert!(!coin_data.frozen, EFROZEN_ACCOUNT);
        coin_data.balance = coin_data.balance - amount;
        event::emit_event(&mut borrow_global_mut<CoinStore>(addr).withdraw_event,
        WithdrawEvent { amount, coin_type: std::type_info::type_name<T>() });
        mint_internal<T>(amount)
    }

    /// Deposit coin to an account's balance
    public fun deposit<T>(to: address, coin: Coin<T>): u128 acquires CoinStore {
        let coin_data = get_coin_data<T>(to);
        assert!(!coin_data.frozen, EFROZEN_ACCOUNT);
        let amount = burn_internal(coin);
        coin_data.balance = coin_data.balance + amount;
        event::emit_event(&mut borrow_global_mut<CoinStore>(to).deposit_event,
            DepositEvent { amount, coin_type: type_info::type_name<T>() });
        amount
    }

    // Pure functions

    /// Merge two coins together, preserving total value
    public fun merge<T>(a: Coin<T>, b: Coin<T>): Coin<T> {
        join(&mut a, b);
        a
    }

    /// Split one coin into two coins, preserving total value
    public fun split<T>(a: Coin<T>, amount: u128): (Coin<T>, Coin<T>) {
        let b = extract(&mut a, amount);
        (a, b)
    }

    /// Same as merge but using mutable
    public fun join<T>(a: &mut Coin<T>, b: Coin<T>) {
        a.value = a.value + burn_internal(b);
    }

    /// Same as split
    public fun extract<T>(a: &mut Coin<T>, amount: u128): Coin<T> {
        a.value = a.value - amount;
        mint_internal(amount)
    }

    /// Returns the `value` passed in `coin`.
    public fun value<CoinType>(coin: &Coin<CoinType>): u128 {
        coin.value
    }

    public fun split_approval<CoinType>(approval: &mut Approval<CoinType>, amount: u128): Approval<CoinType> {
        approval.amount = approval.amount - amount;
        Approval { acct: approval.acct, amount }
    }

    #[view]
    /// Returns the balance of `owner` for provided `CoinType`.
    public fun balance<T>(owner: address): u128 acquires CoinStore {
        let coin_data = get_coin_data<T>(owner);
        coin_data.balance
    }

    #[view]
    /// Returns the name of the coin.
    public fun name<CoinType>(): std::string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).name
    }

    #[view]
    /// Returns the symbol of the coin, usually a shorter version of the name.
    public fun symbol<CoinType>(): std::string::String acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).symbol
    }

    #[view]
    /// Returns the number of decimals used to get its user representation.
    /// For example, if `decimals` equals `2`, a balance of `505` coins should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    public fun decimals<CoinType>(): u8 acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).decimals
    }

    #[view]
    /// Returns the amount of coin in existence.
    public fun supply<CoinType>(): u128 acquires CoinInfo {
        borrow_global<CoinInfo<CoinType>>(coin_address<CoinType>()).supply
    }

    #[test]
    fun coin_test() {}
}
