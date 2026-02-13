#[test_only]
module std::test_leader_ban_registry {
    use std::bcs;
    use std::option;
    use std::vector;
    use aptos_std::ed25519;
    use supra_framework::supra_coin;
    use supra_framework::coin;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::stake;
    use supra_framework::leader_ban_registry;
    use supra_framework::leader_ban_registry_config;
    use std::signer;
    use supra_framework::account;

    fun setup_staking_modules(
        sender: &signer,
        validator_1: &signer,
        validator_2: &signer,
        validator_3: &signer,
        validator_4: &signer
    ) {
        // initialise stake module
        let (_v_1_s_key, v_1_p_key) = ed25519::generate_keys();
        let (_v_2_s_key, v_2_p_key) = ed25519::generate_keys();
        let (_v_3_s_key, v_3_p_key) = ed25519::generate_keys();
        let (_v_4_s_key, v_4_p_key) = ed25519::generate_keys();

        stake::initialize_for_test(sender);
        account::create_account_for_test(signer::address_of(validator_1));
        coin::register<SupraCoin>(validator_1);
        supra_coin::mint(sender, signer::address_of(validator_1), 10000000000);

        account::create_account_for_test(signer::address_of(validator_2));
        coin::register<SupraCoin>(validator_2);
        supra_coin::mint(sender, signer::address_of(validator_2), 10000000000);

        account::create_account_for_test(signer::address_of(validator_3));
        coin::register<SupraCoin>(validator_3);
        supra_coin::mint(sender, signer::address_of(validator_3), 10000000000);

        account::create_account_for_test(signer::address_of(validator_4));
        coin::register<SupraCoin>(validator_4);
        supra_coin::mint(sender, signer::address_of(validator_4), 10000000000);

        let active_coin = coin::withdraw<SupraCoin>(validator_1, 500);
        let pending_coin = coin::withdraw<SupraCoin>(validator_1, 1000);
        stake::create_stake_pool(
            validator_1,
            active_coin,
            pending_coin,
            500000000
        );

        let active_coin = coin::withdraw<SupraCoin>(validator_2, 500);
        let pending_coin = coin::withdraw<SupraCoin>(validator_2, 1000);
        stake::create_stake_pool(
            validator_2,
            active_coin,
            pending_coin,
            500000000
        );

        let active_coin = coin::withdraw<SupraCoin>(validator_3, 500);
        let pending_coin = coin::withdraw<SupraCoin>(validator_3, 1000);
        stake::create_stake_pool(
            validator_3,
            active_coin,
            pending_coin,
            500000000
        );

        let active_coin = coin::withdraw<SupraCoin>(validator_4, 500);
        let pending_coin = coin::withdraw<SupraCoin>(validator_4, 1000);
        stake::create_stake_pool(
            validator_4,
            active_coin,
            pending_coin,
            500000000
        );

        stake::join_validator_set_for_test(
            &ed25519::public_key_to_unvalidated(&v_4_p_key),
            validator_4,
            signer::address_of(validator_4),
            false
        );
        stake::join_validator_set_for_test(
            &ed25519::public_key_to_unvalidated(&v_3_p_key),
            validator_3,
            signer::address_of(validator_3),
            false
        );
        stake::join_validator_set_for_test(
            &ed25519::public_key_to_unvalidated(&v_2_p_key),
            validator_2,
            signer::address_of(validator_2),
            false
        );
        stake::join_validator_set_for_test(
            &ed25519::public_key_to_unvalidated(&v_1_p_key),
            validator_1,
            signer::address_of(validator_1),
            true
        ); // on epoch change
    }

