// Ported from https://github.com/aave/aptos-aave-v3, modules
// `aave_pool::token_base`, `aave_pool::a_token_factory` and
// `aave_pool::variable_debt_token_factory`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// aTokens and variable debt tokens.
///
/// Both are fungible assets so balances are transferable and readable through
/// the standard interface, but the authoritative number is the scaled balance
/// in the table: a balance divided by the reserve index at the time it was
/// written. Interest then accrues by moving the index alone, with no per-user
/// writes.
module bench::aave_tokens {
    use std::option;
    use std::string;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::{Self, BurnRef, Metadata, MintRef, TransferRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use bench::aave_math;

    const EINVALID_MINT_AMOUNT: u64 = 40;
    const EINVALID_BURN_AMOUNT: u64 = 41;
    const EINSUFFICIENT_BALANCE: u64 = 42;

    struct UserState has store, copy, drop {
        balance: u128,
        /// Index at the last balance write, used to split the balance into
        /// principal and accrued interest without a second slot.
        additional_data: u128
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TokenRefs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TokenState has key {
        scaled_total_supply: u256,
        underlying: address,
        user_state: Table<address, UserState>
    }

    public fun create_token(
        owner: &signer,
        seed: vector<u8>,
        name: vector<u8>,
        symbol: vector<u8>,
        decimals: u8,
        underlying: address
    ): address {
        let ctor = object::create_named_object(owner, seed);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &ctor,
            option::none(),
            string::utf8(name),
            string::utf8(symbol),
            decimals,
            string::utf8(b""),
            string::utf8(b"")
        );
        let token_signer = object::generate_signer(&ctor);
        move_to(
            &token_signer,
            TokenRefs {
                mint_ref: fungible_asset::generate_mint_ref(&ctor),
                burn_ref: fungible_asset::generate_burn_ref(&ctor),
                transfer_ref: fungible_asset::generate_transfer_ref(&ctor)
            }
        );
        move_to(
            &token_signer,
            TokenState { scaled_total_supply: 0, underlying, user_state: table::new() }
        );
        object::address_from_constructor_ref(&ctor)
    }

    public fun metadata(token: address): Object<Metadata> {
        object::address_to_object<Metadata>(token)
    }

    public fun underlying(token: address): address acquires TokenState {
        borrow_global<TokenState>(token).underlying
    }

    public fun scaled_balance_of(user: address, token: address): u256 acquires TokenState {
        let state = borrow_global<TokenState>(token);
        if (table::contains(&state.user_state, user)) {
            (table::borrow(&state.user_state, user).balance as u256)
        } else { 0 }
    }

    public fun previous_index(user: address, token: address): u256 acquires TokenState {
        let state = borrow_global<TokenState>(token);
        if (table::contains(&state.user_state, user)) {
            (table::borrow(&state.user_state, user).additional_data as u256)
        } else { 0 }
    }

    public fun scaled_total_supply(token: address): u256 acquires TokenState {
        borrow_global<TokenState>(token).scaled_total_supply
    }

    public fun balance_of(user: address, token: address, index: u256): u256 acquires TokenState {
        aave_math::ray_mul(scaled_balance_of(user, token), index)
    }

    public fun total_supply(token: address, index: u256): u256 acquires TokenState {
        aave_math::ray_mul(scaled_total_supply(token), index)
    }

    /// Returns true when the recipient held nothing before this mint, which is
    /// what the pool uses to decide whether to flip a user-config bit.
    public fun mint_scaled(
        to: address,
        amount: u256,
        index: u256,
        token: address,
        rounding_up: bool
    ): bool acquires TokenState, TokenRefs {
        let amount_scaled =
            if (rounding_up) {
                aave_math::ray_div_up(amount, index)
            } else {
                aave_math::ray_div_down(amount, index)
            };
        assert!(amount_scaled != 0, EINVALID_MINT_AMOUNT);

        let state = borrow_global_mut<TokenState>(token);
        let entry =
            table::borrow_mut_with_default(
                &mut state.user_state,
                to,
                UserState { balance: 0, additional_data: 0 }
            );
        let old_scaled_balance = (entry.balance as u256);
        entry.balance = ((old_scaled_balance + amount_scaled) as u128);
        entry.additional_data = (index as u128);
        state.scaled_total_supply = state.scaled_total_supply + amount_scaled;

        let refs = borrow_global<TokenRefs>(token);
        primary_fungible_store::mint(&refs.mint_ref, to, (amount_scaled as u64));
        old_scaled_balance == 0
    }

    public fun burn_scaled(
        from: address,
        amount: u256,
        index: u256,
        token: address,
        rounding_up: bool
    ) acquires TokenState, TokenRefs {
        let amount_scaled =
            if (rounding_up) {
                aave_math::ray_div_up(amount, index)
            } else {
                aave_math::ray_div_down(amount, index)
            };
        assert!(amount_scaled != 0, EINVALID_BURN_AMOUNT);

        let state = borrow_global_mut<TokenState>(token);
        assert!(table::contains(&state.user_state, from), EINSUFFICIENT_BALANCE);
        let entry = table::borrow_mut(&mut state.user_state, from);
        let old_scaled_balance = (entry.balance as u256);
        assert!(old_scaled_balance >= amount_scaled, EINSUFFICIENT_BALANCE);
        entry.balance = ((old_scaled_balance - amount_scaled) as u128);
        entry.additional_data = (index as u128);
        state.scaled_total_supply = state.scaled_total_supply - amount_scaled;

        let refs = borrow_global<TokenRefs>(token);
        primary_fungible_store::burn(&refs.burn_ref, from, (amount_scaled as u64));
    }

    /// Moves an unscaled `amount` between two holders at the given index.
    public fun transfer_scaled(
        from: address,
        to: address,
        amount: u256,
        index: u256,
        token: address
    ) acquires TokenState, TokenRefs {
        let amount_scaled = aave_math::ray_div(amount, index);
        transfer_scaled_amount(from, to, amount_scaled, index, token);
    }

    /// Moves an already-scaled amount, which liquidation needs so that the
    /// seized collateral matches the scaled balance it burned.
    public fun transfer_scaled_amount(
        from: address,
        to: address,
        amount_scaled: u256,
        index: u256,
        token: address
    ) acquires TokenState, TokenRefs {
        if (amount_scaled == 0) {
            return
        };
        let state = borrow_global_mut<TokenState>(token);
        assert!(table::contains(&state.user_state, from), EINSUFFICIENT_BALANCE);
        let sender = table::borrow_mut(&mut state.user_state, from);
        let sender_balance = (sender.balance as u256);
        assert!(sender_balance >= amount_scaled, EINSUFFICIENT_BALANCE);
        sender.balance = ((sender_balance - amount_scaled) as u128);
        sender.additional_data = (index as u128);

        let recipient =
            table::borrow_mut_with_default(
                &mut state.user_state,
                to,
                UserState { balance: 0, additional_data: 0 }
            );
        let new_balance = (recipient.balance as u256) + amount_scaled;
        recipient.balance = (new_balance as u128);
        recipient.additional_data = (index as u128);

        let refs = borrow_global<TokenRefs>(token);
        primary_fungible_store::transfer_with_ref(
            &refs.transfer_ref, from, to, (amount_scaled as u64)
        );
    }
}
