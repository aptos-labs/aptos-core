// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//
// Small multiplayer poker dapp leveraging AWS Nitro Enclave attestation.
// - Table (game server) runs in TEE: registers on-chain with attestation, runs hands, settles.
// - Players enter with APT locked as chips, request leave after current hand, get 95% back (5% fee to server).

module poker::poker {
    use std::error;
    use std::signer;
    use std::vector;

    use aptos_framework::aws_nitro_utils;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::event;
    use aptos_std::table::{Self, Table};

    // Minimum players to run a hand.
    const MIN_PLAYERS: u64 = 2;
    // Fee in basis points (500 = 5%).
    const FEE_BPS: u64 = 500;
    const BPS_DENOM: u64 = 10000;

    const EINVALID_ATTESTATION: u64 = 1;
    const ETABLE_ALREADY_EXISTS: u64 = 2;
    const ETABLE_NOT_FOUND: u64 = 3;
    const EPLAYER_ALREADY_AT_TABLE: u64 = 4;
    const EPLAYER_NOT_AT_TABLE: u64 = 5;
    const EINSUFFICIENT_BALANCE: u64 = 6;
    const ENOT_TABLE_OWNER: u64 = 7;
    const EINVALID_SETTLE_SUM: u64 = 8;
    const EZERO_AMOUNT: u64 = 9;
    const EMIN_PLAYERS: u64 = 10;

    #[event]
    struct TableRegistered has drop, store {
        table: address,
    }

    #[event]
    struct PlayerEntered has drop, store {
        table: address,
        player: address,
        amount: u64,
    }

    #[event]
    struct LeaveRequested has drop, store {
        table: address,
        player: address,
    }

    #[event]
    struct PlayerLeft has drop, store {
        table: address,
        player: address,
        payout: u64,
        fee: u64,
    }

    #[event]
    struct HandSettled has drop, store {
        table: address,
    }

    struct TableInfo has key {
        escrow: Coin<AptosCoin>,
        /// Chip balance per player (logical chips backed by escrow).
        balances: Table<address, u64>,
        /// Players who requested to leave after current hand.
        pending_leave: Table<address, bool>,
        fee_pool: Coin<AptosCoin>,
        min_players: u64,
    }

    /// Register a new table. Only succeeds if attestation is valid (TEE).
    public entry fun register_table(table: &signer, attestation_doc: vector<u8>) {
        assert!(aws_nitro_utils::verify_attestation(attestation_doc), error::invalid_argument(EINVALID_ATTESTATION));
        register_table_internal(table);
    }

    fun register_table_internal(table: &signer) {
        let table_addr = signer::address_of(table);
        assert!(!exists<TableInfo>(table_addr), error::already_exists(ETABLE_ALREADY_EXISTS));
        let zero = coin::zero<AptosCoin>();
        move_to(table, TableInfo {
            escrow: zero,
            balances: table::new<address, u64>(),
            pending_leave: table::new<address, bool>(),
            fee_pool: coin::zero<AptosCoin>(),
            min_players: MIN_PLAYERS,
        });
        event::emit(TableRegistered { table: table_addr });
    }

    /// Player locks APT as chips and joins the table. They play from the next hand.
    public entry fun enter_table(table_addr: address, player: &signer, amount: u64) acquires TableInfo {
        assert!(amount > 0, error::invalid_argument(EZERO_AMOUNT));
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        let player_addr = signer::address_of(player);
        let table = borrow_global_mut<TableInfo>(table_addr);
        assert!(!table::contains(&table.balances, player_addr), error::invalid_argument(EPLAYER_ALREADY_AT_TABLE));

        let chips = coin::withdraw<AptosCoin>(player, amount);
        coin::merge<AptosCoin>(&mut table.escrow, chips);
        table::add(&mut table.balances, player_addr, amount);
        event::emit(PlayerEntered { table: table_addr, player: player_addr, amount });
    }

    /// Request to leave after the current hand finishes.
    public entry fun request_leave(table_addr: address, player: &signer) acquires TableInfo {
        let player_addr = signer::address_of(player);
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        let table = borrow_global_mut<TableInfo>(table_addr);
        assert!(table::contains(&table.balances, player_addr), error::invalid_argument(EPLAYER_NOT_AT_TABLE));
        table::add(&mut table.pending_leave, player_addr, true);
        event::emit(LeaveRequested { table: table_addr, player: player_addr });
    }

    /// Apply hand result (only table signer). Deduct from losers, add to winners; sum of deduct must equal sum of add.
    public entry fun settle_hand(
        table: &signer,
        table_addr: address,
        deduct_from: vector<address>,
        deduct_amounts: vector<u64>,
        add_to: vector<address>,
        add_amounts: vector<u64>,
    ) acquires TableInfo {
        let table_signer_addr = signer::address_of(table);
        assert!(table_signer_addr == table_addr, error::permission_denied(ENOT_TABLE_OWNER));
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        assert!(vector::length(&deduct_from) == vector::length(&deduct_amounts), error::invalid_argument(EINVALID_SETTLE_SUM));
        assert!(vector::length(&add_to) == vector::length(&add_amounts), error::invalid_argument(EINVALID_SETTLE_SUM));

        let table_info = borrow_global_mut<TableInfo>(table_addr);
        let mut total_deduct = 0u64;
        let mut total_add = 0u64;
        let len_d = vector::length(&deduct_from);
        let len_a = vector::length(&add_to);
        let i = 0u64;
        while (i < len_d) {
            let addr = *vector::borrow(&deduct_from, i);
            let amt = *vector::borrow(&deduct_amounts, i);
            total_deduct = total_deduct + amt;
            assert!(table::contains(&table_info.balances, addr), error::invalid_argument(EPLAYER_NOT_AT_TABLE));
            let bal = table::borrow_mut(&mut table_info.balances, addr);
            assert!(*bal >= amt, error::invalid_argument(EINSUFFICIENT_BALANCE));
            *bal = *bal - amt;
            i = i + 1;
        };
        let j = 0u64;
        while (j < len_a) {
            let addr = *vector::borrow(&add_to, j);
            let amt = *vector::borrow(&add_amounts, j);
            total_add = total_add + amt;
            assert!(table::contains(&table_info.balances, addr), error::invalid_argument(EPLAYER_NOT_AT_TABLE));
            let bal = table::borrow_mut(&mut table_info.balances, addr);
            *bal = *bal + amt;
            j = j + 1;
        };
        assert!(total_deduct == total_add, error::invalid_argument(EINVALID_SETTLE_SUM));
        event::emit(HandSettled { table: table_addr });
    }