    #[
        test(
            sender = @supra_framework,
            validator_1 = @0xdead01,
            validator_2 = @0xdead02,
            validator_3 = @0xdead03,
            validator_4 = @0xdead04
        )
    ]
    fun test_ban_registy_e2e(
        sender: &signer,
        validator_1: &signer,
        validator_2: &signer,
        validator_3: &signer,
        validator_4: &signer
    ) {
        // initialise ban registry config
        let ban_params = leader_ban_registry_config::get_test_ban_registry_params_v0();
        let ban_config_bytes = bcs::to_bytes(&ban_params);
        leader_ban_registry_config::initialize(sender, ban_config_bytes);

        // initialise ban registry
        leader_ban_registry::initialize_leader_ban_registry(sender);
        setup_staking_modules(
            sender,
            validator_1,
            validator_2,
            validator_3,
            validator_4
        );

        // ===== Part 1: Basic ban -> probation -> reinstatement =====
        // Config: initial_ban_duration=4, probation_duration=4, max_ban_duration=20, committee_size=4

        // Round 0: No failures, registry empty
        leader_ban_registry::update_ban_registry(0, 0, option::some(0), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 0, 1);

        // Round 1: Validator 2 (index 1) fails to propose -> banned
        leader_ban_registry::update_ban_registry(0, 1, option::some(2), vector[1]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 2);
        assert!(
            signer::address_of(validator_2)
                == leader_ban_registry::get_pool_address_from_vp(
                    vector::borrow(&ban_registry, 0)
                ),
            3
        );

        // Round 4: Ban still active (duration=4, rounds_served=3, remaining=1)
        leader_ban_registry::update_ban_registry(0, 4, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 4);
        let vp = vector::borrow(&ban_registry, 0);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 100);

        // Round 5: Ban expires (rounds_served=4), transitions to probation
        leader_ban_registry::update_ban_registry(0, 5, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 5); // Still in registry (on probation)
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 101);

        // Round 9: Probation expires (rounds_served=4), fully reinstated
        leader_ban_registry::update_ban_registry(0, 9, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 0, 6);

        // ===== Part 2: Consecutive bans via re-ban during probation =====

        // Round 12: Validator 1 (index 0) fails -> banned, consecutive=0, duration=4
        leader_ban_registry::update_ban_registry(0, 12, option::some(1), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(
            signer::address_of(validator_1)
                == leader_ban_registry::get_pool_address_from_vp(vp),
            7
        );
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 0, 8);

        // Round 16: Ban expires (duration=4) -> probation
        leader_ban_registry::update_ban_registry(0, 16, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 102);

        // Round 17: Re-ban during probation -> consecutive=1, duration=8
        leader_ban_registry::update_ban_registry(0, 17, option::some(1), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 1, 9);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 103);
        assert!(vector::length(&ban_registry) == 1, 10);

        // Round 25: Ban expires (duration=8) -> probation
        leader_ban_registry::update_ban_registry(0, 25, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 104);

        // Round 26: Re-ban during probation -> consecutive=2, duration=16
        leader_ban_registry::update_ban_registry(0, 26, option::some(1), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 2, 11);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 105);
        assert!(vector::length(&ban_registry) == 1, 12);

        // Round 42: Ban expires (duration=16) -> probation
        leader_ban_registry::update_ban_registry(0, 42, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 106);

        // Round 43: Re-ban during probation -> consecutive=3, duration=min(32,20)=20
        leader_ban_registry::update_ban_registry(0, 43, option::some(1), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 3, 13);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 107);
        assert!(vector::length(&ban_registry) == 1, 14);

        // Round 63: Ban expires (duration=20) -> probation
        leader_ban_registry::update_ban_registry(0, 63, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 108);

        // Round 64: Re-ban during probation -> consecutive=4, duration=min(64,20)=20
        leader_ban_registry::update_ban_registry(0, 64, option::some(1), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 4, 15);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 109);
        assert!(vector::length(&ban_registry) == 1, 16);

        // ===== Part 3: Natural expiry of max-duration ban =====
        // Ban at round 64 with consecutive=4, duration=20
        // Ban expires at round 84, probation starts at 84, probation expires at 88

        // At round 83, the ban should still be active (1 round remaining)
        leader_ban_registry::update_ban_registry(0, 83, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 17);
        let vp = vector::borrow(&ban_registry, 0);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 110); // Still banned

        // At round 84, the ban expires and validator transitions to probation
        leader_ban_registry::update_ban_registry(0, 84, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 18); // Still in registry (on probation)
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 111); // Now on probation

        // At round 87, probation should still be active (1 round remaining)
        leader_ban_registry::update_ban_registry(0, 87, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 19);
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 112);

        // At round 88, probation expires and validator is fully reinstated
        leader_ban_registry::update_ban_registry(0, 88, option::some(1), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 0, 20); // Fully removed from registry

        // ===== Part 4: Multi-validator banning with proposer limit =====
        // Now lets try to fail 3 of them
        leader_ban_registry::update_ban_registry(0, 90, option::some(3), vector[0, 1, 2]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        // only 2 will be banned because of proposer limit of 2
        assert!(vector::length(&ban_registry) == 2, 21);

        // Re-ban both validators to increase consecutive count
        // Round 94: re-ban, consecutive_bans=1, duration=8, resets from round 94
        leader_ban_registry::update_ban_registry(
            0, 90 + (4 * 1), option::some(2), vector[0, 1]
        );
        // Round 98: re-ban, consecutive_bans=2, duration=16, resets from round 98
        leader_ban_registry::update_ban_registry(
            0, 90 + (4 * 2), option::some(2), vector[0, 1]
        );

        // ===== Part 5: Cross-epoch ban management =====
        // Now test ban expiry across epoch change WITHOUT re-banning
        // Ban was set at round 98 with duration 16
        // First trigger epoch change
        leader_ban_registry::on_new_epoch();

        // In epoch 1, advance rounds without re-banning to let the ban expire naturally
        // For validators banned at epoch 0 round 98:
        // After on_new_epoch: rounds_served_in_previous_epochs = 0
        // (since epoch_earned == latest_view.epoch and round_earned == latest_view.round)

        // At epoch 1 round 0, ban should still be active (rounds_served = 0 + 0 = 0, need 16)
        leader_ban_registry::update_ban_registry(1, 0, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 2, 22);
        let vp = vector::borrow(&ban_registry, 0);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 113);

        // At epoch 1 round 15, ban should still be active (rounds_served = 0 + 15 = 15, need 16)
        leader_ban_registry::update_ban_registry(1, 15, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 2, 23);

        // At epoch 1 round 16, ban expires and validators transition to probation
        leader_ban_registry::update_ban_registry(1, 16, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 2, 24); // Still in registry (on probation)
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 114);

        // At epoch 1 round 19, probation still active (1 round remaining)
        leader_ban_registry::update_ban_registry(1, 19, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 2, 25);

        // At epoch 1 round 20, probation expires and validators are fully reinstated
        leader_ban_registry::update_ban_registry(1, 20, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 0, 26);

        // ===== Part 6: Re-ban during probation =====
        // Test: Re-banning during probation should increase consecutive count
        // Ban validator 1 fresh
        leader_ban_registry::update_ban_registry(1, 25, option::some(2), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        assert!(vector::length(&ban_registry) == 1, 27);
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 0, 115);
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 116);

        // Ban duration = 4 rounds, so ban expires at round 25 + 4 = 29
        // Transition to probation at round 29
        leader_ban_registry::update_ban_registry(1, 29, option::some(2), vector::empty());
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::is_on_probation_from_vp(vp), 117);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 0, 118);

        // Re-ban during probation should increase consecutive count and reset to banned state
        leader_ban_registry::update_ban_registry(1, 30, option::some(2), vector[0]);
        let ban_registry = leader_ban_registry::get_ban_registry();
        let vp = vector::borrow(&ban_registry, 0);
        assert!(leader_ban_registry::get_consecutive_count_from_vp(vp) == 1, 119); // Consecutive count increased
        assert!(!leader_ban_registry::is_on_probation_from_vp(vp), 120); // Back to banned state

    }
}
