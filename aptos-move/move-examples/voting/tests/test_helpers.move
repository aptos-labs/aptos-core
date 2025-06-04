#[test_only]
module voting::test_helpers {
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_std::math128;
    use voting::ve_token;
    use voting::vote_token;

    public fun setup() {
        timestamp::set_time_has_started_for_testing(&account::create_signer_for_test(@aptos_framework));
        vote_token::init_for_test(deployer());
        ve_token::init_for_test(deployer());
    }

    public fun expected_voting_power(locked_amount: u64, num_epochs_locked: u64): u128 {
        let locked_amount = locked_amount as u128;
        let num_epochs_locked = num_epochs_locked as u128;
        let (multiplier_num, multiplier_denom) = ve_token::voting_multiplier();
        locked_amount + math128::mul_div(locked_amount, multiplier_num * num_epochs_locked, multiplier_denom)
    }

    public fun end_epoch() {
        fast_forward_epochs(1);
    }

    public fun fast_forward_epochs(epochs: u64) {
        timestamp::fast_forward_seconds(ve_token::epoch_duration() * epochs);
    }

    public fun fast_forward_past_unlock_delay() {
        timestamp::fast_forward_seconds(ve_token::unlock_delay());
    }

    public fun mint_vote_tokens(user: address, amount: u64) {
        vote_token::mint_to(
            deployer(),
            vote_token::token(),
            vector[user],
            vector[amount]
        );
    }

    public inline fun deployer(): &signer {
        &account::create_signer_for_test(@voting)
    }
}