#[test_only]
module vote_lockup::vote_tests {
    use aptos_framework::fungible_asset;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use std::signer;
    use std::vector;
    use vote_lockup::epoch;
    use vote_lockup::test_helpers;
    use vote_lockup::vote::{Self, VotingCertificate};
    use vote_lockup::voting_token;

    #[test(voter_1 = @0xcafe, voter_2 = @0xdead)]
    fun test_create_and_withdraw_lock(voter_1: &signer, voter_2: &signer) {
        test_helpers::set_up();
        let voting_tokens = voting_token::test_mint(1020);
        // Create a lock from an account's primary store.
        let voter_1_addr = signer::address_of(voter_1);
        primary_fungible_store::deposit(voter_1_addr, voting_tokens);
        assert!(voting_token::balance(voter_1_addr) == 1020, 0);
        let voter_1_certificate = vote::create_lock(voter_1, 520, 100);

        // Create a lock from direct $VOTING tokens.
        let voter_2_certificate = mint_and_create_lock(voter_2, 100, 50);

        // The certificates should contain $VOTING for the amount deposited.
        assert!(voting_token::balance(voter_1_addr) == 500, 0);
        let voter_2_addr = signer::address_of(voter_2);
        assert!(voting_token::balance(voter_2_addr) == 0, 0);
        verify_balances_and_manifested_total_supply(
            vector[voter_1_certificate, voter_2_certificate],
            vector[520, 100],
            vector[100, 50],
        );

        // Fast forward to unlock and withdraw.
        epoch::fast_forward(50);
        vote::withdraw_entry(voter_2, voter_2_certificate);
        assert!(voting_token::balance(voter_2_addr) == 100, 0);
        // One certificate still remains with 50 epochs of lockup.
        verify_balances_and_manifested_total_supply(vector[voter_1_certificate],vector[520],vector[50]);
        epoch::fast_forward(50);
        assert!(vote::total_voting_power() == 0, 0);
        vote::withdraw_entry(voter_1, voter_1_certificate);
        assert!(voting_token::balance(voter_1_addr) == 1020, 0);

        // Check that the certificate objects has been deleted.
        verify_is_deleted(voter_1_certificate);
        verify_is_deleted(voter_2_certificate);
    }

    #[test(voter_1 = @0xcafe)]
    fun test_extend_lockup_and_increase_amount(voter_1: &signer) {
        test_helpers::set_up();
        // Original certificate is locked for 100 epochs.
        let voting_certificate = mint_and_create_lock(voter_1, 520, 104);
        verify_balances_and_manifested_total_supply(vector[voting_certificate], vector[520], vector[104]);

        // Fast forward 50 epochs so the certificate is only locked for another 50. Extend the lock to 100 epochs and
        // verify.
        epoch::fast_forward(52);
        verify_balances_and_manifested_total_supply(vector[voting_certificate], vector[520], vector[52]);
        vote::extend_lockup(voter_1, voting_certificate, 104);
        verify_balances_and_manifested_total_supply(vector[voting_certificate], vector[520], vector[104]);

        // Fast forward another 50 epochs. certificate lock should have 50 epochs remaining. Increase the amount and
        // verify.
        epoch::fast_forward(52);
        vote::increase_amount_with(voting_certificate, voting_token::test_mint(520));
        verify_balances_and_manifested_total_supply(vector[voting_certificate], vector[1040], vector[52]);
    }

    #[test(voter_1 = @0xcafe)]
    #[expected_failure(abort_code = 5, location = vote_lockup::vote)]
    fun test_cannot_withdraw_locked_tokens(voter_1: &signer) {
        test_helpers::set_up();
        let voting_certificate = mint_and_create_lock(voter_1, 500, 100);
        assert!(vote::get_lockup_expiration_epoch(voting_certificate) == 200, 0);
        epoch::fast_forward(98);
        // Should fail because only 98 epochs have passed while the lockup is for 100.
        vote::withdraw_entry(voter_1, voting_certificate);
    }

    #[test(voter_1 = @0xcafe)]
    #[expected_failure(abort_code = 65539, location = aptos_framework::fungible_asset)]
    fun test_cannot_extract_tokens_from_certificate(voter_1: &signer) {
        test_helpers::set_up();
        let voting_certificate = mint_and_create_lock(voter_1, 500, 100);
        fungible_asset::deposit(voting_certificate, fungible_asset::withdraw(voter_1, voting_certificate, 1));
    }

    #[test(voter_1 = @0xcafe)]
    #[expected_failure(abort_code = 65539, location = aptos_framework::fungible_asset)]
    fun test_cannot_deposit_tokens_into_certificate(voter_1: &signer) {
        test_helpers::set_up();
        let voting_certificate = mint_and_create_lock(voter_1, 500, 100);
        fungible_asset::deposit(voting_certificate, voting_token::test_mint(100));
    }

    fun verify_balances_and_manifested_total_supply(
        certificates: vector<Object<VotingCertificate>>,
        locked_amounts: vector<u64>,
        remaining_locked_epochs: vector<u64>,
    ) {
        let curr_epoch = epoch::now();
        let epoch = curr_epoch;
        let max_lockup_epochs = vote::max_lockup_epochs();
        while (epoch <= curr_epoch + max_lockup_epochs + 1) {
            let total_balance = 0;
            vector::enumerate_ref(&locked_amounts, |i, amount| {
                let amount: u64 = *amount;
                let remaining_lockup_epochs = *vector::borrow(&remaining_locked_epochs, i);
                let lockup_end = curr_epoch + remaining_lockup_epochs;
                let certificate: Object<VotingCertificate> = *vector::borrow(&certificates, i);
                assert!(fungible_asset::balance(certificate) == amount, 0);
                assert!(vote::remaining_lockup_epochs(certificate) == remaining_lockup_epochs, 0);
                assert!(vote::get_lockup_expiration_epoch(certificate) == lockup_end, 0);
                let balance_at_epoch = vote::get_voting_power_at_epoch(certificate, epoch);
                if (epoch >= lockup_end) {
                    assert!(balance_at_epoch == 0, 0);
                } else {
                    let expected_balance = amount * (lockup_end - epoch) / max_lockup_epochs;
                    assert!(balance_at_epoch == expected_balance, 0);
                    total_balance = total_balance + expected_balance;
                };
            });
            assert!((total_balance as u128) == vote::total_voting_power_at(epoch), 0);
            epoch = epoch + 1;
        }
    }

    public fun mint_and_create_lock(owner: &signer, amount: u64, lockup_epochs: u64): Object<VotingCertificate> {
        vote::create_lock_with(voting_token::test_mint(amount), lockup_epochs, signer::address_of(owner))
    }

    fun verify_is_deleted(certificate: Object<VotingCertificate>) {
        let certificate_address = object::object_address(&certificate);
        assert!(!vote::certificate_exists(certificate_address), 0);
        assert!(!object::is_object(certificate_address), 0);
    }
}
