#[test_only]
module voting::ve_token_tests {
    use aptos_framework::primary_fungible_store;
    use std::signer;
    use voting::test_helpers::{Self, expected_voting_power};
    use voting::ve_token;
    use voting::vote_token;

    #[test(user = @0x1234)]
    fun test_lock_should_transfer_tokens_and_create_locks(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1000);

        ve_token::lock(user, 1000);

        assert!(primary_fungible_store::balance(user_addr, vote_token::token()) == 0);
        assert!(ve_token::num_user_locks(user_addr) == 1);
        assert!(ve_token::num_global_locks() == 1);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 1000);
        assert!(ve_token::global_locked_amount_at(1) == 1000);
    }

    #[test(user = @0x1234)]
    fun test_lock_should_add_to_existing_locks(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);

        ve_token::lock(user, 1000);
        ve_token::lock(user, 500);

        assert!(ve_token::num_user_locks(user_addr) == 1);
        assert!(ve_token::num_global_locks() == 1);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 1500);
        assert!(ve_token::global_locked_amount_at(1) == 1500);
    }

    #[test(user = @0x1234)]
    fun test_lock_should_not_add_to_previous_epoch_lock(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);
        assert!(ve_token::current_epoch() == 1);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        assert!(ve_token::current_epoch() == 2);
        assert!(ve_token::num_user_locks(user_addr) == 2);
        assert!(ve_token::num_global_locks() == 2);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 1000);
        assert!(ve_token::locked_amount_at(user_addr, 2) == 500);
        assert!(ve_token::global_locked_amount_at(1) == 1000);
        assert!(ve_token::global_locked_amount_at(2) == 500);
    }

    #[test(user_1 = @0x1234, user_2 = @0x5678)]
    fun test_lock_with_multiple_users_should_correctly_update_global_locks(
        user_1: &signer, user_2: &signer
    ) {
        test_helpers::setup();

        let user_addr_1 = signer::address_of(user_1);
        test_helpers::mint_vote_tokens(user_addr_1, 500);
        let user_addr_2 = signer::address_of(user_2);
        test_helpers::mint_vote_tokens(user_addr_2, 700);

        ve_token::lock(user_1, 500);
        ve_token::lock(user_2, 700);

        assert!(primary_fungible_store::balance(user_addr_1, vote_token::token()) == 0);
        assert!(primary_fungible_store::balance(user_addr_2, vote_token::token()) == 0);
        assert!(ve_token::num_user_locks(user_addr_1) == 1);
        assert!(ve_token::num_user_locks(user_addr_2) == 1);
        assert!(ve_token::num_global_locks() == 1);
        assert!(ve_token::locked_amount_at(user_addr_1, 1) == 500);
        assert!(ve_token::locked_amount_at(user_addr_2, 1) == 700);
        assert!(ve_token::global_locked_amount_at(1) == 1200);
    }

    #[test(user_1 = @0x1234, user_2 = @0x5678)]
    fun test_lock_with_multiple_users_and_epochs_should_correctly_update_global_locks(
        user_1: &signer, user_2: &signer
    ) {
        test_helpers::setup();

        let user_addr_1 = signer::address_of(user_1);
        test_helpers::mint_vote_tokens(user_addr_1, 800);
        let user_addr_2 = signer::address_of(user_2);
        test_helpers::mint_vote_tokens(user_addr_2, 1200);

        ve_token::lock(user_1, 500);
        ve_token::lock(user_2, 500);
        test_helpers::end_epoch();
        ve_token::lock(user_2, 700);
        test_helpers::end_epoch();
        ve_token::lock(user_1, 300);

        assert!(ve_token::num_user_locks(user_addr_1) == 2);
        assert!(ve_token::num_user_locks(user_addr_2) == 2);
        assert!(ve_token::num_global_locks() == 3);
        assert!(ve_token::locked_amount_at(user_addr_1, 1) == 500);
        assert!(ve_token::locked_amount_at(user_addr_1, 3) == 300);
        assert!(ve_token::locked_amount_at(user_addr_2, 1) == 500);
        assert!(ve_token::locked_amount_at(user_addr_2, 2) == 700);
        assert!(ve_token::global_locked_amount_at(1) == 1000);
        assert!(ve_token::global_locked_amount_at(2) == 700);
        assert!(ve_token::global_locked_amount_at(3) == 300);
    }

    #[test(user = @0x1234)]
    fun test_unlock_should_unlock_from_earliest_lock(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        ve_token::unlock(user, 500);

        assert!(primary_fungible_store::balance(user_addr, vote_token::token()) == 0);
        assert!(ve_token::num_user_locks(user_addr) == 2);
        assert!(ve_token::num_global_locks() == 2);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 500);
        assert!(ve_token::locked_amount_at(user_addr, 2) == 500);
        assert!(ve_token::global_locked_amount_at(1) == 500);
        assert!(ve_token::global_locked_amount_at(2) == 500);
    }

    #[test(user = @0x1234)]
    fun test_unlock_should_unlock_from_multiple_locks(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 2200);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 700);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        ve_token::unlock(user, 1900);

        assert!(ve_token::num_user_locks(user_addr) == 1);
        assert!(ve_token::num_global_locks() == 1);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 0);
        assert!(ve_token::locked_amount_at(user_addr, 2) == 0);
        assert!(ve_token::locked_amount_at(user_addr, 3) == 300);
        assert!(ve_token::global_locked_amount_at(1) == 0);
        assert!(ve_token::global_locked_amount_at(2) == 0);
        assert!(ve_token::global_locked_amount_at(3) == 300);
    }

    #[test(user_1 = @0x1234, user_2 = @0x5678)]
    fun test_unlock_with_multiple_users_should_correctly_update_locks(
        user_1: &signer, user_2: &signer
    ) {
        test_helpers::setup();

        let user_addr_1 = signer::address_of(user_1);
        test_helpers::mint_vote_tokens(user_addr_1, 1500);
        let user_addr_2 = signer::address_of(user_2);
        test_helpers::mint_vote_tokens(user_addr_2, 3000);

        ve_token::lock(user_1, 1000);
        ve_token::lock(user_2, 2000);
        test_helpers::end_epoch();
        ve_token::lock(user_1, 500);
        ve_token::lock(user_2, 1000);

        ve_token::unlock(user_1, 1200);
        ve_token::unlock(user_2, 1000);

        assert!(ve_token::num_user_locks(user_addr_1) == 1);
        assert!(ve_token::num_user_locks(user_addr_2) == 2);
        assert!(ve_token::num_global_locks() == 2);
        assert!(ve_token::locked_amount_at(user_addr_1, 1) == 0);
        assert!(ve_token::locked_amount_at(user_addr_1, 2) == 300);
        assert!(ve_token::locked_amount_at(user_addr_2, 1) == 1000);
        assert!(ve_token::locked_amount_at(user_addr_2, 2) == 1000);
        assert!(ve_token::global_locked_amount_at(1) == 1000);
        assert!(ve_token::global_locked_amount_at(2) == 1300);
    }

    #[test(user = @0x1234)]
    #[expected_failure(abort_code = voting::ve_token::ECANNOT_UNLOCK_MORE_THAN_TOTAL_LOCKED)]
    fun test_unlock_should_abort_if_insufficient_total_locked(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        ve_token::unlock(user, 2000);
    }

    #[test(user = @0x1234)]
    fun test_withdraw_should_transfer_tokens(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        ve_token::unlock(user, 1500);
        test_helpers::fast_forward_past_unlock_delay();
        assert!(primary_fungible_store::balance(user_addr, vote_token::token()) == 0);
        ve_token::withdraw(user);

        assert!(primary_fungible_store::balance(user_addr, vote_token::token()) == 1500);
        assert!(ve_token::num_user_locks(user_addr) == 0);
        assert!(ve_token::num_global_locks() == 0);
    }

    #[test(user = @0x1234)]
    #[expected_failure(abort_code = voting::ve_token::EZERO_WITHDRAWABLE_AMOUNT)]
    fun test_withdraw_should_abort_if_zero_withdrawable(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1500);

        ve_token::lock(user, 1000);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        ve_token::unlock(user, 1500);
        ve_token::withdraw(user);
    }

    #[test(user = @0x1234)]
    fun test_voting_power_at_should_not_include_current_epoch(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1800);

        let epoch_1_lock = 1300;
        ve_token::lock(user, epoch_1_lock);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        assert!(ve_token::voting_power_at(user_addr, 1) == 0);
        assert!(ve_token::voting_power_at(user_addr, 2) == expected_voting_power(epoch_1_lock, 1));
    }

    #[test(user = @0x1234)]
    fun test_voting_power_at_should_calculate_based_on_lock_duration(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1800);

        let epoch_1_lock = 1000;
        ve_token::lock(user, epoch_1_lock);

        assert!(ve_token::voting_power_at(user_addr, 1) == 0);
        let expected_voting_power_at_epoch_2 = expected_voting_power(epoch_1_lock, 1);
        assert!(ve_token::voting_power_at(user_addr, 2) == expected_voting_power_at_epoch_2);
        let expected_voting_power_at_epoch_20 = expected_voting_power(epoch_1_lock, 19);
        assert!(ve_token::voting_power_at(user_addr, 20) == expected_voting_power_at_epoch_20);
        let expected_voting_power_at_epoch_53 = expected_voting_power(epoch_1_lock, 52);
        assert!(ve_token::voting_power_at(user_addr, 53) == expected_voting_power_at_epoch_53);
        let expected_voting_power_at_epoch_55 = expected_voting_power(epoch_1_lock, 52);
        assert!(ve_token::voting_power_at(user_addr, 55) == expected_voting_power_at_epoch_55);
    }

    #[test(user = @0x1234)]
    fun test_voting_power_at_should_correctly_calculate_from_multiple_locks(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1800);

        let epoch_1_lock = 1000;
        ve_token::lock(user, epoch_1_lock);
        test_helpers::end_epoch();
        let epoch_2_lock = 500;
        ve_token::lock(user, epoch_2_lock);
        test_helpers::end_epoch();
        let epoch_3_lock = 300;
        ve_token::lock(user, epoch_3_lock);

        assert!(ve_token::voting_power_at(user_addr, 1) == 0);
        let expected_voting_power_at_epoch_2 =
            expected_voting_power(epoch_1_lock, 1);
        assert!(ve_token::voting_power_at(user_addr, 2) == expected_voting_power_at_epoch_2);
        let expected_voting_power_at_epoch_3 =
            expected_voting_power(epoch_1_lock, 2) +
                expected_voting_power(epoch_2_lock, 1);
        assert!(ve_token::voting_power_at(user_addr, 3) == expected_voting_power_at_epoch_3);
        let expected_voting_power_at_epoch_20 =
            expected_voting_power(epoch_1_lock, 19) +
                expected_voting_power(epoch_2_lock, 18) +
                expected_voting_power(epoch_3_lock, 17);
        assert!(ve_token::voting_power_at(user_addr, 20) == expected_voting_power_at_epoch_20);
        let expected_voting_power_at_epoch_53 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 51) +
                expected_voting_power(epoch_3_lock, 50);
        assert!(ve_token::voting_power_at(user_addr, 53) == expected_voting_power_at_epoch_53);
        let expected_voting_power_at_epoch_54 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 52) +
                expected_voting_power(epoch_3_lock, 51);
        assert!(ve_token::voting_power_at(user_addr, 54) == expected_voting_power_at_epoch_54);
        let expected_voting_power_at_epoch_55 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 52) +
                expected_voting_power(epoch_3_lock, 52);
        assert!(ve_token::voting_power_at(user_addr, 55) == expected_voting_power_at_epoch_55);
    }

    #[test(user = @0x1234)]
    fun test_total_voting_power_at_should_not_include_current_epoch(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1800);

        let epoch_1_lock = 1300;
        ve_token::lock(user, epoch_1_lock);
        test_helpers::end_epoch();
        ve_token::lock(user, 500);

        assert!(ve_token::total_voting_power_at(1) == 0);
        assert!(ve_token::total_voting_power_at(2) == expected_voting_power(epoch_1_lock, 1));
    }

    #[test(user_1 = @0x1234, user_2 = @0x4567)]
    fun test_total_voting_power_at_should_calculate_based_on_lock_duration(
        user_1: &signer,
        user_2: &signer
    ) {
        test_helpers::setup();

        test_helpers::mint_vote_tokens(signer::address_of(user_1), 500);
        test_helpers::mint_vote_tokens(signer::address_of(user_2), 500);

        let epoch_1_lock = 1000;
        ve_token::lock(user_1, epoch_1_lock / 2);
        ve_token::lock(user_2, epoch_1_lock / 2);

        assert!(ve_token::total_voting_power_at(1) == 0);
        let expected_voting_power_at_epoch_2 = expected_voting_power(epoch_1_lock, 1);
        assert!(ve_token::total_voting_power_at(2) == expected_voting_power_at_epoch_2);
        let expected_voting_power_at_epoch_20 = expected_voting_power(epoch_1_lock, 19);
        assert!(ve_token::total_voting_power_at(20) == expected_voting_power_at_epoch_20);
        let expected_voting_power_at_epoch_53 = expected_voting_power(epoch_1_lock, 52);
        assert!(ve_token::total_voting_power_at(53) == expected_voting_power_at_epoch_53);
        let expected_voting_power_at_epoch_55 = expected_voting_power(epoch_1_lock, 52);
        assert!(ve_token::total_voting_power_at(55) == expected_voting_power_at_epoch_55);
    }

    #[test(user_1 = @0x1234, user_2 = @0x4567)]
    fun test_total_voting_power_at_should_correctly_calculate_from_multiple_locks(
        user_1: &signer,
        user_2: &signer
    ) {
        test_helpers::setup();

        test_helpers::mint_vote_tokens(signer::address_of(user_1), 1300);
        test_helpers::mint_vote_tokens(signer::address_of(user_2), 500);

        let epoch_1_lock = 1000;
        ve_token::lock(user_1, epoch_1_lock);
        test_helpers::end_epoch();
        let epoch_2_lock = 500;
        ve_token::lock(user_2, epoch_2_lock);
        test_helpers::end_epoch();
        let epoch_3_lock = 300;
        ve_token::lock(user_1, epoch_3_lock);

        assert!(ve_token::total_voting_power_at(1) == 0);
        let expected_voting_power_at_epoch_2 =
            expected_voting_power(epoch_1_lock, 1);
        assert!(ve_token::total_voting_power_at(2) == expected_voting_power_at_epoch_2);
        let expected_voting_power_at_epoch_3 =
            expected_voting_power(epoch_1_lock, 2) +
                expected_voting_power(epoch_2_lock, 1);
        assert!(ve_token::total_voting_power_at(3) == expected_voting_power_at_epoch_3);
        let expected_voting_power_at_epoch_20 =
            expected_voting_power(epoch_1_lock, 19) +
                expected_voting_power(epoch_2_lock, 18) +
                expected_voting_power(epoch_3_lock, 17);
        assert!(ve_token::total_voting_power_at(20) == expected_voting_power_at_epoch_20);
        let expected_voting_power_at_epoch_53 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 51) +
                expected_voting_power(epoch_3_lock, 50);
        assert!(ve_token::total_voting_power_at(53) == expected_voting_power_at_epoch_53);
        let expected_voting_power_at_epoch_54 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 52) +
                expected_voting_power(epoch_3_lock, 51);
        assert!(ve_token::total_voting_power_at(54) == expected_voting_power_at_epoch_54);
        let expected_voting_power_at_epoch_55 =
            expected_voting_power(epoch_1_lock, 52) +
                expected_voting_power(epoch_2_lock, 52) +
                expected_voting_power(epoch_3_lock, 52);
        assert!(ve_token::total_voting_power_at(55) == expected_voting_power_at_epoch_55);
    }

    #[test(user = @0x1234)]
    fun test_compact_locks_should_compact_one_lock(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1000000);

        let locked_amount = 1000;
        let num_locks = ve_token::maximum_lock_epochs();
        let total_locked_amount = locked_amount * num_locks;
        for (i in 0..num_locks) {
            ve_token::lock(user, locked_amount);
            test_helpers::end_epoch();
        };
        // We're now at epoch 54 while the locks are from 1 -> 52. Lock 1 is now be past the lookback window.
        test_helpers::end_epoch();

        assert!(ve_token::num_user_locks(user_addr) == num_locks);
        assert!(ve_token::total_locked_amount(user_addr) == total_locked_amount);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);
        ve_token::compact_user_locks(user);

        // Epoch 1 has been compacted into lock 2.
        assert!(ve_token::num_user_locks(user_addr) == num_locks - 1);
        assert!(ve_token::total_locked_amount(user_addr) == total_locked_amount);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);
        assert!(ve_token::locked_amount_at(user_addr, 1) == 0);
        assert!(ve_token::locked_amount_at(user_addr, 2) == locked_amount * 2);
        assert!(ve_token::locked_amount_at(user_addr, 3) == locked_amount);
    }

    #[test(user = @0x1234)]
    fun test_compact_locks_should_be_called_in_lock(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1000000);

        let locked_amount = 1000;
        let num_locks = ve_token::maximum_lock_epochs() + 3;
        let total_locked_amount = locked_amount * num_locks;
        for (i in 0..num_locks) {
            ve_token::lock(user, locked_amount);
            test_helpers::end_epoch();
        };
        // Although we created 55 locks, ve_token::lock() automatically call locks so we should have 53 locks remaining.
        // These are locks 3 -> 55 because current epoch is 55 which doesn't count in voting power. Locks 3-54 do.
        assert!(ve_token::num_user_locks(user_addr) == 53);
        assert!(ve_token::total_locked_amount(user_addr) == total_locked_amount);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);

        // Epochs 1-2 has been compacted into lock 3.
        assert!(ve_token::locked_amount_at(user_addr, 1) == 0);
        assert!(ve_token::locked_amount_at(user_addr, 2) == 0);
        assert!(ve_token::locked_amount_at(user_addr, 3) == locked_amount * 3);
        assert!(ve_token::locked_amount_at(user_addr, 4) == locked_amount);
        assert!(ve_token::locked_amount_at(user_addr, 5) == locked_amount);
    }

    #[test(user_1 = @0x1234, user_2 = @0x4567)]
    fun test_compact_global_locks(user_1: &signer, user_2: &signer) {
        test_helpers::setup();

        test_helpers::mint_vote_tokens(signer::address_of(user_1), 1000000);
        test_helpers::mint_vote_tokens(signer::address_of(user_2), 1000000);

        let locked_amount_1 = 1000;
        let locked_amount_2 = 2000;
        let num_locks = ve_token::maximum_lock_epochs();
        let total_locked_amount = (locked_amount_1 + locked_amount_2) * num_locks / 2;
        for (i in 0..num_locks) {
            // User 1 only locks in odd epochs and user 2 only locks in even epochs.
            if ((i + 1) % 2 == 0) {
                ve_token::lock(user_1, locked_amount_1);
            } else {
                ve_token::lock(user_2, locked_amount_2);
            };
            test_helpers::end_epoch();
        };
        test_helpers::end_epoch();
        // We're now at epoch 54. Global locks are from 1 -> 52. Global lock 1 will be compacted.
        assert!(ve_token::num_global_locks() == num_locks);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);
        ve_token::compact_global_locks();

        // Epochs 1 has been compacted into lock 2.
        assert!(ve_token::num_global_locks() == num_locks - 1);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);
        assert!(ve_token::global_locked_amount_at(1) == 0);
        assert!(ve_token::global_locked_amount_at(2) == locked_amount_1 + locked_amount_2);
        assert!(ve_token::global_locked_amount_at(3) == locked_amount_2);
        assert!(ve_token::global_locked_amount_at(4) == locked_amount_1);
    }

    #[test(user = @0x1234)]
    fun test_compact_global_locks_should_be_called_in_lock(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1000000);

        let locked_amount = 1000;
        let num_locks = ve_token::maximum_lock_epochs() + 3;
        let total_locked_amount = locked_amount * num_locks;
        for (i in 0..num_locks) {
            ve_token::lock(user, locked_amount);
            test_helpers::end_epoch();
        };
        // Although we created 55 locks, ve_token::lock() automatically call locks so we should have 53 locks remaining.
        // These are locks 3 -> 55 because current epoch is 55 which doesn't count in voting power. Locks 3-54 do.
        assert!(ve_token::num_global_locks() == 53);
        assert!(ve_token::total_global_locked_amount() == total_locked_amount);

        // Epochs 1-2 has been compacted into lock 3.
        assert!(ve_token::global_locked_amount_at(1) == 0);
        assert!(ve_token::global_locked_amount_at(2) == 0);
        assert!(ve_token::global_locked_amount_at(3) == locked_amount * 3);
        assert!(ve_token::global_locked_amount_at(4) == locked_amount);
        assert!(ve_token::global_locked_amount_at(5) == locked_amount);
    }
}
