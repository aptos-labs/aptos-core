module token_v2::coin {
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::object::{Self, Object, object_address, is_owner, generate_transfer_ref, DeleteRef, generate_delete_ref};
    use std::signer;
    use token_v2::common::{increase_supply, FreezeCap, assert_freeze_cap_and_asset_match, MintCap, BurnCap, decrease_supply, get_freeze_from_asset_owner_caps, put_freeze_to_asset_owner_caps, get_burn_from_asset_owner_caps, put_burn_to_asset_owner_caps, get_mint_from_asset_owner_caps, put_mint_to_asset_owner_caps};
    use std::vector;
    use std::error;
    use std::signer::address_of;
    #[test_only]
    use token_v2::common::{create_and_initialize_test_token, get_current_supply, remove_fungible_asset_metadata, asset_owner_caps_contain_mint, asset_owner_caps_contain_freeze, asset_owner_caps_contain_burn};
    use std::option::Option;
    use std::option;

    /// The coin resource existence error.
    const ECOIN: u64 = 1;
    /// Amount cannot be zero.
    const EAMOUNT_CANNOT_BE_ZERO: u64 = 2;
    /// Not the owner.
    const ENOT_OWNER: u64 = 3;
    /// CoinStore errors.
    const ECOIN_STORE: u64 = 4;
    /// The token account has asset mismatch.
    const EASSET_ADDRESS_MISMATCH: u64 = 5;
    /// The token account has positive balance so cannot be deleted.
    const EBALANCE_NOT_ZERO: u64 = 6;
    /// The token account is still frozen so cannot be deleted.
    const ESTILL_FROZEN: u64 = 7;
    /// The coin object existence error.
    const ECOIN_OBJECT: u64 = 8;
    /// Insufficient balance.
    const EINSUFFICIENT_BALANCE: u64 = 9;
    /// Coin type mismatch.
    const ECOIN_TYPE_MISMATCH: u64 = 10;
    /// The vector of `CashedCoin` for merging is empty.
    const EEMPTY_CASHED_COIN_VECTOR: u64 = 11;

    struct CoinStore has key {
        index: SmartTable<address, Object<Coin>>
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Coin has key {
        asset_addr: address,
        balance: u64,
        frozen: bool,
        delete_ref: DeleteRef,
    }

    struct CashedCoin {
        asset_addr: address,
        amount: u64,
    }

    /// ================================================================================================================
    /// Public functions
    /// ================================================================================================================

    /// Ensure the coin store exists. If not, create it.
    public fun ensure_coin_store(account: &signer) {
        if (!exists<CoinStore>(signer::address_of(account))) {
            move_to(account, CoinStore {
                index: smart_table::new()
            })
        }
    }

    /// Merge a vector of CashedCoins into one.
    public fun merge_cash(cash: vector<CashedCoin>): CashedCoin {
        assert!(!vector::is_empty(&cash), error::invalid_argument(EEMPTY_CASHED_COIN_VECTOR));
        let asset_addr = vector::borrow(&cash, 0).asset_addr;
        vector::fold(cash, CashedCoin {
            asset_addr,
            amount: 0
        }, | c1, c2 | { merge_cash_internal(c1, c2) })
    }

    /// Check the balance of an account.
    public fun balance_of<T: key>(coin_owner: address, asset: &Object<T>): u64 acquires CoinStore, Coin {
        let coin_obj_opt = get_coin_object(coin_owner, object_address(asset), false);
        if (option::is_none(&coin_obj_opt)) {
            return 0
        };
        let coin_obj = option::extract(&mut coin_obj_opt);
        borrow_global<Coin>(verify(&coin_obj)).balance
    }

    /// Mint the `amount` of coin with MintCap.
    public fun mint_with_cap<T: key>(cap: &MintCap, asset: &Object<T>, amount: u64): CashedCoin {
        // This ensures amount > 0;
        increase_supply(cap, asset, amount);
        mint_cash(object_address(asset), amount)
    }

    /// Mint fungible tokens as the owner of the base asset.
    public fun mint_by_asset_owner<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) acquires CoinStore, Coin {
        let mint_cap = get_mint_from_asset_owner_caps(asset_owner, asset);
        let cash = mint_with_cap(&mint_cap, asset, amount);
        put_mint_to_asset_owner_caps(asset_owner, asset, mint_cap);
        deposit(cash, to);
    }

    /// Burn the `amount` of coin with MintCap.
    public fun burn_with_cap<T: key>(
        cap: &BurnCap,
        asset: &Object<T>,
        amount: u64,
        from_account: address
    ) acquires CoinStore, Coin {
        // This ensures amount > 0;
        decrease_supply(cap, asset, amount);
        let cash_to_burn = withdraw_internal(from_account, asset, amount);
        let CashedCoin {
            asset_addr: _,
            amount: _,
        } = cash_to_burn;
    }

    /// Burn fungible tokens as the owner of the base asset.
    public fun burn_by_asset_owner<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        from_account: address
    ) acquires CoinStore, Coin {
        let burn_cap = get_burn_from_asset_owner_caps(asset_owner, asset);
        burn_with_cap(&burn_cap, asset, amount, from_account);
        put_burn_to_asset_owner_caps(asset_owner, asset, burn_cap);
    }

    /// Check the coin account of `coin_owner` is frozen or not.
    public fun is_frozen<T: key>(coin_owner: address, asset: &Object<T>): bool acquires CoinStore, Coin {
        let coin_obj_opt = get_coin_object(coin_owner, object_address(asset), false);
        if (option::is_none(&coin_obj_opt)) {
            return false
        };
        let coin_obj = option::extract(&mut coin_obj_opt);
        borrow_global<Coin>(verify(&coin_obj)).frozen
    }

    /// Freeze the coin account of `coin_owner` with FreezeCap.
    public fun freeze_with_cap<T: key>(
        cap: &FreezeCap,
        coin_owner: address,
        asset: &Object<T>
    ) acquires CoinStore, Coin {
        set_frozen_with_cap(cap, coin_owner, asset, true);
    }

    /// Unfreeze the coin account of `coin_owner` with FreezeCap.
    public fun unfreeze_with_cap<T: key>(
        cap: &FreezeCap,
        coin_owner: address,
        asset: &Object<T>
    ) acquires CoinStore, Coin {
        set_frozen_with_cap(cap, coin_owner, asset, false);
    }

    public fun freeze_by_asset_owner<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        coin_owner: address,
    ) acquires Coin, CoinStore {
        let freeze_cap = get_freeze_from_asset_owner_caps(asset_owner, asset);
        freeze_with_cap(&freeze_cap, coin_owner, asset);
        put_freeze_to_asset_owner_caps(asset_owner, asset, freeze_cap);
    }

    public fun unfreeze_by_asset_owner<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        coin_owner: address
    ) acquires Coin, CoinStore {
        let freeze_cap = get_freeze_from_asset_owner_caps(asset_owner, asset);
        unfreeze_with_cap(&freeze_cap, coin_owner, asset);
        put_freeze_to_asset_owner_caps(asset_owner, asset, freeze_cap);
    }

    public fun withdraw<T: key>(
        account: &signer,
        asset: &Object<T>,
        amount: u64
    ): CashedCoin acquires CoinStore, Coin {
        let account_address = signer::address_of(account);
        withdraw_internal(account_address, asset, amount)
    }

    public fun deposit(cash: CashedCoin, to: address) acquires CoinStore, Coin {
        let coin = borrow_coin_mut(to, cash.asset_addr, true /* create token object if not exist */);
        deposit_cash(coin, cash);
    }

    public fun transfer<T: key>(
        account: &signer,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) acquires CoinStore, Coin {
        // This ensures amount > 0;
        let cash = withdraw(account, asset, amount);
        deposit(cash, to);
    }

    /// ================================================================================================================
    /// Private functions
    /// ================================================================================================================
    fun mint_cash(asset_addr: address, amount: u64): CashedCoin {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        CashedCoin {
            asset_addr,
            amount
        }
    }

    fun withdraw_cash(coin: &mut Coin, amount: u64): CashedCoin {
        assert!(amount != 0, error::invalid_argument(EAMOUNT_CANNOT_BE_ZERO));
        assert!(coin.balance >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));
        assert!(!coin.frozen, error::invalid_argument(ESTILL_FROZEN));
        coin.balance = coin.balance - amount;
        CashedCoin {
            asset_addr: coin.asset_addr,
            amount
        }
    }

    fun merge_cash_internal(cash1: CashedCoin, cash2: CashedCoin): CashedCoin {
        let CashedCoin {
            asset_addr,
            amount: amount1
        } = cash1;
        let CashedCoin {
            asset_addr: asset_addr2,
            amount: amount2,
        } = cash2;
        // Make sure the cash is for the same asset.
        assert!(asset_addr == asset_addr2, error::invalid_argument(ECOIN_TYPE_MISMATCH));
        CashedCoin {
            asset_addr,
            amount: amount1 + amount2
        }
    }

    fun deposit_cash(coin: &mut Coin, cash: CashedCoin) {
        assert!(!coin.frozen, error::invalid_argument(ESTILL_FROZEN));
        let CashedCoin { asset_addr, amount } = cash;
        // ensure merging the same coin
        assert!(coin.asset_addr == asset_addr, error::invalid_argument(ECOIN_TYPE_MISMATCH));
        coin.balance = coin.balance + amount;
    }

    fun get_coin_object(
        coin_owner: address,
        asset_address: address,
        create_on_demand: bool
    ): Option<Object<Coin>> acquires CoinStore {
        assert!(exists<CoinStore>(coin_owner), error::not_found(ECOIN_STORE));
        let index_table = &mut borrow_global_mut<CoinStore>(coin_owner).index;
        if (!smart_table::contains(index_table, asset_address)) {
            if (create_on_demand) {
                create_coin_object_from_asset(coin_owner, asset_address);
            } else {
                return option::none()
            }
        };
        let coin_obj = *smart_table::borrow(index_table, asset_address);
        assert!(is_owner(coin_obj, coin_owner), error::internal(ENOT_OWNER));
        option::some(coin_obj)
    }

    /// Create a zero-balance coin object of the passed-in asset.
    fun create_coin_object_from_asset(account: address, asset_address: address): Object<Coin> {
        // Must review carefully here.
        let asset_signer = aptos_framework::create_signer::create_signer(asset_address);
        let creator_ref = object::create_object_from_object(&asset_signer);
        let coin_signer = object::generate_signer(&creator_ref);
        // Transfer the owner to `account`.
        object::transfer_call(&asset_signer, address_of(&coin_signer), account);

        // Disable transfer of coin object so the
        let transfer_ref = generate_transfer_ref(&creator_ref);
        object::disable_ungated_transfer(&transfer_ref);

        move_to(&coin_signer, Coin {
            asset_addr: asset_address,
            balance: 0,
            frozen: false,
            delete_ref: generate_delete_ref(&creator_ref)
        });
        object::object_from_constructor_ref<Coin>(&creator_ref)
    }

    fun remove_coin_object(coin_owner: address, asset_address: address): Object<Coin> acquires CoinStore {
        assert!(exists<CoinStore>(coin_owner), error::not_found(ECOIN_STORE));
        let index_table = &mut borrow_global_mut<CoinStore>(coin_owner).index;
        assert!(smart_table::contains(index_table, asset_address), error::not_found(ECOIN_OBJECT));
        let coin_obj = smart_table::remove(index_table, asset_address);
        assert!(is_owner(coin_obj, coin_owner), error::internal(ENOT_OWNER));
        coin_obj
    }

    fun withdraw_internal<T: key>(
        account: address,
        asset: &Object<T>,
        amount: u64
    ): CashedCoin acquires CoinStore, Coin {
        let asset_address = object_address(asset);
        let coin = borrow_coin_mut(account, asset_address, false /* create token object if not exist */);
        let cash = withdraw_cash(coin, amount);
        // Clean up token obj if balance drops to 0 and not frozen.
        if (coin.balance == 0 && !coin.frozen) {
            let coin_obj = remove_coin_object(account, asset_address);
            let coin_obj_addr = verify(&coin_obj);
            let Coin {
                asset_addr,
                balance,
                frozen,
                delete_ref
            } = move_from<Coin>(coin_obj_addr);
            assert!(asset_addr == asset_address, error::internal(EASSET_ADDRESS_MISMATCH));
            assert!(balance == 0, error::internal(EBALANCE_NOT_ZERO));
            assert!(!frozen, error::internal(ESTILL_FROZEN));
            object::delete(delete_ref);
        };
        cash
    }

    inline fun set_frozen_with_cap<T: key>(
        cap: &FreezeCap,
        coin_owner: address,
        asset: &Object<T>,
        frozen: bool
    ) acquires CoinStore, Coin {
        assert_freeze_cap_and_asset_match(cap, asset);
        let coin = borrow_coin_mut(coin_owner, object_address(asset), true);
        coin.frozen = frozen;
    }

    inline fun borrow_coin(coin_owner: address, asset_address: address): &Coin acquires CoinStore, Coin {
        let coin_obj_opt = get_coin_object(coin_owner, asset_address, false);
        assert!(option::is_some(&coin_obj_opt), error::not_found(ECOIN_OBJECT));
        let coin_obj = option::extract(&mut coin_obj_opt);
        borrow_global<Coin>(verify(&coin_obj))
    }

    inline fun borrow_coin_mut(
        coin_owner: address,
        asset_address: address,
        create_on_demand: bool
    ): &mut Coin acquires CoinStore, Coin {
        let coin_obj_opt = get_coin_object(coin_owner, asset_address, create_on_demand);
        assert!(option::is_some(&coin_obj_opt), error::not_found(ECOIN_OBJECT));
        let coin_obj = option::extract(&mut coin_obj_opt);
        borrow_global_mut<Coin>(verify(&coin_obj))
    }

    inline fun verify(coin_obj: &Object<Coin>): address {
        let coin_address = object::object_address(coin_obj);
        assert!(
            exists<Coin>(coin_address),
            error::not_found(ECOIN),
        );
        coin_address
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    fun test_basic_flow(creator: &signer, aaron: &signer) acquires CoinStore, Coin {
        let asset = create_and_initialize_test_token(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        asset_owner_caps_contain_mint(&asset);
        asset_owner_caps_contain_freeze(&asset);
        asset_owner_caps_contain_burn(&asset);

        // Mint
        mint_by_asset_owner(creator, &asset, 100, creator_address);
        // Transfer
        transfer(creator, &asset, 90, aaron_address);
        assert!(balance_of(creator_address, &asset) == 10, 0);
        assert!(balance_of(aaron_address, &asset) == 90, 1);

        let cash1 = withdraw(aaron, &asset, 10);
        let cash2 = withdraw(aaron, &asset, 20);
        let cash3 = withdraw(aaron, &asset, 30);
        // Merge cash
        let all_cash = merge_cash(vector[cash1, cash2, cash3]);
        deposit(all_cash, creator_address);
        assert!(balance_of(creator_address, &asset) == 70, 2);

        // Burn
        assert!(get_current_supply(&asset) == 100, 3);
        burn_by_asset_owner(creator, &asset, 70, creator_address);
        assert!(balance_of(creator_address, &asset) == 0, 4);
        assert!(get_current_supply(&asset) == 30, 5);

        // Freeze
        freeze_by_asset_owner(creator, &asset, aaron_address);
        assert!(is_frozen(aaron_address, &asset), 6);
        unfreeze_by_asset_owner(creator, &asset, aaron_address);
        assert!(!is_frozen(aaron_address, &asset), 7);

        // Remove fungibility
        burn_by_asset_owner(creator, &asset, 30, aaron_address);
        remove_fungible_asset_metadata(&asset);
    }

    #[test(creator = @0xcafe)]
    fun test_non_existent_token(creator: &signer) acquires CoinStore, Coin {
        let asset = create_and_initialize_test_token(creator);
        let creator_address = signer::address_of(creator);
        assert!(balance_of(creator_address, &asset) == 0, 0);
        assert!(!is_frozen(creator_address, &asset), 1);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x10007, location = Self)]
    fun test_freeze(creator: &signer, aaron: &signer) acquires CoinStore, Coin {
        let asset = create_and_initialize_test_token(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        mint_by_asset_owner(creator, &asset, 100, aaron_address);
        freeze_by_asset_owner(creator, &asset, aaron_address);
        transfer(aaron, &asset, 90, creator_address);
    }
}
