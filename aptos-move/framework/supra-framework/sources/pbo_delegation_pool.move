/**
Supra note: This is customization for PBO.

Allow multiple delegators to participate in the same stake pool in order to collect the minimum
stake required to join the validator set. Delegators are rewarded out of the validator rewards
proportionally to their stake and provided the same stake-management API as the stake pool owner.

The main accounting logic in the delegation pool contract handles the following:
1. Tracks how much stake each delegator owns, privately deposited as well as earned.
Accounting individual delegator stakes is achieved through the shares-based pool defined at
<code>aptos_std::pool_u64</code>, hence delegators own shares rather than absolute stakes into the delegation pool.
2. Tracks rewards earned by the stake pool, implicitly by the delegation one, in the meantime
and distribute them accordingly.
3. Tracks lockup cycles on the stake pool in order to separate inactive stake (not earning rewards)
from pending_inactive stake (earning rewards) and allow its delegators to withdraw the former.
4. Tracks how much commission fee has to be paid to the operator out of incoming rewards before
distributing them to the internal pool_u64 pools.

In order to distinguish between stakes in different states and route rewards accordingly,
separate pool_u64 pools are used for individual stake states:
1. one of <code>active</code> + <code>pending_active</code> stake
2. one of <code>inactive</code> stake FOR each past observed lockup cycle (OLC) on the stake pool
3. one of <code>pending_inactive</code> stake scheduled during this ongoing OLC

As stake-state transitions and rewards are computed only at the stake pool level, the delegation pool
gets outdated. To mitigate this, at any interaction with the delegation pool, a process of synchronization
to the underlying stake pool is executed before the requested operation itself.

At synchronization:
 - stake deviations between the two pools are actually the rewards produced in the meantime.
 - the commission fee is extracted from the rewards, the remaining stake is distributed to the internal
pool_u64 pools and then the commission stake used to buy shares for operator.
 - if detecting that the lockup expired on the stake pool, the delegation pool will isolate its
pending_inactive stake (now inactive) and create a new pool_u64 to host future pending_inactive stake
scheduled this newly started lockup.
Detecting a lockup expiration on the stake pool resumes to detecting new inactive stake.

Accounting main invariants:
 - each stake-management operation (add/unlock/reactivate/withdraw) and operator change triggers
the synchronization process before executing its own function.
 - each OLC maps to one or more real lockups on the stake pool, but not the opposite. Actually, only a real
lockup with 'activity' (which inactivated some unlocking stake) triggers the creation of a new OLC.
 - unlocking and/or unlocked stake originating from different real lockups are never mixed together into
the same pool_u64. This invalidates the accounting of which rewards belong to whom.
 - no delegator can have unlocking and/or unlocked stake (pending withdrawals) in different OLCs. This ensures
delegators do not have to keep track of the OLCs when they unlocked. When creating a new pending withdrawal,
the existing one is executed (withdrawn) if is already inactive.
 - <code>add_stake</code> fees are always refunded, but only after the epoch when they have been charged ends.
 - withdrawing pending_inactive stake (when validator had gone inactive before its lockup expired)
does not inactivate any stake additional to the requested one to ensure OLC would not advance indefinitely.
 - the pending withdrawal exists at an OLC iff delegator owns some shares within the shares pool of that OLC.

Example flow:
<ol>
<li>A node operator creates a delegation pool by calling
<code>initialize_delegation_pool</code> and sets
its commission fee to 0% (for simplicity). A stake pool is created with no initial stake and owned by
a resource account controlled by the delegation pool.</li>
<li>Delegator A adds 100 stake which is converted to 100 shares into the active pool_u64</li>
<li>Operator joins the validator set as the stake pool has now the minimum stake</li>
<li>The stake pool earned rewards and now has 200 active stake. A's active shares are worth 200 coins as
the commission fee is 0%.</li>
<li></li>
<ol>
    <li>A requests <code>unlock</code> for 100 stake</li>
    <li>Synchronization detects 200 - 100 active rewards which are entirely (0% commission) added to the active pool.</li>
    <li>100 coins = (100 * 100) / 200 = 50 shares are redeemed from the active pool and exchanged for 100 shares
into the pending_inactive one on A's behalf</li>
</ol>
<li>Delegator B adds 200 stake which is converted to (200 * 50) / 100 = 100 shares into the active pool</li>
<li>The stake pool earned rewards and now has 600 active and 200 pending_inactive stake.</li>
<li></li>
<ol>
    <li>A requests <code>reactivate_stake</code> for 100 stake</li>
    <li>
    Synchronization detects 600 - 300 active and 200 - 100 pending_inactive rewards which are both entirely
    distributed to their corresponding pools
    </li>
    <li>
    100 coins = (100 * 100) / 200 = 50 shares are redeemed from the pending_inactive pool and exchanged for
    (100 * 150) / 600 = 25 shares into the active one on A's behalf
    </li>
</ol>
<li>The lockup expires on the stake pool, inactivating the entire pending_inactive stake</li>
<li></li>
<ol>
    <li>B requests <code>unlock</code> for 100 stake</li>
    <li>
    Synchronization detects no active or pending_inactive rewards, but 0 -> 100 inactive stake on the stake pool,
    so it advances the observed lockup cycle and creates a pool_u64 for the new lockup, hence allowing previous
    pending_inactive shares to be redeemed</li>
    <li>
    100 coins = (100 * 175) / 700 = 25 shares are redeemed from the active pool and exchanged for 100 shares
    into the new pending_inactive one on B's behalf
    </li>
</ol>
<li>The stake pool earned rewards and now has some pending_inactive rewards.</li>
<li></li>
<ol>
    <li>A requests <code>withdraw</code> for its entire inactive stake</li>
    <li>
    Synchronization detects no new inactive stake, but some pending_inactive rewards which are distributed
    to the (2nd) pending_inactive pool
    </li>
    <li>
    A's 50 shares = (50 * 100) / 50 = 100 coins are redeemed from the (1st) inactive pool and 100 stake is
    transferred to A
    </li>
</ol>
</ol>
 */
module supra_framework::pbo_delegation_pool {
    use std::error;
    use std::features;
    use std::signer;
    use std::vector;
    use std::option::{Self, Option};

    use aptos_std::math64;
    use aptos_std::math128;
    use aptos_std::pool_u64_unbound::{Self as pool_u64, total_coins};
    use aptos_std::table::{Self, Table};
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::fixed_point64::{Self, FixedPoint64};

    use supra_framework::coin::{Self, Coin};
    use supra_framework::account;
    use supra_framework::supra_account;
    use supra_framework::supra_coin::SupraCoin;
    use supra_framework::event::{Self, EventHandle, emit};
    use supra_framework::stake::{Self, get_operator};
    use supra_framework::staking_config;
    use supra_framework::timestamp;
    use supra_framework::multisig_account;

    const MODULE_SALT: vector<u8> = b"supra_framework::pbo_delegation_pool";

    /// Delegation pool owner capability does not exist at the provided account.
    const EOWNER_CAP_NOT_FOUND: u64 = 1;

    /// Account is already owning a delegation pool.
    const EOWNER_CAP_ALREADY_EXISTS: u64 = 2;

    /// Delegation pool does not exist at the provided pool address.
    const EDELEGATION_POOL_DOES_NOT_EXIST: u64 = 3;

    /// There is a pending withdrawal to be executed before `unlock`ing any new stake.
    const EPENDING_WITHDRAWAL_EXISTS: u64 = 4;

    /// Commission percentage has to be between 0 and `MAX_FEE` - 100%.
    const EINVALID_COMMISSION_PERCENTAGE: u64 = 5;

    /// There is not enough `active` stake on the stake pool to `unlock`.
    const ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK: u64 = 6;

    /// Slashing (if implemented) should not be applied to already `inactive` stake.
    /// Not only it invalidates the accounting of past observed lockup cycles (OLC),
    /// but is also unfair to delegators whose stake has been inactive before validator started misbehaving.
    /// Additionally, the inactive stake does not count on the voting power of validator.
    const ESLASHED_INACTIVE_STAKE_ON_PAST_OLC: u64 = 7;

    /// Delegator's active balance cannot be less than `MIN_COINS_ON_SHARES_POOL`.
    const EDELEGATOR_ACTIVE_BALANCE_TOO_LOW: u64 = 8;

    /// Delegator's pending_inactive balance cannot be less than `MIN_COINS_ON_SHARES_POOL`.
    const EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW: u64 = 9;

    /// Creating delegation pools is not enabled yet.
    const EDELEGATION_POOLS_DISABLED: u64 = 10;

    /// Cannot request to withdraw zero stake.
    const EWITHDRAW_ZERO_STAKE: u64 = 11;

    /// Function is deprecated.
    const EDEPRECATED_FUNCTION: u64 = 12;

    /// The function is disabled or hasn't been enabled.
    const EDISABLED_FUNCTION: u64 = 13;

    /// Partial governance voting hasn't been enabled on this delegation pool.
    const EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED: u64 = 14;

    /// The voter does not have sufficient stake to create a proposal.
    const EINSUFFICIENT_PROPOSER_STAKE: u64 = 15;

    /// The voter does not have any voting power on this proposal.
    const ENO_VOTING_POWER: u64 = 16;

    /// The stake pool has already voted on the proposal before enabling partial governance voting on this delegation pool.
    const EALREADY_VOTED_BEFORE_ENABLE_PARTIAL_VOTING: u64 = 17;

    /// The account is not the operator of the stake pool.
    const ENOT_OPERATOR: u64 = 18;

    /// Chaning beneficiaries for operators is not supported.
    const EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED: u64 = 19;

    /// Commission percentage increase is too large.
    const ETOO_LARGE_COMMISSION_INCREASE: u64 = 20;

    /// Commission percentage change is too late in this lockup period, and should be done at least a quarter (1/4) of the lockup duration before the lockup cycle ends.
    const ETOO_LATE_COMMISSION_CHANGE: u64 = 21;

    /// Changing operator commission rate in delegation pool is not supported.
    const ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED: u64 = 22;

    /// Vector length is not the same.
    const EVECTOR_LENGTH_NOT_SAME: u64 = 23;

    /// Coin value is not the same with principle stake.
    const ECOIN_VALUE_NOT_SAME_AS_PRINCIPAL_STAKE: u64 = 24;

    /// Requested amount too high, the balance would fall below principle stake after unlock
    const EAMOUNT_REQUESTED_NOT_UNLOCKABLE: u64 = 25;

    /// Active share is not the same in stake pool and delegation pool
    const EACTIVE_COIN_VALUE_NOT_SAME_STAKE_DELEGATION_POOL: u64 = 26;

    /// Provided admin address is not a multisig account
    const EADMIN_NOT_MULTISIG: u64 = 27;

    /// Delegator address does not exist in pool tables
    const EDELEGATOR_DOES_NOT_EXIST: u64 = 28;

    ///Pool unlock time in past
    const ESTARTUP_TIME_IN_PAST: u64 = 29;

    //Pool unlock schedule is empty
    const EEMPTY_UNLOCK_SCHEDULE: u64 = 30;

    //Pool unlock schedule has a zero fraction
    const ESCHEDULE_WITH_ZERO_FRACTION: u64 = 31;

    //Pool unlock has zero period duration
    const EPERIOD_DURATION_IS_ZERO: u64 = 32;

    // Zero denominator in unlock schedule
    const EDENOMINATOR_IS_ZERO: u64 = 33;

    // Sum of numerators must be less than denominator
    const ENUMERATORS_GRATER_THAN_DENOMINATOR: u64 = 34;

    const EADMIN_ADDRESS_CANNOT_BE_ZERO: u64 = 35;

    const ENOT_AUTHORIZED: u64 = 36;

    const ENEW_IS_SAME_AS_OLD_DELEGATOR: u64 = 37;

    /// Minimum amount of coins to be unlocked.
    const EMINIMUM_UNLOCK_AMOUNT: u64 = 38;

    /// Balance is not enough.
    const EBALANCE_NOT_SUFFICIENT: u64 = 39;

    /// Thrown by `lock_delegators_stakes` when a given delegator has less than the specified
    /// amount of stake available in the specified stake pool.
    const EINSUFFICIENT_STAKE_TO_LOCK: u64 = 40;

    const EUNLOCKING_ALREADY_STARTED: u64 = 41;

    const MAX_U64: u64 = 18446744073709551615;

    /// Maximum operator percentage fee(of double digit precision): 22.85% is represented as 2285
    const MAX_FEE: u64 = 10000;

    const VALIDATOR_STATUS_INACTIVE: u64 = 4;

    /// Special shareholder temporarily owning the `add_stake` fees charged during this epoch.
    /// On each `add_stake` operation any resulted fee is used to buy active shares for this shareholder.
    /// First synchronization after this epoch ends will distribute accumulated fees to the rest of the pool as refunds.
    const NULL_SHAREHOLDER: address = @0x0;

    /// Minimum coins to exist on a shares pool at all times.
    /// Enforced per delegator for both active and pending_inactive pools.
    /// This constraint ensures the share price cannot overly increase and lead to
    /// substantial loses when buying shares (can lose at most 1 share which may
    /// be worth a lot if current share price is high).
    /// This constraint is not enforced on inactive pools as they only allow redeems
    /// (can lose at most 1 coin regardless of current share price).
    const MIN_COINS_ON_SHARES_POOL: u64 = 100000000;

    /// Scaling factor of shares pools used within the delegation pool
    const SHARES_SCALING_FACTOR: u64 = 10000000000000000;

    /// Maximum commission percentage increase per lockup cycle. 10% is represented as 1000.
    const MAX_COMMISSION_INCREASE: u64 = 1000;

    /// Capability that represents ownership over privileged operations on the underlying stake pool.
    struct DelegationPoolOwnership has key, store {
        /// equal to address of the resource account owning the stake pool
        pool_address: address
    }

    struct ObservedLockupCycle has copy, drop, store {
        index: u64
    }

    struct UnlockSchedule has copy, drop, store {
        // The vesting schedule as a list of fractions that vest for each period. The last number is repeated until the
        // vesting amount runs out.
        // For example [1/24, 1/24, 1/48] with a period of 1 month means that after vesting starts, the first two months
        // will vest 1/24 of the original total amount. From the third month only, 1/48 will vest until the vesting fund
        // runs out.
        // u32/u32 should be sufficient to support vesting schedule fractions.
        schedule: vector<FixedPoint64>,
        // When the vesting should start.
        start_timestamp_secs: u64,
        // In seconds. How long each vesting period is. For example 1 month.
        period_duration: u64,
        // Last vesting period, 1-indexed. For example if 2 months have passed, the last vesting period, if distribution
        // was requested, would be 2. Default value is 0 which means there have been no vesting periods yet.
        last_unlock_period: u64,
        cumulative_unlocked_fraction: FixedPoint64
    }

    struct DelegationPool has key {
        multisig_admin: Option<address>,
        // Shares pool of `active` + `pending_active` stake
        active_shares: pool_u64::Pool,
        // Index of current observed lockup cycle on the delegation pool since its creation
        observed_lockup_cycle: ObservedLockupCycle,
        // Shares pools of `inactive` stake on each ended OLC and `pending_inactive` stake on the current one.
        // Tracks shares of delegators who requested withdrawals in each OLC
        inactive_shares: Table<ObservedLockupCycle, pool_u64::Pool>,
        // Mapping from delegator address to the OLC of its pending withdrawal if having one
        pending_withdrawals: Table<address, ObservedLockupCycle>,
        // Signer capability of the resource account owning the stake pool
        stake_pool_signer_cap: account::SignerCapability,
        // Total (inactive) coins on the shares pools over all ended OLCs
        total_coins_inactive: u64,
        // Commission fee paid to the node operator out of pool rewards
        operator_commission_percentage: u64,
        // Unlock schedule for principle/initial stake, same for everyone
        principle_unlock_schedule: UnlockSchedule,
        // From shareholders to their initial stake
        principle_stake: Table<address, u64>,
        // The events emitted by stake-management operations on the delegation pool
        add_stake_events: EventHandle<AddStakeEvent>,
        reactivate_stake_events: EventHandle<ReactivateStakeEvent>,
        unlock_stake_events: EventHandle<UnlockStakeEvent>,
        withdraw_stake_events: EventHandle<WithdrawStakeEvent>,
        distribute_commission_events: EventHandle<DistributeCommissionEvent>
    }

    struct VotingRecordKey has copy, drop, store {
        voter: address,
        proposal_id: u64
    }

    /// Track delgated voter of each delegator.
    struct VoteDelegation has copy, drop, store {
        // The account who can vote on behalf of this delegator.
        voter: address,
        // The account that will become the voter in the next lockup period. Changing voter address needs 1 lockup
        // period to take effects.
        pending_voter: address,
        // Tracks the last known lockup cycle end when the voter was updated. This will be used to determine when
        // the new voter becomes effective.
        // If <locked_until_secs of the stake pool> != last_locked_until_secs, it means that a lockup period has passed.
        // This is slightly different from ObservedLockupCycle because ObservedLockupCycle cannot detect if a lockup
        // period is passed when there is no unlocking during the lockup period.
        last_locked_until_secs: u64
    }

    /// Track total voteing power of each voter.
    struct DelegatedVotes has copy, drop, store {
        // The total number of active shares delegated to this voter by all delegators.
        active_shares: u128,
        // The total number of pending inactive shares delegated to this voter by all delegators
        pending_inactive_shares: u128,
        // Total active shares delegated to this voter in the next lockup cycle.
        // `active_shares_next_lockup` might be different `active_shares` when some delegators change their voter.
        active_shares_next_lockup: u128,
        // Tracks the last known lockup cycle end when the voter was updated. This will be used to determine when
        // the new voter becomes effective.
        // If <locked_until_secs of the stake pool> != last_locked_until_secs, it means that a lockup period has passed.
        // This is slightly different from ObservedLockupCycle because ObservedLockupCycle cannot detect if a lockup
        // period is passed when there is no unlocking during the lockup period.
        last_locked_until_secs: u64
    }

    /// Track governance information of a delegation(e.g. voter delegation/voting power calculation).
    /// This struct should be stored in the delegation pool resource account.
    struct GovernanceRecords has key {
        // `votes` tracks voting power usage of each voter on each proposal.
        votes: SmartTable<VotingRecordKey, u64>,
        // `votes_per_proposal` tracks voting power usage of this stake pool on each proposal. Key is proposal_id.
        votes_per_proposal: SmartTable<u64, u64>,
        vote_delegation: SmartTable<address, VoteDelegation>,
        delegated_votes: SmartTable<address, DelegatedVotes>,
        vote_events: EventHandle<VoteEvent>,
        create_proposal_events: EventHandle<CreateProposalEvent>,
        // Note: a DelegateVotingPowerEvent event only means that the delegator tries to change its voter. The change
        // won't take effect until the next lockup period.
        delegate_voting_power_events: EventHandle<DelegateVotingPowerEvent>
    }

    struct BeneficiaryForOperator has key {
        beneficiary_for_operator: address
    }

    struct NextCommissionPercentage has key {
        commission_percentage_next_lockup_cycle: u64,
        effective_after_secs: u64
    }