    /// Process leaving players: send 95% back, 5% to table fee pool. Only table signer.
    /// Pass the list of player addresses that requested leave (table/TEE tracks this off-chain).
    public entry fun settle_leaving_players(
        table: &signer,
        table_addr: address,
        leaving_players: vector<address>,
    ) acquires TableInfo {
        let table_signer_addr = signer::address_of(table);
        assert!(table_signer_addr == table_addr, error::permission_denied(ENOT_TABLE_OWNER));
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));

        let table_info = borrow_global_mut<TableInfo>(table_addr);
        vector::for_each_ref(&leaving_players, |addr| {
            if (table::contains(&table_info.pending_leave, *addr) && table::contains(&table_info.balances, *addr)) {
                let balance = *table::borrow(&table_info.balances, *addr);
                if (balance > 0) {
                    let fee = (balance * FEE_BPS) / BPS_DENOM;
                    let payout = balance - fee;
                    let pay_coins = coin::extract<AptosCoin>(&mut table_info.escrow, balance);
                    let fee_coin = coin::extract<AptosCoin>(&mut pay_coins, fee);
                    coin::merge<AptosCoin>(&mut table_info.fee_pool, fee_coin);
                    coin::deposit(*addr, pay_coins);
                    event::emit(PlayerLeft { table: table_addr, player: *addr, payout, fee });
                };
                table::remove(&mut table_info.balances, *addr);
                table::remove(&mut table_info.pending_leave, *addr);
            };
        });
    }

    #[view]
    public fun table_exists(table_addr: address): bool {
        exists<TableInfo>(table_addr)
    }

    #[view]
    public fun min_players(table_addr: address): u64 acquires TableInfo {
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        borrow_global<TableInfo>(table_addr).min_players
    }

    #[view]
    public fun player_balance(table_addr: address, player: address): u64 acquires TableInfo {
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        let table = borrow_global<TableInfo>(table_addr);
        if (table::contains(&table.balances, player)) {
            *table::borrow(&table.balances, player)
        } else {
            0
        }
    }

    #[view]
    public fun player_pending_leave(table_addr: address, player: address): bool acquires TableInfo {
        assert!(exists<TableInfo>(table_addr), error::not_found(ETABLE_NOT_FOUND));
        let table = borrow_global<TableInfo>(table_addr);
        table::contains(&table.pending_leave, player)
    }

    // ============ Tests ============
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::aptos_account;
    #[test_only]
    use aptos_framework::aptos_coin;
    #[test_only]
    use aptos_framework::coin::BurnCapability;
    #[test_only]
    use aptos_framework::timestamp;

    #[test_only]
    fun fake_attestation(): vector<u8> {
        // In real unit tests verify_attestation will return false for this; use register_table_for_test.
        vector::empty<u8>()
    }

    #[test_only]
    /// Register table without attestation for testing.
    public entry fun register_table_for_test(table: &signer) {
        register_table_internal(table);
    }

    #[test(aptos_framework = @0x1, table = @0xf00d, alice = @0xa11c, bob = @0xb0b)]
    fun test_register_enter_and_leave_flow(
        aptos_framework: &signer,
        table: &signer,
        alice: &signer,
        bob: &signer,
    ) acquires TableInfo {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);
        let table_addr = signer::address_of(table);
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        account::create_account_for_test(table_addr);
        aptos_account::create_account(alice_addr);
        aptos_account::create_account(bob_addr);
        coin::register<AptosCoin>(table);
        let coins_a = coin::mint<AptosCoin>(1000, &mint_cap);
        let coins_b = coin::mint<AptosCoin>(1000, &mint_cap);
        coin::deposit(alice_addr, coins_a);
        coin::deposit(bob_addr, coins_b);
        coin::destroy_mint_cap(mint_cap);

        register_table_for_test(table);
        assert!(table_exists(table_addr), 0);
        assert!(min_players(table_addr) == MIN_PLAYERS, 1);

        enter_table(table_addr, alice, 100);
        enter_table(table_addr, bob, 200);
        assert!(player_balance(table_addr, alice_addr) == 100, 2);
        assert!(player_balance(table_addr, bob_addr) == 200, 3);

        request_leave(table_addr, alice);
        assert!(player_pending_leave(table_addr, alice_addr), 4);

        settle_leaving_players(table, table_addr, vector[alice_addr]);
        assert!(player_balance(table_addr, alice_addr) == 0, 5);
        assert!(player_balance(table_addr, bob_addr) == 200, 6);

        request_leave(table_addr, bob);
        settle_leaving_players(table, table_addr, vector[bob_addr]);
        assert!(player_balance(table_addr, bob_addr) == 0, 7);

        coin::destroy_burn_cap(burn_cap);
    }
}
