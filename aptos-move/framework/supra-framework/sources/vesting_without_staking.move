/// Copyright (c) Supra -- 2024 - 2025
/// Vesting without staking contract
///
module supra_framework::vesting_without_staking {
    use std::bcs;
    use std::error;
    use std::fixed_point32::{Self, FixedPoint32};
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{utf8, String};
    use std::vector;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_std::math64::min;

    use supra_framework::account::{Self, SignerCapability, new_event_handle};
    use supra_framework::supra_account::{assert_account_is_registered_for_supra};
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::coin::{Self, Coin};
    use supra_framework::event::{Self, EventHandle, emit_event};
    use supra_framework::system_addresses;
    use supra_framework::timestamp;

    friend supra_framework::genesis;

    const VESTING_POOL_SALT: vector<u8> = b"supra_framework::vesting";

    /// Withdrawal address is invalid.
    const EINVALID_WITHDRAWAL_ADDRESS: u64 = 1;
    /// Vesting schedule cannot be empty.
    const EEMPTY_VESTING_SCHEDULE: u64 = 2;
    /// Vesting period cannot be 0.
    const EZERO_VESTING_SCHEDULE_PERIOD: u64 = 3;
    /// Shareholders list cannot be empty.
    const ENO_SHAREHOLDERS: u64 = 4;
    /// The length of shareholders and shares lists don't match.
    const ESHARES_LENGTH_MISMATCH: u64 = 5;
    /// Deprecated.
    ///
    /// Vesting cannot start before or at the current block timestamp. Has to be in the future.
    const EVESTING_START_TOO_SOON: u64 = 6;
    /// The signer is not the admin of the vesting contract.
    const ENOT_ADMIN: u64 = 7;
    /// Vesting contract needs to be in active state.
    const EVESTING_CONTRACT_NOT_ACTIVE: u64 = 8;
    /// Admin can only withdraw from an inactive (paused or terminated) vesting contract.
    const EVESTING_CONTRACT_STILL_ACTIVE: u64 = 9;
    /// No vesting contract found at provided address.
    const EVESTING_CONTRACT_NOT_FOUND: u64 = 10;
    /// Grant amount cannot be 0.
    const EZERO_GRANT: u64 = 12;
    /// Vesting account has no other management roles beside admin.
    const EVESTING_ACCOUNT_HAS_NO_ROLES: u64 = 13;
    /// The vesting account has no such management role.
    const EROLE_NOT_FOUND: u64 = 14;
    /// Account is not admin or does not have the required role to take this action.
    const EPERMISSION_DENIED: u64 = 15;
    /// Zero items were provided to a *_many function.
    const EVEC_EMPTY_FOR_MANY_FUNCTION: u64 = 16;
    /// Balance is the same in the contract and the shareholders' left amount.
    const EBALANCE_MISMATCH: u64 = 17;
    /// Shareholder address is not exist
    const ESHAREHOLDER_NOT_EXIST: u64 = 18;
    /// Invalid vesting schedule parameter
    const EINVALID_VESTING_SCHEDULE: u64 = 19;

    /// Vesting contract states.
    /// Vesting contract is active and distributions can be made.
    const VESTING_POOL_ACTIVE: u64 = 1;
    /// Vesting contract has been terminated and all funds have been released back to the withdrawal address.
    const VESTING_POOL_TERMINATED: u64 = 2;

    /// Roles that can manage certain aspects of the vesting account beyond the main admin.
    const ROLE_BENEFICIARY_RESETTER: vector<u8> = b"ROLE_BENEFICIARY_RESETTER";

    struct VestingSchedule has copy, drop, store {
        // The vesting schedule as a list of fractions that vest for each period. The last number is repeated until the
        // vesting amount runs out.
        // For example [1/24, 1/24, 1/48] with a period of 1 month means that after vesting starts, the first two months
        // will vest 1/24 of the original total amount. From the third month only, 1/48 will vest until the vesting fund
        // runs out.
        // u32/u32 should be sufficient to support vesting schedule fractions.
        schedule: vector<FixedPoint32>,
        // When the vesting should start.
        start_timestamp_secs: u64,
        // In seconds. How long each vesting period is. For example 1 month.
        period_duration: u64,
        // Last vesting period, 1-indexed. For example if 2 months have passed, the last vesting period, if distribution
        // was requested, would be 2. Default value is 0 which means there have been no vesting periods yet.
        last_vested_period: u64
    }

    struct VestingRecord has copy, store, drop {
        init_amount: u64,
        left_amount: u64,
        last_vested_period: u64
    }

    struct VestingContract has key {
        state: u64,
        admin: address,
        beneficiaries: SimpleMap<address, address>,
        shareholders: SimpleMap<address, VestingRecord>,
        vesting_schedule: VestingSchedule,
        // Withdrawal address where all funds would be released back to if the admin ends the vesting for a specific
        // account or terminates the entire vesting contract.
        withdrawal_address: address,
        // Used to control resource.
        signer_cap: SignerCapability,

        // Events.
        set_beneficiary_events: EventHandle<SetBeneficiaryEvent>,
        vest_events: EventHandle<VestEvent>,
        terminate_events: EventHandle<TerminateEvent>,
        admin_withdraw_events: EventHandle<AdminWithdrawEvent>,
        shareholder_removed_events: EventHandle<ShareHolderRemovedEvent>
    }

    struct VestingAccountManagement has key {
        roles: SimpleMap<String, address>
    }

    struct AdminStore has key {
        vesting_contracts: vector<address>,
        // Used to create resource accounts for new vesting contracts so there's no address collision.
        nonce: u64,
        create_events: EventHandle<CreateVestingContractEvent>
    }

    struct CreateVestingContractEvent has drop, store {
        grant_amount: u64,
        withdrawal_address: address,
        vesting_contract_address: address
    }

