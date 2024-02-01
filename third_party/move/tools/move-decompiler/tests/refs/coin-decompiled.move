module 0x1::coin {
    struct AggregatableCoin<phantom T0> has store {
        value: 0x1::aggregator::Aggregator,
    }
    
    struct BurnCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct Coin<phantom T0> has store {
        value: u64,
    }
    
    struct CoinInfo<phantom T0> has key {
        name: 0x1::string::String,
        symbol: 0x1::string::String,
        decimals: u8,
        supply: 0x1::option::Option<0x1::optional_aggregator::OptionalAggregator>,
    }
    
    struct CoinStore<phantom T0> has key {
        coin: Coin<T0>,
        frozen: bool,
        deposit_events: 0x1::event::EventHandle<DepositEvent>,
        withdraw_events: 0x1::event::EventHandle<WithdrawEvent>,
    }
    
    struct Deposit<phantom T0> has drop, store {
        account: address,
        amount: u64,
    }
    
    struct DepositEvent has drop, store {
        amount: u64,
    }
    
    struct FreezeCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct MintCapability<phantom T0> has copy, store {
        dummy_field: bool,
    }
    
    struct SupplyConfig has key {
        allow_upgrades: bool,
    }
    
    struct Withdraw<phantom T0> has drop, store {
        account: address,
        amount: u64,
    }
    
    struct WithdrawEvent has drop, store {
        amount: u64,
    }
    
    public fun allow_supply_upgrades(arg0: &signer, arg1: bool) acquires SupplyConfig {
        0x1::system_addresses::assert_aptos_framework(arg0);
        borrow_global_mut<SupplyConfig>(@0x1).allow_upgrades = arg1;
    }
    
    public fun balance<T0>(arg0: address) : u64 acquires CoinStore {
        assert!(is_account_registered<T0>(arg0), 0x1::error::not_found(5));
        borrow_global<CoinStore<T0>>(arg0).coin.value
    }
    
    public fun burn<T0>(arg0: Coin<T0>, arg1: &BurnCapability<T0>) acquires CoinInfo {
        let Coin { value: v0 } = arg0;
        assert!(v0 > 0, 0x1::error::invalid_argument(9));
        let v1 = &mut borrow_global_mut<CoinInfo<T0>>(coin_address<T0>()).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v1)) {
            let v2 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v1);
            0x1::optional_aggregator::sub(v2, v0 as u128);
        };
    }
    
    public fun burn_from<T0>(arg0: address, arg1: u64, arg2: &BurnCapability<T0>) acquires CoinInfo, CoinStore {
        if (arg1 == 0) {
            return
        };
        burn<T0>(extract<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, arg1), arg2);
    }
    
    fun coin_address<T0>() : address {
        let v0 = 0x1::type_info::type_of<T0>();
        0x1::type_info::account_address(&v0)
    }
    
    public(friend) fun collect_into_aggregatable_coin<T0>(arg0: address, arg1: u64, arg2: &mut AggregatableCoin<T0>) acquires CoinStore {
        if (arg1 == 0) {
            return
        };
        let v0 = extract<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, arg1);
        merge_aggregatable_coin<T0>(arg2, v0);
    }
    
    public fun decimals<T0>() : u8 acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).decimals
    }
    
    public fun deposit<T0>(arg0: address, arg1: Coin<T0>) acquires CoinStore {
        assert!(is_account_registered<T0>(arg0), 0x1::error::not_found(5));
        let v0 = borrow_global_mut<CoinStore<T0>>(arg0);
        assert!(!v0.frozen, 0x1::error::permission_denied(10));
        let v1 = DepositEvent{amount: arg1.value};
        0x1::event::emit_event<DepositEvent>(&mut v0.deposit_events, v1);
        let v2 = Deposit<T0>{
            account : arg0, 
            amount  : arg1.value,
        };
        0x1::event::emit<Deposit<T0>>(v2);
        merge<T0>(&mut v0.coin, arg1);
    }
    
    public fun destroy_burn_cap<T0>(arg0: BurnCapability<T0>) {
        let BurnCapability {  } = arg0;
    }
    
    public fun destroy_freeze_cap<T0>(arg0: FreezeCapability<T0>) {
        let FreezeCapability {  } = arg0;
    }
    
    public fun destroy_mint_cap<T0>(arg0: MintCapability<T0>) {
        let MintCapability {  } = arg0;
    }
    
    public fun destroy_zero<T0>(arg0: Coin<T0>) {
        let Coin { value: v0 } = arg0;
        assert!(v0 == 0, 0x1::error::invalid_argument(7));
    }
    
    public(friend) fun drain_aggregatable_coin<T0>(arg0: &mut AggregatableCoin<T0>) : Coin<T0> {
        let v0 = 0x1::aggregator::read(&arg0.value);
        assert!(v0 <= 18446744073709551615, 0x1::error::out_of_range(14));
        0x1::aggregator::sub(&mut arg0.value, v0);
        Coin<T0>{value: v0 as u64}
    }
    
    public fun extract<T0>(arg0: &mut Coin<T0>, arg1: u64) : Coin<T0> {
        assert!(arg0.value >= arg1, 0x1::error::invalid_argument(6));
        arg0.value = arg0.value - arg1;
        Coin<T0>{value: arg1}
    }
    
    public fun extract_all<T0>(arg0: &mut Coin<T0>) : Coin<T0> {
        arg0.value = 0;
        Coin<T0>{value: arg0.value}
    }
    
    public(friend) fun force_deposit<T0>(arg0: address, arg1: Coin<T0>) acquires CoinStore {
        assert!(is_account_registered<T0>(arg0), 0x1::error::not_found(5));
        merge<T0>(&mut borrow_global_mut<CoinStore<T0>>(arg0).coin, arg1);
    }
    
    public entry fun freeze_coin_store<T0>(arg0: address, arg1: &FreezeCapability<T0>) acquires CoinStore {
        borrow_global_mut<CoinStore<T0>>(arg0).frozen = true;
    }
    
    public fun initialize<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        initialize_internal<T0>(arg0, arg1, arg2, arg3, arg4, false)
    }
    
    public(friend) fun initialize_aggregatable_coin<T0>(arg0: &signer) : AggregatableCoin<T0> {
        AggregatableCoin<T0>{value: 0x1::aggregator_factory::create_aggregator(arg0, 18446744073709551615)}
    }
    
    fun initialize_internal<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool, arg5: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(coin_address<T0>() == v0, 0x1::error::invalid_argument(1));
        assert!(!exists<CoinInfo<T0>>(v0), 0x1::error::already_exists(2));
        assert!(0x1::string::length(&arg1) <= 32, 0x1::error::invalid_argument(12));
        assert!(0x1::string::length(&arg2) <= 10, 0x1::error::invalid_argument(13));
        let v1 = if (arg4) {
            0x1::option::some<0x1::optional_aggregator::OptionalAggregator>(0x1::optional_aggregator::new(340282366920938463463374607431768211455, arg5))
        } else {
            0x1::option::none<0x1::optional_aggregator::OptionalAggregator>()
        };
        let v2 = CoinInfo<T0>{
            name     : arg1, 
            symbol   : arg2, 
            decimals : arg3, 
            supply   : v1,
        };
        move_to<CoinInfo<T0>>(arg0, v2);
        let v3 = BurnCapability<T0>{dummy_field: false};
        let v4 = FreezeCapability<T0>{dummy_field: false};
        let v5 = MintCapability<T0>{dummy_field: false};
        (v3, v4, v5)
    }
    
    public(friend) fun initialize_supply_config(arg0: &signer) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        let v0 = SupplyConfig{allow_upgrades: false};
        move_to<SupplyConfig>(arg0, v0);
    }
    
    public(friend) fun initialize_with_parallelizable_supply<T0>(arg0: &signer, arg1: 0x1::string::String, arg2: 0x1::string::String, arg3: u8, arg4: bool) : (BurnCapability<T0>, FreezeCapability<T0>, MintCapability<T0>) {
        0x1::system_addresses::assert_aptos_framework(arg0);
        initialize_internal<T0>(arg0, arg1, arg2, arg3, arg4, true)
    }
    
    public fun is_account_registered<T0>(arg0: address) : bool {
        exists<CoinStore<T0>>(arg0)
    }
    
    public(friend) fun is_aggregatable_coin_zero<T0>(arg0: &AggregatableCoin<T0>) : bool {
        0x1::aggregator::read(&arg0.value) == 0
    }
    
    public fun is_coin_initialized<T0>() : bool {
        exists<CoinInfo<T0>>(coin_address<T0>())
    }
    
    public fun is_coin_store_frozen<T0>(arg0: address) : bool acquires CoinStore {
        if (!is_account_registered<T0>(arg0)) {
            return true
        };
        borrow_global<CoinStore<T0>>(arg0).frozen
    }
    
    public fun merge<T0>(arg0: &mut Coin<T0>, arg1: Coin<T0>) {
        let Coin { value: v0 } = arg1;
        arg0.value = arg0.value + v0;
    }
    
    public(friend) fun merge_aggregatable_coin<T0>(arg0: &mut AggregatableCoin<T0>, arg1: Coin<T0>) {
        let Coin { value: v0 } = arg1;
        0x1::aggregator::add(&mut arg0.value, v0 as u128);
    }
    
    public fun mint<T0>(arg0: u64, arg1: &MintCapability<T0>) : Coin<T0> acquires CoinInfo {
        if (arg0 == 0) {
            return Coin<T0>{value: 0}
        };
        let v0 = &mut borrow_global_mut<CoinInfo<T0>>(coin_address<T0>()).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v0)) {
            let v1 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v0);
            0x1::optional_aggregator::add(v1, arg0 as u128);
        };
        Coin<T0>{value: arg0}
    }
    
    public fun name<T0>() : 0x1::string::String acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).name
    }
    
    public fun register<T0>(arg0: &signer) {
        let v0 = 0x1::signer::address_of(arg0);
        if (is_account_registered<T0>(v0)) {
            return
        };
        0x1::account::register_coin<T0>(v0);
        let v1 = Coin<T0>{value: 0};
        let v2 = 0x1::account::new_event_handle<DepositEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<WithdrawEvent>(arg0);
        let v4 = CoinStore<T0>{
            coin            : v1, 
            frozen          : false, 
            deposit_events  : v2, 
            withdraw_events : v3,
        };
        move_to<CoinStore<T0>>(arg0, v4);
    }
    
    public fun supply<T0>() : 0x1::option::Option<u128> acquires CoinInfo {
        let v0 = &borrow_global<CoinInfo<T0>>(coin_address<T0>()).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v0)) {
            0x1::option::some<u128>(0x1::optional_aggregator::read(0x1::option::borrow<0x1::optional_aggregator::OptionalAggregator>(v0)))
        } else {
            0x1::option::none<u128>()
        }
    }
    
    public fun symbol<T0>() : 0x1::string::String acquires CoinInfo {
        borrow_global<CoinInfo<T0>>(coin_address<T0>()).symbol
    }
    
    public entry fun transfer<T0>(arg0: &signer, arg1: address, arg2: u64) acquires CoinStore {
        let v0 = withdraw<T0>(arg0, arg2);
        deposit<T0>(arg1, v0);
    }
    
    public entry fun unfreeze_coin_store<T0>(arg0: address, arg1: &FreezeCapability<T0>) acquires CoinStore {
        borrow_global_mut<CoinStore<T0>>(arg0).frozen = false;
    }
    
    public entry fun upgrade_supply<T0>(arg0: &signer) acquires CoinInfo, SupplyConfig {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(coin_address<T0>() == v0, 0x1::error::invalid_argument(1));
        assert!(borrow_global_mut<SupplyConfig>(@0x1).allow_upgrades, 0x1::error::permission_denied(11));
        let v1 = &mut borrow_global_mut<CoinInfo<T0>>(v0).supply;
        if (0x1::option::is_some<0x1::optional_aggregator::OptionalAggregator>(v1)) {
            let v2 = 0x1::option::borrow_mut<0x1::optional_aggregator::OptionalAggregator>(v1);
            if (!0x1::optional_aggregator::is_parallelizable(v2)) {
                0x1::optional_aggregator::switch(v2);
            };
        };
    }
    
    public fun value<T0>(arg0: &Coin<T0>) : u64 {
        arg0.value
    }
    
    public fun withdraw<T0>(arg0: &signer, arg1: u64) : Coin<T0> acquires CoinStore {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(is_account_registered<T0>(v0), 0x1::error::not_found(5));
        let v1 = borrow_global_mut<CoinStore<T0>>(v0);
        assert!(!v1.frozen, 0x1::error::permission_denied(10));
        let v2 = WithdrawEvent{amount: arg1};
        0x1::event::emit_event<WithdrawEvent>(&mut v1.withdraw_events, v2);
        let v3 = Withdraw<T0>{
            account : v0, 
            amount  : arg1,
        };
        0x1::event::emit<Withdraw<T0>>(v3);
        extract<T0>(&mut v1.coin, arg1)
    }
    
    public fun zero<T0>() : Coin<T0> {
        Coin<T0>{value: 0}
    }
    
    // decompiled from Move bytecode v6
}
