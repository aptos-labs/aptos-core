module vesting::vesting {
    use aptos_framework::vesting;
    use std::fixed_point32::{Self, FixedPoint32};
    use std::signer;
    use std::vector;
    use aptos_std::simple_map;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::timestamp;

    #[test_only]
    use aptos_framework::aptos_account;
    #[test_only]
    use aptos_framework::stake;

    const MIN_STAKE: u64 = 100000000000000; // 1M APT coins with 8 decimals.
    const GRANT_AMOUNT: u64 = 20000000000000000; // 200M APT coins with 8 decimals.
    const VESTING_SCHEDULE_CLIFF: u64 = 31536000; // 1 year
    const VESTING_PERIOD: u64 = 2592000; // 30 days
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    public entry fun setup(admin: &signer) {
        setup_vesting_contract(
            admin,
            &vector[signer::address_of(admin)],
            &vector[MIN_STAKE],
            signer::address_of(admin),
            0
        );
    }

    public fun setup_vesting_contract(
        admin: &signer,
        shareholders: &vector<address>,
        shares: &vector<u64>,
        withdrawal_address: address,
        commission_percentage: u64,
    ): address {
        setup_vesting_contract_with_schedule(
            admin,
            shareholders,
            shares,
            withdrawal_address,
            commission_percentage,
            &vector[3, 2, 1],
            48,
        )
    }

    public fun setup_vesting_contract_with_schedule(
        admin: &signer,
        shareholders: &vector<address>,
        shares: &vector<u64>,
        withdrawal_address: address,
        commission_percentage: u64,
        vesting_numerators: &vector<u64>,
        vesting_denominator: u64,
    ): address {
        let schedule = vector::empty<FixedPoint32>();
        vector::for_each_ref(vesting_numerators, |num| {
            vector::push_back(&mut schedule, fixed_point32::create_from_rational(*num, vesting_denominator));
        });
        let vesting_schedule = vesting::create_vesting_schedule(
            schedule,
            timestamp::now_seconds() + VESTING_SCHEDULE_CLIFF,
            VESTING_PERIOD,
        );

        let admin_address = signer::address_of(admin);
        let buy_ins = simple_map::create<address, Coin<AptosCoin>>();
        vector::enumerate_ref(shares, |i, share| {
            let shareholder = *vector::borrow(shareholders, i);
            //simple_map::add(&mut buy_ins, shareholder, stake::mint_coins(*share));
            simple_map::add(&mut buy_ins, shareholder, coin::withdraw(admin, *share));
        });

        vesting::create_vesting_contract(
            admin,
            shareholders,
            buy_ins,
            vesting_schedule,
            withdrawal_address,
            admin_address,
            admin_address,
            commission_percentage,
            vector[],
        )
    }

    #[test(admin = @0x42, aptos_framework = @0x1)]
    fun test_setup(admin: &signer, aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        //coin::transfer(aptos_framework, signer::address_of(admin), 10);
        stake::initialize_for_test_custom(aptos_framework, MIN_STAKE, GRANT_AMOUNT * 10, 3600, true, 10, 10000, 1000000);
        aptos_account::create_account(signer::address_of(admin));
        coin::deposit(signer::address_of(admin), stake::mint_coins(MIN_STAKE));
        setup(admin);
    }
}