    struct AddStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_added: u64,
        add_stake_fee: u64
    }

    struct ReactivateStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_reactivated: u64
    }

    struct UnlockStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_unlocked: u64
    }

    struct WithdrawStakeEvent has drop, store {
        pool_address: address,
        delegator_address: address,
        amount_withdrawn: u64
    }

    struct DistributeCommissionEvent has drop, store {
        pool_address: address,
        operator: address,
        commission_active: u64,
        commission_pending_inactive: u64
    }

    #[event]
    struct UnlockScheduleUpdated has drop, store {
        pool_address: address,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        unlock_start_time: u64,
        unlock_duration: u64
    }

    #[event]
    struct DistributeCommission has drop, store {
        pool_address: address,
        operator: address,
        beneficiary: address,
        commission_active: u64,
        commission_pending_inactive: u64
    }

    #[event]
    struct DelegatorReplacemendEvent has drop, store {
        pool_address: address,
        old_delegator: address,
        new_delegator: address
    }

    struct VoteEvent has drop, store {
        voter: address,
        proposal_id: u64,
        delegation_pool: address,
        num_votes: u64,
        should_pass: bool
    }

    struct CreateProposalEvent has drop, store {
        proposal_id: u64,
        voter: address,
        delegation_pool: address
    }

    struct DelegateVotingPowerEvent has drop, store {
        pool_address: address,
        delegator: address,
        voter: address
    }

    #[event]
    struct SetBeneficiaryForOperator has drop, store {
        operator: address,
        old_beneficiary: address,
        new_beneficiary: address
    }

    #[event]
    struct CommissionPercentageChange has drop, store {
        pool_address: address,
        owner: address,
        commission_percentage_next_lockup_cycle: u64
    }

    #[event]
    struct UnlockScheduleApplied has drop, store {
        pool_address: address,
        delegator: address,
        amount: u64
    }

    #[view]
    /// Return whether supplied address `addr` is owner of a delegation pool.
    public fun owner_cap_exists(addr: address): bool {
        exists<DelegationPoolOwnership>(addr)
    }

    #[view]
    /// Return address of the delegation pool owned by `owner` or fail if there is none.
    public fun get_owned_pool_address(owner: address): address acquires DelegationPoolOwnership {
        assert_owner_cap_exists(owner);
        borrow_global<DelegationPoolOwnership>(owner).pool_address
    }

    #[view]
    /// Return whether a delegation pool exists at supplied address `addr`.
    public fun delegation_pool_exists(addr: address): bool {
        exists<DelegationPool>(addr)
    }

    #[view]
    /// Return whether a delegation pool has already enabled partial govnernance voting.
    public fun partial_governance_voting_enabled(pool_address: address): bool {
        exists<GovernanceRecords>(pool_address)
            && stake::get_delegated_voter(pool_address) == pool_address
    }

    #[view]
    /// Return the index of current observed lockup cycle on delegation pool `pool_address`.
    public fun observed_lockup_cycle(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        borrow_global<DelegationPool>(pool_address).observed_lockup_cycle.index
    }

    #[view]
    /// Return whether the commission percentage for the next lockup cycle is effective.
    public fun is_next_commission_percentage_effective(
        pool_address: address
    ): bool acquires NextCommissionPercentage {
        exists<NextCommissionPercentage>(pool_address)
            && timestamp::now_seconds()
                >= borrow_global<NextCommissionPercentage>(pool_address).effective_after_secs
    }

    #[view]
    /// Return the operator commission percentage set on the delegation pool `pool_address`.
    public fun operator_commission_percentage(
        pool_address: address
    ): u64 acquires DelegationPool, NextCommissionPercentage {
        assert_delegation_pool_exists(pool_address);
        if (is_next_commission_percentage_effective(pool_address)) {
            operator_commission_percentage_next_lockup_cycle(pool_address)
        } else {
            borrow_global<DelegationPool>(pool_address).operator_commission_percentage
        }
    }

    #[view]
    /// Return the operator commission percentage for the next lockup cycle.
    public fun operator_commission_percentage_next_lockup_cycle(
        pool_address: address
    ): u64 acquires DelegationPool, NextCommissionPercentage {
        assert_delegation_pool_exists(pool_address);
        if (exists<NextCommissionPercentage>(pool_address)) {
            borrow_global<NextCommissionPercentage>(pool_address).commission_percentage_next_lockup_cycle
        } else {
            borrow_global<DelegationPool>(pool_address).operator_commission_percentage
        }
    }

    #[view]
    /// Return the number of delegators owning active stake within `pool_address`.
    public fun shareholders_count_active_pool(pool_address: address): u64 acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        pool_u64::shareholders_count(
            &borrow_global<DelegationPool>(pool_address).active_shares
        )
    }

    #[view]
    /// Return the stake amounts on `pool_address` in the different states:
    /// (`active`,`inactive`,`pending_active`,`pending_inactive`)
    public fun get_delegation_pool_stake(pool_address: address): (u64, u64, u64, u64) {
        assert_delegation_pool_exists(pool_address);
        stake::get_stake(pool_address)
    }

    #[view]
    /// Return whether the given delegator has any withdrawable stake. If they recently requested to unlock
    /// some stake and the stake pool's lockup cycle has not ended, their coins are not withdrawable yet.
    public fun get_pending_withdrawal(
        pool_address: address, delegator_address: address
    ): (bool, u64) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (lockup_cycle_ended, _, pending_inactive, _, commission_pending_inactive) =
            calculate_stake_pool_drift(pool);

        let (withdrawal_exists, withdrawal_olc) =
            pending_withdrawal_exists(pool, delegator_address);
        if (!withdrawal_exists) {
            // if no pending withdrawal, there is neither inactive nor pending_inactive stake
            (false, 0)
        } else {
            // delegator has either inactive or pending_inactive stake due to automatic withdrawals
            let inactive_shares = table::borrow(&pool.inactive_shares, withdrawal_olc);
            if (withdrawal_olc.index < pool.observed_lockup_cycle.index) {
                // if withdrawal's lockup cycle ended on delegation pool then it is inactive
                (true, pool_u64::balance(inactive_shares, delegator_address))
            } else {
                pending_inactive = pool_u64::shares_to_amount_with_total_coins(
                    inactive_shares,
                    pool_u64::shares(inactive_shares, delegator_address),
                    // exclude operator pending_inactive rewards not converted to shares yet
                    pending_inactive - commission_pending_inactive
                );
                // if withdrawal's lockup cycle ended ONLY on stake pool then it is also inactive
                (lockup_cycle_ended, pending_inactive)
            }
        }
    }

    #[view]
    /// Return total stake owned by `delegator_address` within delegation pool `pool_address`
    /// in each of its individual states: (`active`,`inactive`,`pending_inactive`)
    public fun get_stake(
        pool_address: address, delegator_address: address
    ): (u64, u64, u64) acquires DelegationPool, BeneficiaryForOperator {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (lockup_cycle_ended, active, _, commission_active, commission_pending_inactive) =

            calculate_stake_pool_drift(pool);

        let total_active_shares = pool_u64::total_shares(&pool.active_shares);
        let delegator_active_shares =
            pool_u64::shares(&pool.active_shares, delegator_address);

        let (_, _, pending_active, _) = stake::get_stake(pool_address);
        if (pending_active == 0) {
            // zero `pending_active` stake indicates that either there are no `add_stake` fees or
            // previous epoch has ended and should identify shares owning these fees as released
            total_active_shares = total_active_shares
                - pool_u64::shares(&pool.active_shares, NULL_SHAREHOLDER);
            if (delegator_address == NULL_SHAREHOLDER) {
                delegator_active_shares = 0
            }
        };
        active = pool_u64::shares_to_amount_with_total_stats(
            &pool.active_shares,
            delegator_active_shares,
            // exclude operator active rewards not converted to shares yet
            active - commission_active,
            total_active_shares
        );

        // get state and stake (0 if there is none) of the pending withdrawal
        let (withdrawal_inactive, withdrawal_stake) =
            get_pending_withdrawal(pool_address, delegator_address);
        // report non-active stakes accordingly to the state of the pending withdrawal
        let (inactive, pending_inactive) =
            if (withdrawal_inactive) (withdrawal_stake, 0)
            else (0, withdrawal_stake);

        // should also include commission rewards in case of the operator account
        // operator rewards are actually used to buy shares which is introducing
        // some imprecision (received stake would be slightly less)
        // but adding rewards onto the existing stake is still a good approximation
        if (delegator_address == beneficiary_for_operator(get_operator(pool_address))) {
            active = active + commission_active;
            // in-flight pending_inactive commission can coexist with already inactive withdrawal
            if (lockup_cycle_ended) {
                inactive = inactive + commission_pending_inactive
            } else {
                pending_inactive = pending_inactive + commission_pending_inactive
            }
        };

        (active, inactive, pending_inactive)
    }

    #[view]
    /// Return refundable stake to be extracted from added `amount` at `add_stake` operation on pool `pool_address`.
    /// If the validator produces rewards this epoch, added stake goes directly to `pending_active` and
    /// does not earn rewards. However, all shares within a pool appreciate uniformly and when this epoch ends:
    /// - either added shares are still `pending_active` and steal from rewards of existing `active` stake
    /// - or have moved to `pending_inactive` and get full rewards (they displaced `active` stake at `unlock`)
    /// To mitigate this, some of the added stake is extracted and fed back into the pool as placeholder
    /// for the rewards the remaining stake would have earned if active:
    /// extracted-fee = (amount - extracted-fee) * reward-rate% * (100% - operator-commission%)
    public fun get_add_stake_fee(
        pool_address: address, amount: u64
    ): u64 acquires DelegationPool, NextCommissionPercentage {
        if (stake::is_current_epoch_validator(pool_address)) {
            let (rewards_rate, rewards_rate_denominator) =
                staking_config::get_reward_rate(&staking_config::get());
            if (rewards_rate_denominator > 0) {
                assert_delegation_pool_exists(pool_address);

                rewards_rate = rewards_rate
                    * (MAX_FEE - operator_commission_percentage(pool_address));
                rewards_rate_denominator = rewards_rate_denominator * MAX_FEE;
                (
                    (((amount as u128) * (rewards_rate as u128))
                        / ((rewards_rate as u128) + (rewards_rate_denominator as u128))) as u64
                )
            } else { 0 }
        } else { 0 }
    }

    #[view]
    /// Return whether `pending_inactive` stake can be directly withdrawn from
    /// the delegation pool, implicitly its stake pool, in the special case
    /// the validator had gone inactive before its lockup expired.
    public fun can_withdraw_pending_inactive(pool_address: address): bool {
        stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE
            && timestamp::now_seconds() >= stake::get_lockup_secs(pool_address)
    }

    #[view]
    /// Return the total voting power of a delegator in a delegation pool. This function syncs DelegationPool to the
    /// latest state.
    public fun calculate_and_update_voter_total_voting_power(
        pool_address: address, voter: address
    ): u64 acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(pool_address);
        // Delegation pool need to be synced to explain rewards(which could change the coin amount) and
        // commission(which could cause share transfer).
        synchronize_delegation_pool(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let latest_delegated_votes =
            update_and_borrow_mut_delegated_votes(pool, governance_records, voter);
        calculate_total_voting_power(pool, latest_delegated_votes)
    }

    #[view]
    /// Return the latest delegated voter of a delegator in a delegation pool. This function syncs DelegationPool to the
    /// latest state.
    public fun calculate_and_update_delegator_voter(
        pool_address: address, delegator_address: address
    ): address acquires DelegationPool, GovernanceRecords {
        assert_partial_governance_voting_enabled(pool_address);
        calculate_and_update_delegator_voter_internal(
            borrow_global<DelegationPool>(pool_address),
            borrow_global_mut<GovernanceRecords>(pool_address),
            delegator_address
        )
    }

    #[view]
    /// Return the address of the stake pool to be created with the provided owner, and seed.
    public fun get_expected_stake_pool_address(
        owner: address, delegation_pool_creation_seed: vector<u8>
    ): address {
        let seed = create_resource_account_seed(delegation_pool_creation_seed);
        account::create_resource_address(&owner, seed)
    }

    #[view]
    /// Return the minimum remaining time in seconds for commission change, which is one fourth of the lockup duration.
    public fun min_remaining_secs_for_commission_change(): u64 {
        let config = staking_config::get();
        staking_config::get_recurring_lockup_duration(&config) / 4
    }

    /// Initialize a delegation pool without actual coin but withdraw from the owner's account.
    public entry fun initialize_delegation_pool_with_amount(
        owner: &signer,
        multisig_admin: address,
        amount: u64,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
        delegator_address: vector<address>,
        principle_stake: vector<u64>,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        unlock_start_time: u64,
        unlock_duration: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert!(
            coin::balance<SupraCoin>(signer::address_of(owner)) >= amount,
            error::invalid_argument(EBALANCE_NOT_SUFFICIENT)
        );
        let coin = coin::withdraw<SupraCoin>(owner, amount);

        initialize_delegation_pool(
            owner,
            option::some(multisig_admin),
            operator_commission_percentage,
            delegation_pool_creation_seed,
            delegator_address,
            principle_stake,
            coin,
            unlock_numerators,
            unlock_denominator,
            unlock_start_time,
            unlock_duration
        )
    }

    /// Initialize a delegation pool without actual coin but withdraw from the owner's account.
    public entry fun initialize_delegation_pool_with_amount_without_multisig_admin(
        owner: &signer,
        amount: u64,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
        delegator_address: vector<address>,
        principle_stake: vector<u64>,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        unlock_start_time: u64,
        unlock_duration: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert!(
            coin::balance<SupraCoin>(signer::address_of(owner)) >= amount,
            error::invalid_argument(EBALANCE_NOT_SUFFICIENT)
        );
        let coin = coin::withdraw<SupraCoin>(owner, amount);

        initialize_delegation_pool(
            owner,
            option::none<address>(),
            operator_commission_percentage,
            delegation_pool_creation_seed,
            delegator_address,
            principle_stake,
            coin,
            unlock_numerators,
            unlock_denominator,
            unlock_start_time,
            unlock_duration
        )
    }

    #[view]
    /// Return the unlock schedule of the pool as (schedule, start_time, period_duration, last_unlock_period, cumulative_unlocked_fraction)
    public fun get_unlock_schedule(
        pool_address: address
    ): (vector<FixedPoint64>, u64, u64, u64, FixedPoint64) acquires DelegationPool {
        let uschedule =
            borrow_global<DelegationPool>(pool_address).principle_unlock_schedule;
        (
            uschedule.schedule,
            uschedule.start_timestamp_secs,
            uschedule.period_duration,
            uschedule.last_unlock_period,
            uschedule.cumulative_unlocked_fraction
        )

    }
    
    // Create `vector<FixedPoint64>` for schedule fractions from numerators and a denominator
    // Pre-condition: It is assumed that `validate_unlock_schedule_params` is called before this
    // If the denominator is zero, this function would fail in `create_from_rational`
    fun create_schedule_fractions(unlock_numerators: &vector<u64>, unlock_denominator: u64) : vector<FixedPoint64> {
        
    //Create unlock schedule
        let schedule = vector::empty();
        vector::for_each_ref(
            unlock_numerators,
            |e| {
                let fraction =
                    fixed_point64::create_from_rational(
                        (*e as u128), (unlock_denominator as u128)
                    );
                vector::push_back(&mut schedule, fraction);
            }
        );
        
        schedule

    }

    /// Pre-condition: `cumulative_unlocked_fraction` should be zero, which would indicate that even
    /// though there are principle stake holders, none of those have yet called `unlock` on the pool
    /// thus it is ``safe'' to change the schedule
    /// This is a temporary measure to allow Supra Foundation to change the schedule for those pools
    /// there were initialized with ``dummy/default'' schedule. This method must be disabled
    /// before external validators are allowed to join the validator set.
    public entry fun update_unlocking_schedule(
        multisig_admin: &signer,
        pool_address: address,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        unlock_start_time: u64,
        unlock_duration: u64
    ) acquires DelegationPool {
        assert!(
            is_admin(signer::address_of(multisig_admin), pool_address),
            error::permission_denied(ENOT_AUTHORIZED)
        );
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        assert!(
            fixed_point64::is_zero(
                pool.principle_unlock_schedule.cumulative_unlocked_fraction
            ),
            error::invalid_state(EUNLOCKING_ALREADY_STARTED)
        );

        validate_unlock_schedule_params(
            &unlock_numerators,
            unlock_denominator,
            unlock_start_time,
            unlock_duration
        );

        //Create unlock schedule fractions
        let schedule = create_schedule_fractions(&unlock_numerators,unlock_denominator);
       
        pool.principle_unlock_schedule = UnlockSchedule {
            schedule: schedule,
            start_timestamp_secs: unlock_start_time,
            period_duration: unlock_duration,
            last_unlock_period: 0,
            cumulative_unlocked_fraction: fixed_point64::create_from_rational(0, 1)
        };
        event::emit(
            UnlockScheduleUpdated {
                pool_address,
                unlock_numerators,
                unlock_denominator,
                unlock_start_time,
                unlock_duration
            }
        );

    }

    // All sanity checks for unlock schedule parameters in one common function
    fun validate_unlock_schedule_params(
        unlock_numerators: &vector<u64>,
        unlock_denominator: u64,
        _unlock_start_time: u64,
        unlock_duration: u64
    ) {
        //Unlock duration can not be zero
        assert!(unlock_duration > 0, error::invalid_argument(EPERIOD_DURATION_IS_ZERO));
        //Fraction denominator can not be zero
        assert!(unlock_denominator != 0, error::invalid_argument(EDENOMINATOR_IS_ZERO));
        let numerator_length = vector::length(unlock_numerators);
        //Fraction numerators can not be empty
        assert!(
            numerator_length > 0,
            error::invalid_argument(EEMPTY_UNLOCK_SCHEDULE)
        );
        //First and last numerator can not be zero
        assert!(
            *vector::borrow(unlock_numerators, 0) != 0,
            error::invalid_argument(ESCHEDULE_WITH_ZERO_FRACTION)
        );
        assert!(
            *vector::borrow(unlock_numerators, numerator_length - 1) != 0,
            error::invalid_argument(ESCHEDULE_WITH_ZERO_FRACTION)
        );

        let sum = vector::foldr(*unlock_numerators, 0, |e, a| { e + a });
        //Sum of numerators can not be greater than denominators
        assert!(
            sum <= unlock_denominator,
            error::invalid_argument(ENUMERATORS_GRATER_THAN_DENOMINATOR)
        );

    }

    /// Initialize a delegation pool of custom fixed `operator_commission_percentage`.
    /// A resource account is created from `owner` signer and its supplied `delegation_pool_creation_seed`
    /// to host the delegation pool resource and own the underlying stake pool.
    /// Ownership over setting the operator/voter is granted to `owner` who has both roles initially.
    public fun initialize_delegation_pool(
        owner: &signer,
        multisig_admin: option::Option<address>,
        operator_commission_percentage: u64,
        delegation_pool_creation_seed: vector<u8>,
        delegator_address: vector<address>,
        principle_stake: vector<u64>,
        coin: Coin<SupraCoin>,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        unlock_start_time: u64,
        unlock_duration: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {

        //if there is an admin, it must be a multisig
        if (option::is_some<address>(&multisig_admin)) {
            // `ms_admin` is guaranteed to be NOT `@0x0` here
            let ms_admin = option::get_with_default<address>(&multisig_admin, @0x0);
            assert!(
                ms_admin != @0x0,
                error::invalid_argument(EADMIN_ADDRESS_CANNOT_BE_ZERO)
            );
            assert!(
                multisig_account::num_signatures_required(ms_admin) >= 2,
                EADMIN_NOT_MULTISIG
            );
        };
        // fail if the length of delegator_address and principle_stake is not the same
        assert!(
            vector::length(&delegator_address) == vector::length(&principle_stake),
            error::invalid_argument(EVECTOR_LENGTH_NOT_SAME)
        );
        //Delegation pool must be enabled
        assert!(
            features::delegation_pools_enabled(),
            error::invalid_state(EDELEGATION_POOLS_DISABLED)
        );

        
        validate_unlock_schedule_params(
            &unlock_numerators,
            unlock_denominator,
            unlock_start_time,
            unlock_duration
        );

        let owner_address = signer::address_of(owner);
        assert!(
            !owner_cap_exists(owner_address),
            error::already_exists(EOWNER_CAP_ALREADY_EXISTS)
        );
        assert!(
            operator_commission_percentage <= MAX_FEE,
            error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE)
        );

        let sum = vector::fold(principle_stake, 0, |a, e| { a + e });
        // fail if the value of coin and the sum of principle_stake is not the same
        assert!(
            coin::value(&coin) == sum,
            error::invalid_state(ECOIN_VALUE_NOT_SAME_AS_PRINCIPAL_STAKE)
        );
        // generate a seed to be used to create the resource account hosting the delegation pool
        let seed = create_resource_account_seed(delegation_pool_creation_seed);

        let (stake_pool_signer, stake_pool_signer_cap) =
            account::create_resource_account(owner, seed);
        coin::register<SupraCoin>(&stake_pool_signer);

        // stake_pool_signer will be owner of the stake pool and have its `stake::OwnerCapability`
        let pool_address = signer::address_of(&stake_pool_signer);
        stake::initialize_stake_owner(
            &stake_pool_signer,
            0,
            owner_address,
            owner_address
        );
        coin::deposit(pool_address, coin);

        let inactive_shares = table::new<ObservedLockupCycle, pool_u64::Pool>();
        table::add(
            &mut inactive_shares,
            olc_with_index(0),
            pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
        );

        let delegator_address_copy = copy delegator_address;
        let principle_stake_copy = copy principle_stake;
        // initialize the principle stake table
        let principle_stake_table = table::new<address, u64>();
        // initialize the principle stake table
        while (vector::length(&delegator_address) > 0) {
            let delegator = vector::pop_back(&mut delegator_address);
            let stake = vector::pop_back(&mut principle_stake);
            table::add(&mut principle_stake_table, delegator, stake);
        };

        //Create unlock schedule
        let schedule = create_schedule_fractions(&unlock_numerators,unlock_denominator);
        
        move_to(
            &stake_pool_signer,
            DelegationPool {
                multisig_admin: multisig_admin,
                active_shares: pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR),
                observed_lockup_cycle: olc_with_index(0),
                inactive_shares,
                pending_withdrawals: table::new<address, ObservedLockupCycle>(),
                stake_pool_signer_cap,
                total_coins_inactive: 0,
                operator_commission_percentage,
                principle_unlock_schedule: UnlockSchedule {
                    schedule: schedule,
                    start_timestamp_secs: unlock_start_time,
                    period_duration: unlock_duration,
                    last_unlock_period: 0,
                    cumulative_unlocked_fraction: fixed_point64::create_from_rational(
                        0, 1
                    )
                },
                principle_stake: principle_stake_table,
                add_stake_events: account::new_event_handle<AddStakeEvent>(
                    &stake_pool_signer
                ),
                reactivate_stake_events: account::new_event_handle<ReactivateStakeEvent>(
                    &stake_pool_signer
                ),
                unlock_stake_events: account::new_event_handle<UnlockStakeEvent>(
                    &stake_pool_signer
                ),
                withdraw_stake_events: account::new_event_handle<WithdrawStakeEvent>(
                    &stake_pool_signer
                ),
                distribute_commission_events: account::new_event_handle<
                    DistributeCommissionEvent>(&stake_pool_signer)
            }
        );

        // save delegation pool ownership and resource account address (inner stake pool address) on `owner`
        move_to(owner, DelegationPoolOwnership { pool_address });

        // Add stake to each delegator
        while (vector::length(&delegator_address_copy) > 0) {
            let delegator = vector::pop_back(&mut delegator_address_copy);
            let stake = vector::pop_back(&mut principle_stake_copy);
            add_stake_initialization(delegator, pool_address, stake);
        };
        let (active_stake, _, _, _) = stake::get_stake(pool_address);
        // fail if coin in StakePool.active does not match with the balance in active_shares pool.
        assert!(
            active_stake
                == pool_u64::total_coins(
                    &borrow_global<DelegationPool>(pool_address).active_shares
                ),
            error::invalid_state(EACTIVE_COIN_VALUE_NOT_SAME_STAKE_DELEGATION_POOL)
        );
        // All delegation pool enable partial governace voting by default once the feature flag is enabled.
        if (features::partial_governance_voting_enabled()
            && features::delegation_pool_partial_governance_voting_enabled()) {
            enable_partial_governance_voting(pool_address);
        }
    }

    public entry fun fund_delegators_with_locked_stake(
        funder: &signer,
        pool_address: address,
        delegators: vector<address>,
        stakes: vector<u64>
    ) acquires DelegationPool, BeneficiaryForOperator, GovernanceRecords, NextCommissionPercentage {
        {
            assert!(
                is_admin(signer::address_of(funder), pool_address),
                error::permission_denied(ENOT_AUTHORIZED)
            );
        };
        let principle_stake_table =
            &mut (borrow_global_mut<DelegationPool>(pool_address).principle_stake);

        vector::zip_reverse(
            delegators,
            stakes,
            |delegator, stake| {
                // Ignore if stake to be added is `0`
                if (stake > 0) {
                    // Compute the actual stake that would be added, `principle_stake` has to be
                    // populated in the table accordingly
                    if (table::contains(principle_stake_table, delegator)) {
                        let stake_amount =
                            table::borrow_mut(principle_stake_table, delegator);
                        *stake_amount = *stake_amount + stake;
                    } else {
                        table::add(principle_stake_table, delegator, stake);
                    };

                    // Record the details of the lockup event. Note that only the newly locked
                    // amount is reported and not the total locked amount.
                    event::emit(
                        UnlockScheduleApplied { pool_address, delegator, amount: stake }
                    );
                }
            }
        );

        fund_delegators_with_stake(funder, pool_address, delegators, stakes);
    }

    public entry fun fund_delegators_with_stake(
        funder: &signer,
        pool_address: address,
        delegators: vector<address>,
        stakes: vector<u64>
    ) acquires DelegationPool, BeneficiaryForOperator, GovernanceRecords, NextCommissionPercentage {
        //length equality check is performed by `zip_reverse`
        vector::zip_reverse(
            delegators,
            stakes,
            |delegator, stake| {
                fund_delegator_stake(funder, pool_address, delegator, stake);
            }
        );
    }

    #[view]
    public fun is_admin(user_addr: address, pool_address: address): bool acquires DelegationPool {
        let option_multisig = get_admin(pool_address);
        if (!option::is_some(&option_multisig)) {
            return false
        } else {
            user_addr == *option::borrow(&option_multisig)
        }
    }

    #[view]
    public fun get_admin(pool_address: address): option::Option<address> acquires DelegationPool {
        return borrow_global<DelegationPool>(pool_address).multisig_admin
    }

    #[view]
    /// Return the beneficiary address of the operator.
    public fun beneficiary_for_operator(operator: address): address acquires BeneficiaryForOperator {
        if (exists<BeneficiaryForOperator>(operator)) {
            return borrow_global<BeneficiaryForOperator>(operator).beneficiary_for_operator
        } else {
            operator
        }
    }

    /// Enable partial governance voting on a stake pool. The voter of this stake pool will be managed by this module.
    /// THe existing voter will be replaced. The function is permissionless.
    public entry fun enable_partial_governance_voting(
        pool_address: address
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert!(
            features::partial_governance_voting_enabled(),
            error::invalid_state(EDISABLED_FUNCTION)
        );
        assert!(
            features::delegation_pool_partial_governance_voting_enabled(),
            error::invalid_state(EDISABLED_FUNCTION)
        );
        assert_delegation_pool_exists(pool_address);
        // synchronize delegation and stake pools before any user operation.
        synchronize_delegation_pool(pool_address);

        let delegation_pool = borrow_global<DelegationPool>(pool_address);
        let stake_pool_signer = retrieve_stake_pool_owner(delegation_pool);
        // delegated_voter is managed by the stake pool itself, which signer capability is managed by DelegationPool.
        // So voting power of this stake pool can only be used through this module.
        stake::set_delegated_voter(
            &stake_pool_signer, signer::address_of(&stake_pool_signer)
        );

        move_to(
            &stake_pool_signer,
            GovernanceRecords {
                votes: smart_table::new(),
                votes_per_proposal: smart_table::new(),
                vote_delegation: smart_table::new(),
                delegated_votes: smart_table::new(),
                vote_events: account::new_event_handle<VoteEvent>(&stake_pool_signer),
                create_proposal_events: account::new_event_handle<CreateProposalEvent>(
                    &stake_pool_signer
                ),
                delegate_voting_power_events: account::new_event_handle<
                    DelegateVotingPowerEvent>(&stake_pool_signer)
            }
        );
    }

    fun assert_owner_cap_exists(owner: address) {
        assert!(owner_cap_exists(owner), error::not_found(EOWNER_CAP_NOT_FOUND));
    }

    fun assert_delegation_pool_exists(pool_address: address) {
        assert!(
            delegation_pool_exists(pool_address),
            error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST)
        );
    }

    fun assert_min_active_balance(
        pool: &DelegationPool, delegator_address: address
    ) {
        let balance = pool_u64::balance(&pool.active_shares, delegator_address);
        assert!(
            balance >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EDELEGATOR_ACTIVE_BALANCE_TOO_LOW)
        );
    }

    fun assert_min_pending_inactive_balance(
        pool: &DelegationPool, delegator_address: address
    ) {
        let balance =
            pool_u64::balance(pending_inactive_shares_pool(pool), delegator_address);
        assert!(
            balance >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EDELEGATOR_PENDING_INACTIVE_BALANCE_TOO_LOW)
        );
    }

    fun assert_partial_governance_voting_enabled(pool_address: address) {
        assert_delegation_pool_exists(pool_address);
        assert!(
            partial_governance_voting_enabled(pool_address),
            error::invalid_state(EPARTIAL_GOVERNANCE_VOTING_NOT_ENABLED)
        );
    }

    fun coins_to_redeem_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool, shareholder: address, amount: u64
    ): u64 {
        // find how many coins would be redeemed if supplying `amount`
        let redeemed_coins =
            pool_u64::shares_to_amount(
                src_shares_pool,
                amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
            );
        // if balance drops under threshold then redeem it entirely
        let src_balance = pool_u64::balance(src_shares_pool, shareholder);
        if (src_balance - redeemed_coins < MIN_COINS_ON_SHARES_POOL) {
            amount = src_balance;
        };
        amount
    }

    fun coins_to_transfer_to_ensure_min_stake(
        src_shares_pool: &pool_u64::Pool,
        dst_shares_pool: &pool_u64::Pool,
        shareholder: address,
        amount: u64
    ): u64 {
        // find how many coins would be redeemed from source if supplying `amount`
        let redeemed_coins =
            pool_u64::shares_to_amount(
                src_shares_pool,
                amount_to_shares_to_redeem(src_shares_pool, shareholder, amount)
            );
        // if balance on destination would be less than threshold then redeem difference to threshold
        let dst_balance = pool_u64::balance(dst_shares_pool, shareholder);
        if (dst_balance + redeemed_coins < MIN_COINS_ON_SHARES_POOL) {
            // `redeemed_coins` >= `amount` - 1 as redeem can lose at most 1 coin
            amount = MIN_COINS_ON_SHARES_POOL - dst_balance + 1;
        };
        // check if new `amount` drops balance on source under threshold and adjust
        coins_to_redeem_to_ensure_min_stake(src_shares_pool, shareholder, amount)
    }

    /// Retrieves the shared resource account owning the stake pool in order
    /// to forward a stake-management operation to this underlying pool.
    fun retrieve_stake_pool_owner(pool: &DelegationPool): signer {
        account::create_signer_with_capability(&pool.stake_pool_signer_cap)
    }

    /// Get the address of delegation pool reference `pool`.
    fun get_pool_address(pool: &DelegationPool): address {
        account::get_signer_capability_address(&pool.stake_pool_signer_cap)
    }

    /// Get the active share amount of the delegator.
    fun get_delegator_active_shares(
        pool: &DelegationPool, delegator: address
    ): u128 {
        pool_u64::shares(&pool.active_shares, delegator)
    }

    /// Get the pending inactive share amount of the delegator.
    fun get_delegator_pending_inactive_shares(
        pool: &DelegationPool, delegator: address
    ): u128 {
        pool_u64::shares(pending_inactive_shares_pool(pool), delegator)
    }

    /// Get the used voting power of a voter on a proposal.
    fun get_used_voting_power(
        governance_records: &GovernanceRecords, voter: address, proposal_id: u64
    ): u64 {
        let votes = &governance_records.votes;
        let key = VotingRecordKey { voter, proposal_id };
        *smart_table::borrow_with_default(votes, key, &0)
    }

    /// Create the seed to derive the resource account address.
    fun create_resource_account_seed(
        delegation_pool_creation_seed: vector<u8>
    ): vector<u8> {
        let seed = vector::empty<u8>();
        // include module salt (before any subseeds) to avoid conflicts with other modules creating resource accounts
        vector::append(&mut seed, MODULE_SALT);
        // include an additional salt in case the same resource account has already been created
        vector::append(&mut seed, delegation_pool_creation_seed);
        seed
    }

    /// Borrow the mutable used voting power of a voter on a proposal.
    inline fun borrow_mut_used_voting_power(
        governance_records: &mut GovernanceRecords, voter: address, proposal_id: u64
    ): &mut u64 {
        let votes = &mut governance_records.votes;
        let key = VotingRecordKey { proposal_id, voter };
        smart_table::borrow_mut_with_default(votes, key, 0)
    }

    /// Update VoteDelegation of a delegator to up-to-date then borrow_mut it.
    fun update_and_borrow_mut_delegator_vote_delegation(
        pool: &DelegationPool,
        governance_records: &mut GovernanceRecords,
        delegator: address
    ): &mut VoteDelegation {
        let pool_address = get_pool_address(pool);
        let locked_until_secs = stake::get_lockup_secs(pool_address);

        let vote_delegation_table = &mut governance_records.vote_delegation;
        // By default, a delegator's delegated voter is itself.
        // TODO: recycle storage when VoteDelegation equals to default value.
        if (!smart_table::contains(vote_delegation_table, delegator)) {
            return smart_table::borrow_mut_with_default(
                vote_delegation_table,
                delegator,
                VoteDelegation {
                    voter: delegator,
                    last_locked_until_secs: locked_until_secs,
                    pending_voter: delegator
                }
            )
        };

        let vote_delegation = smart_table::borrow_mut(vote_delegation_table, delegator);
        // A lockup period has passed since last time `vote_delegation` was updated. Pending voter takes effect.
        if (vote_delegation.last_locked_until_secs < locked_until_secs
            && vote_delegation.voter != vote_delegation.pending_voter) {
            vote_delegation.voter = vote_delegation.pending_voter;
        };
        vote_delegation
    }

    /// Update DelegatedVotes of a voter to up-to-date then borrow_mut it.
    fun update_and_borrow_mut_delegated_votes(
        pool: &DelegationPool, governance_records: &mut GovernanceRecords, voter: address
    ): &mut DelegatedVotes {
        let pool_address = get_pool_address(pool);
        let locked_until_secs = stake::get_lockup_secs(pool_address);

        let delegated_votes_per_voter = &mut governance_records.delegated_votes;
        // By default, a delegator's voter is itself.
        // TODO: recycle storage when DelegatedVotes equals to default value.
        if (!smart_table::contains(delegated_votes_per_voter, voter)) {
            let active_shares = get_delegator_active_shares(pool, voter);
            let inactive_shares = get_delegator_pending_inactive_shares(pool, voter);
            return smart_table::borrow_mut_with_default(
                delegated_votes_per_voter,
                voter,
                DelegatedVotes {
                    active_shares,
                    pending_inactive_shares: inactive_shares,
                    active_shares_next_lockup: active_shares,
                    last_locked_until_secs: locked_until_secs
                }
            )
        };

        let delegated_votes = smart_table::borrow_mut(delegated_votes_per_voter, voter);
        // A lockup period has passed since last time `delegated_votes` was updated. Pending voter takes effect.
        if (delegated_votes.last_locked_until_secs < locked_until_secs) {
            delegated_votes.active_shares = delegated_votes.active_shares_next_lockup;
            delegated_votes.pending_inactive_shares = 0;
            delegated_votes.last_locked_until_secs = locked_until_secs;
        };
        delegated_votes
    }

    fun olc_with_index(index: u64): ObservedLockupCycle {
        ObservedLockupCycle { index }
    }

    /// Given the amounts of shares in `active_shares` pool and `inactive_shares` pool, calculate the total voting
    /// power, which equals to the sum of the coin amounts.
    fun calculate_total_voting_power(
        delegation_pool: &DelegationPool, latest_delegated_votes: &DelegatedVotes
    ): u64 {
        let active_amount =
            pool_u64::shares_to_amount(
                &delegation_pool.active_shares, latest_delegated_votes.active_shares
            );
        let pending_inactive_amount =
            pool_u64::shares_to_amount(
                pending_inactive_shares_pool(delegation_pool),
                latest_delegated_votes.pending_inactive_shares
            );
        active_amount + pending_inactive_amount
    }

    /// Update VoteDelegation of a delegator to up-to-date then return the latest voter.
    fun calculate_and_update_delegator_voter_internal(
        pool: &DelegationPool,
        governance_records: &mut GovernanceRecords,
        delegator: address
    ): address {
        let vote_delegation =
            update_and_borrow_mut_delegator_vote_delegation(
                pool, governance_records, delegator
            );
        vote_delegation.voter
    }

    /// Update DelegatedVotes of a voter to up-to-date then return the total voting power of this voter.
    fun calculate_and_update_delegated_votes(
        pool: &DelegationPool, governance_records: &mut GovernanceRecords, voter: address
    ): u64 {
        let delegated_votes =
            update_and_borrow_mut_delegated_votes(pool, governance_records, voter);
        calculate_total_voting_power(pool, delegated_votes)
    }

    /// Allows an owner to change the operator of the underlying stake pool.
    public entry fun set_operator(
        owner: &signer, new_operator: address
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        // ensure the old operator is paid its uncommitted commission rewards
        synchronize_delegation_pool(pool_address);
        stake::set_operator(
            &retrieve_stake_pool_owner(borrow_global<DelegationPool>(pool_address)),
            new_operator
        );
    }

    /// Allows an operator to change its beneficiary. Any existing unpaid commission rewards will be paid to the new
    /// beneficiary. To ensures payment to the current beneficiary, one should first call `synchronize_delegation_pool`
    /// before switching the beneficiary. An operator can set one beneficiary for delegation pools, not a separate
    /// one for each pool.
    public entry fun set_beneficiary_for_operator(
        operator: &signer, new_beneficiary: address
    ) acquires BeneficiaryForOperator {
        assert!(
            features::operator_beneficiary_change_enabled(),
            std::error::invalid_state(EOPERATOR_BENEFICIARY_CHANGE_NOT_SUPPORTED)
        );
        // The beneficiay address of an operator is stored under the operator's address.
        // So, the operator does not need to be validated with respect to a staking pool.
        let operator_addr = signer::address_of(operator);
        let old_beneficiary = beneficiary_for_operator(operator_addr);
        if (exists<BeneficiaryForOperator>(operator_addr)) {
            borrow_global_mut<BeneficiaryForOperator>(operator_addr).beneficiary_for_operator =
                new_beneficiary;
        } else {
            move_to(
                operator,
                BeneficiaryForOperator { beneficiary_for_operator: new_beneficiary }
            );
        };

        emit(
            SetBeneficiaryForOperator {
                operator: operator_addr,
                old_beneficiary,
                new_beneficiary
            }
        );
    }

    /// Allows an owner to update the commission percentage for the operator of the underlying stake pool.
    public entry fun update_commission_percentage(
        owner: &signer, new_commission_percentage: u64
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert!(
            features::commission_change_delegation_pool_enabled(),
            error::invalid_state(ECOMMISSION_RATE_CHANGE_NOT_SUPPORTED)
        );
        assert!(
            new_commission_percentage <= MAX_FEE,
            error::invalid_argument(EINVALID_COMMISSION_PERCENTAGE)
        );
        let owner_address = signer::address_of(owner);
        let pool_address = get_owned_pool_address(owner_address);
        assert!(
            operator_commission_percentage(pool_address) + MAX_COMMISSION_INCREASE
                >= new_commission_percentage,
            error::invalid_argument(ETOO_LARGE_COMMISSION_INCREASE)
        );
        assert!(
            stake::get_remaining_lockup_secs(pool_address)
                >= min_remaining_secs_for_commission_change(),
            error::invalid_state(ETOO_LATE_COMMISSION_CHANGE)
        );

        // synchronize delegation and stake pools before any user operation. this ensures:
        // (1) the operator is paid its uncommitted commission rewards with the old commission percentage, and
        // (2) any pending commission percentage change is applied before the new commission percentage is set.
        synchronize_delegation_pool(pool_address);

        if (exists<NextCommissionPercentage>(pool_address)) {
            let commission_percentage =
                borrow_global_mut<NextCommissionPercentage>(pool_address);
            commission_percentage.commission_percentage_next_lockup_cycle = new_commission_percentage;
            commission_percentage.effective_after_secs = stake::get_lockup_secs(
                pool_address
            );
        } else {
            let delegation_pool = borrow_global<DelegationPool>(pool_address);
            let pool_signer =
                account::create_signer_with_capability(
                    &delegation_pool.stake_pool_signer_cap
                );
            move_to(
                &pool_signer,
                NextCommissionPercentage {
                    commission_percentage_next_lockup_cycle: new_commission_percentage,
                    effective_after_secs: stake::get_lockup_secs(pool_address)
                }
            );
        };

        event::emit(
            CommissionPercentageChange {
                pool_address,
                owner: owner_address,
                commission_percentage_next_lockup_cycle: new_commission_percentage
            }
        );
    }

    /// Allows an owner to change the delegated voter of the underlying stake pool.
    public entry fun set_delegated_voter(
        owner: &signer, new_voter: address
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        // No one can change delegated_voter once the partial governance voting feature is enabled.
        assert!(
            !features::delegation_pool_partial_governance_voting_enabled(),
            error::invalid_state(EDEPRECATED_FUNCTION)
        );
        let pool_address = get_owned_pool_address(signer::address_of(owner));
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);
        stake::set_delegated_voter(
            &retrieve_stake_pool_owner(borrow_global<DelegationPool>(pool_address)),
            new_voter
        );
    }

    /// Allows a delegator to delegate its voting power to a voter. If this delegator already has a delegated voter,
    /// this change won't take effects until the next lockup period.
    public entry fun delegate_voting_power(
        delegator: &signer, pool_address: address, new_voter: address
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert_partial_governance_voting_enabled(pool_address);

        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        let delegator_address = signer::address_of(delegator);
        let delegation_pool = borrow_global<DelegationPool>(pool_address);
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let delegator_vote_delegation =
            update_and_borrow_mut_delegator_vote_delegation(
                delegation_pool, governance_records, delegator_address
            );
        let pending_voter: address = delegator_vote_delegation.pending_voter;

        // No need to update if the voter doesn't really change.
        if (pending_voter != new_voter) {
            delegator_vote_delegation.pending_voter = new_voter;
            let active_shares =
                get_delegator_active_shares(delegation_pool, delegator_address);
            // <active shares> of <pending voter of shareholder> -= <active_shares>
            // <active shares> of <new voter of shareholder> += <active_shares>
            let pending_delegated_votes =
                update_and_borrow_mut_delegated_votes(
                    delegation_pool, governance_records, pending_voter
                );
            pending_delegated_votes.active_shares_next_lockup = pending_delegated_votes.active_shares_next_lockup
                - active_shares;

            let new_delegated_votes =
                update_and_borrow_mut_delegated_votes(
                    delegation_pool, governance_records, new_voter
                );
            new_delegated_votes.active_shares_next_lockup = new_delegated_votes.active_shares_next_lockup
                + active_shares;
        };

        event::emit_event(
            &mut governance_records.delegate_voting_power_events,
            DelegateVotingPowerEvent {
                pool_address,
                delegator: delegator_address,
                voter: new_voter
            }
        );
    }

    /// Add `amount` of coins to the delegation pool `pool_address` during initialization of pool.
    fun add_stake_initialization(
        delegator_address: address, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords {
        // short-circuit if amount to add is 0 so no event is emitted
        if (amount == 0) { return };

        let pool = borrow_global_mut<DelegationPool>(pool_address);

        stake::add_stake(&retrieve_stake_pool_owner(pool), amount);

        buy_in_active_shares(pool, delegator_address, amount);
        assert_min_active_balance(pool, delegator_address);
    }

    fun fund_delegator_stake(
        funder: &signer,
        pool_address: address,
        delegator_address: address,
        amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        // short-circuit if amount to add is 0 so no event is emitted
        if (amount == 0) { return };
        // fail unlock of less than `MIN_COINS_ON_SHARES_POOL`
        assert!(
            amount >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EMINIMUM_UNLOCK_AMOUNT)
        );
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        // fee to be charged for adding `amount` stake on this delegation pool at this epoch
        let add_stake_fee = get_add_stake_fee(pool_address, amount);

        supra_account::transfer(funder, pool_address, amount);
        let pool = borrow_global_mut<DelegationPool>(pool_address);

        // stake the entire amount to the stake pool
        stake::add_stake(&retrieve_stake_pool_owner(pool), amount);

        // but buy shares for delegator just for the remaining amount after fee
        buy_in_active_shares(pool, delegator_address, amount - add_stake_fee);
        assert_min_active_balance(pool, delegator_address);

        // grant temporary ownership over `add_stake` fees to a separate shareholder in order to:
        // - not mistake them for rewards to pay the operator from
        // - distribute them together with the `active` rewards when this epoch ends
        // in order to appreciate all shares on the active pool atomically
        buy_in_active_shares(pool, NULL_SHAREHOLDER, add_stake_fee);

        event::emit_event(
            &mut pool.add_stake_events,
            AddStakeEvent {
                pool_address,
                delegator_address,
                amount_added: amount,
                add_stake_fee
            }
        );

    }

    /// Add `amount` of coins to the delegation pool `pool_address`.
    public entry fun add_stake(
        delegator: &signer, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        fund_delegator_stake(
            delegator,
            pool_address,
            signer::address_of(delegator),
            amount
        )
    }

    fun replace_in_smart_tables<Key: copy + drop, Val>(
        table: &mut SmartTable<Key, Val>,
        old_entry: Key,
        new_entry: Key
    ) {
        if (smart_table::contains(table, old_entry)) {
            let val = smart_table::remove(table, old_entry);
            smart_table::add(table, new_entry, val);
        }
    }

    /// Reactivates the `pending_inactive` stake of `delegator`.
    ///
    /// This function must remain private because it must only be called by an authorized entity and it is the
    /// callers responsibility to ensure that this is true. Authorized entities currently include the delegator
    /// itself and the multisig admin of the delegation pool, which must be controlled by The Supra Foundation.
    ///
    /// Note that this function is only temporarily intended to work as specified above and exists to enable The
    /// Supra Foundation to ensure that the allocations of all investors are subject to the terms specified in the
    /// corresponding legal contracts. It will be deactivated before the validator set it opened up to external
    /// validator-owners to prevent it from being abused, from which time forward only the delegator will be
    /// authorized to reactivate their own stake.
    fun authorized_reactivate_stake(
        delegator: address, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        // short-circuit if amount to reactivate is 0 so no event is emitted
        if (amount == 0) { return };
        // fail unlock of less than `MIN_COINS_ON_SHARES_POOL`
        assert!(
            amount >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EMINIMUM_UNLOCK_AMOUNT)
        );
        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        let pool = borrow_global_mut<DelegationPool>(pool_address);

        amount = coins_to_transfer_to_ensure_min_stake(
            pending_inactive_shares_pool(pool),
            &pool.active_shares,
            delegator,
            amount
        );
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        amount = redeem_inactive_shares(pool, delegator, amount, observed_lockup_cycle);

        stake::reactivate_stake(&retrieve_stake_pool_owner(pool), amount);

        buy_in_active_shares(pool, delegator, amount);
        assert_min_active_balance(pool, delegator);

        event::emit_event(
            &mut pool.reactivate_stake_events,
            ReactivateStakeEvent {
                pool_address,
                delegator_address: delegator,
                amount_reactivated: amount
            }
        );
    }

    /// Withdraws the specified `amount` from the `inactive` stake belonging to the given `delegator_address`
    /// to the address of the `DelegationPool`'s `multisig_admin`, if available.
    ///
    /// Note that this function is only temporarily intended to work as specified above and exists to enable The
    /// Supra Foundation to ensure that the allocations of all investors are subject to the terms specified in the
    /// corresponding legal contracts. It will be deactivated before the validator set it opened up to external
    /// validator-owners to prevent it from being abused.
    fun admin_withdraw(
        multisig_admin: &signer,
        pool_address: address,
        delegator_address: address,
        amount: u64
    ) acquires DelegationPool, GovernanceRecords {
        // Ensure that the caller is the admin of the delegation pool.
        {
            assert!(
                is_admin(signer::address_of(multisig_admin), pool_address),
                error::permission_denied(ENOT_AUTHORIZED)
            );
        };
        assert!(amount > 0, error::invalid_argument(EWITHDRAW_ZERO_STAKE));
        withdraw_internal(
            borrow_global_mut<DelegationPool>(pool_address),
            delegator_address,
            amount,
            signer::address_of(multisig_admin)
        );
    }

    /// Updates the `principle_stake` of each `delegator` in `delegators` according to the amount specified
    /// at the corresponding index of `new_principle_stakes`. Also ensures that the `delegator`'s `active` stake
    /// is as close to the specified amount as possible. The locked amount is subject to the vesting schedule
    /// specified when the delegation pool corresponding to `pool_address` was created.
    ///
    /// Note that this function is only temporarily intended to work as specified above and exists to enable The
    /// Supra Foundation to ensure that the allocations of all investors are subject to the terms specified in the
    /// corresponding legal contracts. It will be deactivated before the validator set it opened up to external
    /// validator-owners to prevent it from being abused.
    public entry fun lock_delegators_stakes(
        multisig_admin: &signer,
        pool_address: address,
        delegators: vector<address>,
        new_principle_stakes: vector<u64>
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        // Ensure that the caller is the admin of the delegation pool.
        {
            assert!(
                is_admin(signer::address_of(multisig_admin), pool_address),
                error::permission_denied(ENOT_AUTHORIZED)
            );
        };

        // Synchronize the delegation and stake pools before any user operation.
        synchronize_delegation_pool(pool_address);

        // Ensure that each `delegator` has an `active` stake balance that is as close to
        // `principle_stake`  as possible.
        vector::zip_reverse(
            delegators,
            new_principle_stakes,
            |delegator, principle_stake| {
                let (active, inactive, pending_inactive) =
                    get_stake(pool_address, delegator);

                // Ensure that all stake to be locked is made `active`.
                if (active < principle_stake) {
                    // The amount to lock can be covered by reactivating some previously unlocked stake.
                    // Only reactivate the required amount to avoid unnecessarily interfering with
                    // in-progress withdrawals.
                    let amount_to_reactivate = principle_stake - active;

                    // Ensure that we do not try to reactivate more than the available `pending_inactive` stake.
                    // This should be enforced by functions within `authorized_reactivate_stake`, but checking
                    // again here makes the correctness of this function easier to reason about.
                    if (amount_to_reactivate > pending_inactive) {
                        amount_to_reactivate = pending_inactive;
                    };

                    if (amount_to_reactivate > MIN_COINS_ON_SHARES_POOL) {
                        // Reactivate the required amount of `pending_inactive` stake first.
                        authorized_reactivate_stake(
                            delegator, pool_address, amount_to_reactivate
                        );
                    };

                    let active_and_pending_inactive = active + pending_inactive;

                    if (active_and_pending_inactive < principle_stake) {
                        // Need to reactivate some of the `inactive` stake.
                        let amount_to_withdraw =
                            principle_stake - active_and_pending_inactive;

                        // Ensure that we do not try to withdraw more stake than the `inactive` stake.
                        if (amount_to_withdraw > inactive) {
                            amount_to_withdraw = inactive;
                        };

                        if (amount_to_withdraw > MIN_COINS_ON_SHARES_POOL) {
                            // Withdraw the minimum required amount to the admin's address.
                            admin_withdraw(
                                multisig_admin,
                                pool_address,
                                delegator,
                                amount_to_withdraw
                            );
                            // Then allocate it to the delegator again.
                            fund_delegator_stake(
                                multisig_admin,
                                pool_address,
                                delegator,
                                amount_to_withdraw
                            );
                        }
                    }
                };
                // else: The amount to lock can be covered by the currently `active` stake.

                // Update the delegator's principle stake and record the details of the lockup event.
                let principle_stake_table =
                    &mut (borrow_global_mut<DelegationPool>(pool_address).principle_stake);
                table::upsert(principle_stake_table, delegator, principle_stake);
                event::emit(
                    UnlockScheduleApplied {
                        pool_address,
                        delegator,
                        amount: principle_stake
                    }
                );
            }
        );
    }

    ///CAUTION: This is to be used only in the rare circumstances where multisig_admin is convinced that a delegator was the
    /// rightful owner of `old_delegator` but has lost access and the delegator is also the rightful
    /// owner of `new_delegator` , Only for those stakeholders which were added at the time of creation
    /// This does not apply to anyone who added stake later or operator
    ///
    /// Note that this function is only temporarily intended to work as specified above and exists to enable The
    /// Supra Foundation to ensure that the allocations of all investors are subject to the terms specified in the
    /// corresponding legal contracts. It will be deactivated before the validator set it opened up to external
    /// validator-owners to prevent it from being abused.
    public entry fun replace_delegator(
        multisig_admin: &signer,
        pool_address: address,
        old_delegator: address,
        new_delegator: address
    ) acquires DelegationPool, GovernanceRecords {

        //Ensure that authorized admin is calling
        let admin_addr = signer::address_of(multisig_admin);
        assert!(
            is_admin(admin_addr, pool_address),
            error::permission_denied(ENOT_AUTHORIZED)
        );
        //Ensure replacement address is different
        assert!(
            old_delegator != new_delegator,
            error::invalid_argument(ENEW_IS_SAME_AS_OLD_DELEGATOR)
        );
        //Ensure it is a valid `pool_addres`
        assert!(
            exists<DelegationPool>(pool_address),
            error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST)
        );

        let pool: &mut DelegationPool = borrow_global_mut<DelegationPool>(pool_address);
        //Ensure `old_delegator` is part of original principle stakers before commencing the replacement
        assert!(
            table::contains(&pool.principle_stake, old_delegator),
            error::unavailable(EDELEGATOR_DOES_NOT_EXIST)
        );

        //replace in `active_shares` pool
        {
            let active_pool = &mut pool.active_shares;
            let active_shares = pool_u64::shares(active_pool, old_delegator);
            pool_u64::transfer_shares(
                active_pool,
                old_delegator,
                new_delegator,
                active_shares
            );
        };

        //replace in `inactive_shares` pool
        let (withdrawal_exists, withdrawal_olc) =
            pending_withdrawal_exists(pool, old_delegator);
        if (withdrawal_exists) {
            let inactive_pool =
                table::borrow_mut(&mut pool.inactive_shares, withdrawal_olc);
            let inactive_shares = pool_u64::shares(inactive_pool, old_delegator);
            pool_u64::transfer_shares(
                inactive_pool,
                old_delegator,
                new_delegator,
                inactive_shares
            );

            //replace in `pending_withdrawals`
            {
                let pending_withdrawals = &mut pool.pending_withdrawals;
                let val = table::remove(pending_withdrawals, old_delegator);
                table::add(pending_withdrawals, new_delegator, val);
            };

        };

        //replace in governance records
        {
            if (features::partial_governance_voting_enabled()) {
                let grecords = borrow_global_mut<GovernanceRecords>(pool_address);
                replace_in_smart_tables(
                    &mut grecords.vote_delegation, old_delegator, new_delegator
                );
                replace_in_smart_tables(
                    &mut grecords.delegated_votes, old_delegator, new_delegator
                );
                let old_keys: vector<VotingRecordKey> = vector::empty();
                let new_keys: vector<VotingRecordKey> = vector::empty();
                smart_table::for_each_ref<VotingRecordKey, u64>(
                    &grecords.votes,
                    |key, _val| {
                        let VotingRecordKey { voter, proposal_id } = *key;
                        if (voter == old_delegator) {
                            vector::push_back(
                                &mut new_keys,
                                VotingRecordKey {
                                    voter: new_delegator,
                                    proposal_id: proposal_id
                                }
                            );
                            vector::push_back(&mut old_keys, *key);
                        };

                    }
                );

                vector::zip_ref(
                    &old_keys,
                    &new_keys,
                    |old, new| {
                        replace_in_smart_tables(&mut grecords.votes, *old, *new);
                    }
                );
            }
        };
        // replace in principle_stake table
        {
            let val = table::remove(&mut pool.principle_stake, old_delegator);
            table::add(&mut pool.principle_stake, new_delegator, val);
        };

        event::emit(
            DelegatorReplacemendEvent { pool_address, old_delegator, new_delegator }
        );

    }

    #[view]
    public fun is_principle_stakeholder(
        delegator_addr: address, pool_addr: address
    ): bool acquires DelegationPool {
        let pool = borrow_global<DelegationPool>(pool_addr);
        table::contains(&pool.principle_stake, delegator_addr)
    }

    #[view]
    public fun get_principle_stake(
        delegator_addr: address, pool_addr: address
    ): u64 acquires DelegationPool {
        let pool = borrow_global<DelegationPool>(pool_addr);
        if (!table::contains(&pool.principle_stake, delegator_addr)) { 0 }
        else {
            *table::borrow(&pool.principle_stake, delegator_addr)
        }
    }

    #[view]
    /// Provides how much amount is unlockable based on `principle_unlock_schedule.cumulative_unlocked_fraction`
    /// Note that `cumulative_unlocked_fraction` is not updated in this function so the information may not be
    /// accurate as time passes
    public fun cached_unlockable_balance(
        delegator_addr: address, pool_addr: address
    ): u64 acquires DelegationPool {
        assert!(
            exists<DelegationPool>(pool_addr),
            error::invalid_argument(EDELEGATION_POOL_DOES_NOT_EXIST)
        );
        let pool = borrow_global<DelegationPool>(pool_addr);
        let delegator_active_balance =
            pool_u64::balance(&pool.active_shares, delegator_addr);
        let unlockable_fraction =
            pool.principle_unlock_schedule.cumulative_unlocked_fraction;
        let delegator_principle_stake =
            *table::borrow(&pool.principle_stake, delegator_addr);

        //To avoid problem even if fraction is slightly above 1
        let unlockable_principle_stake =
            (
                math128::min(
                    fixed_point64::multiply_u128(
                        (delegator_principle_stake as u128), unlockable_fraction
                    ),
                    (delegator_principle_stake as u128)
                ) as u64
            );
        let locked_amount = delegator_principle_stake - unlockable_principle_stake;

        assert!(
            delegator_active_balance >= locked_amount,
            error::invalid_state(EDELEGATOR_ACTIVE_BALANCE_TOO_LOW)
        );
        delegator_active_balance - locked_amount

    }

    /// Note: this does not synchronize with stake pool, therefore the answer may be conservative
    // This function may return `false` even if the amount is indeed `unlockable`
    public fun can_principle_unlock(
        delegator_addr: address, pool_address: address, amount: u64
    ): bool acquires DelegationPool {

        let principle_stake_table =
            &borrow_global<DelegationPool>(pool_address).principle_stake;

        if (!table::contains(principle_stake_table, delegator_addr)) {
            return false
        };

        let unlock_schedule =
            &mut borrow_global_mut<DelegationPool>(pool_address).principle_unlock_schedule;
        let one = fixed_point64::create_from_rational(1, 1);
        if (fixed_point64::greater_or_equal(
            unlock_schedule.cumulative_unlocked_fraction, one
        )) {
            return true
        };
        if (unlock_schedule.start_timestamp_secs > timestamp::now_seconds()) {
            let unlockable_amount =
                cached_unlockable_balance(delegator_addr, pool_address);
            return amount <= unlockable_amount
        };

        //subtraction safety due to check above
        let unlock_periods_passed =
            (timestamp::now_seconds() - unlock_schedule.start_timestamp_secs)
                / unlock_schedule.period_duration;
        let last_unlocked_period = unlock_schedule.last_unlock_period;
        let schedule_length = vector::length(&unlock_schedule.schedule);
        let cfraction = unlock_schedule.cumulative_unlocked_fraction;
        while (last_unlocked_period < unlock_periods_passed
            && fixed_point64::less(cfraction, one)
            && last_unlocked_period < schedule_length) {
            let next_fraction =
                *vector::borrow(&unlock_schedule.schedule, last_unlocked_period);
            cfraction = fixed_point64::add(cfraction, next_fraction);
            last_unlocked_period = last_unlocked_period + 1;
        };
        if (last_unlocked_period < unlock_periods_passed
            && fixed_point64::less(cfraction, one)) {
            let final_fraction =
                *vector::borrow(&unlock_schedule.schedule, schedule_length - 1);
            // Acclerate calculation to current period and don't update last_unlocked_period since it is not used anymore
            cfraction = fixed_point64::add(
                cfraction,
                fixed_point64::multiply_u128_return_fixpoint64(
                    (unlock_periods_passed - last_unlocked_period as u128),
                    final_fraction
                )
            );
            cfraction = fixed_point64::min(cfraction, one);
        };
        unlock_schedule.cumulative_unlocked_fraction = cfraction;
        unlock_schedule.last_unlock_period = unlock_periods_passed;
        let unlockable_amount = cached_unlockable_balance(delegator_addr, pool_address);
        amount <= unlockable_amount
    }

    /// Unlock `amount` from the active + pending_active stake of `delegator` or
    /// at most how much active stake there is on the stake pool.
    public entry fun unlock(
        delegator: &signer, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        // short-circuit if amount to unlock is 0 so no event is emitted
        if (amount == 0) { return };
        // fail unlock of less than `MIN_COINS_ON_SHARES_POOL`
        assert!(
            amount >= MIN_COINS_ON_SHARES_POOL,
            error::invalid_argument(EMINIMUM_UNLOCK_AMOUNT)
        );
        // fail unlock of more stake than `active` on the stake pool
        let (active, _, _, _) = stake::get_stake(pool_address);
        assert!(
            amount <= active,
            error::invalid_argument(ENOT_ENOUGH_ACTIVE_STAKE_TO_UNLOCK)
        );

        // synchronize delegation and stake pools before any user operation
        synchronize_delegation_pool(pool_address);

        let delegator_address = signer::address_of(delegator);
        // fail if the amount after withdraw is less than the principle stake and the lockup time is not expired
        if (is_principle_stakeholder(delegator_address, pool_address)) {
            assert!(
                can_principle_unlock(delegator_address, pool_address, amount),
                error::invalid_argument(EAMOUNT_REQUESTED_NOT_UNLOCKABLE)
            );
        };
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        amount = coins_to_transfer_to_ensure_min_stake(
            &pool.active_shares,
            pending_inactive_shares_pool(pool),
            delegator_address,
            amount
        );
        amount = redeem_active_shares(pool, delegator_address, amount);
        stake::unlock(&retrieve_stake_pool_owner(pool), amount);

        buy_in_pending_inactive_shares(pool, delegator_address, amount);
        assert_min_pending_inactive_balance(pool, delegator_address);

        event::emit_event(
            &mut pool.unlock_stake_events,
            UnlockStakeEvent { pool_address, delegator_address, amount_unlocked: amount }
        );
        let (active_stake, _, pending_active, _) = stake::get_stake(pool_address);
        assert!(
            active_stake + pending_active == pool_u64::total_coins(&pool.active_shares),
            error::invalid_state(EACTIVE_COIN_VALUE_NOT_SAME_STAKE_DELEGATION_POOL)
        );
    }

    /// Move `amount` of coins from pending_inactive to active.
    public entry fun reactivate_stake(
        delegator: &signer, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        let delegator_address = signer::address_of(delegator);
        authorized_reactivate_stake(delegator_address, pool_address, amount)
    }

    /// Withdraw `amount` of owned inactive stake from the delegation pool at `pool_address`.
    public entry fun withdraw(
        delegator: &signer, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert!(amount > 0, error::invalid_argument(EWITHDRAW_ZERO_STAKE));
        // Synchronize the delegation and stake pools before any user operation.
        synchronize_delegation_pool(pool_address);
        let delegator_address = signer::address_of(delegator);
        withdraw_internal(
            borrow_global_mut<DelegationPool>(pool_address),
            delegator_address,
            amount,
            delegator_address
        );
    }

    // TODO: `recipient_address` must be removed and replaced with `delegator_address` before the
    // validator set is opened to non-Foundation validator-owners.
    fun withdraw_internal(
        pool: &mut DelegationPool,
        delegator_address: address,
        amount: u64,
        recipient_address: address
    ) acquires GovernanceRecords {
        // TODO: recycle storage when a delegator fully exits the delegation pool.
        // short-circuit if amount to withdraw is 0 so no event is emitted
        if (amount == 0) { return };

        let pool_address = get_pool_address(pool);
        let (withdrawal_exists, withdrawal_olc) =
            pending_withdrawal_exists(pool, delegator_address);
        // exit if no withdrawal or (it is pending and cannot withdraw pending_inactive stake from stake pool)
        if (!(
            withdrawal_exists
                && (
                    withdrawal_olc.index < pool.observed_lockup_cycle.index
                        || can_withdraw_pending_inactive(pool_address)
                )
        )) { return };

        if (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
            amount = coins_to_redeem_to_ensure_min_stake(
                pending_inactive_shares_pool(pool),
                delegator_address,
                amount
            )
        };
        amount = redeem_inactive_shares(
            pool,
            delegator_address,
            amount,
            withdrawal_olc
        );

        let stake_pool_owner = &retrieve_stake_pool_owner(pool);
        // stake pool will inactivate entire pending_inactive stake at `stake::withdraw` to make it withdrawable
        // however, bypassing the inactivation of excess stake (inactivated but not withdrawn) ensures
        // the OLC is not advanced indefinitely on `unlock`-`withdraw` paired calls
        if (can_withdraw_pending_inactive(pool_address)) {
            // get excess stake before being entirely inactivated
            let (_, _, _, pending_inactive) = stake::get_stake(pool_address);
            if (withdrawal_olc.index == pool.observed_lockup_cycle.index) {
                // `amount` less excess if withdrawing pending_inactive stake
                pending_inactive = pending_inactive - amount
            };
            // escape excess stake from inactivation
            stake::reactivate_stake(stake_pool_owner, pending_inactive);
            stake::withdraw(stake_pool_owner, amount);
            // restore excess stake to the pending_inactive state
            stake::unlock(stake_pool_owner, pending_inactive);
        } else {
            // no excess stake if `stake::withdraw` does not inactivate at all
            stake::withdraw(stake_pool_owner, amount);
        };
        supra_account::transfer(stake_pool_owner, recipient_address, amount);

        // commit withdrawal of possibly inactive stake to the `total_coins_inactive`
        // known by the delegation pool in order to not mistake it for slashing at next synchronization
        let (_, inactive, _, _) = stake::get_stake(pool_address);
        pool.total_coins_inactive = inactive;

        event::emit_event(
            &mut pool.withdraw_stake_events,
            WithdrawStakeEvent { pool_address, delegator_address, amount_withdrawn: amount }
        );
    }

    /// Return the unique observed lockup cycle where delegator `delegator_address` may have
    /// unlocking (or already unlocked) stake to be withdrawn from delegation pool `pool`.
    /// A bool is returned to signal if a pending withdrawal exists at all.
    fun pending_withdrawal_exists(
        pool: &DelegationPool, delegator_address: address
    ): (bool, ObservedLockupCycle) {
        if (table::contains(&pool.pending_withdrawals, delegator_address)) {
            (true, *table::borrow(&pool.pending_withdrawals, delegator_address))
        } else {
            (false, olc_with_index(0))
        }
    }

    /// Return a mutable reference to the shares pool of `pending_inactive` stake on the
    /// delegation pool, always the last item in `inactive_shares`.
    fun pending_inactive_shares_pool_mut(pool: &mut DelegationPool): &mut pool_u64::Pool {
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        table::borrow_mut(&mut pool.inactive_shares, observed_lockup_cycle)
    }

    fun pending_inactive_shares_pool(pool: &DelegationPool): &pool_u64::Pool {
        table::borrow(&pool.inactive_shares, pool.observed_lockup_cycle)
    }

    /// Execute the pending withdrawal of `delegator_address` on delegation pool `pool`
    /// if existing and already inactive to allow the creation of a new one.
    /// `pending_inactive` stake would be left untouched even if withdrawable and should
    /// be explicitly withdrawn by delegator
    fun execute_pending_withdrawal(
        pool: &mut DelegationPool, delegator_address: address
    ) acquires GovernanceRecords {
        let (withdrawal_exists, withdrawal_olc) =
            pending_withdrawal_exists(pool, delegator_address);
        if (withdrawal_exists
            && withdrawal_olc.index < pool.observed_lockup_cycle.index) {
            withdraw_internal(
                pool,
                delegator_address,
                MAX_U64,
                delegator_address
            );
        }
    }

    /// Buy shares into the active pool on behalf of delegator `shareholder` who
    /// deposited `coins_amount`. This function doesn't make any coin transfer.
    fun buy_in_active_shares(
        pool: &mut DelegationPool, shareholder: address, coins_amount: u64
    ): u128 acquires GovernanceRecords {
        let new_shares = pool_u64::amount_to_shares(&pool.active_shares, coins_amount);
        // No need to buy 0 shares.
        if (new_shares == 0) {
            return 0
        };

        // Always update governance records before any change to the shares pool.
        let pool_address = get_pool_address(pool);
        if (partial_governance_voting_enabled(pool_address)) {
            update_governance_records_for_buy_in_active_shares(
                pool, pool_address, new_shares, shareholder
            );
        };

        pool_u64::buy_in(&mut pool.active_shares, shareholder, coins_amount);
        new_shares
    }

    /// Buy shares into the pending_inactive pool on behalf of delegator `shareholder` who
    /// redeemed `coins_amount` from the active pool to schedule it for unlocking.
    /// If delegator's pending withdrawal exists and has been inactivated, execute it firstly
    /// to ensure there is always only one withdrawal request.
    fun buy_in_pending_inactive_shares(
        pool: &mut DelegationPool, shareholder: address, coins_amount: u64
    ): u128 acquires GovernanceRecords {
        let new_shares =
            pool_u64::amount_to_shares(
                pending_inactive_shares_pool(pool), coins_amount
            );
        // never create a new pending withdrawal unless delegator owns some pending_inactive shares
        if (new_shares == 0) {
            return 0
        };

        // Always update governance records before any change to the shares pool.
        let pool_address = get_pool_address(pool);
        if (partial_governance_voting_enabled(pool_address)) {
            update_governance_records_for_buy_in_pending_inactive_shares(
                pool, pool_address, new_shares, shareholder
            );
        };

        // cannot buy inactive shares, only pending_inactive at current lockup cycle
        pool_u64::buy_in(
            pending_inactive_shares_pool_mut(pool), shareholder, coins_amount
        );

        // execute the pending withdrawal if exists and is inactive before creating a new one
        execute_pending_withdrawal(pool, shareholder);

        // save observed lockup cycle for the new pending withdrawal
        let observed_lockup_cycle = pool.observed_lockup_cycle;
        assert!(
            *table::borrow_mut_with_default(
                &mut pool.pending_withdrawals, shareholder, observed_lockup_cycle
            ) == observed_lockup_cycle,
            error::invalid_state(EPENDING_WITHDRAWAL_EXISTS)
        );

        new_shares
    }

    /// Convert `coins_amount` of coins to be redeemed from shares pool `shares_pool`
    /// to the exact number of shares to redeem in order to achieve this.
    fun amount_to_shares_to_redeem(
        shares_pool: &pool_u64::Pool, shareholder: address, coins_amount: u64
    ): u128 {
        if (coins_amount >= pool_u64::balance(shares_pool, shareholder)) {
            // cap result at total shares of shareholder to pass `EINSUFFICIENT_SHARES` on subsequent redeem
            pool_u64::shares(shares_pool, shareholder)
        } else {
            pool_u64::amount_to_shares(shares_pool, coins_amount)
        }
    }

    /// Redeem shares from the active pool on behalf of delegator `shareholder` who
    /// wants to unlock `coins_amount` of its active stake.
    /// Extracted coins will be used to buy shares into the pending_inactive pool and
    /// be available for withdrawal when current OLC ends.
    fun redeem_active_shares(
        pool: &mut DelegationPool, shareholder: address, coins_amount: u64
    ): u64 acquires GovernanceRecords {
        let shares_to_redeem =
            amount_to_shares_to_redeem(&pool.active_shares, shareholder, coins_amount);
        // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
        if (shares_to_redeem == 0) return 0;

        // Always update governance records before any change to the shares pool.
        let pool_address = get_pool_address(pool);
        if (partial_governance_voting_enabled(pool_address)) {
            update_governanace_records_for_redeem_active_shares(
                pool,
                pool_address,
                shares_to_redeem,
                shareholder
            );
        };

        pool_u64::redeem_shares(&mut pool.active_shares, shareholder, shares_to_redeem)
    }

    /// Redeem shares from the inactive pool at `lockup_cycle` < current OLC on behalf of
    /// delegator `shareholder` who wants to withdraw `coins_amount` of its unlocked stake.
    /// Redeem shares from the pending_inactive pool at `lockup_cycle` == current OLC on behalf of
    /// delegator `shareholder` who wants to reactivate `coins_amount` of its unlocking stake.
    /// For latter case, extracted coins will be used to buy shares into the active pool and
    /// escape inactivation when current lockup ends.
    fun redeem_inactive_shares(
        pool: &mut DelegationPool,
        shareholder: address,
        coins_amount: u64,
        lockup_cycle: ObservedLockupCycle
    ): u64 acquires GovernanceRecords {
        let shares_to_redeem =
            amount_to_shares_to_redeem(
                table::borrow(&pool.inactive_shares, lockup_cycle),
                shareholder,
                coins_amount
            );
        // silently exit if not a shareholder otherwise redeem would fail with `ESHAREHOLDER_NOT_FOUND`
        if (shares_to_redeem == 0) return 0;

        // Always update governance records before any change to the shares pool.
        let pool_address = get_pool_address(pool);
        // Only redeem shares from the pending_inactive pool at `lockup_cycle` == current OLC.
        if (partial_governance_voting_enabled(pool_address)
            && lockup_cycle.index == pool.observed_lockup_cycle.index) {
            update_governanace_records_for_redeem_pending_inactive_shares(
                pool,
                pool_address,
                shares_to_redeem,
                shareholder
            );
        };

        let inactive_shares = table::borrow_mut(&mut pool.inactive_shares, lockup_cycle);
        // 1. reaching here means delegator owns inactive/pending_inactive shares at OLC `lockup_cycle`
        let redeemed_coins =
            pool_u64::redeem_shares(inactive_shares, shareholder, shares_to_redeem);

        // if entirely reactivated pending_inactive stake or withdrawn inactive one,
        // re-enable unlocking for delegator by deleting this pending withdrawal
        if (pool_u64::shares(inactive_shares, shareholder) == 0) {
            // 2. a delegator owns inactive/pending_inactive shares only at the OLC of its pending withdrawal
            // 1 & 2: the pending withdrawal itself has been emptied of shares and can be safely deleted
            table::remove(&mut pool.pending_withdrawals, shareholder);
        };
        // destroy inactive shares pool of past OLC if all its stake has been withdrawn
        if (lockup_cycle.index < pool.observed_lockup_cycle.index
            && total_coins(inactive_shares) == 0) {
            pool_u64::destroy_empty(
                table::remove(&mut pool.inactive_shares, lockup_cycle)
            );
        };

        redeemed_coins
    }

    /// Calculate stake deviations between the delegation and stake pools in order to
    /// capture the rewards earned in the meantime, resulted operator commission and
    /// whether the lockup expired on the stake pool.
    fun calculate_stake_pool_drift(pool: &DelegationPool): (bool, u64, u64, u64, u64) {
        let (active, inactive, pending_active, pending_inactive) =
            stake::get_stake(get_pool_address(pool));
        assert!(
            inactive >= pool.total_coins_inactive,
            error::invalid_state(ESLASHED_INACTIVE_STAKE_ON_PAST_OLC)
        );
        // determine whether a new lockup cycle has been ended on the stake pool and
        // inactivated SOME `pending_inactive` stake which should stop earning rewards now,
        // thus requiring separation of the `pending_inactive` stake on current observed lockup
        // and the future one on the newly started lockup
        let lockup_cycle_ended = inactive > pool.total_coins_inactive;

        // actual coins on stake pool belonging to the active shares pool
        active = active + pending_active;
        // actual coins on stake pool belonging to the shares pool hosting `pending_inactive` stake
        // at current observed lockup cycle, either pending: `pending_inactive` or already inactivated:
        if (lockup_cycle_ended) {
            // `inactive` on stake pool = any previous `inactive` stake +
            // any previous `pending_inactive` stake and its rewards (both inactivated)
            pending_inactive = inactive - pool.total_coins_inactive
        };

        // on stake-management operations, total coins on the internal shares pools and individual
        // stakes on the stake pool are updated simultaneously, thus the only stakes becoming
        // unsynced are rewards and slashes routed exclusively to/out the stake pool

        // operator `active` rewards not persisted yet to the active shares pool
        let pool_active = total_coins(&pool.active_shares);
        let commission_active =
            if (active > pool_active) {
                math64::mul_div(
                    active - pool_active, pool.operator_commission_percentage, MAX_FEE
                )
            } else {
                // handle any slashing applied to `active` stake
                0
            };
        // operator `pending_inactive` rewards not persisted yet to the pending_inactive shares pool
        let pool_pending_inactive = total_coins(pending_inactive_shares_pool(pool));
        let commission_pending_inactive =
            if (pending_inactive > pool_pending_inactive) {
                math64::mul_div(
                    pending_inactive - pool_pending_inactive,
                    pool.operator_commission_percentage,
                    MAX_FEE
                )
            } else {
                // handle any slashing applied to `pending_inactive` stake
                0
            };

        (
            lockup_cycle_ended,
            active,
            pending_inactive,
            commission_active,
            commission_pending_inactive
        )
    }

    /// Synchronize delegation and stake pools: distribute yet-undetected rewards to the corresponding internal
    /// shares pools, assign commission to operator and eventually prepare delegation pool for a new lockup cycle.
    public entry fun synchronize_delegation_pool(
        pool_address: address
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let (
            lockup_cycle_ended,
            active,
            pending_inactive,
            commission_active,
            commission_pending_inactive
        ) = calculate_stake_pool_drift(pool);

        // zero `pending_active` stake indicates that either there are no `add_stake` fees or
        // previous epoch has ended and should release the shares owning the existing fees
        let (_, _, pending_active, _) = stake::get_stake(pool_address);
        if (pending_active == 0) {
            // renounce ownership over the `add_stake` fees by redeeming all shares of
            // the special shareholder, implicitly their equivalent coins, out of the active shares pool
            redeem_active_shares(pool, NULL_SHAREHOLDER, MAX_U64);
        };

        // distribute rewards remaining after commission, to delegators (to already existing shares)
        // before buying shares for the operator for its entire commission fee
        // otherwise, operator's new shares would additionally appreciate from rewards it does not own

        // update total coins accumulated by `active` + `pending_active` shares
        // redeemed `add_stake` fees are restored and distributed to the rest of the pool as rewards
        pool_u64::update_total_coins(&mut pool.active_shares, active
            - commission_active);
        // update total coins accumulated by `pending_inactive` shares at current observed lockup cycle
        pool_u64::update_total_coins(
            pending_inactive_shares_pool_mut(pool),
            pending_inactive - commission_pending_inactive
        );

        // reward operator its commission out of uncommitted active rewards (`add_stake` fees already excluded)
        buy_in_active_shares(
            pool,
            beneficiary_for_operator(stake::get_operator(pool_address)),
            commission_active
        );
        // reward operator its commission out of uncommitted pending_inactive rewards
        buy_in_pending_inactive_shares(
            pool,
            beneficiary_for_operator(stake::get_operator(pool_address)),
            commission_pending_inactive
        );

        event::emit_event(
            &mut pool.distribute_commission_events,
            DistributeCommissionEvent {
                pool_address,
                operator: stake::get_operator(pool_address),
                commission_active,
                commission_pending_inactive
            }
        );

        if (features::operator_beneficiary_change_enabled()) {
            emit(
                DistributeCommission {
                    pool_address,
                    operator: stake::get_operator(pool_address),
                    beneficiary: beneficiary_for_operator(
                        stake::get_operator(pool_address)
                    ),
                    commission_active,
                    commission_pending_inactive
                }
            )
        };

        // advance lockup cycle on delegation pool if already ended on stake pool (AND stake explicitly inactivated)
        if (lockup_cycle_ended) {
            // capture inactive coins over all ended lockup cycles (including this ending one)
            let (_, inactive, _, _) = stake::get_stake(pool_address);
            pool.total_coins_inactive = inactive;

            // advance lockup cycle on the delegation pool
            pool.observed_lockup_cycle.index = pool.observed_lockup_cycle.index + 1;
            // start new lockup cycle with a fresh shares pool for `pending_inactive` stake
            table::add(
                &mut pool.inactive_shares,
                pool.observed_lockup_cycle,
                pool_u64::create_with_scaling_factor(SHARES_SCALING_FACTOR)
            );
        };

        if (is_next_commission_percentage_effective(pool_address)) {
            pool.operator_commission_percentage = borrow_global<NextCommissionPercentage>(
                pool_address
            ).commission_percentage_next_lockup_cycle;
        }
    }

    fun update_governance_records_for_buy_in_active_shares(
        pool: &DelegationPool,
        pool_address: address,
        new_shares: u128,
        shareholder: address
    ) acquires GovernanceRecords {
        // <active shares> of <shareholder> += <new_shares> ---->
        // <active shares> of <current voter of shareholder> += <new_shares>
        // <active shares> of <next voter of shareholder> += <new_shares>
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let vote_delegation =
            update_and_borrow_mut_delegator_vote_delegation(
                pool, governance_records, shareholder
            );
        let current_voter = vote_delegation.voter;
        let pending_voter = vote_delegation.pending_voter;
        let current_delegated_votes =
            update_and_borrow_mut_delegated_votes(
                pool, governance_records, current_voter
            );
        current_delegated_votes.active_shares = current_delegated_votes.active_shares
            + new_shares;
        if (pending_voter == current_voter) {
            current_delegated_votes.active_shares_next_lockup = current_delegated_votes.active_shares_next_lockup
                + new_shares;
        } else {
            let pending_delegated_votes =
                update_and_borrow_mut_delegated_votes(
                    pool, governance_records, pending_voter
                );
            pending_delegated_votes.active_shares_next_lockup = pending_delegated_votes.active_shares_next_lockup
                + new_shares;
        };
    }

    fun update_governance_records_for_buy_in_pending_inactive_shares(
        pool: &DelegationPool,
        pool_address: address,
        new_shares: u128,
        shareholder: address
    ) acquires GovernanceRecords {
        // <pending inactive shares> of <shareholder> += <new_shares>   ---->
        // <pending inactive shares> of <current voter of shareholder> += <new_shares>
        // no impact on <pending inactive shares> of <next voter of shareholder>
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let current_voter =
            calculate_and_update_delegator_voter_internal(
                pool, governance_records, shareholder
            );
        let current_delegated_votes =
            update_and_borrow_mut_delegated_votes(
                pool, governance_records, current_voter
            );
        current_delegated_votes.pending_inactive_shares = current_delegated_votes.pending_inactive_shares
            + new_shares;
    }

    fun update_governanace_records_for_redeem_active_shares(
        pool: &DelegationPool,
        pool_address: address,
        shares_to_redeem: u128,
        shareholder: address
    ) acquires GovernanceRecords {
        // <active shares> of <shareholder> -= <shares_to_redeem> ---->
        // <active shares> of <current voter of shareholder> -= <shares_to_redeem>
        // <active shares> of <next voter of shareholder> -= <shares_to_redeem>
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let vote_delegation =
            update_and_borrow_mut_delegator_vote_delegation(
                pool, governance_records, shareholder
            );
        let current_voter = vote_delegation.voter;
        let pending_voter = vote_delegation.pending_voter;
        let current_delegated_votes =
            update_and_borrow_mut_delegated_votes(
                pool, governance_records, current_voter
            );
        current_delegated_votes.active_shares = current_delegated_votes.active_shares
            - shares_to_redeem;
        if (current_voter == pending_voter) {
            current_delegated_votes.active_shares_next_lockup = current_delegated_votes.active_shares_next_lockup
                - shares_to_redeem;
        } else {
            let pending_delegated_votes =
                update_and_borrow_mut_delegated_votes(
                    pool, governance_records, pending_voter
                );
            pending_delegated_votes.active_shares_next_lockup = pending_delegated_votes.active_shares_next_lockup
                - shares_to_redeem;
        };
    }

    fun update_governanace_records_for_redeem_pending_inactive_shares(
        pool: &DelegationPool,
        pool_address: address,
        shares_to_redeem: u128,
        shareholder: address
    ) acquires GovernanceRecords {
        // <pending inactive shares> of <shareholder> -= <shares_to_redeem>  ---->
        // <pending inactive shares> of <current voter of shareholder> -= <shares_to_redeem>
        // no impact on <pending inactive shares> of <next voter of shareholder>
        let governance_records = borrow_global_mut<GovernanceRecords>(pool_address);
        let current_voter =
            calculate_and_update_delegator_voter_internal(
                pool, governance_records, shareholder
            );
        let current_delegated_votes =
            update_and_borrow_mut_delegated_votes(
                pool, governance_records, current_voter
            );
        current_delegated_votes.pending_inactive_shares = current_delegated_votes.pending_inactive_shares
            - shares_to_redeem;
    }

    #[deprecated]
    /// Deprecated, prefer math64::mul_div
    public fun multiply_then_divide(x: u64, y: u64, z: u64): u64 {
        math64::mul_div(x, y, z)
    }

    #[test_only]
    use supra_framework::reconfiguration;
    #[test_only]
    use supra_framework::stake::fast_forward_to_unlock;
    #[test_only]
    use supra_framework::timestamp::fast_forward_seconds;

    #[test_only]
    const CONSENSUS_KEY_1: vector<u8> = x"c1bd3bcb387e4ee9a909f6304a1c9902661b0ecfb1e148c7892b210c7f353dfd";

    #[test_only]
    const CONSENSUS_POP_1: vector<u8> = x"a9d6c1f1270f2d1454c89a83a4099f813a56dc7db55591d46aa4e6ccae7898b234029ba7052f18755e6fa5e6b73e235f14efc4e2eb402ca2b8f56bad69f965fc11b7b25eb1c95a06f83ddfd023eac4559b6582696cfea97b227f4ce5bdfdfed0";

    #[test_only]
    const EPOCH_DURATION: u64 = 60;
    #[test_only]
    const LOCKUP_CYCLE_SECONDS: u64 = 2592000;

    #[test_only]
    const ONE_SUPRA: u64 = 100000000;

    #[test_only]
    const VALIDATOR_STATUS_PENDING_ACTIVE: u64 = 1;
    #[test_only]
    const VALIDATOR_STATUS_ACTIVE: u64 = 2;
    #[test_only]
    const VALIDATOR_STATUS_PENDING_INACTIVE: u64 = 3;

    #[test_only]
    const DELEGATION_POOLS: u64 = 11;

    #[test_only]
    const MODULE_EVENT: u64 = 26;

    #[test_only]
    const OPERATOR_BENEFICIARY_CHANGE: u64 = 39;

    #[test_only]
    const COMMISSION_CHANGE_DELEGATION_POOL: u64 = 42;

    #[test_only]
    public fun end_aptos_epoch() {
        stake::end_epoch(); // additionally forwards EPOCH_DURATION seconds
        reconfiguration::reconfigure_for_test_custom();
    }

    #[test_only]
    public fun initialize_for_test(supra_framework: &signer) {
        initialize_for_test_custom(
            supra_framework,
            100 * ONE_SUPRA,
            100000000000 * ONE_SUPRA,
            LOCKUP_CYCLE_SECONDS,
            true,
            1,
            100,
            1000000
        );
    }

    #[test_only]
    public fun initialize_for_test_no_reward(supra_framework: &signer) {
        initialize_for_test_custom(
            supra_framework,
            100 * ONE_SUPRA,
            10000000 * ONE_SUPRA,
            LOCKUP_CYCLE_SECONDS,
            true,
            0,
            100,
            1000000
        );
    }

    #[test_only]
    public fun initialize_for_test_custom(
        supra_framework: &signer,
        minimum_stake: u64,
        maximum_stake: u64,
        recurring_lockup_secs: u64,
        allow_validator_set_change: bool,
        rewards_rate_numerator: u64,
        rewards_rate_denominator: u64,
        voting_power_increase_limit: u64
    ) {
        account::create_account_for_test(signer::address_of(supra_framework));
        stake::initialize_for_test_custom(
            supra_framework,
            minimum_stake,
            maximum_stake,
            recurring_lockup_secs,
            allow_validator_set_change,
            rewards_rate_numerator,
            rewards_rate_denominator,
            voting_power_increase_limit
        );
        reconfiguration::initialize_for_test(supra_framework);
        features::change_feature_flags_for_testing(
            supra_framework,
            vector[
                DELEGATION_POOLS,
                MODULE_EVENT,
                OPERATOR_BENEFICIARY_CHANGE,
                COMMISSION_CHANGE_DELEGATION_POOL
            ],
            vector[]
        );
    }

    #[test_only]
    public fun initialize_test_validator(
        validator: &signer,
        amount: u64,
        should_join_validator_set: bool,
        should_end_epoch: bool,
        commission_percentage: u64,
        delegator_address: vector<address>,
        principle_stake: vector<u64>,
        coin: Coin<SupraCoin>,
        multisig_admin: option::Option<address>,
        unlock_numerators: vector<u64>,
        unlock_denominator: u64,
        principle_lockup_time: u64,
        unlock_duration: u64
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        let validator_address = signer::address_of(validator);
        if (!account::exists_at(validator_address)) {
            account::create_account_for_test(validator_address);
        };

        initialize_delegation_pool(
            validator,
            multisig_admin,
            commission_percentage,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            unlock_numerators,
            unlock_denominator,
            principle_lockup_time,
            unlock_duration
        );
        let pool_address = get_owned_pool_address(validator_address);

        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1);

        if (amount > 0) {
            stake::mint(validator, amount);
            add_stake(validator, pool_address, amount);
        };

        if (should_join_validator_set) {
            stake::join_validator_set(validator, pool_address);
        };

        if (should_end_epoch) {
            end_aptos_epoch();
        };
    }

    #[test_only]
    fun unlock_with_min_stake_disabled(
        delegator: &signer, pool_address: address, amount: u64
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        synchronize_delegation_pool(pool_address);

        let pool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_address = signer::address_of(delegator);

        amount = redeem_active_shares(pool, delegator_address, amount);
        stake::unlock(&retrieve_stake_pool_owner(pool), amount);
        buy_in_pending_inactive_shares(pool, delegator_address, amount);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x3000A, location = Self)]
    public entry fun test_delegation_pools_disabled(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        features::change_feature_flags_for_testing(
            supra_framework, vector[], vector[DELEGATION_POOLS]
        );

        initialize_delegation_pool(
            validator,
            option::none(),
            0,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        )
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_set_operator_and_delegated_voter(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(
            validator,
            option::none(),
            0,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(validator_address);

        assert!(stake::get_operator(pool_address) == @0x123, 1);
        assert!(stake::get_delegated_voter(pool_address) == @0x123, 1);

        set_operator(validator, @0x111);
        assert!(stake::get_operator(pool_address) == @0x111, 2);

        set_delegated_voter(validator, @0x112);
        assert!(stake::get_delegated_voter(pool_address) == @0x112, 2);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_operator(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        // account does not own any delegation pool
        set_operator(validator, @0x111);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public entry fun test_cannot_set_delegated_voter(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        // account does not own any delegation pool
        set_delegated_voter(validator, @0x112);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x80002, location = Self)]
    public entry fun test_already_owns_delegation_pool(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_delegation_pool(
            validator,
            option::none(),
            0,
            x"00",
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        initialize_delegation_pool(
            validator,
            option::none(),
            0,
            x"01",
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            0,
            12
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 0x1000B, location = Self)]
    public entry fun test_cannot_withdraw_zero_stake(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_delegation_pool(
            validator,
            option::none(),
            0,
            x"00",
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        withdraw(validator, get_owned_pool_address(signer::address_of(validator)), 0);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_initialize_delegation_pool(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[];
        let principle_stake = vector[];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        let validator_address = signer::address_of(validator);
        initialize_delegation_pool(
            validator,
            option::none(),
            1234,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        assert_owner_cap_exists(validator_address);
        let pool_address = get_owned_pool_address(validator_address);
        assert_delegation_pool_exists(pool_address);

        assert!(stake::stake_pool_exists(pool_address), 0);
        assert!(stake::get_operator(pool_address) == validator_address, 0);
        assert!(stake::get_delegated_voter(pool_address) == validator_address, 0);

        assert!(observed_lockup_cycle(pool_address) == 0, 0);
        assert!(total_coins_inactive(pool_address) == 0, 0);
        assert!(operator_commission_percentage(pool_address) == 1234, 0);
        assert_inactive_shares_pool(pool_address, 0, true, 0);
        stake::assert_stake_pool(pool_address, 0, 0, 0, 0);
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_add_stake_fee(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test_custom(
            supra_framework,
            100 * ONE_SUPRA,
            10000000 * ONE_SUPRA,
            LOCKUP_CYCLE_SECONDS,
            true,
            1,
            100,
            1000000
        );
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[0, 0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);

        // create delegation pool with 37.35% operator commission
        initialize_delegation_pool(
            validator,
            option::none(),
            3735,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(validator_address);

        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1);

        // zero `add_stake` fee as validator is not producing rewards this epoch
        assert!(
            get_add_stake_fee(pool_address, 1000000 * ONE_SUPRA) == 0,
            0
        );

        // add 1M SUPRA, join the validator set and activate this stake
        stake::mint(validator, 1000000 * ONE_SUPRA);
        add_stake(validator, pool_address, 1000000 * ONE_SUPRA);

        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        // `add_stake` fee for 100000 coins: 100000 * 0.006265 / (1 + 0.006265)
        assert!(
            get_add_stake_fee(pool_address, 100000 * ONE_SUPRA) == 62259941466,
            0
        );

        // add pending_active stake from multiple delegators
        stake::mint(delegator1, 100000 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 100000 * ONE_SUPRA);
        stake::mint(delegator2, 10000 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 10000 * ONE_SUPRA);

        end_aptos_epoch();
        // delegators should own the same amount as initially deposited
        assert_delegation(
            delegator1_address,
            pool_address,
            10000000000000,
            0,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            1000000000000,
            0,
            0
        );

        // add more stake from delegator 1
        stake::mint(delegator1, 10000 * ONE_SUPRA);
        let (delegator1_active, _, _) = get_stake(pool_address, delegator1_address);
        add_stake(delegator1, pool_address, 10000 * ONE_SUPRA);

        let fee = get_add_stake_fee(pool_address, 10000 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            delegator1_active + 10000 * ONE_SUPRA - fee,
            0,
            0
        );

        // delegator 2 should not benefit in any way from this new stake
        assert_delegation(
            delegator2_address,
            pool_address,
            1000000000000,
            0,
            0
        );

        // add more stake from delegator 2
        stake::mint(delegator2, 100000 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 100000 * ONE_SUPRA);

        end_aptos_epoch();
        // delegators should own the same amount as initially deposited + any rewards produced
        // 10000000000000 * 1% * (100 - 37.35)%
        assert_delegation(
            delegator1_address,
            pool_address,
            11062650000001,
            0,
            0
        );
        // 1000000000000 * 1% * (100 - 37.35)%
        assert_delegation(
            delegator2_address,
            pool_address,
            11006265000001,
            0,
            0
        );

        // in-flight operator commission rewards do not automatically restake/compound
        synchronize_delegation_pool(pool_address);

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        assert_delegation(
            delegator1_address,
            pool_address,
            11062650000001,
            0,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            11006265000001,
            0,
            0
        );

        end_aptos_epoch();
        // delegators should own previous stake * 1.006265
        assert_delegation(
            delegator1_address,
            pool_address,
            11131957502251,
            0,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            11075219250226,
            0,
            0
        );

        // add more stake from delegator 1
        stake::mint(delegator1, 20000 * ONE_SUPRA);
        (delegator1_active, _, _) = get_stake(pool_address, delegator1_address);
        add_stake(delegator1, pool_address, 20000 * ONE_SUPRA);

        fee = get_add_stake_fee(pool_address, 20000 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            delegator1_active + 20000 * ONE_SUPRA - fee,
            0,
            0
        );

        // delegator 1 unlocks his entire newly added stake
        unlock(delegator1, pool_address, 20000 * ONE_SUPRA - fee);
        end_aptos_epoch();
        // delegator 1 should own previous 11131957502250 active * 1.006265 and 20000 coins pending_inactive
        assert_delegation(
            delegator1_address,
            pool_address,
            11201699216002,
            0,
            2000000000000
        );

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        synchronize_delegation_pool(pool_address);
        assert_delegation(
            delegator1_address,
            pool_address,
            11201699216002,
            0,
            2000000000000
        );

        let reward_period_start_time_in_sec = timestamp::now_seconds();
        // Enable rewards rate decrease. Initially rewards rate is still 1% every epoch. Rewards rate halves every year.
        let one_year_in_secs: u64 = 31536000;
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(2, 100),
            fixed_point64::create_from_rational(6, 1000),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(50, 100)
        );
        features::change_feature_flags_for_testing(
            supra_framework,
            vector[features::get_periodical_reward_rate_decrease_feature()],
            vector[]
        );

        // add more stake from delegator 1
        stake::mint(delegator1, 20000 * ONE_SUPRA);
        let delegator1_pending_inactive: u64;
        (delegator1_active, _, delegator1_pending_inactive) = get_stake(
            pool_address, delegator1_address
        );
        fee = get_add_stake_fee(pool_address, 20000 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 20000 * ONE_SUPRA);

        assert_delegation(
            delegator1_address,
            pool_address,
            delegator1_active + 20000 * ONE_SUPRA - fee,
            0,
            delegator1_pending_inactive
        );

        // delegator 1 unlocks his entire newly added stake
        unlock(delegator1, pool_address, 20000 * ONE_SUPRA - fee);
        end_aptos_epoch();
        // delegator 1 should own previous 11201699216002 active * ~1.01253 and 20000 * ~1.01253 + 20000 coins pending_inactive
        assert_delegation(
            delegator1_address,
            pool_address,
            11342056366822,
            0,
            4025059974939
        );

        // stakes should remain the same - `Self::get_stake` correctly calculates them
        synchronize_delegation_pool(pool_address);
        assert_delegation(
            delegator1_address,
            pool_address,
            11342056366822,
            0,
            4025059974939
        );

        fast_forward_seconds(one_year_in_secs);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_never_create_pending_withdrawal_if_no_shares_bought(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0 * ONE_SUPRA];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            false,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 10 * ONE_SUPRA);
        add_stake(delegator, pool_address, 10 * ONE_SUPRA);
        end_aptos_epoch();

        unlock(validator, pool_address, 100 * ONE_SUPRA);

        stake::assert_stake_pool(pool_address, 91000000000, 0, 0, 10000000000);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 91910000000, 0, 0, 10100000000);

        unlock_with_min_stake_disabled(delegator, pool_address, 1);
        // request 1 coins * 910 / 919.1 = 0.99 shares to redeem * 1.01 price -> 0 coins out
        // 1 coins lost at redeem due to 0.99 shares being burned
        assert_delegation(
            delegator_address,
            pool_address,
            1009999999,
            0,
            0
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            false,
            0,
            false,
            0
        );

        unlock_with_min_stake_disabled(delegator, pool_address, 2);
        // request 2 coins * 909.99 / 919.1 = 1.98 shares to redeem * 1.01 price -> 1 coins out
        // with 1 coins buy 1 * 100 / 101 = 0.99 shares in pending_inactive pool * 1.01 -> 0 coins in
        // 1 coins lost at redeem due to 1.98 - 1.01 shares being burned + 1 coins extracted
        synchronize_delegation_pool(pool_address);
        assert_delegation(
            delegator_address,
            pool_address,
            1009999997,
            0,
            0
        );
        // the pending withdrawal has been created as > 0 pending_inactive shares have been bought
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            0,
            false,
            0
        );

        // successfully delete the pending withdrawal (redeem all owned shares even worth 0 coins)
        reactivate_stake(delegator, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator_address,
            pool_address,
            1009999997,
            0,
            0
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            false,
            0,
            false,
            0
        );

        // unlock min coins to own some pending_inactive balance (have to disable min-balance checks)
        unlock_with_min_stake_disabled(delegator, pool_address, 3);
        // request 3 coins * 909.99 / 919.09 = 2.97 shares to redeem * 1.01 price -> 2 coins out
        // with 2 coins buy 2 * 100 / 101 = 1.98 shares in pending_inactive pool * 1.01 -> 1 coins in
        // 1 coins lost at redeem due to 2.97 - 2 * 1.01 shares being burned + 2 coins extracted
        synchronize_delegation_pool(pool_address);
        assert_delegation(
            delegator_address,
            pool_address,
            1009999994,
            0,
            1
        );
        // the pending withdrawal has been created as > 0 pending_inactive shares have been bought
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            0,
            false,
            1
        );

        reactivate_stake(delegator, pool_address, MIN_COINS_ON_SHARES_POOL);
        // redeem 1 coins >= delegator balance -> all shares are redeemed and pending withdrawal is deleted
        assert_delegation(
            delegator_address,
            pool_address,
            1009999995,
            0,
            0
        );
        // the pending withdrawal has been deleted as delegator has 0 pending_inactive shares now
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
    }

    // The test case abort because the amount of stake is less than the minimum amount of stake
    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 65574, location = Self)]
    public entry fun test_add_stake_min_amount(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            MIN_COINS_ON_SHARES_POOL - 1,
            false,
            false,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_add_stake_single(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[];
        let principle_stake = vector[];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            false,
            false,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // validator is inactive => added stake is `active` by default
        stake::assert_stake_pool(pool_address, 1000 * ONE_SUPRA, 0, 0, 0);
        assert_delegation(
            validator_address,
            pool_address,
            1000 * ONE_SUPRA,
            0,
            0
        );

        // zero `add_stake` fee as validator is not producing rewards this epoch
        assert!(
            get_add_stake_fee(pool_address, 250 * ONE_SUPRA) == 0,
            0
        );

        // check `add_stake` increases `active` stakes of delegator and stake pool
        stake::mint(validator, 300 * ONE_SUPRA);
        let balance = coin::balance<SupraCoin>(validator_address);
        add_stake(validator, pool_address, 250 * ONE_SUPRA);

        // check added stake have been transferred out of delegator account
        assert!(
            coin::balance<SupraCoin>(validator_address) == balance - 250 * ONE_SUPRA,
            0
        );
        // zero `add_stake` fee charged from added stake
        assert_delegation(
            validator_address,
            pool_address,
            1250 * ONE_SUPRA,
            0,
            0
        );
        // zero `add_stake` fee transferred to null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        // added stake is automatically `active` on inactive validator
        stake::assert_stake_pool(pool_address, 1250 * ONE_SUPRA, 0, 0, 0);

        // activate validator
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // add 250 coins being pending_active until next epoch
        stake::mint(validator, 250 * ONE_SUPRA);
        add_stake(validator, pool_address, 250 * ONE_SUPRA);

        let fee1 = get_add_stake_fee(pool_address, 250 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            1500 * ONE_SUPRA - fee1,
            0,
            0
        );
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1, 0, 0);
        stake::assert_stake_pool(
            pool_address,
            1250 * ONE_SUPRA,
            0,
            250 * ONE_SUPRA,
            0
        );

        // add 100 additional coins being pending_active until next epoch
        stake::mint(validator, 100 * ONE_SUPRA);
        add_stake(validator, pool_address, 100 * ONE_SUPRA);

        let fee2 = get_add_stake_fee(pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            1600 * ONE_SUPRA - fee1 - fee2,
            0,
            0
        );
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1 + fee2, 0, 0);
        stake::assert_stake_pool(
            pool_address,
            1250 * ONE_SUPRA,
            0,
            350 * ONE_SUPRA,
            0
        );

        end_aptos_epoch();
        // delegator got its `add_stake` fees back + 1250 * 1% * (100% - 0%) active rewards
        assert_delegation(
            validator_address,
            pool_address,
            161250000000,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 161250000000, 0, 0, 0);

        // check that shares of null shareholder have been released
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        synchronize_delegation_pool(pool_address);
        assert!(
            pool_u64::shares(
                &borrow_global<DelegationPool>(pool_address).active_shares,
                NULL_SHAREHOLDER
            ) == 0,
            0
        );
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);

        // add 200 coins being pending_active until next epoch
        stake::mint(validator, 200 * ONE_SUPRA);
        add_stake(validator, pool_address, 200 * ONE_SUPRA);

        fee1 = get_add_stake_fee(pool_address, 200 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            181250000000 - fee1,
            0,
            0
        );
        // check `add_stake` fee has been transferred to the null shareholder
        assert_delegation(NULL_SHAREHOLDER, pool_address, fee1 - 1, 0, 0);
        stake::assert_stake_pool(pool_address, 161250000000, 0, 20000000000, 0);

        end_aptos_epoch();
        // delegator got its `add_stake` fee back + 161250000000 * 1% active rewards
        assert_delegation(
            validator_address,
            pool_address,
            182862500000,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 182862500000, 0, 0, 0);

        // check that shares of null shareholder have been released
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
        synchronize_delegation_pool(pool_address);
        assert!(
            pool_u64::shares(
                &borrow_global<DelegationPool>(pool_address).active_shares,
                NULL_SHAREHOLDER
            ) == 0,
            0
        );
        assert_delegation(NULL_SHAREHOLDER, pool_address, 0, 0, 0);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_add_stake_many(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::assert_stake_pool(pool_address, 1000 * ONE_SUPRA, 0, 0, 0);
        assert_delegation(
            validator_address,
            pool_address,
            1000 * ONE_SUPRA,
            0,
            0
        );

        // add 250 coins from second account
        stake::mint(delegator, 250 * ONE_SUPRA);
        add_stake(delegator, pool_address, 250 * ONE_SUPRA);

        let fee1 = get_add_stake_fee(pool_address, 250 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            250 * ONE_SUPRA - fee1,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            1000 * ONE_SUPRA,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            1000 * ONE_SUPRA,
            0,
            250 * ONE_SUPRA,
            0
        );

        end_aptos_epoch();
        // 1000 * 1.01 active stake + 250 pending_active stake
        stake::assert_stake_pool(pool_address, 1260 * ONE_SUPRA, 0, 0, 0);
        // delegator got its `add_stake` fee back
        assert_delegation(
            delegator_address,
            pool_address,
            250 * ONE_SUPRA,
            0,
            0
        );
        // actual active rewards have been distributed to their earner(s)
        assert_delegation(
            validator_address,
            pool_address,
            100999999999,
            0,
            0
        );

        // add another 250 coins from first account
        stake::mint(validator, 250 * ONE_SUPRA);
        add_stake(validator, pool_address, 250 * ONE_SUPRA);

        fee1 = get_add_stake_fee(pool_address, 250 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            125999999999 - fee1,
            0,
            0
        );
        assert_delegation(
            delegator_address,
            pool_address,
            250 * ONE_SUPRA,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            1260 * ONE_SUPRA,
            0,
            250 * ONE_SUPRA,
            0
        );

        // add another 100 coins from second account
        stake::mint(delegator, 100 * ONE_SUPRA);
        add_stake(delegator, pool_address, 100 * ONE_SUPRA);

        let fee2 = get_add_stake_fee(pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            350 * ONE_SUPRA - fee2,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            125999999999 - fee1,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            1260 * ONE_SUPRA,
            0,
            350 * ONE_SUPRA,
            0
        );

        end_aptos_epoch();
        // both delegators got their `add_stake` fees back
        // 250 * 1.01 active stake + 100 pending_active stake
        assert_delegation(
            delegator_address,
            pool_address,
            35250000001,
            0,
            0
        );
        // 1010 * 1.01 active stake + 250 pending_active stake
        assert_delegation(
            validator_address,
            pool_address,
            127009999998,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 162260000000, 0, 0, 0);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_unlock_single(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address_vec = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        // add 200 coins pending_active until next epoch
        stake::mint(validator, 200 * ONE_SUPRA);
        add_stake(validator, pool_address, 200 * ONE_SUPRA);

        let fee = get_add_stake_fee(pool_address, 200 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            300 * ONE_SUPRA - fee,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            100 * ONE_SUPRA,
            0,
            200 * ONE_SUPRA,
            0
        );

        // cannot unlock pending_active stake (only 100/300 stake can be displaced)
        unlock(validator, pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            200 * ONE_SUPRA - fee,
            0,
            100 * ONE_SUPRA
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            100 * ONE_SUPRA
        );
        stake::assert_stake_pool(
            pool_address,
            0,
            0,
            200 * ONE_SUPRA,
            100 * ONE_SUPRA
        );
        assert_inactive_shares_pool(pool_address, 0, true, 100 * ONE_SUPRA);

        // reactivate entire pending_inactive stake progressively
        reactivate_stake(validator, pool_address, 50 * ONE_SUPRA);

        assert_delegation(
            validator_address,
            pool_address,
            250 * ONE_SUPRA - fee,
            0,
            50 * ONE_SUPRA
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            50 * ONE_SUPRA
        );
        stake::assert_stake_pool(
            pool_address,
            50 * ONE_SUPRA,
            0,
            200 * ONE_SUPRA,
            50 * ONE_SUPRA
        );

        reactivate_stake(validator, pool_address, 50 * ONE_SUPRA);

        assert_delegation(
            validator_address,
            pool_address,
            300 * ONE_SUPRA - fee,
            0,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            100 * ONE_SUPRA,
            0,
            200 * ONE_SUPRA,
            0
        );
        // pending_inactive shares pool has not been deleted (as can still `unlock` this OLC)
        assert_inactive_shares_pool(pool_address, 0, true, 0);

        end_aptos_epoch();
        // 10000000000 * 1.01 active stake + 20000000000 pending_active stake
        assert_delegation(
            validator_address,
            pool_address,
            301 * ONE_SUPRA,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 301 * ONE_SUPRA, 0, 0, 0);

        // can unlock more than at previous epoch as the pending_active stake became active
        unlock(validator, pool_address, 150 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            15100000001,
            0,
            14999999999
        );
        stake::assert_stake_pool(pool_address, 15100000001, 0, 0, 14999999999);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            14999999999
        );

        assert!(
            stake::get_remaining_lockup_secs(pool_address)
                == LOCKUP_CYCLE_SECONDS - EPOCH_DURATION,
            0
        );
        end_aptos_epoch(); // additionally forwards EPOCH_DURATION seconds

        // pending_inactive stake should have not been inactivated
        // 15100000001 * 1.01 active stake + 14999999999 pending_inactive * 1.01 stake
        assert_delegation(
            validator_address,
            pool_address,
            15251000001,
            0,
            15149999998
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            15149999998
        );
        stake::assert_stake_pool(pool_address, 15251000001, 0, 0, 15149999998);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS - 3 * EPOCH_DURATION);
        end_aptos_epoch(); // additionally forwards EPOCH_DURATION seconds and expires lockup cycle

        // 15251000001 * 1.01 active stake + 15149999998 * 1.01 pending_inactive(now inactive) stake
        assert_delegation(
            validator_address,
            pool_address,
            15403510001,
            15301499997,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            15301499997
        );
        stake::assert_stake_pool(pool_address, 15403510001, 15301499997, 0, 0);

        // add 50 coins from another account
        stake::mint(delegator, 50 * ONE_SUPRA);
        add_stake(delegator, pool_address, 50 * ONE_SUPRA);

        // observed lockup cycle should have advanced at `add_stake`(on synchronization)
        assert!(observed_lockup_cycle(pool_address) == 1, 0);

        fee = get_add_stake_fee(pool_address, 50 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            4999999999 - fee,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            15403510001,
            15301499997,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            15403510001,
            15301499997,
            50 * ONE_SUPRA,
            0
        );

        // cannot withdraw stake unlocked by others
        withdraw(delegator, pool_address, 50 * ONE_SUPRA);
        assert!(coin::balance<SupraCoin>(delegator_address) == 0, 0);

        // withdraw own unlocked stake
        withdraw(validator, pool_address, 15301499997);
        assert!(coin::balance<SupraCoin>(validator_address) == 15301499997, 0);
        assert_delegation(
            validator_address,
            pool_address,
            15403510001,
            0,
            0
        );
        // pending withdrawal has been executed and deleted
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        // inactive shares pool on OLC 0 has been deleted because its stake has been withdrawn
        assert_inactive_shares_pool(pool_address, 0, false, 0);

        // new pending withdrawal can be created on lockup cycle 1
        unlock(validator, pool_address, 5403510001);
        assert_delegation(
            validator_address,
            pool_address,
            10000000000,
            0,
            5403510000
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            5403510000
        );

        // end lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 10000000000 * 1.01 active stake + 5403510000 * 1.01 pending_inactive(now inactive) stake
        assert_delegation(
            validator_address,
            pool_address,
            10100000000,
            5457545100,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            true,
            5457545100
        );

        // unlock when the pending withdrawal exists and gets automatically executed
        let balance = coin::balance<SupraCoin>(validator_address);
        unlock(validator, pool_address, 10100000000);
        assert!(
            coin::balance<SupraCoin>(validator_address) == balance + 5457545100,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            0,
            0,
            10100000000
        );
        // this is the new pending withdrawal replacing the executed one
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            2,
            false,
            10100000000
        );

        // create dummy validator to ensure the existing validator can leave the set
        let delegator_address_vec = vector[@0x010];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);

        // lockup time updated as you see above we `fast_forward_seconds` alog with `end_aptos_epoch`
        principle_lockup_time = LOCKUP_CYCLE_SECONDS
            + (LOCKUP_CYCLE_SECONDS - (3 * EPOCH_DURATION)) + (5 * EPOCH_DURATION);
        initialize_test_validator(
            delegator,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();

        // expire lockup cycle on the stake pool
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        let observed_lockup_cycle = observed_lockup_cycle(pool_address);
        end_aptos_epoch();

        // observed lockup cycle should be unchanged as no stake has been inactivated
        synchronize_delegation_pool(pool_address);
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // stake is pending_inactive as it has not been inactivated
        stake::assert_stake_pool(pool_address, 5100500001, 0, 0, 10303010000);
        // 10100000000 * 1.01 * 1.01 pending_inactive stake
        assert_delegation(
            validator_address,
            pool_address,
            0,
            0,
            10303010000
        );
        // the pending withdrawal should be reported as still pending
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            2,
            false,
            10303010000
        );

        // validator is inactive and lockup expired => pending_inactive stake is withdrawable
        balance = coin::balance<SupraCoin>(validator_address);
        withdraw(validator, pool_address, 10303010000);

        assert!(
            coin::balance<SupraCoin>(validator_address) == balance + 10303010000,
            0
        );
        assert_delegation(validator_address, pool_address, 0, 0, 0);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        stake::assert_stake_pool(pool_address, 5100500001, 0, 0, 0);
        // pending_inactive shares pool has not been deleted (as can still `unlock` this OLC)
        assert_inactive_shares_pool(
            pool_address,
            observed_lockup_cycle(pool_address),
            true,
            0
        );

        stake::mint(validator, 30 * ONE_SUPRA);
        add_stake(validator, pool_address, 30 * ONE_SUPRA);
        unlock(validator, pool_address, 10 * ONE_SUPRA);

        assert_delegation(
            validator_address,
            pool_address,
            2000000000,
            0,
            999999999
        );
        // the pending withdrawal should be reported as still pending
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            2,
            false,
            999999999
        );

        balance = coin::balance<SupraCoin>(validator_address);
        // pending_inactive balance would be under threshold => redeem entire balance
        withdraw(validator, pool_address, 1);
        // pending_inactive balance has been withdrawn and the pending withdrawal executed
        assert_delegation(
            validator_address,
            pool_address,
            2000000000,
            0,
            999999998
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            2,
            false,
            999999998
        );
        assert!(
            coin::balance<SupraCoin>(validator_address) == balance + 1,
            0
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_total_coins_inactive(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[0, 0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            200 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[1],
            1,
            principle_lockup_time,
            60
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 100 * ONE_SUPRA);
        stake::mint(delegator2, 200 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 100 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 200 * ONE_SUPRA);
        end_aptos_epoch();

        assert_delegation(
            delegator1_address,
            pool_address,
            100 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            0
        );

        // unlock some stake from delegator 1
        unlock(delegator1, pool_address, 50 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            5000000000,
            0,
            4999999999
        );

        // move to lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // delegator 1 pending_inactive stake has been inactivated
        assert_delegation(
            delegator1_address,
            pool_address,
            5050000000,
            5049999998,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            202 * ONE_SUPRA,
            0,
            0
        );

        synchronize_delegation_pool(pool_address);
        assert!(total_coins_inactive(pool_address) == 5049999998, 0);

        // unlock some stake from delegator 2
        unlock(delegator2, pool_address, 50 * ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            15200000001,
            0,
            4999999999
        );

        // withdraw some of inactive stake of delegator 1
        withdraw(delegator1, pool_address, 2049999998);
        assert_delegation(
            delegator1_address,
            pool_address,
            5050000000,
            3000000001,
            0
        );
        assert!(total_coins_inactive(pool_address) == 3000000001, 0);

        // move to lockup cycle 2
        let (_, inactive, _, pending_inactive) = stake::get_stake(pool_address);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // delegator 2 pending_inactive stake has been inactivated
        assert_delegation(
            delegator1_address,
            pool_address,
            5100500000,
            3000000001,
            0
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            15352000001,
            5049999998,
            0
        );

        // total_coins_inactive remains unchanged in the absence of user operations
        assert!(total_coins_inactive(pool_address) == inactive, 0);
        synchronize_delegation_pool(pool_address);
        // total_coins_inactive == previous inactive stake + previous pending_inactive stake and its rewards
        assert!(
            total_coins_inactive(pool_address)
                == inactive + pending_inactive + pending_inactive / 100,
            0
        );

        // withdraw some of inactive stake of delegator 2
        let total_coins_inactive = total_coins_inactive(pool_address);
        withdraw(delegator2, pool_address, 3049999998);
        assert!(
            total_coins_inactive(pool_address) == total_coins_inactive - 3049999997,
            0
        );

        // unlock some stake from delegator `validator`
        unlock(validator, pool_address, 50 * ONE_SUPRA);

        // create dummy validator to ensure the existing validator can leave the set
        let delegator_address = vector[@0x010];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        principle_lockup_time = (2 * LOCKUP_CYCLE_SECONDS) + (4 * EPOCH_DURATION);
        initialize_test_validator(
            delegator1,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();

        // move to lockup cycle 3
        (_, inactive, _, pending_inactive) = stake::get_stake(pool_address);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // pending_inactive stake has not been inactivated as validator is inactive
        let (_, inactive_now, _, pending_inactive_now) = stake::get_stake(pool_address);
        assert!(inactive_now == inactive, inactive_now);
        assert!(pending_inactive_now == pending_inactive, pending_inactive_now);

        // total_coins_inactive remains unchanged in the absence of a new OLC
        synchronize_delegation_pool(pool_address);
        assert!(total_coins_inactive(pool_address) == inactive, 0);

        // withdraw entire pending_inactive stake
        withdraw(validator, pool_address, MAX_U64);
        assert!(total_coins_inactive(pool_address) == inactive, 0);
        (_, _, _, pending_inactive) = stake::get_stake(pool_address);
        assert!(pending_inactive == 0, pending_inactive);

        // withdraw entire inactive stake
        withdraw(delegator1, pool_address, MAX_U64);
        withdraw(delegator2, pool_address, MAX_U64);
        assert!(total_coins_inactive(pool_address) == 0, 0);
        (_, inactive, _, _) = stake::get_stake(pool_address);
        assert!(inactive == 0, inactive);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_reactivate_stake_single(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[];
        let principle_stake = vector[];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            200 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // unlock some stake from the active one
        unlock(validator, pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            100 * ONE_SUPRA,
            0,
            100 * ONE_SUPRA
        );
        stake::assert_stake_pool(
            pool_address,
            100 * ONE_SUPRA,
            0,
            0,
            100 * ONE_SUPRA
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            100 * ONE_SUPRA
        );

        // add some stake to pending_active state
        stake::mint(validator, 150 * ONE_SUPRA);
        add_stake(validator, pool_address, 150 * ONE_SUPRA);

        let fee = get_add_stake_fee(pool_address, 150 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            250 * ONE_SUPRA - fee,
            0,
            100 * ONE_SUPRA
        );
        stake::assert_stake_pool(
            pool_address,
            100 * ONE_SUPRA,
            0,
            150 * ONE_SUPRA,
            100 * ONE_SUPRA
        );

        // can reactivate only pending_inactive stake
        reactivate_stake(validator, pool_address, 150 * ONE_SUPRA);

        assert_delegation(
            validator_address,
            pool_address,
            350 * ONE_SUPRA - fee,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            200 * ONE_SUPRA,
            0,
            150 * ONE_SUPRA,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );

        end_aptos_epoch();
        // 20000000000 active stake * 1.01 + 15000000000 pending_active stake
        assert_delegation(
            validator_address,
            pool_address,
            35200000000,
            0,
            0
        );

        // unlock stake added at previous epoch (expect some imprecision when moving shares)
        unlock(validator, pool_address, 150 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            20200000001,
            0,
            14999999999
        );
        stake::assert_stake_pool(pool_address, 20200000001, 0, 0, 14999999999);

        // inactivate pending_inactive stake
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 20200000001 active stake * 1.01 + 14999999999 pending_inactive stake * 1.01
        assert_delegation(
            validator_address,
            pool_address,
            20402000001,
            15149999998,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            15149999998
        );

        // cannot reactivate inactive stake
        reactivate_stake(validator, pool_address, 15149999998);
        assert_delegation(
            validator_address,
            pool_address,
            20402000001,
            15149999998,
            0
        );

        // unlock stake in the new lockup cycle (the pending withdrawal is executed)
        unlock(validator, pool_address, 100 * ONE_SUPRA);
        assert!(coin::balance<SupraCoin>(validator_address) == 15149999998, 0);
        assert_delegation(
            validator_address,
            pool_address,
            10402000002,
            0,
            9999999999
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            9999999999
        );

        // reactivate the new pending withdrawal almost entirely
        reactivate_stake(validator, pool_address, 8999999999);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            1000000000
        );
        // reactivate remaining stake of the new pending withdrawal
        reactivate_stake(validator, pool_address, 1000000000);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_withdraw_many(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[1],
            1,
            principle_lockup_time,
            60
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_SUPRA);
        add_stake(delegator, pool_address, 200 * ONE_SUPRA);

        unlock(validator, pool_address, 100 * ONE_SUPRA);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            100 * ONE_SUPRA
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_delegation(
            delegator_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            90899999999,
            10100000000,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            10100000000
        );
        assert_inactive_shares_pool(pool_address, 0, true, 100 * ONE_SUPRA);

        // check cannot withdraw inactive stake unlocked by others
        withdraw(delegator, pool_address, MAX_U64);
        assert_delegation(
            delegator_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            90899999999,
            10100000000,
            0
        );

        unlock(delegator, pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            10000000000,
            0,
            9999999999
        );
        assert_delegation(
            validator_address,
            pool_address,
            90900000000,
            10100000000,
            0
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            9999999999
        );

        // check cannot withdraw inactive stake unlocked by others even if owning pending_inactive
        withdraw(delegator, pool_address, MAX_U64);
        assert_delegation(
            delegator_address,
            pool_address,
            10000000000,
            0,
            9999999999
        );
        assert_delegation(
            validator_address,
            pool_address,
            90900000000,
            10100000000,
            0
        );

        // withdraw entire owned inactive stake
        let balance = coin::balance<SupraCoin>(validator_address);
        withdraw(validator, pool_address, MAX_U64);
        assert!(
            coin::balance<SupraCoin>(validator_address) == balance + 10100000000,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        assert_inactive_shares_pool(pool_address, 0, false, 0);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_delegation(
            delegator_address,
            pool_address,
            10100000000,
            10099999998,
            0
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            true,
            10099999998
        );
        assert_inactive_shares_pool(pool_address, 1, true, 9999999999);

        // use too small of an unlock amount to actually transfer shares to the pending_inactive pool
        // check that no leftovers have been produced on the stake or delegation pools
        stake::assert_stake_pool(pool_address, 101909000001, 10099999998, 0, 0);
        unlock_with_min_stake_disabled(delegator, pool_address, 1);
        stake::assert_stake_pool(pool_address, 101909000001, 10099999998, 0, 0);
        assert_delegation(
            delegator_address,
            pool_address,
            10100000000,
            10099999998,
            0
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            true,
            10099999998
        );

        // implicitly execute the pending withdrawal by unlocking min stake to buy 1 share
        unlock_with_min_stake_disabled(delegator, pool_address, 2);
        stake::assert_stake_pool(pool_address, 101909000000, 0, 0, 1);
        assert_delegation(
            delegator_address,
            pool_address,
            10099999998,
            0,
            1
        );
        // old pending withdrawal has been replaced
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            2,
            false,
            1
        );
        assert_inactive_shares_pool(pool_address, 1, false, 0);
        assert_inactive_shares_pool(pool_address, 2, true, 1);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_inactivate_no_excess_stake(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address_vec = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1200 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::none(),
            vector[1],
            1,
            principle_lockup_time,
            60
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_SUPRA);
        add_stake(delegator, pool_address, 200 * ONE_SUPRA);

        // create inactive and pending_inactive stakes on the stake pool
        unlock(validator, pool_address, 200 * ONE_SUPRA);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(delegator, pool_address, 100 * ONE_SUPRA);

        // check no excess pending_inactive is inactivated in the special case
        // the validator had gone inactive before its lockup expired

        let observed_lockup_cycle = observed_lockup_cycle(pool_address);

        // create dummy validator to ensure the existing validator can leave the set
        let delegator_address_vec = vector[@0x010];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        principle_lockup_time = LOCKUP_CYCLE_SECONDS + (2 * EPOCH_DURATION);
        initialize_test_validator(
            delegator,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        // inactivate validator
        stake::leave_validator_set(validator, pool_address);
        end_aptos_epoch();
        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_INACTIVE, 0);

        // expire lockup afterwards
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        synchronize_delegation_pool(pool_address);
        // no new inactive stake detected => OLC does not advance
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // pending_inactive stake has not been inactivated
        stake::assert_stake_pool(
            pool_address,
            113231100001,
            20200000000,
            0,
            10200999997
        );
        assert_delegation(
            delegator_address,
            pool_address,
            10201000000,
            0,
            10200999997
        );
        assert_delegation(
            validator_address,
            pool_address,
            103030100000,
            20200000000,
            0
        );

        // withdraw some inactive stake (remaining pending_inactive is not inactivated)
        withdraw(validator, pool_address, 200000000);
        stake::assert_stake_pool(
            pool_address,
            113231100001,
            20000000001,
            0,
            10200999997
        );
        assert_delegation(
            delegator_address,
            pool_address,
            10201000000,
            0,
            10200999997
        );
        assert_delegation(
            validator_address,
            pool_address,
            103030100000,
            20000000001,
            0
        );

        // withdraw some pending_inactive stake (remaining pending_inactive is not inactivated)
        withdraw(delegator, pool_address, 200999997);
        stake::assert_stake_pool(
            pool_address,
            113231100001,
            20000000001,
            0,
            10000000001
        );
        assert_delegation(
            delegator_address,
            pool_address,
            10201000000,
            0,
            10000000001
        );
        assert_delegation(
            validator_address,
            pool_address,
            103030100000,
            20000000001,
            0
        );

        // no new inactive stake detected => OLC does not advance
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        unlock(delegator, pool_address, 10201000000);
        withdraw(delegator, pool_address, 10201000000);
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        assert_delegation(
            delegator_address,
            pool_address,
            0,
            0,
            10000000002
        );
        assert_delegation(
            validator_address,
            pool_address,
            103030100001,
            20000000001,
            0
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            20000000001
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            10000000002
        );
        stake::assert_stake_pool(
            pool_address,
            103030100001,
            20000000001,
            0,
            10000000002
        );

        // reactivate validator
        stake::join_validator_set(validator, pool_address);
        assert!(
            stake::get_validator_state(pool_address) == VALIDATOR_STATUS_PENDING_ACTIVE,
            0
        );
        end_aptos_epoch();

        assert!(stake::get_validator_state(pool_address) == VALIDATOR_STATUS_ACTIVE, 0);
        // no rewards have been produced yet and no stake inactivated as lockup has been refreshed
        stake::assert_stake_pool(
            pool_address,
            103030100001,
            20000000001,
            0,
            10000000002
        );

        synchronize_delegation_pool(pool_address);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            20000000001
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            10000000002
        );
        assert!(observed_lockup_cycle(pool_address) == observed_lockup_cycle, 0);

        // cannot withdraw pending_inactive stake anymore
        withdraw(delegator, pool_address, 10000000002);
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            10000000002
        );

        // earning rewards is resumed from this epoch on
        end_aptos_epoch();
        stake::assert_stake_pool(
            pool_address,
            104060401001,
            20000000001,
            0,
            10100000002
        );

        // new pending_inactive stake earns rewards but so does the old one
        unlock(validator, pool_address, 104060401001);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            104060401000
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            10100000002
        );
        end_aptos_epoch();
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            105101005010
        );
        assert_pending_withdrawal(
            delegator_address,
            pool_address,
            true,
            1,
            false,
            10201000002
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_active_stake_rewards(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[];
        let principle_stake = vector[];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        // 100000000000 active stake * 1.01
        assert_delegation(
            validator_address,
            pool_address,
            1010 * ONE_SUPRA,
            0,
            0
        );

        // add stake in pending_active state
        stake::mint(validator, 200 * ONE_SUPRA);
        add_stake(validator, pool_address, 200 * ONE_SUPRA);

        let fee = get_add_stake_fee(pool_address, 200 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            1210 * ONE_SUPRA - fee,
            0,
            0
        );

        end_aptos_epoch();
        // 101000000000 active stake * 1.01 + 20000000000 pending_active stake with no rewards
        assert_delegation(
            validator_address,
            pool_address,
            122010000000,
            0,
            0
        );

        end_aptos_epoch();
        // 122010000000 active stake * 1.01
        assert_delegation(
            validator_address,
            pool_address,
            123230100000,
            0,
            0
        );

        // 123230100000 active stake * 1.01
        end_aptos_epoch();
        // 124462401000 active stake * 1.01
        end_aptos_epoch();
        // 125707025010 active stake * 1.01
        end_aptos_epoch();
        // 126964095260 active stake * 1.01
        end_aptos_epoch();
        // 128233736212 active stake * 1.01
        end_aptos_epoch();
        assert_delegation(
            validator_address,
            pool_address,
            129516073574,
            0,
            0
        );

        // unlock 200 coins from delegator `validator`
        unlock(validator, pool_address, 200 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            109516073575,
            0,
            19999999999
        );

        // end this lockup cycle
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // 109516073575 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        assert_delegation(
            validator_address,
            pool_address,
            110611234310,
            20199999998,
            0
        );

        end_aptos_epoch();
        // 110611234310 active stake * 1.01 + 20199999998 inactive stake
        assert_delegation(
            validator_address,
            pool_address,
            111717346653,
            20199999998,
            0
        );

        // add stake in pending_active state
        stake::mint(validator, 1000 * ONE_SUPRA);
        add_stake(validator, pool_address, 1000 * ONE_SUPRA);

        fee = get_add_stake_fee(pool_address, 1000 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            211717346653 - fee,
            20199999998,
            0
        );

        end_aptos_epoch();
        // 111717346653 active stake * 1.01 + 100000000000 pending_active stake + 20199999998 inactive stake
        assert_delegation(
            validator_address,
            pool_address,
            212834520119,
            20199999998,
            0
        );

        end_aptos_epoch();
        // 212834520119 active stake * 1.01 + 20199999998 inactive stake
        assert_delegation(
            validator_address,
            pool_address,
            214962865320,
            20199999998,
            0
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_active_stake_rewards_multiple(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            200 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);
        stake::mint(delegator, 300 * ONE_SUPRA);
        add_stake(delegator, pool_address, 300 * ONE_SUPRA);

        let fee = get_add_stake_fee(pool_address, 300 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            300 * ONE_SUPRA - fee,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            0
        );
        stake::assert_stake_pool(
            pool_address,
            200 * ONE_SUPRA,
            0,
            300 * ONE_SUPRA,
            0
        );

        end_aptos_epoch();
        // `delegator` got its `add_stake` fee back and `validator` its active stake rewards
        assert_delegation(
            delegator_address,
            pool_address,
            300 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            20199999999,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 502 * ONE_SUPRA, 0, 0, 0);

        // delegators earn their own rewards from now on
        end_aptos_epoch();
        assert_delegation(
            delegator_address,
            pool_address,
            303 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            20401999999,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 50702000000, 0, 0, 0);

        // delegators earn their own rewards from now on
        end_aptos_epoch();
        assert_delegation(
            delegator_address,
            pool_address,
            30603000000,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            20606019999,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 51209020000, 0, 0, 0);

        end_aptos_epoch();
        assert_delegation(
            delegator_address,
            pool_address,
            30909030000,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            20812080199,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 51721110200, 0, 0, 0);

        // add more stake in pending_active state than currently active
        stake::mint(delegator, 1000 * ONE_SUPRA);
        add_stake(delegator, pool_address, 1000 * ONE_SUPRA);

        fee = get_add_stake_fee(pool_address, 1000 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            130909030000 - fee,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            20812080199,
            0,
            0
        );

        end_aptos_epoch();
        // `delegator` got its `add_stake` fee back and `validator` its active stake rewards
        assert_delegation(
            delegator_address,
            pool_address,
            131218120300,
            0,
            0
        );
        assert_delegation(
            validator_address,
            pool_address,
            21020201001,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 152238321302, 0, 0, 0);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    public entry fun test_pending_inactive_stake_rewards(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[];
        let principle_stake = vector[];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        end_aptos_epoch();
        assert_delegation(
            validator_address,
            pool_address,
            1010 * ONE_SUPRA,
            0,
            0
        );

        // unlock 200 coins from delegator `validator`
        unlock(validator, pool_address, 200 * ONE_SUPRA);
        assert_delegation(
            validator_address,
            pool_address,
            81000000001,
            0,
            19999999999
        );

        end_aptos_epoch(); // 81000000001 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        end_aptos_epoch(); // 81810000001 active stake * 1.01 + 20199999998 pending_inactive stake * 1.01

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 82628100001 active stake * 1.01 + 20401999997 pending_inactive stake * 1.01
        end_aptos_epoch(); // 83454381001 active stake * 1.01 + 20606019996 pending_inactive stake(now inactive)
        assert_delegation(
            validator_address,
            pool_address,
            84288924811,
            20606019996,
            0
        );

        // unlock 200 coins from delegator `validator` which implicitly executes its pending withdrawal
        unlock(validator, pool_address, 200 * ONE_SUPRA);
        assert!(coin::balance<SupraCoin>(validator_address) == 20606019996, 0);
        assert_delegation(
            validator_address,
            pool_address,
            64288924812,
            0,
            19999999999
        );

        // lockup cycle is not ended, pending_inactive stake is still earning
        end_aptos_epoch(); // 64288924812 active stake * 1.01 + 19999999999 pending_inactive stake * 1.01
        end_aptos_epoch(); // 64931814060 active stake * 1.01 + 20199999998 pending_inactive stake * 1.01
        end_aptos_epoch(); // 65581132200 active stake * 1.01 + 20401999997 pending_inactive stake * 1.01
        end_aptos_epoch(); // 66236943522 active stake * 1.01 + 20606019996 pending_inactive stake * 1.01
        assert_delegation(
            validator_address,
            pool_address,
            66899312957,
            0,
            20812080195
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch(); // 66899312957 active stake * 1.01 + 20812080195 pending_inactive stake * 1.01
        end_aptos_epoch(); // 67568306086 active stake * 1.01 + 21020200996 pending_inactive stake(now inactive)
        end_aptos_epoch(); // 68243989147 active stake * 1.01 + 21020200996 inactive stake
        assert_delegation(
            validator_address,
            pool_address,
            68926429037,
            21020200996,
            0
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_out_of_order_redeem(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        stake::mint(delegator1, 300 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 300 * ONE_SUPRA);

        stake::mint(delegator2, 300 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 300 * ONE_SUPRA);

        end_aptos_epoch();

        // create the pending withdrawal of delegator 1 in lockup cycle 0
        unlock(delegator1, pool_address, 150 * ONE_SUPRA);
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            true,
            0,
            false,
            14999999999
        );

        // move to lockup cycle 1
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // create the pending withdrawal of delegator 2 in lockup cycle 1
        unlock(delegator2, pool_address, 150 * ONE_SUPRA);
        assert_pending_withdrawal(
            delegator2_address,
            pool_address,
            true,
            1,
            false,
            14999999999
        );
        // 14999999999 pending_inactive stake * 1.01
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            true,
            0,
            true,
            15149999998
        );

        // move to lockup cycle 2
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_pending_withdrawal(
            delegator2_address,
            pool_address,
            true,
            1,
            true,
            15149999998
        );
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            true,
            0,
            true,
            15149999998
        );

        // both delegators who unlocked at different lockup cycles should be able to withdraw their stakes
        withdraw(delegator1, pool_address, 15149999998);
        withdraw(delegator2, pool_address, 5149999998);

        assert_pending_withdrawal(
            delegator2_address,
            pool_address,
            true,
            1,
            true,
            10000000001
        );
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        assert!(coin::balance<SupraCoin>(delegator1_address) == 15149999998, 0);
        assert!(coin::balance<SupraCoin>(delegator2_address) == 5149999997, 0);

        // recreate the pending withdrawal of delegator 1 in lockup cycle 2
        unlock(delegator1, pool_address, 100 * ONE_SUPRA);

        // move to lockup cycle 3
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        assert_pending_withdrawal(
            delegator2_address,
            pool_address,
            true,
            1,
            true,
            10000000001
        );
        // 9999999999 pending_inactive stake * 1.01
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            true,
            2,
            true,
            10099999998
        );

        // withdraw inactive stake of delegator 2 left from lockup cycle 1 in cycle 3
        withdraw(delegator2, pool_address, 10000000001);
        assert!(coin::balance<SupraCoin>(delegator2_address) == 15149999998, 0);
        assert_pending_withdrawal(
            delegator2_address,
            pool_address,
            false,
            0,
            false,
            0
        );

        // withdraw inactive stake of delegator 1 left from previous lockup cycle
        withdraw(delegator1, pool_address, 10099999998);
        assert!(
            coin::balance<SupraCoin>(delegator1_address) == 15149999998 + 10099999998,
            0
        );
        assert_pending_withdrawal(
            delegator1_address,
            pool_address,
            false,
            0,
            false,
            0
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_operator_fee(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let validator_address = signer::address_of(validator);
        account::create_account_for_test(validator_address);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;

        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(
            validator,
            option::none(),
            1265,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[1],
            1,
            principle_lockup_time,
            60
        );
        let pool_address = get_owned_pool_address(validator_address);
        assert!(stake::get_operator(pool_address) == validator_address, 0);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        // validator is inactive and added stake is instantly `active`
        stake::assert_stake_pool(pool_address, 300 * ONE_SUPRA, 0, 0, 0);

        // validator does not produce rewards yet
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 300 * ONE_SUPRA, 0, 0, 0);

        // therefore, there are no operator commission rewards yet
        assert_delegation(validator_address, pool_address, 0, 0, 0);

        // activate validator
        stake::rotate_consensus_key(validator, pool_address, CONSENSUS_KEY_1);
        stake::join_validator_set(validator, pool_address);
        end_aptos_epoch();

        // produce active rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30300000000, 0, 0, 0);

        // 300000000 active rewards * 0.1265
        assert_delegation(validator_address, pool_address, 37950000, 0, 0);
        // 10000000000 active stake * (1 + 1% reward-rate * 0.8735)
        assert_delegation(
            delegator1_address,
            pool_address,
            10087350000,
            0,
            0
        );
        // 20000000000 active stake * 1.008735
        assert_delegation(
            delegator2_address,
            pool_address,
            20174700000,
            0,
            0
        );

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30603000000, 0, 0, 0);

        // 603000000 active rewards * 0.1265 instead of
        // 303000000 active rewards * 0.1265 + 37950000 active stake * 1.008735
        // because operator commission rewards are not automatically restaked compared to already owned stake
        assert_delegation(validator_address, pool_address, 76279500, 0, 0);
        // 10087350000 active stake * 1.008735 + some of the rewards of previous commission if restaked
        assert_delegation(
            delegator1_address,
            pool_address,
            10175573500,
            0,
            0
        );
        // 20174700000 active stake * 1.008735 + some of the rewards of previous commission if restaked
        assert_delegation(
            delegator2_address,
            pool_address,
            20351147000,
            0,
            0
        );

        // restake operator commission rewards
        synchronize_delegation_pool(pool_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 30909030000, 0, 0, 0);

        // 306030000 active rewards * 0.1265 + 76279500 active stake * 1.008735
        assert_delegation(
            validator_address,
            pool_address,
            115658596,
            0,
            0
        );
        // 10175573500 active stake * 1.008735
        assert_delegation(
            delegator1_address,
            pool_address,
            10264457134,
            0,
            0
        );
        // 20351147000 active stake * 1.008735
        assert_delegation(
            delegator2_address,
            pool_address,
            20528914269,
            0,
            0
        );

        // check operator is rewarded by pending_inactive stake too
        unlock(delegator2, pool_address, 100 * ONE_SUPRA);
        stake::assert_stake_pool(pool_address, 20909030001, 0, 0, 9999999999);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 21118120301, 0, 0, 10099999998);

        assert_pending_withdrawal(
            validator_address,
            pool_address,
            false,
            0,
            false,
            0
        );
        // distribute operator pending_inactive commission rewards
        synchronize_delegation_pool(pool_address);
        // 99999999 pending_inactive rewards * 0.1265
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            false,
            12649998
        );

        // 209090300 active rewards * 0.1265 + 115658596 active stake * 1.008735
        // 99999999 pending_inactive rewards * 0.1265
        assert_delegation(
            validator_address,
            pool_address,
            143118796,
            0,
            12649998
        );
        // 10264457134 active stake * 1.008735
        assert_delegation(
            delegator1_address,
            pool_address,
            10354117168,
            0,
            0
        );
        // 10528914270 active stake * 1.008735
        // 9999999999 pending_inactive stake * 1.008735
        assert_delegation(
            delegator2_address,
            pool_address,
            10620884336,
            0,
            10087349999
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 21329301504, 10200999997, 0, 0);

        // operator pending_inactive rewards on previous epoch have been inactivated
        // 211181203 active rewards * 0.1265 + 143118796 active stake * 1.008735
        // 100999999 pending_inactive rewards * 0.1265 + 12649998 pending_inactive stake * 1.008735
        assert_delegation(
            validator_address,
            pool_address,
            171083360,
            25536995,
            0
        );
        // distribute operator pending_inactive commission rewards
        synchronize_delegation_pool(pool_address);
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            25536995
        );

        // check operator is not rewarded by `add_stake` fees
        stake::mint(delegator1, 100 * ONE_SUPRA);
        assert!(
            get_add_stake_fee(pool_address, 100 * ONE_SUPRA) > 0,
            0
        );
        add_stake(delegator1, pool_address, 100 * ONE_SUPRA);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 31542594519, 10200999997, 0, 0);

        // 213293015 active rewards * 0.1265 + 171083360 active stake * 1.008735
        assert_delegation(
            validator_address,
            pool_address,
            199559340,
            25536995,
            0
        );

        // unlock some more stake to produce pending_inactive commission
        // 10620884336 active stake * (1.008735 ^ 2 epochs)
        // 10087349999 pending_inactive stake * 1.008735
        assert_delegation(
            delegator2_address,
            pool_address,
            10807241561,
            10175463001,
            0
        );
        unlock(delegator2, pool_address, 100 * ONE_SUPRA);
        // 10807241561 - 100 SUPRA < `MIN_COINS_ON_SHARES_POOL` thus active stake is entirely unlocked
        assert_delegation(
            delegator2_address,
            pool_address,
            807241561,
            0,
            9999999999
        );
        end_aptos_epoch();

        // in-flight pending_inactive commission can coexist with previous inactive commission
        assert_delegation(
            validator_address,
            pool_address,
            228553872,
            25536996,
            12649999
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            0,
            true,
            25536996
        );

        // distribute in-flight pending_inactive commission, implicitly executing the inactive withdrawal of operator
        coin::register<SupraCoin>(validator);
        synchronize_delegation_pool(pool_address);
        assert!(coin::balance<SupraCoin>(validator_address) == 25536996, 0);

        // in-flight commission has been synced, implicitly used to buy shares for operator
        // expect operator stake to be slightly less than previously reported by `Self::get_stake`
        assert_delegation(
            validator_address,
            pool_address,
            228553872,
            0,
            12649998
        );
        assert_pending_withdrawal(
            validator_address,
            pool_address,
            true,
            1,
            false,
            12649998
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            old_operator = @0x123,
            delegator = @0x010,
            new_operator = @0x020
        )
    ]
    public entry fun test_change_operator(
        supra_framework: &signer,
        old_operator: &signer,
        delegator: &signer,
        new_operator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);

        let old_operator_address = signer::address_of(old_operator);
        account::create_account_for_test(old_operator_address);

        let new_operator_address = signer::address_of(new_operator);
        account::create_account_for_test(new_operator_address);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;

        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(
            old_operator,
            option::none(),
            1265,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(old_operator_address);
        assert!(stake::get_operator(pool_address) == old_operator_address, 0);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_SUPRA);
        add_stake(delegator, pool_address, 200 * ONE_SUPRA);
        unlock(delegator, pool_address, 100 * ONE_SUPRA);

        // activate validator
        stake::rotate_consensus_key(old_operator, pool_address, CONSENSUS_KEY_1);
        stake::join_validator_set(old_operator, pool_address);
        end_aptos_epoch();

        // produce active and pending_inactive rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10100000000, 0, 0, 10100000000);
        assert_delegation(
            old_operator_address,
            pool_address,
            12650000,
            0,
            12650000
        );
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10201000000, 0, 0, 10201000000);
        assert_delegation(
            old_operator_address,
            pool_address,
            25426500,
            0,
            25426500
        );

        // change operator
        set_operator(old_operator, new_operator_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10303010000, 0, 0, 10303010000);
        // 25426500 active stake * 1.008735 and 25426500 pending_inactive stake * 1.008735
        assert_delegation(
            old_operator_address,
            pool_address,
            25648600,
            0,
            25648600
        );
        // 102010000 active rewards * 0.1265 and 102010000 pending_inactive rewards * 0.1265
        assert_delegation(
            new_operator_address,
            pool_address,
            12904265,
            0,
            12904265
        );

        // restake `new_operator` commission rewards
        synchronize_delegation_pool(pool_address);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10406040100, 0, 0, 10406040100);
        // 25648600 active stake * 1.008735 and 25648600 pending_inactive stake * 1.008735
        assert_delegation(
            old_operator_address,
            pool_address,
            25872641,
            0,
            25872641
        );
        // 103030100 active rewards * 0.1265 and 12904265 active stake * 1.008735
        // 103030100 pending_inactive rewards * 0.1265 and 12904265 pending_inactive stake * 1.008735
        assert_delegation(
            new_operator_address,
            pool_address,
            26050290,
            0,
            26050290
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            operator1 = @0x123,
            delegator = @0x010,
            beneficiary = @0x020,
            operator2 = @0x030
        )
    ]
    public entry fun test_set_beneficiary_for_operator(
        supra_framework: &signer,
        operator1: &signer,
        delegator: &signer,
        beneficiary: &signer,
        operator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);

        let operator1_address = signer::address_of(operator1);
        supra_account::create_account(operator1_address);

        let operator2_address = signer::address_of(operator2);
        supra_account::create_account(operator2_address);

        let beneficiary_address = signer::address_of(beneficiary);
        supra_account::create_account(beneficiary_address);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(
            operator1,
            option::none(),
            1265,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(operator1_address);
        assert!(stake::get_operator(pool_address) == operator1_address, 0);
        assert!(beneficiary_for_operator(operator1_address) == operator1_address, 0);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);
        stake::mint(delegator, 2000000 * ONE_SUPRA);
        add_stake(delegator, pool_address, 2000000 * ONE_SUPRA);
        unlock(delegator, pool_address, 1000000 * ONE_SUPRA);

        // activate validator
        stake::rotate_consensus_key(operator1, pool_address, CONSENSUS_KEY_1);
        stake::join_validator_set(operator1, pool_address);
        end_aptos_epoch();

        // produce active and pending_inactive rewards
        end_aptos_epoch();
        stake::assert_stake_pool(
            pool_address,
            101000000000000,
            0,
            0,
            101000000000000
        );
        assert_delegation(
            operator1_address,
            pool_address,
            126500000000,
            0,
            126500000000
        );
        end_aptos_epoch();
        stake::assert_stake_pool(
            pool_address,
            102010000000000,
            0,
            0,
            102010000000000
        );
        assert_delegation(
            operator1_address,
            pool_address,
            254265000000,
            0,
            254265000000
        );
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(operator1, pool_address, ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(operator1_address) == ONE_SUPRA - 1,
            0
        );

        set_beneficiary_for_operator(operator1, beneficiary_address);
        assert!(beneficiary_for_operator(operator1_address) == beneficiary_address, 0);
        end_aptos_epoch();

        unlock(beneficiary, pool_address, ONE_SUPRA);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(beneficiary, pool_address, ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(beneficiary_address) == ONE_SUPRA - 1,
            0
        );
        assert!(
            coin::balance<SupraCoin>(operator1_address) == ONE_SUPRA - 1,
            0
        );

        // switch operator to operator2. The rewards should go to operator2 not to the beneficiay of operator1.
        set_operator(operator1, operator2_address);
        end_aptos_epoch();
        unlock(operator2, pool_address, ONE_SUPRA);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(operator2, pool_address, ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(beneficiary_address) == ONE_SUPRA - 1,
            0
        );
        assert!(
            coin::balance<SupraCoin>(operator2_address) == ONE_SUPRA - 1,
            0
        );
    }

    #[test(supra_framework = @supra_framework, operator = @0x123, delegator = @0x010)]
    public entry fun test_update_commission_percentage(
        supra_framework: &signer, operator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);

        let operator_address = signer::address_of(operator);
        account::create_account_for_test(operator_address);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[0];
        let coin = stake::mint_coins(0);
        let principle_lockup_time = 0;
        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(
            operator,
            option::none(),
            1265,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(operator_address);
        assert!(stake::get_operator(pool_address) == operator_address, 0);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_SUPRA);
        add_stake(delegator, pool_address, 200 * ONE_SUPRA);
        unlock(delegator, pool_address, 100 * ONE_SUPRA);

        // activate validator
        stake::rotate_consensus_key(operator, pool_address, CONSENSUS_KEY_1);
        stake::join_validator_set(operator, pool_address);
        end_aptos_epoch();

        // produce active and pending_inactive rewards
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10100000000, 0, 0, 10100000000);
        assert_delegation(
            operator_address,
            pool_address,
            12650000,
            0,
            12650000
        );
        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10201000000, 0, 0, 10201000000);
        assert_delegation(
            operator_address,
            pool_address,
            25426500,
            0,
            25426500
        );

        // change the commission percentage
        update_commission_percentage(operator, 2265);
        // the new commission percentage does not take effect until the next lockup cycle.
        assert!(operator_commission_percentage(pool_address) == 1265, 0);

        // end the lockup cycle
        fast_forward_to_unlock(pool_address);

        // Test that the `get_add_stake_fee` correctly uses the new commission percentage, and returns the correct
        // fee amount 76756290 in the following case, not 86593604 (calculated with the old commission rate).
        assert!(
            get_add_stake_fee(pool_address, 100 * ONE_SUPRA) == 76756290,
            0
        );

        synchronize_delegation_pool(pool_address);
        // the commission percentage is updated to the new one.
        assert!(operator_commission_percentage(pool_address) == 2265, 0);

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10406040100, 10303010000, 0, 0);
        assert_delegation(
            operator_address,
            pool_address,
            62187388,
            38552865,
            0
        );

        end_aptos_epoch();
        stake::assert_stake_pool(pool_address, 10510100501, 10303010000, 0, 0);
        assert_delegation(
            operator_address,
            pool_address,
            86058258,
            38552865,
            0
        );
    }

    #[test(supra_framework = @supra_framework, operator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 196629, location = Self)]
    public entry fun test_last_minute_commission_rate_change_failed(
        supra_framework: &signer, operator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let operator_address = signer::address_of(operator);
        account::create_account_for_test(operator_address);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        // create delegation pool of commission fee 12.65%
        initialize_delegation_pool(
            operator,
            option::none(),
            1265,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let pool_address = get_owned_pool_address(operator_address);
        assert!(stake::get_operator(pool_address) == operator_address, 0);

        let delegator_address = signer::address_of(delegator);
        account::create_account_for_test(delegator_address);

        stake::mint(delegator, 200 * ONE_SUPRA);
        add_stake(delegator, pool_address, 200 * ONE_SUPRA);
        unlock(delegator, pool_address, 100 * ONE_SUPRA);

        // activate validator
        stake::rotate_consensus_key(operator, pool_address, CONSENSUS_KEY_1);
        stake::join_validator_set(operator, pool_address);
        end_aptos_epoch();

        // 30 days are remaining in the lockup period.
        update_commission_percentage(operator, 2215);
        timestamp::fast_forward_seconds(7 * 24 * 60 * 60);
        end_aptos_epoch();

        // 23 days are remaining in the lockup period.
        update_commission_percentage(operator, 2225);
        timestamp::fast_forward_seconds(7 * 24 * 60 * 60);
        end_aptos_epoch();

        // 16 days are remaining in the lockup period.
        update_commission_percentage(operator, 2235);
        timestamp::fast_forward_seconds(7 * 24 * 60 * 60);
        end_aptos_epoch();

        // 9 days are remaining in the lockup period.
        update_commission_percentage(operator, 2245);
        timestamp::fast_forward_seconds(7 * 24 * 60 * 60);
        end_aptos_epoch();

        // 2 days are remaining in the lockup period. So, the following line is expected to fail.
        update_commission_percentage(operator, 2255);
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_min_stake_is_preserved(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x111];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            100 * ONE_SUPRA,
            true,
            false,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let delegator1_address = signer::address_of(delegator1);
        account::create_account_for_test(delegator1_address);

        let delegator2_address = signer::address_of(delegator2);
        account::create_account_for_test(delegator2_address);

        // add stake without fees as validator is not active yet
        stake::mint(delegator1, 50 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 50 * ONE_SUPRA);
        stake::mint(delegator2, 16 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 16 * ONE_SUPRA);

        // validator becomes active and share price is 1
        end_aptos_epoch();

        assert_delegation(
            delegator1_address,
            pool_address,
            5000000000,
            0,
            0
        );
        // pending_inactive balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4900000000,
            0,
            100000000
        );

        // pending_inactive balance is over threshold
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            5000000000,
            0,
            0
        );

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            5000000000,
            0,
            0
        );

        // active balance would be under threshold => move entire balance
        unlock(
            delegator1,
            pool_address,
            5000000000 - (MIN_COINS_ON_SHARES_POOL - 1)
        );
        assert_delegation(
            delegator1_address,
            pool_address,
            0,
            0,
            5000000000
        );

        // active balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            100000000,
            0,
            4900000000
        );

        // active balance is over threshold
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            0,
            0,
            5000000000
        );

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(
            delegator1,
            pool_address,
            4000000000 - (MIN_COINS_ON_SHARES_POOL - 1)
        );
        assert_delegation(
            delegator1_address,
            pool_address,
            3900000001,
            0,
            1099999999
        );

        // active + pending_inactive balance < 2 * MIN_COINS_ON_SHARES_POOL
        // stake can live on only one of the shares pools
        assert_delegation(
            delegator2_address,
            pool_address,
            16 * ONE_SUPRA,
            0,
            0
        );
        unlock(delegator2, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator2_address,
            pool_address,
            1500000000,
            0,
            100000000
        );
        reactivate_stake(delegator2, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator2_address,
            pool_address,
            1600000000,
            0,
            0
        );

        unlock(delegator2, pool_address, ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            1500000000,
            0,
            100000000
        );
        reactivate_stake(delegator2, pool_address, 2 * ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            16 * ONE_SUPRA,
            0,
            0
        );

        // share price becomes 1.01 on both pools
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            3800000001,
            0,
            1199999999
        );
        end_aptos_epoch();
        assert_delegation(
            delegator1_address,
            pool_address,
            3838000001,
            0,
            1211999998
        );

        // pending_inactive balance is over threshold
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            3938000000,
            0,
            1111999999
        );

        // 1 coin < 1.01 so no shares are redeemed
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4037999999,
            0,
            1012000000
        );

        // pending_inactive balance is over threshold
        // requesting 2 coins actually redeems 1 coin from pending_inactive pool
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4137999998,
            0,
            912000001
        );

        // 1 coin < 1.01 so no shares are redeemed
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4237999997,
            0,
            812000002
        );

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4337999996,
            0,
            712000003
        );

        // pending_inactive balance would be under threshold => move MIN_COINS_ON_SHARES_POOL coins
        unlock(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4237999996,
            0,
            812000002
        );

        // pending_inactive balance would be under threshold => move entire balance
        reactivate_stake(delegator1, pool_address, MIN_COINS_ON_SHARES_POOL);
        assert_delegation(
            delegator1_address,
            pool_address,
            4337999995,
            0,
            712000003
        );
    }

    #[test(staker = @0xe256f4f4e2986cada739e339895cf5585082ff247464cab8ec56eea726bd2263)]
    public entry fun test_get_expected_stake_pool_address(staker: address) {
        let pool_address = get_expected_stake_pool_address(staker, vector[0x42, 0x42]);
        assert!(
            pool_address
                == @0xcb5678be9ec64067c2c3f9f8de78e19509411b053d723d2788ebf1f7ba02f04b,
            0
        );
    }

    #[test_only]
    public fun assert_delegation(
        delegator_address: address,
        pool_address: address,
        active_stake: u64,
        inactive_stake: u64,
        pending_inactive_stake: u64
    ) acquires DelegationPool, BeneficiaryForOperator {
        let (actual_active, actual_inactive, actual_pending_inactive) =
            get_stake(pool_address, delegator_address);
        assert!(actual_active == active_stake, actual_active);
        assert!(actual_inactive == inactive_stake, actual_inactive);
        assert!(
            actual_pending_inactive == pending_inactive_stake, actual_pending_inactive
        );
    }

    #[test_only]
    public fun assert_pending_withdrawal(
        delegator_address: address,
        pool_address: address,
        exists: bool,
        olc: u64,
        inactive: bool,
        stake: u64
    ) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        let (withdrawal_exists, withdrawal_olc) =
            pending_withdrawal_exists(pool, delegator_address);
        assert!(withdrawal_exists == exists, 0);
        assert!(withdrawal_olc.index == olc, withdrawal_olc.index);
        let (withdrawal_inactive, withdrawal_stake) =
            get_pending_withdrawal(pool_address, delegator_address);
        assert!(withdrawal_inactive == inactive, 0);
        assert!(withdrawal_stake == stake, withdrawal_stake);
    }

    #[test_only]
    public fun assert_inactive_shares_pool(
        pool_address: address,
        olc: u64,
        exists: bool,
        stake: u64
    ) acquires DelegationPool {
        assert_delegation_pool_exists(pool_address);
        let pool = borrow_global<DelegationPool>(pool_address);
        assert!(table::contains(&pool.inactive_shares, olc_with_index(olc)) == exists, 0);
        if (exists) {
            let actual_stake =
                total_coins(table::borrow(&pool.inactive_shares, olc_with_index(olc)));
            assert!(actual_stake == stake, actual_stake);
        } else {
            assert!(0 == stake, 0);
        }
    }

    #[test_only]
    public fun total_coins_inactive(pool_address: address): u64 acquires DelegationPool {
        borrow_global<DelegationPool>(pool_address).total_coins_inactive
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 65561, location = Self)]
    public entry fun test_withdraw_before_principle_lockup_time_fail(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 1000000;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        // validator has 1000 SUPRA active stake and it is not in the table.
        unlock(validator, pool_address, 1000 * ONE_SUPRA);
        // Expected an error as the active share will fall below the principle stake for delegator.
        unlock(delegator, pool_address, 10 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_withdraw_after_principle_lockup_time(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 1000000;
        initialize_test_validator(
            validator,
            1000 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        timestamp::fast_forward_seconds(1000000);
        // after the principle lockup_time, unlock all the stake is fine
        unlock(delegator, pool_address, 100 * ONE_SUPRA);
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_unlock_mutiple_delegators(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        unlock(delegator1, pool_address, 11 * ONE_SUPRA);
        unlock(delegator2, pool_address, 21 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator1 = @0x010)]
    public entry fun test_unlock_mutiple_times(
        supra_framework: &signer, validator: &signer, delegator1: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[1000 * ONE_SUPRA, 1000 * ONE_SUPRA];
        let coin = stake::mint_coins(2000 * ONE_SUPRA);
        let principle_lockup_time = 1000000;
        let delegator1_address = signer::address_of(delegator1);
        supra_account::create_account(delegator1_address);
        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        stake::mint(delegator1, 1000 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 1000 * ONE_SUPRA);
        // There is fee apply when unlock stake
        unlock(delegator1, pool_address, 200 * ONE_SUPRA);
        unlock(delegator1, pool_address, 600 * ONE_SUPRA);
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    #[expected_failure(abort_code = 65561, location = Self)]
    public entry fun test_multiple_users(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010];
        let principle_stake = vector[1000 * ONE_SUPRA];
        let coin = stake::mint_coins(1000 * ONE_SUPRA);
        let principle_lockup_time = 1000000;
        let delegator1_address = signer::address_of(delegator1);
        let delegator2_address = signer::address_of(delegator2);
        supra_account::create_account(delegator1_address);
        supra_account::create_account(delegator2_address);
        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        stake::mint(delegator1, 1000 * ONE_SUPRA);
        add_stake(delegator1, pool_address, 1000 * ONE_SUPRA);
        stake::mint(delegator2, 1000 * ONE_SUPRA);
        add_stake(delegator2, pool_address, 1000 * ONE_SUPRA);
        end_aptos_epoch();
        assert_delegation(
            delegator2_address,
            pool_address,
            1000 * ONE_SUPRA,
            0,
            0
        );
        assert_delegation(
            delegator1_address,
            pool_address,
            200999999999,
            0,
            0
        );
        unlock(delegator2, pool_address, 1000 * ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            0,
            0,
            1000 * ONE_SUPRA
        );
        unlock(delegator1, pool_address, 1000 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            101000000001,
            0,
            99999999999
        );
        unlock(delegator1, pool_address, 1000 * ONE_SUPRA);
    }

    #[test_only]
    fun generate_multisig_account(
        owner: &signer, addition_owner: vector<address>, threshold: u64
    ): address {
        let owner_addr = aptos_std::signer::address_of(owner);
        let multisig_addr =
            multisig_account::get_next_multisig_account_address(owner_addr);
        multisig_account::create_with_owners(
            owner,
            addition_owner,
            threshold,
            vector[],
            vector[],
            300
        );
        multisig_addr
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 327716, location = Self)]
    /// if admin is option::none() calling to `replace_delegator` should fail
    public entry fun test_replace_delegation_without_multisig_failure(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        replace_delegator(
            &account::create_signer_for_test(multisig),
            pool_address,
            @0x010,
            @0x0101
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = EADMIN_NOT_MULTISIG, location = Self)]
    /// if admin is a single signer account pool creation should fail
    public entry fun test_initialize_delegation_pool_with_single_multisig_owner_failure(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[], 1);
        initialize_delegation_pool(
            validator,
            option::some(multisig),
            0,
            vector::empty<u8>(),
            delegator_address,
            principle_stake,
            coin,
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    #[expected_failure(abort_code = 327716, location = Self)]
    /// if admin is multi signer calling `replace_delegator` if it's not the same signer which was initialized, it should fail
    public entry fun test_replace_delegation_with_different_multisig_failure(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let new_multisig = generate_multisig_account(
            supra_framework, vector[@0x12234], 2
        );
        let multisig_signer = account::create_signer_for_test(new_multisig);

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        replace_delegator(
            &multisig_signer,
            pool_address,
            @0x010,
            @0x0101
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_lose_shares_small(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[ONE_SUPRA, 1000 * ONE_SUPRA];
        let coin = stake::mint_coins(1001 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        stake::mint(validator, 150 * ONE_SUPRA);
        add_stake(validator, pool_address, 150 * ONE_SUPRA);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );
        let index = 0;
        while (index < 1828) {
            end_aptos_epoch();
            index = index + 1;
        };
        let delegator1_address = signer::address_of(delegator1);
        let delegator2_address = signer::address_of(delegator2);
        assert_delegation(
            delegator1_address,
            pool_address,
            7933617798152065,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 9913173264836398460, 0, 0, 0);
        unlock(delegator1, pool_address, 1 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            7933617698152064,
            0,
            100000000
        );
        stake::assert_stake_pool(
            pool_address,
            9913173264736398460,
            0,
            0,
            100000000
        );
        unlock(delegator1, pool_address, 1 * ONE_SUPRA);
        stake::assert_stake_pool(
            pool_address,
            9913173264636398461,
            0,
            0,
            199999999
        );
        assert_delegation(
            delegator1_address,
            pool_address,
            7933617598152064,
            0,
            199999999
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            7933617798152065133,
            0,
            0
        );
        unlock(delegator2, pool_address, 1000 * ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            7933617698152065134,
            0,
            99999999999
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator1 = @0x010,
            delegator2 = @0x020
        )
    ]
    public entry fun test_lose_shares_large(
        supra_framework: &signer,
        validator: &signer,
        delegator1: &signer,
        delegator2: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        let delegator_address = vector[@0x010, @0x020];
        let principle_stake = vector[ONE_SUPRA, 90000000000 * ONE_SUPRA];
        let coin = stake::mint_coins(90000000001 * ONE_SUPRA);
        let principle_lockup_time = 0;
        initialize_test_validator(
            validator,
            100 * ONE_SUPRA,
            true,
            true,
            0,
            delegator_address,
            principle_stake,
            coin,
            option::none(),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        stake::mint(validator, 150 * ONE_SUPRA);
        add_stake(validator, pool_address, 150 * ONE_SUPRA);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );
        let index = 0;
        while (index < 10) {
            end_aptos_epoch();
            index = index + 1;
        };
        let delegator1_address = signer::address_of(delegator1);
        let delegator2_address = signer::address_of(delegator2);
        assert_delegation(
            delegator1_address,
            pool_address,
            110462212,
            0,
            0
        );
        stake::assert_stake_pool(pool_address, 9941599156262803145, 0, 0, 0);
        unlock(delegator1, pool_address, 1 * ONE_SUPRA);
        assert_delegation(
            delegator1_address,
            pool_address,
            0,
            0,
            110462212
        );
        stake::assert_stake_pool(
            pool_address,
            9941599156152340933,
            0,
            0,
            110462212
        );
        stake::assert_stake_pool(
            pool_address,
            9941599156152340933,
            0,
            0,
            110462212
        );
        assert_delegation(
            delegator1_address,
            pool_address,
            0,
            0,
            110462212
        );
        assert_delegation(
            delegator2_address,
            pool_address,
            9941599128700840588,
            0,
            0
        );
        unlock(delegator2, pool_address, 1 * ONE_SUPRA);
        assert_delegation(
            delegator2_address,
            pool_address,
            9941599128600840588,
            0,
            100000000
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123)]
    /// if admin is authorized multi signer, `replace_delegator` should succeed
    public entry fun test_replace_delegation_multisig_success(
        supra_framework: &signer, validator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let old_delegator = @0x010;
        let delegator_address_vec = vector[old_delegator, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let multisig_signer = account::create_signer_for_test(multisig);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let new_delegator = @0x0101;
        assert_delegation(
            old_delegator,
            pool_address,
            100 * ONE_SUPRA,
            0,
            0
        );

        replace_delegator(
            &multisig_signer,
            pool_address,
            @0x010,
            new_delegator
        );
        assert_delegation(
            new_delegator,
            pool_address,
            100 * ONE_SUPRA,
            0,
            0
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// if old_delegator has already unlocked 100, the new_delegator should be able to withdraw 100 coins
    public entry fun test_replace_delegation_before_withdraw_and_after_withdraw_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        unlock(delegator, pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            delegator_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            100 * ONE_SUPRA
        );

        let multisig_signer = account::create_signer_for_test(multisig);
        let new_delegator_address = @0x0101;
        replace_delegator(
            &multisig_signer,
            pool_address,
            delegator_address,
            new_delegator_address
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(
            &account::create_signer_for_test(new_delegator_address),
            pool_address,
            100 * ONE_SUPRA
        );
        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA) - 1,
            0
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 65545, location = Self)]
    /// after replace_delegator` succeeds, old_delegator must not be able to perform unlock or withdraw or vote (if partial_voting is enable)
    public entry fun test_replace_delegation_after_withdraw_using_old_address_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let multisig_signer = account::create_signer_for_test(multisig);
        let new_delegator_address = @0x0101;
        replace_delegator(
            &multisig_signer,
            pool_address,
            delegator_address,
            new_delegator_address
        );

        unlock(delegator, pool_address, 100 * ONE_SUPRA);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(delegator, pool_address, 100 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// after replace_delegator succeeds, new_delegator should be able to perform unlock and withdraw on the funds as per unlocking schedule
    public entry fun test_replace_delegation_unlock_and_withdraw_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let multisig_signer = account::create_signer_for_test(multisig);
        let new_delegator_address = @0x0101;
        let new_delegator_address_signer =
            &account::create_signer_for_test(new_delegator_address);
        replace_delegator(
            &multisig_signer,
            pool_address,
            delegator_address,
            new_delegator_address
        );

        unlock(new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);
        assert_delegation(
            new_delegator_address,
            pool_address,
            200 * ONE_SUPRA,
            0,
            100 * ONE_SUPRA
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA) - 1,
            0
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator = @0x010,
            funder = @0x999
        )
    ]
    /// if delegator is not part of one of the principle stake holder, and not funded with locked stake,
    /// they can unlock/withdraw without restriction
    public entry fun test_unlock_withdraw_multiple_funded_delegator_not_part_of_principle_stake_success(
        supra_framework: &signer,
        validator: &signer,
        delegator: &signer,
        funder: address
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);
        let new_delegator_address2 = @0x0216;
        let new_delegator_address_signer2 =
            account::create_account_for_test(new_delegator_address2);
        let funder_signer = account::create_account_for_test(funder);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(&new_delegator_address_signer, pool_address, (30 * ONE_SUPRA));
        unlock(&new_delegator_address_signer2, pool_address, (70 * ONE_SUPRA));

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        withdraw(&new_delegator_address_signer, pool_address, (30 * ONE_SUPRA));
        let new_delegator_balance = coin::balance<SupraCoin>(new_delegator_address);
        assert!(
            new_delegator_balance == (30 * ONE_SUPRA) - 1,
            new_delegator_balance
        );
        withdraw(&new_delegator_address_signer2, pool_address, (70 * ONE_SUPRA));
        let new_delegator_balance2 = coin::balance<SupraCoin>(new_delegator_address2);
        assert!(
            new_delegator_balance2 == (70 * ONE_SUPRA) - 1,
            new_delegator_balance2
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 327716, location = Self)]
    /// Test that if a if some random person tries to add delegator locked stake, it does not get added to
    /// `principle_stake` table and therefore remains outside the purview of replacement
    public entry fun test_unlock_delegator_not_part_of_principle_stake_cannot_be_locked_by_strangers_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 7776000;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        stake::mint(&new_delegator_address_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA),
            0
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        add_stake(&new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);
        fund_delegators_with_locked_stake(
            validator,
            pool_address,
            vector[new_delegator_address],
            vector[1 * ONE_SUPRA]
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// Test that if a multisig admin adds a delegator with zero stake, it does not get added to
    /// `principle_stake` table and therefore remains outside the purview of replacement
    public entry fun test_unlock_zero_funded_delegator_not_part_of_principle_stake_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 7776000;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        stake::mint(&new_delegator_address_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA),
            0
        );

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        add_stake(&new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);
        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address],
            vector[0]
        );
        {
            // if the locked stake is zero, assert that it does not get added to `principle_stake` table
            let pool = borrow_global<DelegationPool>(pool_address);
            assert!(!table::contains(&pool.principle_stake, new_delegator_address), 999);
        };

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        withdraw(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));
        let new_delegator_balance = coin::balance<SupraCoin>(new_delegator_address);
        assert!(
            new_delegator_balance == (100 * ONE_SUPRA) - 1,
            new_delegator_balance
        );
    }

    #[
        test(
            supra_framework = @supra_framework,
            validator = @0x123,
            delegator = @0x010,
            funder = @0x999
        )
    ]
    /// if a single delegator was not part of one of the principle stake holder, and not funded with locked stake,
    /// they can unlock/withdraw without restriction
    public entry fun test_unlock_withdraw_funded_delegator_not_part_of_principle_stake_success(
        supra_framework: &signer,
        validator: &signer,
        delegator: &signer,
        funder: address
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);
        let funder_signer = account::create_account_for_test(funder);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        fund_delegator_stake(
            &funder_signer,
            pool_address,
            new_delegator_address,
            100 * ONE_SUPRA
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        withdraw(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));
        let new_delegator_balance = coin::balance<SupraCoin>(new_delegator_address);
        assert!(
            new_delegator_balance == (100 * ONE_SUPRA) - 1,
            new_delegator_balance
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// if delegator is not part of one of the principle stake holder, they can unlock/withdraw without restriction
    public entry fun test_unlock_withdraw_delegator_not_part_of_principle_stake_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 0;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            12
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);

        stake::mint(&new_delegator_address_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA),
            0
        );

        add_stake(&new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);
        assert!(coin::balance<SupraCoin>(new_delegator_address) == 0, 0);

        unlock(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));

        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        withdraw(&new_delegator_address_signer, pool_address, (100 * ONE_SUPRA));
        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA) - 1,
            0
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 65561, location = Self)]
    /// say unlocking schedule is 3 month cliff, monthly unlocking of 10% and principle stake is 100 coins then
    /// between 3 and 4 months, check that it's can't unlock there principal stake
    public entry fun test_unlocking_before_cliff_period_funded_delegators_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address = @0x0215;
        let new_delegator_signer = account::create_signer_for_test(new_delegator_address);
        let new_delegator_address2 = @0x0216;

        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);
        assert!(
            get_principle_stake(new_delegator_address, pool_address) == 30 * ONE_SUPRA,
            9
        );
        assert!(
            get_principle_stake(new_delegator_address2, pool_address) == 70 * ONE_SUPRA,
            9
        );

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        unlock(&new_delegator_signer, pool_address, (1 * ONE_SUPRA));
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_unlocking_mixed_principle_stake_before_cliff_period_funded_delegators_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address = @0x0215;
        let new_delegator_address2 = @0x0216;

        stake::mint(&funder_signer, 200 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (200 * ONE_SUPRA),
            0
        );

        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );

        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);
        assert!(
            get_principle_stake(new_delegator_address, pool_address) == 30 * ONE_SUPRA,
            99
        );
        assert!(
            get_principle_stake(new_delegator_address2, pool_address) == 70 * ONE_SUPRA,
            99
        );
        let (amount1, _, _) = get_stake(pool_address, new_delegator_address);
        let (amount2, _, _) = get_stake(pool_address, new_delegator_address2);
        assert!(amount1 > 59 * ONE_SUPRA, 99);
        assert!(amount2 > 138 * ONE_SUPRA, 99);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let new_delegator_address_signer =
            account::create_signer_for_test(new_delegator_address);
        unlock(&new_delegator_address_signer, pool_address, 30 * ONE_SUPRA);
        unlock(&new_delegator_address_signer, pool_address, 1 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 65561, location = Self)]
    public entry fun test_unlocking_mixed_principle_stake_before_cliff_period_funded_delegators_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address = @0x0215;
        let new_delegator_address2 = @0x0216;

        stake::mint(&funder_signer, 200 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (200 * ONE_SUPRA),
            0
        );

        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );

        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let new_delegator_address_signer =
            account::create_signer_for_test(new_delegator_address);
        unlock(&new_delegator_address_signer, pool_address, 30 * ONE_SUPRA);
        unlock(&new_delegator_address_signer, pool_address, 2 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// say unlocking schedule is 3 month cliff, monthly unlocking of 10% and principle stake is 100 coins then
    /// between 3 and 4 months, check that it's can't unlock there principal stacke
    public entry fun test_unlocking_after_cliff_period_initial_and_funded_delegators_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address2 = @0x0216;
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );
        //fund existing delegator_address with locked state, this should still work
        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);

        // 4 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS + 1);
        end_aptos_epoch();
        //10% of 130 so 13 should be unlockable
        unlock(delegator, pool_address, 13 * ONE_SUPRA);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// test that delegators are indeed added to `principle_stake` table, appropriate
    /// stakes they can unlock
    public entry fun test_principle_stake_unlocking_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address2 = @0x0216;
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );
        //fund existing delegator_address with locked state, this should still work
        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);
        //Check that funded delegator was indeed added as a principle stake holder
        assert!(is_principle_stakeholder(new_delegator_address2, pool_address), 9);
        // 4 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let (active_amount, _, _) = get_stake(pool_address, new_delegator_address2);
        assert!(active_amount == 70 * ONE_SUPRA, active_amount);
        let d2_principle_stake = get_principle_stake(
            new_delegator_address2, pool_address
        );
        assert!(
            active_amount == d2_principle_stake,
            active_amount - d2_principle_stake
        );
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        unlock(delegator, pool_address, 13 * ONE_SUPRA);
        // After 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // 30% of 130, so  39*ONE_SUPRA should be unlockable out of which 13 were already unlocked earlier
        let unlock_coin =
            can_principle_unlock(delegator_address, pool_address, 26 * ONE_SUPRA);
        let amount = cached_unlockable_balance(delegator_address, pool_address);
        assert!(unlock_coin, amount);

    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// say unlocking schedule is 3 month cliff, monthly unlocking of 10% and principle stake is 100 coins then
    /// between 3 and 4 months, check that it's can't unlock there principal stacke
    public entry fun test_unlocking_after_3_period_initial_and_funded_delegators_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);
        let one_year_in_secs = 31536000;
        let reward_period_start_time_in_sec = timestamp::now_seconds();
        staking_config::initialize_rewards(
            supra_framework,
            fixed_point64::create_from_rational(1, 100),
            fixed_point64::create_from_rational(1, 100),
            one_year_in_secs,
            reward_period_start_time_in_sec,
            fixed_point64::create_from_rational(0, 100)
        );

        let new_delegator_address2 = @0x0216;
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );
        //fund existing delegator_address with locked state, this should still work
        fund_delegators_with_locked_stake(
            &funder_signer,
            pool_address,
            vector[delegator_address, new_delegator_address2],
            vector[30 * ONE_SUPRA, 70 * ONE_SUPRA]
        );
        assert!(coin::balance<SupraCoin>(funder) == 0, 0);

        // 4 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        unlock(delegator, pool_address, 13 * ONE_SUPRA);

        // After 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 30% of 130, so little bit less than 39*ONE_SUPRA should be unlockable, 13 already unlocked
        unlock(delegator, pool_address, 26 * ONE_SUPRA);

    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 10, location = Self)]
    /// say unlocking schedule is 3 month cliff, monthly unlocking of 10% and principle stake is 100 coins then
    /// between 3 and 4 months, check that it's can't unlock there principal stacke
    public entry fun test_unlocking_before_cliff_period_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[100 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(300 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 10);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    /// say unlocking schedule is 3 month cliff, monthly unlocking of (2,3,1) tange and principle stake is 100 coins then
    /// at the end of 3 months, one can't unlock there principal stacke
    /// after 4 months only 90 should remain locked
    /// after 5 months only 80 should remain locked and so on
    public entry fun test_unlocking_principle_stake_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 14);

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (80 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 15);

        // after 9 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (90 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 16);

        // after 10 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (100 * ONE_SUPRA) - 10
            );
        assert!(unlock_coin, 17);

        // after 11 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(delegator_address, pool_address, 100 * ONE_SUPRA);
        assert!(unlock_coin, 18);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    //Test that `last_unlock_period` increases only up to the point so as to allow
    //cumulative fraction to become greater or equals one and not more than that
    public entry fun test_stop_after_cfraction_one_success(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[1],
            2,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let unlock_coin =
            can_principle_unlock(delegator_address, pool_address, (100 * ONE_SUPRA));
        assert!(unlock_coin, 11);

        let unlock_schedule =
            borrow_global<DelegationPool>(pool_address).principle_unlock_schedule;
        let cfraction_upperbound = fixed_point64::create_from_rational(3, 2);
        assert!(
            fixed_point64::less(
                unlock_schedule.cumulative_unlocked_fraction, cfraction_upperbound
            ),
            unlock_schedule.last_unlock_period
        );
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_4_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_5_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_6_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_7_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_8_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 14);

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (80 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_9_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 14);

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (80 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 15);

        // after 9 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (90 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_10_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 14);

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (80 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 15);

        // after 9 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (90 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 16);

        // after 10 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (100 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    // Test that after unlock schedule change can not happen after a principle stakeholder calls
    // `can_principle_unlock` and the unlock fraction becomes non zero
    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 196649, location = Self)]
    public entry fun test_change_unlock_schedule_fail(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let (_, old_stime, _old_duration, old_last_unlock, old_cfraction) =
            get_unlock_schedule(pool_address);
        // Assert that `get_unlock_schedule` is returning expected values
        assert!(old_stime == principle_lockup_time, old_stime);
        assert!(old_last_unlock == 0, old_last_unlock);
        assert!(fixed_point64::is_zero(old_cfraction), 99);
        // Change schedule to 1 month cliff and monthly 10% vest

        can_principle_unlock(delegator_address, pool_address, 1 * ONE_SUPRA);
        update_unlocking_schedule(
            &account::create_signer_for_test(multisig),
            pool_address,
            vector[1],
            10,
            principle_lockup_time / 3,
            LOCKUP_CYCLE_SECONDS
        );

    }

    // Test that after unlock schedule change, one is able to unlock as per
    // new schedule but NOT as per old schedule
    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 13, location = Self)]
    public entry fun test_change_unlock_schedule(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        let (_, old_stime, _old_duration, old_last_unlock, old_cfraction) =
            get_unlock_schedule(pool_address);
        // Assert that `get_unlock_schedule` is returning expected values
        assert!(old_stime == principle_lockup_time, old_stime);
        assert!(old_last_unlock == 0, old_last_unlock);
        assert!(fixed_point64::is_zero(old_cfraction), 99);
        // Change schedule to 1 month cliff and monthly 10% vest
        update_unlocking_schedule(
            &account::create_signer_for_test(multisig),
            pool_address,
            vector[1],
            10,
            principle_lockup_time / 3,
            LOCKUP_CYCLE_SECONDS
        );
        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let (_, new_stime, _new_duration, new_last_unlock, new_cfraction) =
            get_unlock_schedule(pool_address);
        // Assert that `get_unlock_schedule` is returning expected values
        assert!(
            new_stime == principle_lockup_time / 3,
            new_stime
        );
        assert!(new_last_unlock == 0, new_last_unlock);
        assert!(fixed_point64::is_zero(new_cfraction), 99);
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (30 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        unlock_coin = can_principle_unlock(
            delegator_address,
            pool_address,
            (40 * ONE_SUPRA) - 9
        );
        assert!(unlock_coin, 12);
        unlock_coin = can_principle_unlock(
            delegator_address,
            pool_address,
            (50 * ONE_SUPRA) - 9
        );
        assert!(unlock_coin, 13);

    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    #[expected_failure(abort_code = 20, location = Self)]
    public entry fun test_unlocking_more_principle_stake_after_11_month_failure(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);

        // after 6 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (60 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 13);

        // after 7 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (70 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 14);

        // after 8 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (80 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 15);

        // after 9 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (90 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 16);

        // after 10 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (100 * ONE_SUPRA) - 10
            );
        assert!(unlock_coin, 17);

        // after 11 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (100 * ONE_SUPRA) + 1
            );
        assert!(unlock_coin, 20);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    // Testing whether fast forward is working as expected
    public entry fun test_unlocking_principle_stake_success_can_fastforward(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[100 * ONE_SUPRA];
        let coin = stake::mint_coins(100 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // It's acceptable to round off 9 because this coin will remain locked and won't be transferred anywhere.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (20 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (50 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);

        // after 11 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(delegator_address, pool_address, 100 * ONE_SUPRA);
        assert!(unlock_coin, 18);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    // Testing whether fast forward is working as expected
    public entry fun test_unlocking_principle_stake_success_can_fastforward_nondivisable(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[113 * ONE_SUPRA];
        let coin = stake::mint_coins(113 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // after 2 month unlock reward
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // after 4 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();

        // After three mounth cliff and one extra mouth, 2/10 of the principle stake (113) = 22.6 can be unlocked. minus 9 for rounding off.
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (22 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 11);

        // after 5 months, 5/10 of the principle stake (113) = 56.5 can be unlocked. minus 9 for rounding off.
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (55 * ONE_SUPRA) - 9
            );
        assert!(unlock_coin, 12);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);

        // after 11 months
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        end_aptos_epoch();
        let unlock_coin =
            can_principle_unlock(delegator_address, pool_address, 113 * ONE_SUPRA);
        assert!(unlock_coin, 18);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    // Testing whether fast forward is working as expected
    public entry fun test_unlocking_principle_stake_success_can_fastforward_5_out_of_10(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[1000 * ONE_SUPRA];
        let coin = stake::mint_coins(1000 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        // After cliff, 5/10 of the principle stake (1000) = 500 can be unlocked.
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS * 2);

        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (500 * ONE_SUPRA) - 1
            );
        assert!(unlock_coin, 11);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    // Testing whether fast forward is working as expected
    public entry fun test_unlocking_principle_stake_success_can_fastforward_7_out_of_10(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[1000 * ONE_SUPRA];
        let coin = stake::mint_coins(1000 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        // After cliff, 7/10 of the principle stake (1000) = 700 can be unlocked.
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS * 4);

        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (700 * ONE_SUPRA) - 1
            );
        assert!(unlock_coin, 11);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    // Testing whether fast forward is working as expected
    public entry fun test_unlocking_principle_stake_success_can_fastforward_10_out_of_10(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address];
        let principle_stake = vector[1000 * ONE_SUPRA];
        let coin = stake::mint_coins(1000 * ONE_SUPRA);
        let principle_lockup_time = 7776000; // 3 month cliff
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 3, 1],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS // monthly unlocking
        );
        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        // 3 month
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS);
        // After cliff, all of the principle stake (1000) can be unlocked.
        timestamp::fast_forward_seconds(LOCKUP_CYCLE_SECONDS * 8);

        let unlock_coin =
            can_principle_unlock(
                delegator_address,
                pool_address,
                (1000 * ONE_SUPRA)
            );
        assert!(unlock_coin, 11);
    }

    #[test(supra_framework = @supra_framework, validator = @0x123, delegator = @0x010)]
    public entry fun test_lock_delegator_stake_after_allocation(
        supra_framework: &signer, validator: &signer, delegator: &signer
    ) acquires DelegationPoolOwnership, DelegationPool, GovernanceRecords, BeneficiaryForOperator, NextCommissionPercentage {
        initialize_for_test(supra_framework);
        account::create_account_for_test(signer::address_of(validator));
        let delegator_address = signer::address_of(delegator);
        let delegator_address_vec = vector[delegator_address, @0x020];
        let principle_stake = vector[300 * ONE_SUPRA, 200 * ONE_SUPRA];
        let coin = stake::mint_coins(500 * ONE_SUPRA);
        let principle_lockup_time = 7776000;
        let multisig = generate_multisig_account(validator, vector[@0x12134], 2);

        initialize_test_validator(
            validator,
            0,
            true,
            true,
            0,
            delegator_address_vec,
            principle_stake,
            coin,
            option::some(multisig),
            vector[2, 2, 3],
            10,
            principle_lockup_time,
            LOCKUP_CYCLE_SECONDS
        );

        let validator_address = signer::address_of(validator);
        let pool_address = get_owned_pool_address(validator_address);

        let new_delegator_address = @0x0215;
        let new_delegator_address_signer =
            account::create_account_for_test(new_delegator_address);
        let funder_signer = account::create_signer_for_test(multisig);
        let funder = signer::address_of(&funder_signer);
        stake::mint(&funder_signer, 100 * ONE_SUPRA);
        stake::mint(&new_delegator_address_signer, 100 * ONE_SUPRA);
        assert!(
            coin::balance<SupraCoin>(funder) == (100 * ONE_SUPRA),
            0
        );

        assert!(
            coin::balance<SupraCoin>(new_delegator_address) == (100 * ONE_SUPRA),
            0
        );

        add_stake(&new_delegator_address_signer, pool_address, 100 * ONE_SUPRA);

        //
        // Ensure that `lock_delegators_stakes` can lock newly-allocated stakes.
        //

        // Fund a new delegator.
        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[new_delegator_address],
            vector[1 * ONE_SUPRA]
        );
        // Ensure that its stake is not subject to the pool vesting schedule.
        assert!(!is_principle_stakeholder(new_delegator_address, pool_address), 1);

        // Lock its stake.
        lock_delegators_stakes(
            &funder_signer,
            pool_address,
            vector[new_delegator_address],
            vector[1 * ONE_SUPRA]
        );
        // Ensure that its stake is now subject to the pool vesting schedule.
        assert!(is_principle_stakeholder(new_delegator_address, pool_address), 0);

        //
        // Ensure that `lock_delegators_stakes` reactivates `pending_inactive` stake.
        //

        let delegator = @0x0216;
        let delegator_signer = account::create_signer_for_test(delegator);
        let delegator_allocation = 10 * ONE_SUPRA;
        let half_delegator_allocation = delegator_allocation / 2;
        // A rounding error of 1 Quant is introduced by `unlock`.
        let half_delegator_allocation_with_rounding_error = half_delegator_allocation
            - 1;
        let delegator_allocation_after_rounding_error =
            half_delegator_allocation + half_delegator_allocation_with_rounding_error;

        // Fund another delegator.
        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[delegator_allocation]
        );

        // End the current lockup cycle to ensure that the stake fee that is deducted when the stake
        // is first added has been returned.
        fast_forward_to_unlock(pool_address);

        // Ensure that the entire allocation is marked as active.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(active == delegator_allocation, active);
        assert!(inactive == 0, inactive);
        assert!(pending_inactive == 0, pending_inactive);

        // Unlock half of the initial allocation (i.e. move it to `pending_inactive`).
        unlock(&delegator_signer, pool_address, half_delegator_allocation);

        // Ensure that half of the allocation is marked as `active` and the other half as `pending_inactive`.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(active == half_delegator_allocation, active);
        assert!(inactive == 0, inactive);
        assert!(
            pending_inactive == half_delegator_allocation_with_rounding_error,
            pending_inactive
        );

        // Attempt to lock the full allocation, which should cause the `pending_inactive` allocation
        // to become `active` again.
        lock_delegators_stakes(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[delegator_allocation_after_rounding_error]
        );

        // Ensure that the entire allocation is marked as active again.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(active == delegator_allocation_after_rounding_error, active);
        assert!(inactive == 0, inactive);
        assert!(pending_inactive == 0, pending_inactive);

        // Ensure that the delegator's stake is now subject to the pool vesting schedule.
        assert!(is_principle_stakeholder(delegator, pool_address), 0);

        //
        // Ensure that `lock_delegators_stakes` reactivates `inactive` stake.
        //

        delegator = @0x0217;
        delegator_signer = account::create_signer_for_test(delegator);
        // The amount of staking rewards earned each epoch. See `initialize_for_test_custom`.
        let epoch_reward = delegator_allocation / 100;
        let half_epoch_reward = epoch_reward / 2;
        let delegator_stake = delegator_allocation_after_rounding_error + epoch_reward;
        // The amount of stake withheld due to the withdrawal and restaking process used to
        // recover `inactive` stake.
        let add_stake_fee =
            get_add_stake_fee(
                pool_address, half_delegator_allocation + half_epoch_reward
            );

        // Fund another delegator.
        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[delegator_allocation]
        );

        // End the current lockup cycle to ensure that the stake fee that is deducted when the stake
        // is first added has been returned.
        fast_forward_to_unlock(pool_address);

        // Ensure that the entire allocation is marked as active.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(active == delegator_allocation, active);
        assert!(inactive == 0, inactive);
        assert!(pending_inactive == 0, pending_inactive);

        // Unlock half of the initial allocation (i.e. move it to `pending_inactive`).
        unlock(&delegator_signer, pool_address, half_delegator_allocation);

        // End the current lockup cycle to move the `pending_inactive` stake to `inactive`.
        // This will also distribute staking rewards for the epoch.
        fast_forward_to_unlock(pool_address);

        // Ensure that half of the allocation is marked as `active` and the other half as `inactive`.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(
            active == half_delegator_allocation + half_epoch_reward,
            active
        );
        // Another rounding error is introduced by the second `unlock`.
        assert!(
            inactive
                == half_delegator_allocation_with_rounding_error + half_epoch_reward
                    - 1,
            inactive
        );
        assert!(pending_inactive == 0, pending_inactive);

        // Attempt to lock the full allocation, which should cause the `inactive` allocation
        // to become `active` again.
        lock_delegators_stakes(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[delegator_stake]
        );

        // Ensure that the entire allocation is marked as active again. The fee for adding stake
        // needs to be subtracted from the `active` amount because we've not entered the next epoch yet.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        let expected_active_stake = delegator_stake - add_stake_fee;
        assert!(active == expected_active_stake, active);
        assert!(inactive == 0, inactive);
        assert!(pending_inactive == 0, pending_inactive);

        // Ensure that the delegator's stake is now subject to the pool vesting schedule.
        let pool: &mut DelegationPool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_principle_stake = *table::borrow(&pool.principle_stake, delegator);
        assert!(delegator_principle_stake == delegator_stake, delegator_principle_stake);

        //
        // Ensure that `lock_delegators_stakes` locks the maximum available stake when the amount
        // requested to be locked exceeds the available stake. Also ensure that the same delegator
        // can be funded and its new allocation locked, multiple times, and that the principle stake
        // specified in the most recent call to `lock_delegators_stakes` is applied correctly.
        //

        // Fund the same delegator to ensure that we can lock additional amounts.
        fund_delegators_with_stake(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[delegator_allocation]
        );

        // Calculate the fee for the newly added amount.
        let add_stake_fee = get_add_stake_fee(pool_address, delegator_allocation);
        let expected_total_stake =
            expected_active_stake + delegator_allocation - add_stake_fee;

        // Ensure that the entire allocation is marked as active.
        let (active, inactive, pending_inactive) = get_stake(pool_address, delegator);
        assert!(active == expected_total_stake, active);
        assert!(inactive == 0, inactive);
        assert!(pending_inactive == 0, pending_inactive);

        // Attempt to lock more than the full allocation.
        let more_than_allocated_stake = delegator_allocation * 2;
        lock_delegators_stakes(
            &funder_signer,
            pool_address,
            vector[delegator],
            vector[more_than_allocated_stake]
        );

        // Ensure that the delegator's `principle_stake` has been updated.
        let pool: &mut DelegationPool = borrow_global_mut<DelegationPool>(pool_address);
        let delegator_principle_stake = *table::borrow(&pool.principle_stake, delegator);
        assert!(
            delegator_principle_stake == more_than_allocated_stake,
            delegator_principle_stake
        );
    }
}