    struct SetBeneficiaryEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        shareholder: address,
        old_beneficiary: address,
        new_beneficiary: address
    }

    struct VestEvent has drop, store {
        admin: address,
        shareholder_address: address,
        vesting_contract_address: address,
        period_vested: u64
    }

    struct TerminateEvent has drop, store {
        admin: address,
        vesting_contract_address: address
    }

    struct AdminWithdrawEvent has drop, store {
        admin: address,
        vesting_contract_address: address,
        amount: u64
    }

    struct ShareHolderRemovedEvent has drop, store {
        shareholder: address,
        beneficiary: Option<address>,
        amount: u64
    }

    #[event]
    struct VestingScheduleUpdated has drop, store {
        contract_address: address,
        old_schedule: VestingSchedule,
        new_schedule: VestingSchedule
    }

    #[view]
    /// Return the vesting start timestamp (in seconds) of the vesting contract.
    /// Vesting will start at this time, and once a full period has passed, the first vest will become unlocked.
    ///
    /// This errors out if the vesting contract with the provided address doesn't exist.
    public fun vesting_start_secs(vesting_contract_address: address): u64 acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        borrow_global<VestingContract>(vesting_contract_address).vesting_schedule.start_timestamp_secs
    }

    #[view]
    /// Return the duration of one vesting period (in seconds).
    /// Each vest is released after one full period has started, starting from the specified start_timestamp_secs.
    ///
    /// This errors out if the vesting contract with the provided address doesn't exist.
    public fun period_duration_secs(vesting_contract_address: address): u64 acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        borrow_global<VestingContract>(vesting_contract_address).vesting_schedule.period_duration
    }

    #[view]
    // Return the `withdrawal_address` of the contract
    public fun get_withdrawal_addr(vesting_contract_addr: address): address acquires VestingContract {
        borrow_global<VestingContract>(vesting_contract_addr).withdrawal_address
    }

    #[view]
    // Return the `admin` address of the contract
    public fun get_contract_admin(vesting_contract_addr: address): address acquires VestingContract {
        borrow_global<VestingContract>(vesting_contract_addr).admin
    }

    #[view]
    //Return the vesting record of the shareholder as a tuple `(init_amount, left_amount, last_vested_period)`
    public fun get_vesting_record(
        vesting_contract_address: address, shareholder_address: address
    ): (u64, u64, u64) acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        let vesting_record =
            simple_map::borrow(
                &borrow_global<VestingContract>(vesting_contract_address).shareholders,
                &shareholder_address,
            );
        (
            vesting_record.init_amount,
            vesting_record.left_amount,
            vesting_record.last_vested_period
        )
    }

    #[view]
    /// Return the remaining grant of shareholder
    public fun remaining_grant(
        vesting_contract_address: address, shareholder_address: address
    ): u64 acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        simple_map::borrow(
            &borrow_global<VestingContract>(vesting_contract_address).shareholders,
            &shareholder_address,
        ).left_amount
    }

    #[view]
    /// Return the beneficiary account of the specified shareholder in a vesting contract.
    /// This is the same as the shareholder address by default and only different if it's been explicitly set.
    ///
    /// This errors out if the vesting contract with the provided address doesn't exist.
    public fun beneficiary(
        vesting_contract_address: address, shareholder: address
    ): address acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        get_beneficiary(
            borrow_global<VestingContract>(vesting_contract_address),
            shareholder,
        )
    }

    #[view]
    /// Return all the vesting contracts a given address is an admin of.
    public fun vesting_contracts(admin: address): vector<address> acquires AdminStore {
        if (!exists<AdminStore>(admin)) {
            vector::empty<address>()
        } else {
            borrow_global<AdminStore>(admin).vesting_contracts
        }
    }

    #[view]
    /// Return the vesting contract's vesting schedule. The core schedule is represented as a list of u64-based
    /// fractions, where the rightmmost 32 bits can be divided by 2^32 to get the fraction, and anything else is the
    /// whole number.
    ///
    /// For example 3/48, or 0.0625, will be represented as 268435456. The fractional portion would be
    /// 268435456 / 2^32 = 0.0625. Since there are fewer than 32 bits, the whole number portion is effectively 0.
    /// So 268435456 = 0.0625.
    ///
    /// This errors out if the vesting contract with the provided address doesn't exist.
    public fun vesting_schedule(vesting_contract_address: address): VestingSchedule acquires VestingContract {
        assert_vesting_contract_exists(vesting_contract_address);
        borrow_global<VestingContract>(vesting_contract_address).vesting_schedule
    }

    #[view]
    /// Return the list of all shareholders in the vesting contract.
    public fun shareholders(vesting_contract_address: address): vector<address> acquires VestingContract {
        assert_active_vesting_contract(vesting_contract_address);

        let vesting_contract = borrow_global<VestingContract>(vesting_contract_address);
        let shareholders_address = simple_map::keys(&vesting_contract.shareholders);
        shareholders_address
    }

    #[view]
    /// Return the shareholder address given the beneficiary address in a given vesting contract. If there are multiple
    /// shareholders with the same beneficiary address, only the first shareholder is returned. If the given beneficiary
    /// address is actually a shareholder address, just return the address back.
    ///
    /// This returns 0x0 if no shareholder is found for the given beneficiary / the address is not a shareholder itself.
    public fun shareholder(
        vesting_contract_address: address, shareholder_or_beneficiary: address
    ): address acquires VestingContract {
        assert_active_vesting_contract(vesting_contract_address);

        let shareholders = &shareholders(vesting_contract_address);
        if (vector::contains(shareholders, &shareholder_or_beneficiary)) {
            return shareholder_or_beneficiary
        };
        let vesting_contract = borrow_global<VestingContract>(vesting_contract_address);
        let result = @0x0;
        let (sh_vec, ben_vec) = simple_map::to_vec_pair(vesting_contract.beneficiaries);
        let (found, found_index) = vector::index_of(
            &ben_vec, &shareholder_or_beneficiary
        );
        if (found) {
            result = *vector::borrow(&sh_vec, found_index);
        };
        result
    }

    /// Create a vesting schedule with the given schedule of distributions, a vesting start time and period duration.
    public fun create_vesting_schedule(
        schedule: vector<FixedPoint32>, start_timestamp_secs: u64, period_duration: u64
    ): VestingSchedule {
        let schedule_len = vector::length(&schedule);
        assert!(schedule_len != 0, error::invalid_argument(EEMPTY_VESTING_SCHEDULE));
        // If the first vesting fraction is zero, we can replace it with nonzero by increasing start time
        assert!(
            fixed_point32::get_raw_value(*vector::borrow(&schedule, 0)) != 0,
            error::invalid_argument(EEMPTY_VESTING_SCHEDULE),
        );
        // last vesting fraction must be non zero to ensure that no amount remains unvested forever.
        assert!(
            fixed_point32::get_raw_value(*vector::borrow(&schedule, schedule_len - 1)) !=
            0,
            error::invalid_argument(EEMPTY_VESTING_SCHEDULE),
        );
        assert!(
            period_duration != 0, error::invalid_argument(EZERO_VESTING_SCHEDULE_PERIOD)
        );
        VestingSchedule {
            schedule,
            start_timestamp_secs,
            period_duration,
            last_vested_period: 0
        }
    }

    fun validate_vesting_contract_parameters(
        shareholders: &vector<address>,
        shares: &vector<u64>,
        vesting_numerators: &vector<u64>,
        vesting_denominator: u64,
        period_duration: u64,
        withdrawal_address: address
    ) {
        validate_vesting_schedule(
            vesting_numerators, vesting_denominator, period_duration
        );

        assert!(
            !system_addresses::is_reserved_address(withdrawal_address),
            error::invalid_argument(EINVALID_WITHDRAWAL_ADDRESS),
        );
        assert_account_is_registered_for_supra(withdrawal_address);

        assert!(
            vector::length(shareholders) != 0,
            error::invalid_argument(ENO_SHAREHOLDERS),
        );
        assert!(
            vector::length(shareholders) == vector::length(shares),
            error::invalid_argument(ESHARES_LENGTH_MISMATCH),
        );

    }

    fun validate_vesting_schedule(
        numerators: &vector<u64>, denominator: u64, period_duration: u64
    ) {
        let sum = vector::fold(
            *numerators,
            0,
            |acc, numerator| { acc + numerator },
        );
        assert!(
            sum != 0,
            error::invalid_argument(EINVALID_VESTING_SCHEDULE),
        );
        assert!(
            sum <= denominator,
            error::invalid_argument(EINVALID_VESTING_SCHEDULE),
        );
        assert!(
            denominator != 0,
            error::invalid_argument(EINVALID_VESTING_SCHEDULE),
        );
        assert!(
            period_duration != 0,
            error::invalid_argument(EZERO_VESTING_SCHEDULE_PERIOD),
        );
        assert!(
            !vector::is_empty(numerators),
            error::invalid_argument(EEMPTY_VESTING_SCHEDULE),
        );
        assert!(
            *vector::borrow(numerators, 0) != 0,
            error::invalid_argument(EINVALID_VESTING_SCHEDULE),
        );
        assert!(
            *vector::borrow(numerators, vector::length(numerators) - 1) != 0,
            error::invalid_argument(EINVALID_VESTING_SCHEDULE),
        );

    }

    public entry fun create_vesting_contract_with_amounts(
        admin: &signer,
        shareholders: vector<address>,
        shares: vector<u64>,
        vesting_numerators: vector<u64>,
        vesting_denominator: u64,
        start_timestamp_secs: u64,
        period_duration: u64,
        withdrawal_address: address,
        contract_creation_seed: vector<u8>
    ) acquires AdminStore {

        validate_vesting_contract_parameters(
            &shareholders,
            &shares,
            &vesting_numerators,
            vesting_denominator,
            period_duration,
            withdrawal_address,
        );
        // If this is the first time this admin account has created a vesting contract, initialize the admin store.
        let admin_address = signer::address_of(admin);
        if (!exists<AdminStore>(admin_address)) {
            move_to(
                admin,
                AdminStore {
                    vesting_contracts: vector::empty<address>(),
                    nonce: 0,
                    create_events: new_event_handle<CreateVestingContractEvent>(admin)
                },
            );
        };

        // Initialize the vesting contract in a new resource account. This allows the same admin to create multiple
        // pools.
        let (contract_signer, contract_signer_cap) =
            create_vesting_contract_account(admin, contract_creation_seed);
        let contract_signer_address = signer::address_of(&contract_signer);
        let schedule = vector::map_ref(
            &vesting_numerators,
            |numerator| {
                let event =
                    fixed_point32::create_from_rational(*numerator, vesting_denominator);
                event
            },
        );

        let vesting_schedule =
            create_vesting_schedule(schedule, start_timestamp_secs, period_duration);
        let shareholders_map = simple_map::create<address, VestingRecord>();
        let grant_amount = 0;
        vector::for_each_reverse(
            shares,
            |amount| {
                let shareholder = vector::pop_back(&mut shareholders);
                simple_map::add(
                    &mut shareholders_map,
                    shareholder,
                    VestingRecord {
                        init_amount: amount,
                        left_amount: amount,
                        last_vested_period: vesting_schedule.last_vested_period
                    },
                );
                grant_amount = grant_amount + amount;
            },
        );
        assert!(grant_amount != 0, error::invalid_argument(EZERO_GRANT));
        coin::transfer<SupraCoin>(admin, contract_signer_address, grant_amount);

        let admin_store = borrow_global_mut<AdminStore>(admin_address);
        vector::push_back(&mut admin_store.vesting_contracts, contract_signer_address);
        emit_event(
            &mut admin_store.create_events,
            CreateVestingContractEvent {
                withdrawal_address,
                grant_amount,
                vesting_contract_address: contract_signer_address
            },
        );

        move_to(
            &contract_signer,
            VestingContract {
                state: VESTING_POOL_ACTIVE,
                admin: admin_address,
                shareholders: shareholders_map,
                beneficiaries: simple_map::create<address, address>(),
                vesting_schedule,
                withdrawal_address,
                signer_cap: contract_signer_cap,
                set_beneficiary_events: new_event_handle<SetBeneficiaryEvent>(
                    &contract_signer
                ),
                vest_events: new_event_handle<VestEvent>(&contract_signer),
                terminate_events: new_event_handle<TerminateEvent>(&contract_signer),
                admin_withdraw_events: new_event_handle<AdminWithdrawEvent>(
                    &contract_signer
                ),
                shareholder_removed_events: new_event_handle<ShareHolderRemovedEvent>(
                    &contract_signer
                )
            },
        );
    }

    /// Create a vesting contract with a given configurations.
    public fun create_vesting_contract(
        admin: &signer,
        buy_ins: SimpleMap<address, Coin<SupraCoin>>,
        vesting_schedule: VestingSchedule,
        withdrawal_address: address,
        contract_creation_seed: vector<u8>
    ): address acquires AdminStore {
        assert!(
            !system_addresses::is_reserved_address(withdrawal_address),
            error::invalid_argument(EINVALID_WITHDRAWAL_ADDRESS),
        );
        assert_account_is_registered_for_supra(withdrawal_address);
        let shareholders_address = &simple_map::keys(&buy_ins);
        assert!(
            vector::length(shareholders_address) != 0,
            error::invalid_argument(ENO_SHAREHOLDERS),
        );

        let shareholders = simple_map::create<address, VestingRecord>();
        let grant = coin::zero<SupraCoin>();
        let grant_amount = 0;
        let (shareholders_address, buy_ins) = simple_map::to_vec_pair(buy_ins);
        while (vector::length(&shareholders_address) != 0) {
            let shareholder = vector::pop_back(&mut shareholders_address);
            let buy_in = vector::pop_back(&mut buy_ins);
            let init = coin::value(&buy_in);
            coin::merge(&mut grant, buy_in);
            simple_map::add(
                &mut shareholders,
                shareholder,
                VestingRecord {
                    init_amount: init,
                    left_amount: init,
                    last_vested_period: vesting_schedule.last_vested_period
                },
            );
            grant_amount = grant_amount + init;
        };
        assert!(grant_amount != 0, error::invalid_argument(EZERO_GRANT));

        // If this is the first time this admin account has created a vesting contract, initialize the admin store.
        let admin_address = signer::address_of(admin);
        if (!exists<AdminStore>(admin_address)) {
            move_to(
                admin,
                AdminStore {
                    vesting_contracts: vector::empty<address>(),
                    nonce: 0,
                    create_events: new_event_handle<CreateVestingContractEvent>(admin)
                },
            );
        };

        // Initialize the vesting contract in a new resource account. This allows the same admin to create multiple
        // pools.
        let (contract_signer, contract_signer_cap) =
            create_vesting_contract_account(admin, contract_creation_seed);
        let contract_signer_address = signer::address_of(&contract_signer);
        coin::deposit(contract_signer_address, grant);

        let admin_store = borrow_global_mut<AdminStore>(admin_address);
        vector::push_back(&mut admin_store.vesting_contracts, contract_signer_address);
        emit_event(
            &mut admin_store.create_events,
            CreateVestingContractEvent {
                withdrawal_address,
                grant_amount,
                vesting_contract_address: contract_signer_address
            },
        );

        move_to(
            &contract_signer,
            VestingContract {
                state: VESTING_POOL_ACTIVE,
                admin: admin_address,
                shareholders,
                beneficiaries: simple_map::create<address, address>(),
                vesting_schedule,
                withdrawal_address,
                signer_cap: contract_signer_cap,
                set_beneficiary_events: new_event_handle<SetBeneficiaryEvent>(
                    &contract_signer
                ),
                vest_events: new_event_handle<VestEvent>(&contract_signer),
                terminate_events: new_event_handle<TerminateEvent>(&contract_signer),
                admin_withdraw_events: new_event_handle<AdminWithdrawEvent>(
                    &contract_signer
                ),
                shareholder_removed_events: new_event_handle<ShareHolderRemovedEvent>(
                    &contract_signer
                )
            },
        );

        vector::destroy_empty(buy_ins);
        contract_signer_address
    }

    /// Unlock any vested portion of the grant.
    public entry fun vest(contract_address: address) acquires VestingContract {
        assert_active_vesting_contract(contract_address);
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        // Short-circuit if vesting hasn't started yet.
        if (vesting_contract.vesting_schedule.start_timestamp_secs > timestamp::now_seconds()) {
            return
        };

        let shareholders = simple_map::keys(&vesting_contract.shareholders);
        while (vector::length(&shareholders) != 0) {
            let shareholder = vector::pop_back(&mut shareholders);
            vest_individual(contract_address, shareholder);
        };
        let total_balance = coin::balance<SupraCoin>(contract_address);
        if (total_balance == 0) {
            set_terminate_vesting_contract(contract_address);
        };
    }

    public entry fun vest_individual(
        contract_address: address, shareholder_address: address
    ) acquires VestingContract {
        //check if contract exist, active and shareholder is a member of the contract
        assert_shareholder_exists(contract_address, shareholder_address);

        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        let beneficiary = get_beneficiary(vesting_contract, shareholder_address);
        // Short-circuit if vesting hasn't started yet.
        if (vesting_contract.vesting_schedule.start_timestamp_secs > timestamp::now_seconds()) {
            return
        };

        let vesting_record =
            simple_map::borrow_mut(
                &mut vesting_contract.shareholders, &shareholder_address
            );
        let signer_cap = &vesting_contract.signer_cap;

        // Check if the next vested period has already passed. If not, short-circuit since there's nothing to vest.
        let vesting_schedule = vesting_contract.vesting_schedule;
        let schedule = &vesting_schedule.schedule;
        let last_vested_period = vesting_record.last_vested_period;
        let next_period_to_vest = last_vested_period + 1;
        let last_completed_period =
            (timestamp::now_seconds() - vesting_schedule.start_timestamp_secs) / vesting_schedule
                .period_duration;

        // Index is 0-based while period is 1-based so we need to subtract 1.
        let one = fixed_point32::create_from_rational(1, 1);
        let total_vesting_fraction = fixed_point32::create_from_rational(0, 1);
        while (last_completed_period >= next_period_to_vest
            && fixed_point32::less(total_vesting_fraction, one)
            && next_period_to_vest <= vector::length(schedule)) {
            let schedule_index = next_period_to_vest - 1;
            let vesting_fraction = *vector::borrow(schedule, schedule_index);
            total_vesting_fraction = fixed_point32::add(
                total_vesting_fraction,
                vesting_fraction,
            );
            next_period_to_vest = next_period_to_vest + 1;
        };

        let periods_fast_forward = 0;

        if (last_completed_period >= next_period_to_vest
            && vesting_record.left_amount != 0
            && fixed_point32::less(total_vesting_fraction, one)) {
            let final_fraction = *vector::borrow(schedule, vector::length(schedule) - 1);
            // Determine how many periods is needed based on the left_amount
            periods_fast_forward = last_completed_period - next_period_to_vest + 1;
            let added_fraction = fixed_point32::multiply_u64_return_fixpoint32(
                periods_fast_forward, final_fraction
            );
            // If the added_fraction is greater than or equal to the left_amount, then we can vest all the left_amount
            total_vesting_fraction = fixed_point32::add(
                total_vesting_fraction, added_fraction
            );
        };

        // Make sure the total vesting fraction is not greater than 1.
        total_vesting_fraction = fixed_point32::min(total_vesting_fraction, one);
        // We don't need to check vesting_record.left_amount > 0 because vest_transfer will handle that.
        let transfer_happened = vest_transfer(
            vesting_record,
            signer_cap,
            beneficiary,
            total_vesting_fraction,
        );
        //If no amount was transferred DO NOT advance last_vested_period in the vesting record
        // This check is needed because if the fraction is too low, `vesting_record.init_amount * vesting_fraction`
        // may be 0. By not advancing, we allow for the possibility for `vesting_fraction` to become large enough
        // otherwise, even if vesting period passes and shareholder regularly calls `vest_individual`, the shareholder
        // may never receive any amount.
        if (!transfer_happened) { return };
        next_period_to_vest = next_period_to_vest + periods_fast_forward;
        emit_event(
            &mut vesting_contract.vest_events,
            VestEvent {
                admin: vesting_contract.admin,
                shareholder_address,
                vesting_contract_address: contract_address,
                period_vested: next_period_to_vest - 1
            },
        );

        //update last_vested_period for the shareholder
        vesting_record.last_vested_period = next_period_to_vest - 1;
    }

    // Transfers from the contract to beneficiary `vesting_fraction` of `vesting_record.init_amount`
    // It returns whether any amount was transferred or not.
    fun vest_transfer(
        vesting_record: &mut VestingRecord,
        signer_cap: &SignerCapability,
        beneficiary: address,
        vesting_fraction: FixedPoint32
    ): bool {
        let vesting_signer = account::create_signer_with_capability(signer_cap);

        //amount to be transfer is minimum of what is left and vesting fraction due of init_amount
        let amount =
            min(
                vesting_record.left_amount,
                fixed_point32::multiply_u64(vesting_record.init_amount, vesting_fraction),
            );
        if (amount > 0) {
            //update left_amount for the shareholder
            vesting_record.left_amount = vesting_record.left_amount - amount;
            coin::transfer<SupraCoin>(&vesting_signer, beneficiary, amount);
            true
        } else { false }
    }

    public entry fun set_vesting_schedule(
        admin: &signer,
        contract_address: address,
        vesting_numerators: vector<u64>,
        vesting_denominator: u64,
        period_duration: u64
    ) acquires VestingContract {
        validate_vesting_schedule(
            &vesting_numerators, vesting_denominator, period_duration
        );
        assert_vesting_contract_exists(contract_address);
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);

        let schedule = vector::map_ref(
            &vesting_numerators,
            |numerator| {
                fixed_point32::create_from_rational(*numerator, vesting_denominator)
            },
        );

        let old_period_duration = vesting_contract.vesting_schedule.period_duration;

        let (keys, values) = simple_map::to_vec_pair(vesting_contract.shareholders);
        vector::zip_mut<address, VestingRecord>(
            &mut keys,
            &mut values,
            |shareholder, srecord| {
                let msrecord: &mut VestingRecord = srecord;
                let new_last_vested_period = (
                    msrecord.last_vested_period * old_period_duration
                ) / period_duration;
                msrecord.last_vested_period = new_last_vested_period;
            },
        );
        vesting_contract.shareholders = simple_map::new_from(keys, values);

        let old_schedule = vesting_contract.vesting_schedule;
        vesting_contract.vesting_schedule.schedule = schedule;
        vesting_contract.vesting_schedule.period_duration = period_duration;

        event::emit(
            VestingScheduleUpdated {
                contract_address,
                old_schedule: old_schedule,
                new_schedule: vesting_contract.vesting_schedule
            },
        );

    }

    /// Remove the lockup period for the vesting contract. This can only be called by the admin of the vesting contract.
    /// Example usage: If admin find shareholder suspicious, admin can remove it.
    public entry fun remove_shareholder(
        admin: &signer, contract_address: address, shareholder_address: address
    ) acquires VestingContract {
        assert_shareholder_exists(contract_address, shareholder_address);
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);
        let vesting_signer = get_vesting_account_signer_internal(vesting_contract);
        let shareholder_amount =
            simple_map::borrow(&vesting_contract.shareholders, &shareholder_address).left_amount;
        coin::transfer<SupraCoin>(
            &vesting_signer, vesting_contract.withdrawal_address, shareholder_amount
        );
        emit_event(
            &mut vesting_contract.admin_withdraw_events,
            AdminWithdrawEvent {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount: shareholder_amount
            },
        );

        // remove `shareholder_address`` from `vesting_contract.shareholders`
        let shareholders = &mut vesting_contract.shareholders;
        let (_, shareholders_vesting) =
            simple_map::remove(shareholders, &shareholder_address);

        // remove `shareholder_address` from `vesting_contract.beneficiaries`
        let beneficiary = option::none();
        let shareholder_beneficiaries = &mut vesting_contract.beneficiaries;
        // Not all shareholders have their beneficiaries, so before removing them, we need to check if the beneficiary exists
        if (simple_map::contains_key(shareholder_beneficiaries, &shareholder_address)) {
            let (_, shareholder_baneficiary) =
                simple_map::remove(shareholder_beneficiaries, &shareholder_address);
            beneficiary = option::some(shareholder_baneficiary);
        };

        // Emit ShareHolderRemovedEvent
        emit_event(
            &mut vesting_contract.shareholder_removed_events,
            ShareHolderRemovedEvent {
                shareholder: shareholder_address,
                beneficiary,
                amount: shareholders_vesting.left_amount
            },
        );
    }

    /// Terminate the vesting contract and send all funds back to the withdrawal address.
    public entry fun terminate_vesting_contract(
        admin: &signer, contract_address: address
    ) acquires VestingContract {
        assert_active_vesting_contract(contract_address);

        vest(contract_address);

        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);

        // Distribute remaining coins to withdrawal address of vesting contract.
        let shareholders_address = simple_map::keys(&vesting_contract.shareholders);
        vector::for_each_ref(
            &shareholders_address,
            |shareholder| {
                let shareholder_amount =
                    simple_map::borrow_mut(
                        &mut vesting_contract.shareholders, shareholder
                    );
                shareholder_amount.left_amount = 0;
            },
        );
        set_terminate_vesting_contract(contract_address);
    }

    /// Withdraw all funds to the preset vesting contract's withdrawal address. This can only be called if the contract
    /// has already been terminated.
    public entry fun admin_withdraw(
        admin: &signer, contract_address: address
    ) acquires VestingContract {
        let vesting_contract = borrow_global<VestingContract>(contract_address);
        assert!(
            vesting_contract.state == VESTING_POOL_TERMINATED,
            error::invalid_state(EVESTING_CONTRACT_STILL_ACTIVE),
        );

        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);
        let total_balance = coin::balance<SupraCoin>(contract_address);
        let vesting_signer = get_vesting_account_signer_internal(vesting_contract);
        coin::transfer<SupraCoin>(
            &vesting_signer, vesting_contract.withdrawal_address, total_balance
        );

        emit_event(
            &mut vesting_contract.admin_withdraw_events,
            AdminWithdrawEvent {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                amount: total_balance
            },
        );
    }

    public entry fun set_beneficiary(
        admin: &signer,
        contract_address: address,
        shareholder: address,
        new_beneficiary: address
    ) acquires VestingContract {
        // Verify that the beneficiary account is set up to receive SUPRA. This is a requirement so distribute() wouldn't
        // fail and block all other accounts from receiving SUPRA if one beneficiary is not registered.
        assert_account_is_registered_for_supra(new_beneficiary);

        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);

        let old_beneficiary = get_beneficiary(vesting_contract, shareholder);
        let beneficiaries = &mut vesting_contract.beneficiaries;
        simple_map::upsert(beneficiaries, shareholder, new_beneficiary);

        emit_event(
            &mut vesting_contract.set_beneficiary_events,
            SetBeneficiaryEvent {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address,
                shareholder,
                old_beneficiary,
                new_beneficiary
            },
        );
    }

    /// Remove the beneficiary for the given shareholder. All distributions will sent directly to the shareholder
    /// account.
    public entry fun reset_beneficiary(
        account: &signer, contract_address: address, shareholder: address
    ) acquires VestingAccountManagement, VestingContract {
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        let addr = signer::address_of(account);
        assert!(
            addr == vesting_contract.admin
            || addr
            == get_role_holder(contract_address, utf8(ROLE_BENEFICIARY_RESETTER)),
            error::permission_denied(EPERMISSION_DENIED),
        );

        let beneficiaries = &mut vesting_contract.beneficiaries;
        if (simple_map::contains_key(beneficiaries, &shareholder)) {
            simple_map::remove(beneficiaries, &shareholder);
        };
    }

    public entry fun set_management_role(
        admin: &signer,
        contract_address: address,
        role: String,
        role_holder: address
    ) acquires VestingAccountManagement, VestingContract {
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);

        if (!exists<VestingAccountManagement>(contract_address)) {
            let contract_signer = &get_vesting_account_signer_internal(vesting_contract);
            move_to(
                contract_signer,
                VestingAccountManagement { roles: simple_map::create<String, address>() },
            )
        };
        let roles =
            &mut borrow_global_mut<VestingAccountManagement>(contract_address).roles;
        simple_map::upsert(roles, role, role_holder);
    }

    public entry fun set_beneficiary_resetter(
        admin: &signer, contract_address: address, beneficiary_resetter: address
    ) acquires VestingAccountManagement, VestingContract {
        set_management_role(
            admin,
            contract_address,
            utf8(ROLE_BENEFICIARY_RESETTER),
            beneficiary_resetter,
        );
    }

    public fun get_role_holder(contract_address: address, role: String): address acquires VestingAccountManagement {
        assert!(
            exists<VestingAccountManagement>(contract_address),
            error::not_found(EVESTING_ACCOUNT_HAS_NO_ROLES),
        );
        let roles = &borrow_global<VestingAccountManagement>(contract_address).roles;
        assert!(
            simple_map::contains_key(roles, &role),
            error::not_found(EROLE_NOT_FOUND),
        );
        *simple_map::borrow(roles, &role)
    }

    /// For emergency use in case the admin needs emergency control of vesting contract account.
    public fun get_vesting_account_signer(
        admin: &signer, contract_address: address
    ): signer acquires VestingContract {
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        verify_admin(admin, vesting_contract);
        get_vesting_account_signer_internal(vesting_contract)
    }

    fun get_vesting_account_signer_internal(
        vesting_contract: &VestingContract
    ): signer {
        account::create_signer_with_capability(&vesting_contract.signer_cap)
    }

    /// Create a salt for generating the resource accounts that will be holding the VestingContract.
    /// This address should be deterministic for the same admin and vesting contract creation nonce.
    fun create_vesting_contract_account(
        admin: &signer, contract_creation_seed: vector<u8>
    ): (signer, SignerCapability) acquires AdminStore {
        let admin_store = borrow_global_mut<AdminStore>(signer::address_of(admin));
        let seed = bcs::to_bytes(&signer::address_of(admin));
        vector::append(&mut seed, bcs::to_bytes(&admin_store.nonce));
        admin_store.nonce = admin_store.nonce + 1;

        // Include a salt to avoid conflicts with any other modules out there that might also generate
        // deterministic resource accounts for the same admin address + nonce.
        vector::append(&mut seed, VESTING_POOL_SALT);
        vector::append(&mut seed, contract_creation_seed);

        let (account_signer, signer_cap) = account::create_resource_account(admin, seed);
        // Register the vesting contract account to receive SUPRA
        coin::register<SupraCoin>(&account_signer);

        (account_signer, signer_cap)
    }

    fun verify_admin(admin: &signer, vesting_contract: &VestingContract) {
        assert!(
            signer::address_of(admin) == vesting_contract.admin,
            error::unauthenticated(ENOT_ADMIN),
        );
    }

    fun assert_vesting_contract_exists(contract_address: address) {
        assert!(
            exists<VestingContract>(contract_address),
            error::not_found(EVESTING_CONTRACT_NOT_FOUND),
        );
    }

    fun assert_shareholder_exists(
        contract_address: address, shareholder_address: address
    ) acquires VestingContract {
        assert_active_vesting_contract(contract_address);
        assert!(
            simple_map::contains_key(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_address,
            ),
            error::not_found(ESHAREHOLDER_NOT_EXIST),
        );
    }

    fun assert_active_vesting_contract(contract_address: address) acquires VestingContract {
        assert_vesting_contract_exists(contract_address);
        let vesting_contract = borrow_global<VestingContract>(contract_address);
        assert!(
            vesting_contract.state == VESTING_POOL_ACTIVE,
            error::invalid_state(EVESTING_CONTRACT_NOT_ACTIVE),
        );
    }

    fun get_beneficiary(contract: &VestingContract, shareholder: address): address {
        if (simple_map::contains_key(&contract.beneficiaries, &shareholder)) {
            *simple_map::borrow(&contract.beneficiaries, &shareholder)
        } else { shareholder }
    }

    fun set_terminate_vesting_contract(contract_address: address) acquires VestingContract {
        let vesting_contract = borrow_global_mut<VestingContract>(contract_address);
        vesting_contract.state = VESTING_POOL_TERMINATED;
        emit_event(
            &mut vesting_contract.terminate_events,
            TerminateEvent {
                admin: vesting_contract.admin,
                vesting_contract_address: contract_address
            },
        );
    }

    #[test_only]
    use supra_framework::stake;

    #[test_only]
    use supra_framework::account::create_account_for_test;

    #[test_only]
    const GRANT_AMOUNT: u64 = 1000; // 1000 SUPRA coins with 8 decimals.

    #[test_only]
    const VESTING_SCHEDULE_CLIFF: u64 = 31536000; // 1 year

    #[test_only]
    const VESTING_PERIOD: u64 = 2592000; // 30 days

    #[test_only]
    public entry fun setup(
        supra_framework: &signer, accounts: vector<address>
    ) {
        use supra_framework::supra_account::create_account;
        timestamp::set_time_has_started_for_testing(supra_framework);
        stake::initialize_for_test(supra_framework);
        vector::for_each_ref(
            &accounts,
            |addr| {
                let addr: address = *addr;
                if (!account::exists_at(addr)) {
                    create_account(addr);
                };
            },
        );
    }

    #[test_only]
    public fun setup_vesting_contract(
        admin: &signer,
        shareholders: &vector<address>,
        shares: &vector<u64>,
        withdrawal_address: address
    ): address acquires AdminStore {
        setup_vesting_contract_with_schedule(
            admin,
            shareholders,
            shares,
            withdrawal_address,
            &vector[2, 2, 1],
            10,
        )
    }

    #[test_only]
    public fun setup_vesting_contract_with_schedule(
        admin: &signer,
        shareholders: &vector<address>,
        shares: &vector<u64>,
        withdrawal_address: address,
        vesting_numerators: &vector<u64>,
        vesting_denominator: u64
    ): address acquires AdminStore {
        let schedule = vector::empty<FixedPoint32>();
        vector::for_each_ref(
            vesting_numerators,
            |num| {
                vector::push_back(
                    &mut schedule,
                    fixed_point32::create_from_rational(*num, vesting_denominator),
                );
            },
        );
        let vesting_schedule =
            create_vesting_schedule(
                schedule,
                timestamp::now_seconds() + VESTING_SCHEDULE_CLIFF,
                VESTING_PERIOD,
            );

        let buy_ins = simple_map::create<address, Coin<SupraCoin>>();
        vector::enumerate_ref(
            shares,
            |i, share| {
                let shareholder = *vector::borrow(shareholders, i);
                simple_map::add(&mut buy_ins, shareholder, stake::mint_coins(*share));
            },
        );

        create_vesting_contract(
            admin,
            buy_ins,
            vesting_schedule,
            withdrawal_address,
            vector[],
        )
    }

    #[test_only]
    public fun setup_vesting_contract_with_amount_with_schedule(
        admin: &signer,
        shareholders: vector<address>,
        shares: vector<u64>,
        withdrawal_address: address,
        vesting_numerators: vector<u64>,
        vesting_denominator: u64
    ): address acquires AdminStore {
        create_vesting_contract_with_amounts(
            admin,
            shareholders,
            shares,
            vesting_numerators,
            vesting_denominator,
            timestamp::now_seconds() + VESTING_SCHEDULE_CLIFF,
            VESTING_PERIOD,
            withdrawal_address,
            vector[],
        );
        let admin_store = borrow_global<AdminStore>(signer::address_of(admin));
        let contract_address = vector::borrow(
            &admin_store.vesting_contracts,
            vector::length(&admin_store.vesting_contracts) - 1,
        );
        *contract_address
    }

    #[test_only]
    const ONE_DAY: u64 = 24 * 60 * 60;
    #[test_only]
    const ONE_WEEK: u64 = 7 * 24 * 60 * 60;

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_1y_to_10min(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        // reset schedule to 1% for 1 year
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            1000,
            365 * ONE_DAY,
        );

        // Time is now at the start time, vest will unlock the first period, which is 1/100
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/10 of the total amount modulo rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 7; // 1% of 750 is 7 with rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/100 (2%) vested modulo rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 14; // 2% of 750 is 14 with rounding
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 1 days
        timestamp::fast_forward_seconds(1 * ONE_DAY);
        // From 1year at 1% to 10 min
        set_vesting_schedule(
            admin,
            contract_address,
            vector[1],
            100 * 365 * 24 * 6,
            600,
        );
        // last vested period for shareholder 2 should be number of 10 minute periods in 2 years
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 105120, last_vested_period);

        // no vesting has been called for shareholder 1 yet
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, shareholder 1 will move to minutes equivalent to 631 = (365*2)+1 days,
        //but shareholder 2 vesting will not move because 10 minute periods in 1 day is too small and resultant
        //vesting fraction would be too small to vest even 1 quant
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 105120, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        assert!(shareholder_2_balance == 14, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 105264, last_vested_period);

        //shareholder 1 correctly gets 5 quant for 2 year + 1 day of advancement
        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 5, shareholder_1_balance);

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_1y_to_5min(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        // reset schedule to 1% for 1 year
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            1000,
            365 * ONE_DAY,
        );

        // Time is now at the start time, vest will unlock the first period, which is 1/100
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/100 so approx 7 out of 750 due to rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 7; // 1% of 750 is 7 with rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/100 (2%)  so 14 out of 740 due to rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 14; // 2% of 750 is 14 with rounding
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 1 days
        timestamp::fast_forward_seconds(1 * ONE_DAY);
        // From 1year at 1% to 5 min
        set_vesting_schedule(
            admin,
            contract_address,
            vector[1],
            100 * 365 * 24 * 12,
            300,
        );
        // last vested period for shareholder 2 should be number of 5 min periods in 2 years
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 210240, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, both shareholders should vest
        vest(contract_address);

        // 1 day period is too small to vest anything so last_vested_period and balance will not mvoe
        // for shareholder 2
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 210240, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        assert!(shareholder_2_balance == 14, shareholder_2_balance);

        //shareholder 1 last_vested_amount will move as per 5 min period in 2year + 1 day
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 210528, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        //The balance, instead of being 5, will be 4 because of loss of precision since percentage
        // is too small when vesting happens at such high frequency
        assert!(shareholder_1_balance == 4, shareholder_1_balance);

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_1y_to_1min(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        // reset schedule to 1% for 1 year
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            1000,
            365 * ONE_DAY,
        );

        // Time is now at the start time, vest will unlock the first period, which is 1/100
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/100 of 750, so 7 with rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 7; // 1% of 750 is 7 with rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/100  of 750 so 14 with rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 14; // 2% of 750 is 14 with rounding
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 1 days
        timestamp::fast_forward_seconds(1 * ONE_DAY);
        // From 1year at 1% to 1 min
        set_vesting_schedule(
            admin,
            contract_address,
            vector[1],
            100 * 365 * 24 * 60,
            60,
        );
        // last vested period for shareholder 2 should be number of minutes in 2 years
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1051200, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        vest(contract_address);

        //for shareholder 2, 1 day period is too small to vest anything so last_vested_period and balance will not mvoe
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1051200, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        assert!(shareholder_2_balance == 14, shareholder_2_balance);

        //for shareholder 1, last_vested_period will be numbbeer of minutes in 2 year + 1 day
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 1052640, last_vested_period);

        // precision is too low so the balance will be 4 instead of 5
        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 4, shareholder_1_balance);

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_1y_to_1sec(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        // reset schedule to 1% for 1 year
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            1000,
            365 * ONE_DAY,
        );

        // Time is now at the start time, vest will unlock the first period, which is 1/100
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/100 of 750, so 7 with rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 7; // 1% of 750 is 7 with rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/100  of 750 so 14 with rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = 14; // 2% of 750 is 14 with rounding
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 1 days
        timestamp::fast_forward_seconds(1 * ONE_DAY);
        // From 1year at 1% to 1 second
        set_vesting_schedule(
            admin,
            contract_address,
            vector[100],
            365 * ONE_DAY * 10000,
            1,
        );
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 63072000, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);

        // for shareholder 2, last_vested_period will be number of seconds in 2 years
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 63072000, last_vested_period);
        // 1 day period is too small to vest anything so last_vested_period and balance will not mvoe
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        assert!(shareholder_2_balance == 14, shareholder_2_balance);

        //for shareholder 1, last_vested_period will be number of seconds in 2 year + 1 day
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 63158400, last_vested_period);

        //precision is too low, the fraction loses 38-40% of the value
        // so the balance will be 3 instead of 5
        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 3, shareholder_1_balance);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_decreasing_period(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/10 of the total amount modulo rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 10 / 400 - 1; // diff of 1 because of rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/10 (20%) vested modulo rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 20 / 400 - 2;
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 6 days
        timestamp::fast_forward_seconds(6 * ONE_DAY);
        // Note that per month 10% would now translate to 7/300 , approx 2.3333333% per week
        set_vesting_schedule(
            admin,
            contract_address,
            vector[70],
            3000,
            ONE_WEEK,
        );
        // last vested period for shareholder 2 should be 8, while total 65 days have passed but his
        // last vesting corresponds to 60 days, so 8 weeks have passed
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 8, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 9, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 165, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 9, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 52, shareholder_1_balance);

        // Change the schedule to every 3 days, which now becomes 1% per 3 day from original 10% per month
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            1000,
            3 * ONE_DAY,
        );
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 21, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 21, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 22, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 172, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 22, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 54, shareholder_1_balance);

        timestamp::fast_forward_seconds(2 * ONE_DAY);
        // Note that per month 10% would now translate to 7/300 , approx 2.3333333% per week
        // Change the schedule to every 1 days, which now becomes 1/3% per day from original 10% per month
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            3000,
            ONE_DAY,
        );
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 66, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 66, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 68, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 176, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 68, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 55, shareholder_1_balance);

        timestamp::fast_forward_seconds(3600);
        // Note that per month 10% would now translate to 7/300 , approx 2.3333333% per week
        // Change the schedule to every 1 days, which now becomes 1/3% per day from original 10% per month
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            259200000,
            1,
        );
        //last vesting happened at 68 days meaning 5875200 seconds
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 5875200, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 5875200, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);
        // if the vesting fraction is too low so that vesting amount is zero, last_vested_period should not advance
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 5875200, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 176, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 5875200, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 55, shareholder_1_balance);

        timestamp::fast_forward_seconds(2 * ONE_DAY);
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 6051600, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 181, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 6051600, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 56, shareholder_1_balance);

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    #[expected_failure(abort_code = 262151, location = Self)]
    public entry fun test_set_vesting_schedule_update_by_nonadmin_failure(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 1/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/10 of the total amount modulo rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 10 / 400 - 1; // diff of 1 because of rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/10 (20%) vested modulo rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 20 / 400 - 2;
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 42 days, total 102 days from start
        timestamp::fast_forward_seconds(42 * ONE_DAY);
        // Note that per month 10% would now translate to 4/30 , approx 13.3333333% per 40 days
        set_vesting_schedule(
            shareholder_1,
            contract_address,
            vector[40],
            300,
            40 * ONE_DAY,
        );

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_e2e_with_vesting_schedule_update_increasing_period(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        // Contract with monthly vesting 10%
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[10],
                100,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 1/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );

        // After 1st vesting shareholder 2 should have 1/10 of the total amount modulo rounding
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 10 / 400 - 1; // diff of 1 because of rounding
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, 0);
        assert!(last_vested_period == 1, 0);

        // Fast forward another month, shareholder 2 should now have 2/10 (20%) vested modulo rounding
        timestamp::fast_forward_seconds(period_duration_secs(contract_address));
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = GRANT_AMOUNT * 3 * 20 / 400 - 2;
        shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(shareholder_2_balance == vested_amount_2, shareholder_2_balance);
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(init_amount - left_amount == vested_amount_2, left_amount);
        assert!(last_vested_period == 2, 0);

        // Fast forward time to another 42 days, total 102 days from start
        timestamp::fast_forward_seconds(42 * ONE_DAY);
        // Note that per month 10% would now translate to 4/30 , approx 13.3333333% per 40 days
        set_vesting_schedule(
            admin,
            contract_address,
            vector[40],
            300,
            40 * ONE_DAY,
        );
        // last vested period for shareholder 2 should be 1, while total 102 days have passed, from last vesting at 60 day
        // only one 40 day period has passed
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 2* 40 day period  passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 2, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 13.33% (for one 40 day period = 99) but he already got 148 for 2 month 60 days
        // so instead of 199 shareholder will get 247
        assert!(shareholder_2_balance == 247, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 2, last_vested_period);

        // for shareholder 1, 26.66% of 250 would be 66
        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 66, shareholder_1_balance);

        // Change the schedule to every 2 months, which now becomes 20% per 2 month
        set_vesting_schedule(
            admin,
            contract_address,
            vector[20],
            100,
            2 * VESTING_PERIOD,
        );
        //We are still at 102 days so only 1 * 2 month period
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 1, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 102 days (one 60 day period) passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 247, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 1, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 66, shareholder_1_balance);

        // Total at 112 days
        timestamp::fast_forward_seconds(10 * ONE_DAY);
        // 4 month vesting at 40%
        set_vesting_schedule(
            admin,
            contract_address,
            vector[40],
            100,
            4 * VESTING_PERIOD,
        );
        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 0, last_vested_period);

        let (_, _, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        // When you vest now, both shareholder 1 and shareholder 2 should have 9 weeks passed
        vest(contract_address);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 0, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        // shareholder 2 will get around 17.49 Supra per week but he already got 140 for 2 month 60 days
        // so instead of 157 ( 17.49 * 9) he will get 165
        assert!(shareholder_2_balance == 247, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 0, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 66, shareholder_1_balance);

        // At 122 days (one 4 month period)
        timestamp::fast_forward_seconds(10 * ONE_DAY);
        vest(contract_address);
        //last vesting happened at 60 days as per records
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 1, last_vested_period);

        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, shareholder_2_balance);

        // shareholder 2 will get around 299 Supra per 4 month but he already got 247
        // so instead of 300 he will get 546
        assert!(shareholder_2_balance == 546, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 1, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        assert!(shareholder_1_balance == 165, shareholder_1_balance);

        timestamp::fast_forward_seconds(40 * ONE_DAY);
        set_vesting_schedule(
            admin,
            contract_address,
            vector[10],
            100,
            VESTING_PERIOD,
        );
        vest(contract_address);
        // for shareholder 2 scaled last_vested_period would be 4 and upon calling vest would go to 5
        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_2_address);
        assert!(last_vested_period == 5, last_vested_period);
        let shareholder_2_balance = coin::balance<SupraCoin>(shareholder_2_address);
        assert!(init_amount - left_amount == shareholder_2_balance, 0);
        //1 vesting would occur at 10% rate again, so 546 + 74 would be 620
        assert!(shareholder_2_balance == 620, shareholder_2_balance);

        let (init_amount, left_amount, last_vested_period) =
            get_vesting_record(contract_address, shareholder_1_address);
        assert!(last_vested_period == 5, last_vested_period);

        let shareholder_1_balance = coin::balance<SupraCoin>(shareholder_1_address);
        assert!(init_amount - left_amount == shareholder_1_balance, 0);
        //shareholder 1 already had 165 and another 10% of 250 would be 189 due to rounding
        assert!(shareholder_1_balance == 189, shareholder_1_balance);

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    #[expected_failure(abort_code = 0x30008, location = Self)]
    public entry fun test_termination_after_successful_vesting(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = &vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = &vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        let contract_address =
            setup_vesting_contract_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                &vector[1],
                1,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);

        assert!(
            coin::balance<SupraCoin>(shareholder_1_address) == shareholder_1_share,
            0,
        );
        assert!(
            coin::balance<SupraCoin>(shareholder_2_address) == shareholder_2_share,
            0,
        );
        vest(contract_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    #[expected_failure(abort_code = 0x30008, location = Self)]
    public entry fun entry_test_termination_after_successful_vesting(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, GRANT_AMOUNT);
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                shareholders,
                shares,
                withdrawal_address,
                vector[1],
                1,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;
        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);

        assert!(
            coin::balance<SupraCoin>(shareholder_1_address) == shareholder_1_share,
            0,
        );
        assert!(
            coin::balance<SupraCoin>(shareholder_2_address) == shareholder_2_share,
            0,
        );
        vest(contract_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_premature_call(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = &vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = &vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                shareholders,
                shares,
                withdrawal_address,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        // Because the time is behind the start time, vest will do nothing.
        vest_individual(contract_address, shareholder_1_address);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        // Because the time is behind the start time, vest will do nothing.
        vest_individual(contract_address, shareholder_2_address);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_vest_individual(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = &vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = &vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                shareholders,
                shares,
                withdrawal_address,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest_individual(contract_address, shareholder_1_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 2, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_1_address,
            ).left_amount + vested_amount_1 == shareholder_1_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 2, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_2_address,
            ).left_amount + vested_amount_2 == shareholder_2_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 2,
        );
        vest_individual(contract_address, shareholder_1_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 2, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_1_address,
            ).left_amount + vested_amount_1 == shareholder_1_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 2, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_2_address,
            ).left_amount + vested_amount_2 == shareholder_2_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 3,
        );
        vest_individual(contract_address, shareholder_1_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_1_address,
            ).left_amount + vested_amount_1 == shareholder_1_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_2_address,
            ).left_amount + vested_amount_2 == shareholder_2_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_vest_individual_early_termination_success(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = &vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = &vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                shareholders,
                shares,
                withdrawal_address,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 50,
        );
        vest_individual(contract_address, shareholder_1_address);
        vested_amount_1 = vested_amount_1 + GRANT_AMOUNT / 4;
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_1_address,
            ).left_amount + vested_amount_1 == shareholder_1_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        vest_individual(contract_address, shareholder_2_address);
        vested_amount_2 = vested_amount_2 + (GRANT_AMOUNT * 3 / 4);
        assert!(
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_2_address,
            ).left_amount + vested_amount_2 == shareholder_2_share,
            0,
        );
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        let vesting_record_1 =
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_1_address,
            );
        let vesting_record_2 =
            simple_map::borrow(
                &borrow_global<VestingContract>(contract_address).shareholders,
                &shareholder_2_address,
            );
        //Check that loop only as many vesting periods have passed which is required to vest everything
        assert!(
            vesting_record_1.last_vested_period == 50,
            vesting_record_1.last_vested_period,
        );
        assert!(
            vesting_record_2.last_vested_period == 50,
            vesting_record_2.last_vested_period,
        );

    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder_1 = @0x234, shareholder_2 = @0x345, withdrawal = @111)]
    public entry fun test_end_to_end(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);
        let shareholders = &vector[shareholder_1_address, shareholder_2_address];
        let shareholder_1_share = GRANT_AMOUNT / 4;
        let shareholder_2_share = GRANT_AMOUNT * 3 / 4;
        let shares = &vector[shareholder_1_share, shareholder_2_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                shareholders,
                shares,
                withdrawal_address,
            );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount_1 = 0;
        let vested_amount_2 = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 2, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 2, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 2,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 2, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 2, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 3,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 4,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 5,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 6,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 7,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 8,
        );
        vest(contract_address);
        vested_amount_1 = vested_amount_1 + fraction(shareholder_1_share, 1, 10);
        vested_amount_2 = vested_amount_2 + fraction(shareholder_2_share, 1, 10);
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 9,
        );
        vest(contract_address);
        vested_amount_1 = shareholder_1_share;
        vested_amount_2 = shareholder_2_share;
        assert!(coin::balance<SupraCoin>(shareholder_1_address) == vested_amount_1, 0);
        assert!(coin::balance<SupraCoin>(shareholder_2_address) == vested_amount_2, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x1000C, location = Self)]
    public entry fun test_create_vesting_contract_with_zero_grant_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        setup_vesting_contract(admin, &vector[@1], &vector[0], admin_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x10004, location = Self)]
    public entry fun test_create_vesting_contract_with_no_shareholders_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        setup_vesting_contract(admin, &vector[], &vector[], admin_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = supra_framework::supra_account)]
    public entry fun test_create_vesting_contract_with_invalid_withdrawal_address_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        setup_vesting_contract(admin, &vector[@1, @2], &vector[1], @5);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = supra_framework::supra_account)]
    public entry fun test_create_vesting_contract_with_missing_withdrawal_account_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        setup_vesting_contract(admin, &vector[@1, @2], &vector[1], @11);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x60002, location = supra_framework::supra_account)]
    public entry fun test_create_vesting_contract_with_unregistered_withdrawal_account_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        create_account_for_test(@11);
        setup_vesting_contract(admin, &vector[@1, @2], &vector[1], @11);
    }

    #[test(supra_framework = @0x1)]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    public entry fun test_create_empty_vesting_schedule_should_fail(
        supra_framework: &signer
    ) {
        setup(supra_framework, vector[]);
        create_vesting_schedule(vector[], 1, 1);
    }

    #[test(supra_framework = @0x1)]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    public entry fun test_create_first_element_zero_vesting_schedule_should_fail(
        supra_framework: &signer
    ) {
        setup(supra_framework, vector[]);
        create_vesting_schedule(
            vector[
                fixed_point32::create_from_raw_value(0),
                fixed_point32::create_from_raw_value(8)],
            1,
            1,
        );
    }

    #[test(supra_framework = @0x1)]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    public entry fun test_create_last_element_zero_vesting_schedule_should_fail(
        supra_framework: &signer
    ) {
        setup(supra_framework, vector[]);
        create_vesting_schedule(
            vector[
                fixed_point32::create_from_raw_value(8),
                fixed_point32::create_from_raw_value(0)],
            1,
            1,
        );
    }

    #[test(supra_framework = @0x1)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    public entry fun test_create_vesting_schedule_with_zero_period_duration_should_fail(
        supra_framework: &signer
    ) {
        setup(supra_framework, vector[]);
        create_vesting_schedule(
            vector[fixed_point32::create_from_rational(1, 1)],
            1,
            0,
        );
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234)]
    public entry fun test_last_vest_should_distribute_remaining_amount(
        supra_framework: &signer, admin: &signer, shareholder: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let shareholder_address = signer::address_of(shareholder);
        setup(supra_framework, vector[admin_address, shareholder_address]);
        let contract_address =
            setup_vesting_contract_with_schedule(
                admin,
                &vector[shareholder_address],
                &vector[GRANT_AMOUNT],
                admin_address,
                // First vest = 3/4 but last vest should only be for the remaining 1/4.
                &vector[3],
                4,
            );

        // First vest is 3/4
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + VESTING_PERIOD
        );
        vest(contract_address);
        let vested_amount = fraction(GRANT_AMOUNT, 3, 4);
        let remaining_grant = GRANT_AMOUNT - vested_amount;
        assert!(
            remaining_grant(contract_address, shareholder_address) == remaining_grant, 0
        );

        timestamp::fast_forward_seconds(VESTING_PERIOD);
        // Last vest should be the remaining amount (1/4).
        vest(contract_address);
        remaining_grant = 0;
        assert!(
            remaining_grant(contract_address, shareholder_address) == remaining_grant, 0
        );
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234)]
    public entry fun entry_test_last_vest_should_distribute_remaining_amount(
        supra_framework: &signer, admin: &signer, shareholder: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let shareholder_address = signer::address_of(shareholder);
        setup(supra_framework, vector[admin_address, shareholder_address]);
        stake::mint(admin, GRANT_AMOUNT);
        let contract_address =
            setup_vesting_contract_with_amount_with_schedule(
                admin,
                vector[shareholder_address],
                vector[GRANT_AMOUNT],
                admin_address,
                // First vest = 3/4 but last vest should only be for the remaining 1/4.
                vector[3],
                4,
            );
        // First vest is 3/4
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + VESTING_PERIOD
        );
        vest(contract_address);
        let vested_amount = fraction(GRANT_AMOUNT, 3, 4);
        let remaining_grant = GRANT_AMOUNT - vested_amount;
        assert!(
            remaining_grant(contract_address, shareholder_address) == remaining_grant, 0
        );

        timestamp::fast_forward_seconds(VESTING_PERIOD);
        // Last vest should be the remaining amount (1/4).
        vest(contract_address);
        remaining_grant = 0;
        assert!(
            remaining_grant(contract_address, shareholder_address) == remaining_grant, 0
        );
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234)]
    #[expected_failure(abort_code = 0x30008, location = Self)]
    public entry fun test_cannot_vest_after_contract_is_terminated(
        supra_framework: &signer, admin: &signer, shareholder: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let shareholder_address = signer::address_of(shareholder);
        setup(supra_framework, vector[admin_address, shareholder_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[shareholder_address],
                &vector[GRANT_AMOUNT],
                admin_address,
            );

        // Immediately terminate. Calling vest should now fail.
        terminate_vesting_contract(admin, contract_address);
        vest(contract_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234)]
    #[expected_failure(abort_code = 0x30008, location = Self)]
    public entry fun test_cannot_terminate_twice(
        supra_framework: &signer, admin: &signer, shareholder: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let shareholder_address = signer::address_of(shareholder);
        setup(supra_framework, vector[admin_address, shareholder_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[shareholder_address],
                &vector[GRANT_AMOUNT],
                admin_address,
            );

        // Call terminate_vesting_contract twice should fail.
        terminate_vesting_contract(admin, contract_address);
        terminate_vesting_contract(admin, contract_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234)]
    #[expected_failure(abort_code = 0x30009, location = Self)]
    public entry fun test_cannot_call_admin_withdraw_if_contract_is_not_terminated(
        supra_framework: &signer, admin: &signer, shareholder: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let shareholder_address = signer::address_of(shareholder);
        setup(supra_framework, vector[admin_address, shareholder_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[shareholder_address],
                &vector[GRANT_AMOUNT],
                admin_address,
            );

        // Calling admin_withdraw should fail as contract has not been terminated.
        admin_withdraw(admin, contract_address);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = supra_framework::supra_account)]
    public entry fun test_set_beneficiary_with_missing_account_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@1, @2],
                &vector[GRANT_AMOUNT, GRANT_AMOUNT],
                admin_address,
            );
        set_beneficiary(admin, contract_address, @1, @11);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    #[expected_failure(abort_code = 0x60002, location = supra_framework::supra_account)]
    public entry fun test_set_beneficiary_with_unregistered_account_should_fail(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@1, @2],
                &vector[GRANT_AMOUNT, GRANT_AMOUNT],
                admin_address,
            );
        create_account_for_test(@11);
        set_beneficiary(admin, contract_address, @1, @11);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    public entry fun test_set_beneficiary_should_send_distribution(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address, @11]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@1],
                &vector[GRANT_AMOUNT],
                admin_address,
            );
        set_beneficiary(admin, contract_address, @1, @11);
        assert!(beneficiary(contract_address, @1) == @11, 0);

        // Fast forward to the end of the first period. vest() should now unlock 2/10 of the tokens.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + VESTING_PERIOD
        );
        vest(contract_address);

        let vested_amount = fraction(GRANT_AMOUNT, 2, 10);
        let balance = coin::balance<SupraCoin>(@11);
        assert!(balance == vested_amount, balance);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    public entry fun test_set_management_role(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingAccountManagement, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@11],
                &vector[GRANT_AMOUNT],
                admin_address,
            );
        let role = utf8(b"RANDOM");
        set_management_role(admin, contract_address, role, @12);
        assert!(get_role_holder(contract_address, role) == @12, 0);
        set_management_role(admin, contract_address, role, @13);
        assert!(get_role_holder(contract_address, role) == @13, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    public entry fun test_reset_beneficiary(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingAccountManagement, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(
            supra_framework,
            vector[admin_address, @11, @12],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@11],
                &vector[GRANT_AMOUNT],
                admin_address,
            );
        set_beneficiary(admin, contract_address, @11, @12);
        assert!(beneficiary(contract_address, @11) == @12, 0);

        // Fast forward to the end of the first period. vest() should now unlock 2/10 of the tokens.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);
        let (init_amount, left_amount, last_vested_period) = get_vesting_record(
            contract_address, @11
        );
        assert!(init_amount == GRANT_AMOUNT, init_amount);
        let vested_amount = fraction(GRANT_AMOUNT, 2, 10);
        assert!(
            left_amount == init_amount - vested_amount,
            left_amount,
        );
        assert!(last_vested_period == 1, last_vested_period);

        // Reset the beneficiary.
        reset_beneficiary(admin, contract_address, @11);

        assert!(coin::balance<SupraCoin>(@12) == vested_amount, 0);
        assert!(coin::balance<SupraCoin>(@11) == 0, 1);
    }

    #[test(supra_framework = @0x1, admin = @0x123, resetter = @0x234)]
    public entry fun test_reset_beneficiary_with_resetter_role(
        supra_framework: &signer, admin: &signer, resetter: &signer
    ) acquires AdminStore, VestingAccountManagement, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(
            supra_framework,
            vector[admin_address, @11, @12],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@11],
                &vector[GRANT_AMOUNT],
                admin_address,
            );
        set_beneficiary(admin, contract_address, @11, @12);
        assert!(beneficiary(contract_address, @11) == @12, 0);

        // Reset the beneficiary with the resetter role.
        let resetter_address = signer::address_of(resetter);
        set_beneficiary_resetter(admin, contract_address, resetter_address);
        assert!(
            simple_map::length(
                &borrow_global<VestingAccountManagement>(contract_address).roles,
            ) == 1,
            0,
        );
        reset_beneficiary(resetter, contract_address, @11);
        assert!(beneficiary(contract_address, @11) == @11, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, resetter = @0x234, random = @0x345)]
    #[expected_failure(abort_code = 0x5000F, location = Self)]
    public entry fun test_reset_beneficiary_with_unauthorized(
        supra_framework: &signer,
        admin: &signer,
        resetter: &signer,
        random: &signer
    ) acquires AdminStore, VestingAccountManagement, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(supra_framework, vector[admin_address, @11]);
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@11],
                &vector[GRANT_AMOUNT],
                admin_address,
            );

        // Reset the beneficiary with a random account. This should failed.
        set_beneficiary_resetter(admin, contract_address, signer::address_of(resetter));
        reset_beneficiary(random, contract_address, @11);
    }

    #[test(supra_framework = @0x1, admin = @0x123)]
    public entry fun test_shareholder(
        supra_framework: &signer, admin: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        setup(
            supra_framework,
            vector[admin_address, @11, @12],
        );
        let contract_address =
            setup_vesting_contract(
                admin,
                &vector[@11],
                &vector[GRANT_AMOUNT],
                admin_address,
            );

        // Confirm that the lookup returns the same address when a shareholder is
        // passed for which there is no beneficiary.
        assert!(shareholder(contract_address, @11) == @11, 0);

        // Set a beneficiary for @11.
        set_beneficiary(admin, contract_address, @11, @12);
        assert!(beneficiary(contract_address, @11) == @12, 0);

        // Confirm that lookup from beneficiary to shareholder works when a beneficiary
        // is set.
        assert!(shareholder(contract_address, @12) == @11, 0);

        // Confirm that it returns 0x0 when the address is not in the map.
        assert!(shareholder(contract_address, @33) == @0x0, 0);

        remove_shareholder(admin, contract_address, @11);

        // Confirm that shareholder address does't exist in the map
        assert!(shareholder(contract_address, @11) == @0x0, 0);
    }

    #[test_only]
    fun fraction(total: u64, numerator: u64, denominator: u64): u64 {
        fixed_point32::multiply_u64(
            total, fixed_point32::create_from_rational(numerator, denominator)
        )
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234, withdrawal = @111)]
    public entry fun test_end_to_end_can_fast_forward_divisable(
        supra_framework: &signer,
        admin: &signer,
        shareholder: &signer,
        withdrawal: &signer,
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_address = signer::address_of(shareholder);
        let shareholders = &vector[shareholder_address];
        let shareholder_share = GRANT_AMOUNT;
        let shares = &vector[shareholder_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[admin_address, withdrawal_address, shareholder_address],
        );
        let contract_address = setup_vesting_contract(
            admin, shareholders, shares, withdrawal_address
        );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == GRANT_AMOUNT, 0);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);
        vested_amount = vested_amount + fraction(shareholder_share, 2, 10);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 9,
        );
        vest(contract_address);
        vested_amount = shareholder_share;
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234, withdrawal = @111)]
    public entry fun test_end_to_end_can_fast_forward_nondivisable(
        supra_framework: &signer,
        admin: &signer,
        shareholder: &signer,
        withdrawal: &signer,
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_address = signer::address_of(shareholder);
        let shareholders = &vector[shareholder_address];
        let shareholder_share = 3334;
        let shares = &vector[shareholder_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[admin_address, withdrawal_address, shareholder_address],
        );
        let contract_address = setup_vesting_contract(
            admin, shareholders, shares, withdrawal_address
        );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == 3334, 0);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);

        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);
        vested_amount = vested_amount + fraction(shareholder_share, 2, 10);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 9,
        );
        vest(contract_address);
        vested_amount = shareholder_share;
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234, withdrawal = @111)]
    public entry fun test_end_to_end_can_fast_forward_time_unchanged(
        supra_framework: &signer,
        admin: &signer,
        shareholder: &signer,
        withdrawal: &signer,
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_address = signer::address_of(shareholder);
        let shareholders = &vector[shareholder_address];
        let shareholder_share = 1000;
        let shares = &vector[shareholder_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[admin_address, withdrawal_address, shareholder_address],
        );
        let contract_address = setup_vesting_contract(
            admin, shareholders, shares, withdrawal_address
        );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == 1000, 0);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address),
        );
        vest(contract_address);
        vested_amount = vested_amount + fraction(shareholder_share, 2, 10);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
        // The time is unchanged, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);

        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 9,
        );
        vest(contract_address);
        vested_amount = shareholder_share;
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234, withdrawal = @111)]
    public entry fun test_end_to_end_can_fast_forward_time_5_out_of_10(
        supra_framework: &signer,
        admin: &signer,
        shareholder: &signer,
        withdrawal: &signer,
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_address = signer::address_of(shareholder);
        let shareholders = &vector[shareholder_address];
        let shareholder_share = 1000;
        let shares = &vector[shareholder_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[admin_address, withdrawal_address, shareholder_address],
        );
        let contract_address = setup_vesting_contract_with_schedule(
            admin,
            shareholders,
            shares,
            withdrawal_address,
            &vector[2, 3, 1],
            10,
        );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(coin::balance<SupraCoin>(contract_address) == 1000, 0);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 2,
        );
        vest(contract_address);
        vested_amount = vested_amount + fraction(shareholder_share, 5, 10);
        let shareholder_balance = coin::balance<SupraCoin>(shareholder_address);
        assert!(shareholder_balance + 1 == vested_amount, vested_amount);
    }

    #[test(supra_framework = @0x1, admin = @0x123, shareholder = @0x234, withdrawal = @111)]
    public entry fun test_end_to_end_can_fast_forward_time_7_out_of_10(
        supra_framework: &signer,
        admin: &signer,
        shareholder: &signer,
        withdrawal: &signer,
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_address = signer::address_of(shareholder);
        let shareholders = &vector[shareholder_address];
        let shareholder_share = 1000;
        let shares = &vector[shareholder_share];
        // Create the vesting contract.
        setup(
            supra_framework,
            vector[admin_address, withdrawal_address, shareholder_address],
        );
        let contract_address = setup_vesting_contract_with_schedule(
            admin,
            shareholders,
            shares,
            withdrawal_address,
            &vector[2, 3, 1],
            10,
        );
        assert!(
            vector::length(&borrow_global<AdminStore>(admin_address).vesting_contracts) ==
            1,
            0,
        );
        let vested_amount = 0;
        // Because the time is behind the start time, vest will do nothing.
        vest(contract_address);
        assert!(get_withdrawal_addr(contract_address) == withdrawal_address, 98);
        assert!(get_contract_admin(contract_address) == admin_address, 99);
        assert!(coin::balance<SupraCoin>(contract_address) == 1000, 0);
        assert!(coin::balance<SupraCoin>(shareholder_address) == vested_amount, 0);
        // Time is now at the start time, vest will unlock the first period, which is 2/10.
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + period_duration_secs(contract_address)
            * 4,
        );
        vest(contract_address);
        vested_amount = vested_amount + fraction(shareholder_share, 7, 10);
        let shareholder_balance = coin::balance<SupraCoin>(shareholder_address);
        assert!(shareholder_balance == vested_amount, vested_amount);
    }

    #[test(supra_framework = @0x1, admin = @0x1111, shareholder_1 = @0x2222, shareholder_2 = @0x3333, withdrawal = @0x1111)]
    public entry fun test_full_vesting_after_7_months(
        supra_framework: &signer,
        admin: &signer,
        shareholder_1: &signer,
        shareholder_2: &signer,
        withdrawal: &signer
    ) acquires AdminStore, VestingContract {
        let admin_address = signer::address_of(admin);
        let withdrawal_address = signer::address_of(withdrawal);
        let shareholder_1_address = signer::address_of(shareholder_1);
        let shareholder_2_address = signer::address_of(shareholder_2);

        // Amounts for shareholders
        let shareholder_1_share = 1000000000000000; // 10^15
        let shareholder_2_share = 10000000000000000; // 10^16

        let shareholders = vector[shareholder_1_address, shareholder_2_address];
        let shares = vector[shareholder_1_share, shareholder_2_share];

        // Setup accounts and mint coins
        setup(
            supra_framework,
            vector[
                admin_address,
                withdrawal_address,
                shareholder_1_address,
                shareholder_2_address],
        );
        stake::mint(admin, shareholder_1_share + shareholder_2_share);

        let numerators: vector<u64> = vector[1];
        // Create vesting contract with period_duration = 1, vesting_numerators = [1], vesting_denominator = 1
        let contract_address = setup_vesting_contract_with_amount_with_schedule(
            admin,
            shareholders,
            shares,
            withdrawal_address,
            numerators,
            1,
        );
        set_vesting_schedule(
            admin,
            contract_address,
            numerators,
            1, // Denominators
            1, // Period duration in seconds
        );

        // Fast forward time by 7 months (approx 7 * 30 * 24 * 60 * 60 seconds)
        let seven_months_secs = 7 * 30 * 24 * 60 * 60;
        timestamp::update_global_time_for_test_secs(
            vesting_start_secs(contract_address) + seven_months_secs
        );

        // Both shareholders vest individually
        vest_individual(contract_address, shareholder_1_address);
        vest_individual(contract_address, shareholder_2_address);

        // Assert both shareholders have received their full original amount
        let (init_amount_1, left_amount_1, _) =
            get_vesting_record(contract_address, shareholder_1_address);
        let (init_amount_2, left_amount_2, _) =
            get_vesting_record(contract_address, shareholder_2_address);

        assert!(left_amount_1 == 0, left_amount_1);
        assert!(left_amount_2 == 0, left_amount_2);

        let balance_1 = coin::balance<SupraCoin>(shareholder_1_address);
        let balance_2 = coin::balance<SupraCoin>(shareholder_2_address);

        assert!(balance_1 == shareholder_1_share, balance_1);
        assert!(balance_2 == shareholder_2_share, balance_2);
    }
}
