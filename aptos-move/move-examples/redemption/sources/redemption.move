/// This module implements a simple redemption pool that allows users to redeem wrapped coins for native fungible assets.
///
/// A new pool can only be created by the deployer of the contract.
/// Any operator can deposit native fungible assets into the pool.
/// Operators can withdraw wrapped coins from the pool, up to the amount they have deposited.
/// Users can redeem wrapped coins for native fungible assets at 1:1.
module redemption::redemption {
    use aptos_framework::aptos_account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::object::{Self, Object, ExtendRef};
    use aptos_framework::primary_fungible_store;
    use aptos_std::table::{Self, Table};
    use std::signer;
    use std::string::String;
    use aptos_std::type_info;
    use aptos_framework::event;

    /// Caller is not authorized to perform the operation.
    const EUNAUTHORIZED: u64 = 1;

    /// Operator cannot withdraw more than they have deposited.
    const EBALANCE_EXCEEDED: u64 = 2;

    struct RedemptionPool<phantom WrappedCoin> has key {
        wrapped_coins: Coin<WrappedCoin>,
        to: Object<Metadata>,
        native_store: ExtendRef,
        operator_balances: Table<address, u64>,
    }

    #[event]
    struct CreatePool has drop, store {
        coin: String,
        to: Object<Metadata>,
    }

    #[event]
    struct DepositNative has drop, store {
        coin: String,
        to: Object<Metadata>,
        operator: address,
        amount: u64,
    }

    #[event]
    struct WithdrawWrapped has drop, store {
        coin: String,
        to: Object<Metadata>,
        operator: address,
        amount: u64,
    }

    #[event]
    struct Redeem has drop, store {
        coin: String,
        to: Object<Metadata>,
        user: address,
        amount: u64,
    }

    public entry fun create_pool<WrappedCoin>(redemption_signer: &signer, to: Object<Metadata>) {
        assert!(signer::address_of(redemption_signer) == @redemption, EUNAUTHORIZED);

        // Set owner to 0x0 so no one can withdraw from this store.
        let native_store = &object::create_object(@0x0);
        fungible_asset::create_store(native_store, to);
        move_to(redemption_signer, RedemptionPool<WrappedCoin> {
            wrapped_coins: coin::zero(),
            to,
            native_store: object::generate_extend_ref(native_store),
            operator_balances: table::new(),
        });

        event::emit(CreatePool {
            coin: type_info::type_name<WrappedCoin>(),
            to,
        });
    }

    public entry fun redeem<WrappedCoin>(user: &signer, amount: u64) acquires RedemptionPool {
        let pool = borrow_global_mut<RedemptionPool<WrappedCoin>>(@redemption);
        coin::merge(&mut pool.wrapped_coins, coin::withdraw<WrappedCoin>(user, amount));
        let native_store_signer = &object::generate_signer_for_extending(&pool.native_store);
        let user_addr = signer::address_of(user);
        primary_fungible_store::ensure_primary_store_exists(user_addr, pool.to);
        dispatchable_fungible_asset::transfer(
            native_store_signer,
            object::address_to_object(object::address_from_extend_ref(&pool.native_store)),
            primary_fungible_store::primary_store_inlined(user_addr, pool.to),
            amount
        );

        event::emit(Redeem {
            coin: type_info::type_name<WrappedCoin>(),
            to: pool.to,
            user: signer::address_of(user),
            amount,
        });
    }

    public entry fun deposit_native<WrappedCoin>(operator: &signer, amount: u64) acquires RedemptionPool {
        let operator_addr = signer::address_of(operator);
        let pool = borrow_global_mut<RedemptionPool<WrappedCoin>>(@redemption);
        dispatchable_fungible_asset::transfer(
            operator,
            primary_fungible_store::primary_store_inlined(operator_addr, pool.to),
            object::address_to_object(object::address_from_extend_ref(&pool.native_store)),
            amount);

        let operator_balance = table::borrow_mut_with_default(&mut pool.operator_balances, operator_addr, 0);
        *operator_balance = *operator_balance + amount;

        event::emit(DepositNative {
            coin: type_info::type_name<WrappedCoin>(),
            to: pool.to,
            operator: operator_addr,
            amount,
        });
    }

    public entry fun withdraw_wrapped<WrappedCoin>(operator: &signer, amount: u64) acquires RedemptionPool {
        let operator_addr = signer::address_of(operator);
        let pool = borrow_global_mut<RedemptionPool<WrappedCoin>>(@redemption);
        let operator_balance = table::borrow_mut(&mut pool.operator_balances, operator_addr);
        assert!(*operator_balance >= amount, EBALANCE_EXCEEDED);
        *operator_balance = *operator_balance - amount;

        aptos_account::deposit_coins(operator_addr, coin::extract(&mut pool.wrapped_coins, amount));

        event::emit(WithdrawWrapped {
            coin: type_info::type_name<WrappedCoin>(),
            to: pool.to,
            operator: operator_addr,
            amount,
        });
    }
}
