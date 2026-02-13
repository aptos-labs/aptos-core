/// Maintains the list of banned validators and updates counters on every epoch.
///
/// This implementation assumes that each validator is elected once every `n` consensus rounds on expectation,
/// where `n` is the number of validators in the consensus committee (i.e. the `ValidatorSet`).
module supra_framework::leader_ban_registry {
    use std::error;
    use std::features;
    use std::option;
    use std::option::Option;
    use supra_framework::system_addresses;
    use std::vector;
    use aptos_std::math64::{pow, min};
    use supra_framework::event;
    use supra_framework::stake;
    use supra_framework::leader_ban_registry_config;

    friend supra_framework::block;
    friend supra_framework::genesis;
    friend supra_framework::reconfiguration;

    #[test_only]
    friend supra_framework::test_leader_ban_registry;

    /// Leader ban registry already initialized
    const EBAN_REGISTRY_ALREADY_EXISTS: u64 = 1;
    /// Leader ban registry not initialized
    const EBAN_REGISTRY_NOT_INITIALIZED: u64 = 2;
    /// Latest view already initialized
    const ELATEST_VIEW_ALREADY_EXISTS: u64 = 3;

    /// Information about a ban that is currently in effect for a validator.
    struct ActiveBan has store, drop, copy {
        /// The consensus epoch in which the current ban was issued (if `on_probation` is `false`) or
        /// when its probation period started (if `on_probation` is `true`).
        epoch_earned: u64,
        /// The consensus round in which the current ban was issued (if `on_probation` is `false`) or
        /// when its probation period started (if `on_probation` is `true`).
        round_earned: u64,
        /// Round count incremented on every epoch change
        rounds_served_in_previous_epochs: u64,
        /// If `true` then the current ban period has expired and the validator is currently on probation; i.e., it is
        /// eligible for election but will be banned for a longer period if it once again fails to propose a canonical
        /// block when elected. If `true` then the other fields of this struct denote the consensus view in which
        /// the probation period started.
        on_probation: bool
    }

    /// Holds validator metrics regarding duration pool address etc
    struct ValidatorBans has store, drop, copy {
        /// Information about the ban that is currently in effect.
        active: ActiveBan,
        /// The number of consecutive probations that this validator has failed.
        consecutive_bans: u32,
        /// Validator's pool address
        pool_address: address
    }

    /// Holds ban registry
    struct BanRegistry has drop, store, key {
        /// List of validator active bans with pool address
        bans: vector<ValidatorBans>
    }

    /// Holds latest processed round and epoch
    struct LatestView has drop, store, key, copy {
        /// Epoch
        epoch: u64,
        /// Round
        round: u64
    }

    #[event]
    /// Emits when validator receives a ban or consucutive ban occurred
    struct Banned has drop, store {
        /// Validator's pool address
        pool_address: address,
        /// Epoch
        epoch: u64,
        /// Round
        round: u64,
        /// The number of consecutive probations that this validator has failed.
        consecutive_bans: u32
    }

    #[event]
    /// Emitted when a validator's ban is lifted and its probation period starts. A validator that is on probation is
    /// eligible for election again, but will be banned for longer if it once again fails to propose a canonical block.
    struct ReinstatedWithProbation has drop, store {
        /// Validator's pool address
        pool_address: address,
        /// Epoch
        epoch: u64,
        /// Round
        round: u64
    }

    #[event]
    /// Emitted when a validator's probation period ends without the validator earning a new ban.
    struct Reinstated has drop, store {
        /// Validator's pool address
        pool_address: address,
        /// Epoch
        epoch: u64,
        /// Round
        round: u64
    }

    /// Initialise leader ban registry
    public(friend) fun initialize_leader_ban_registry(
        supra_framework: &signer
    ) {
        system_addresses::assert_supra_framework(supra_framework);
        assert!(
            !exists<BanRegistry>(@supra_framework),
            error::already_exists(EBAN_REGISTRY_ALREADY_EXISTS)
        );
        assert!(
            !exists<LatestView>(@supra_framework),
            error::already_exists(EBAN_REGISTRY_ALREADY_EXISTS)
        );
        move_to(supra_framework, BanRegistry { bans: vector::empty() });
        move_to(supra_framework, LatestView { epoch: 0, round: 0 });
    }

    #[view]
    /// Returns list of validators active ban with it's pool address
    public fun get_ban_registry(): vector<ValidatorBans> acquires BanRegistry {
        if (!exists<BanRegistry>(@supra_framework)) {
            return vector::empty()
        };
        let ban_registry = borrow_global<BanRegistry>(@supra_framework);
        ban_registry.bans
    }

    #[view]
    /// Return latest view
    public fun get_latest_view(): LatestView acquires LatestView {
        if (!exists<LatestView>(@supra_framework)) {
            return LatestView { epoch: 0, round: 0 }
        };
        let latest_view = borrow_global<LatestView>(@supra_framework);
        *latest_view
    }

    #[view]
    /// Returns the number of consensus rounds that a validator is banned for when it fails to propose a
    /// canonical block when elected as leader whilst not on probation.
    public fun get_initial_ban_duration(): u64 {
        let initial_elections_denied =
            leader_ban_registry_config::get_initial_elections_denied();
        let committee_size = stake::get_committee_size();
        committee_size * (initial_elections_denied as u64)
    }

    #[view]
    /// Returns the maximum number of consensus rounds that a validator may banned for when it repeatedly fails to
    /// propose a canonical block when elected as leader whilst on probation.
    public fun get_max_ban_duration(): u64 {
        let max_elections_denied = leader_ban_registry_config::get_max_elections_denied();
        let committee_size = stake::get_committee_size();
        committee_size * (max_elections_denied as u64)
    }

    #[view]
    /// Returns the number of consensus rounds that a validator is considered to be on probation for after having
    /// served its most recent ban.
    public fun get_probation_duration(): u64 {
        let probation_elections = leader_ban_registry_config::get_probation_elections();
        let committee_size = stake::get_committee_size();
        committee_size * (probation_elections as u64)
    }

    #[view]
    /// Returns the number of consensus rounds remaining in the ban for the validator with the given
    /// pool address. Returns 0 if the validator is not banned (including if it is on probation).
    public fun get_remaining_ban_duration(pool_address: address): u64 acquires BanRegistry, LatestView {
        if (!exists<BanRegistry>(@supra_framework) || !exists<LatestView>(@supra_framework)) {
            return 0
        };
        let ban_registry = borrow_global<BanRegistry>(@supra_framework);
        let latest_view = borrow_global<LatestView>(@supra_framework);
        let (found, index) = vector::find(
            &ban_registry.bans,
            |v| {
                let v: &ValidatorBans = v;
                v.pool_address == pool_address && !v.active.on_probation
            }
        );
        if (found) {
            remaining_ban_duration(vector::borrow(&ban_registry.bans, index), latest_view)
        } else {
            0
        }
    }

    #[view]
    /// Returns the number of consensus rounds remaining in the probation period for the validator
    /// with the given pool address. Returns 0 if the validator is not on probation.
    public fun get_remaining_probation_duration(pool_address: address): u64 acquires BanRegistry, LatestView {
        if (!exists<BanRegistry>(@supra_framework) || !exists<LatestView>(@supra_framework)) {
            return 0
        };
        let ban_registry = borrow_global<BanRegistry>(@supra_framework);
        let latest_view = borrow_global<LatestView>(@supra_framework);
        let (found, index) = vector::find(
            &ban_registry.bans,
            |v| {
                let v: &ValidatorBans = v;
                v.pool_address == pool_address && v.active.on_probation
            }
        );
        if (found) {
            let probation_dur = get_probation_duration();
            remaining_probation_duration(vector::borrow(&ban_registry.bans, index), latest_view, probation_dur)
        } else {
            0
        }
    }

    /// Add or update the ban registry as per block metadata
    public(friend) fun update_ban_registry(
        current_epoch: u64,
        current_round: u64,
        proposer_index: Option<u64>,
        failed_proposer_indices: vector<u64>
    ) acquires BanRegistry, LatestView {
        if (!exists<BanRegistry>(@supra_framework)) { return };
        if (!exists<LatestView>(@supra_framework)) { return };
        let ban_registry = borrow_global_mut<BanRegistry>(@supra_framework);
        let latest_view = borrow_global_mut<LatestView>(@supra_framework);
        latest_view.epoch = current_epoch;
        latest_view.round = current_round;

        // ban the failed proposers
        ban_failed_proposers(latest_view, failed_proposer_indices, ban_registry);

        // remove expired bans
        reinstate_expired_bans(latest_view, ban_registry);
    }

    /// Adds failed proposer indices to ban registry
    fun ban_failed_proposers(
        latest_view: &LatestView,
        failed_proposer_indices: vector<u64>,
        ban_registry: &mut BanRegistry
    ) {
        let initial_ban_duration = get_initial_ban_duration();
        if (initial_ban_duration == 0) { return };

        vector::for_each(
            failed_proposer_indices,
            |failed_validator_index| {
                let validator_pool_address_opt =
                    stake::get_pool_address_from_index(failed_validator_index);

                if (option::is_some(&validator_pool_address_opt)) {
                    let validator_pool_address =
                        option::extract(&mut validator_pool_address_opt);
                    let (is_banned, index) = vector::find(
                        &ban_registry.bans,
                        |v| {
                            let v: &ValidatorBans = v;
                            validator_pool_address == v.pool_address
                        }
                    );
                    if (is_banned) {
                        // Validator is already in registry (either banned or on probation).
                        // Re-banning resets the ban period and increases consecutive count.
                        // If the consensus code is implemented correctly then the validator should
                        // not be re-banned whilst serving a ban as it should not be eligible for election
                        // when banned (i.e. this branch should only be taken when a validator is on probation).
                        let bans = vector::borrow_mut(&mut ban_registry.bans, index);
                        bans.consecutive_bans = bans.consecutive_bans + 1;
                        bans.active.round_earned = latest_view.round;
                        bans.active.epoch_earned = latest_view.epoch;
                        bans.active.rounds_served_in_previous_epochs = 0;
                        bans.active.on_probation = false; // Reset to banned state

                        if (features::module_event_enabled()) {
                            event::emit(
                                Banned {
                                    pool_address: validator_pool_address,
                                    epoch: latest_view.epoch,
                                    round: latest_view.round,
                                    consecutive_bans: bans.consecutive_bans
                                }
                            );
                        }
                    } else {
                        let ban_registry_len = vector::length(&ban_registry.bans);
                        if (can_be_banned(ban_registry_len)) {
                            let ban_with_address = ValidatorBans {
                                active: ActiveBan {
                                    epoch_earned: latest_view.epoch,
                                    round_earned: latest_view.round,
                                    rounds_served_in_previous_epochs: 0,
                                    on_probation: false
                                },
                                consecutive_bans: 0,
                                pool_address: validator_pool_address
                            };
                            vector::push_back(&mut ban_registry.bans, ban_with_address);

                            if (features::module_event_enabled()) {
                                event::emit(
                                    Banned {
                                        pool_address: ban_with_address.pool_address,
                                        epoch: ban_with_address.active.epoch_earned,
                                        round: ban_with_address.active.round_earned,
                                        consecutive_bans: ban_with_address.consecutive_bans
                                    }
                                );
                            }
                        }
                    };
                };
            }
        );
    }

    /// Handles ban and probation expiry:
    /// - When probation expires: removes from registry, emits Reinstated
    /// - When ban expires and probation_duration > 0: transitions to probation, emits ReinstatedWithProbation
    /// - When ban expires and probation_duration == 0: removes from registry, emits Reinstated
    fun reinstate_expired_bans(
        latest_view: &LatestView, ban_registry: &mut BanRegistry
    ) {
        let probation_duration = get_probation_duration();

        // First pass: remove validators whose probation has expired.
        // Done before ban-to-probation transitions to avoid iterating just-transitioned validators.
        // Always runs (even when probation_duration == 0) to clean up validators that were already
        // on probation before a config change set probation_duration to 0.
        let pool_addresses_for_full_reinstatement = vector::empty();
        vector::for_each_ref(
            &ban_registry.bans,
            |v| {
                let v: &ValidatorBans = v;
                if (v.active.on_probation
                    && remaining_probation_duration(v, latest_view, probation_duration) == 0) {
                    vector::push_back(
                        &mut pool_addresses_for_full_reinstatement, v.pool_address
                    );
                }
            }
        );

        vector::for_each_ref(
            &pool_addresses_for_full_reinstatement,
            |p| {
                let (found, index) = vector::find(
                    &ban_registry.bans,
                    |v| {
                        let v: &ValidatorBans = v;
                        &v.pool_address == p
                    }
                );
                if (found) {
                    vector::swap_remove(&mut ban_registry.bans, index);

                    if (features::module_event_enabled()) {
                        event::emit(
                            Reinstated {
                                epoch: latest_view.epoch,
                                round: latest_view.round,
                                pool_address: *p
                            }
                        )
                    }
                }
            }
        );

        // Second pass: handle expired bans
        let pool_addresses_with_expired_bans = vector::empty();
        vector::for_each_ref(
            &ban_registry.bans,
            |v| {
                let v: &ValidatorBans = v;
                if (!v.active.on_probation && remaining_ban_duration(v, latest_view) == 0) {
                    vector::push_back(&mut pool_addresses_with_expired_bans, v.pool_address);
                }
            }
        );

        if (probation_duration > 0) {
            // Transition expired bans to probation
            vector::for_each_ref(
                &pool_addresses_with_expired_bans,
                |p| {
                    let (found, index) = vector::find(
                        &ban_registry.bans,
                        |v| {
                            let v: &ValidatorBans = v;
                            &v.pool_address == p
                        }
                    );
                    if (found) {
                        let ban = vector::borrow_mut(&mut ban_registry.bans, index);
                        ban.active.on_probation = true;
                        // Reset active fields so probation duration is calculated from this point
                        ban.active.epoch_earned = latest_view.epoch;
                        ban.active.round_earned = latest_view.round;
                        ban.active.rounds_served_in_previous_epochs = 0;

                        if (features::module_event_enabled()) {
                            event::emit(
                                ReinstatedWithProbation {
                                    epoch: latest_view.epoch,
                                    round: latest_view.round,
                                    pool_address: *p
                                }
                            )
                        }
                    }
                }
            );
        } else {
            // No probation period - directly remove validators whose ban expired
            vector::for_each_ref(
                &pool_addresses_with_expired_bans,
                |p| {
                    let (found, index) = vector::find(
                        &ban_registry.bans,
                        |v| {
                            let v: &ValidatorBans = v;
                            &v.pool_address == p
                        }
                    );
                    if (found) {
                        vector::swap_remove(&mut ban_registry.bans, index);

                        if (features::module_event_enabled()) {
                            event::emit(
                                Reinstated {
                                    epoch: latest_view.epoch,
                                    round: latest_view.round,
                                    pool_address: *p
                                }
                            )
                        }
                    }
                }
            );
        };
    }

    /// Increments the total number of consensus rounds served by each banned validator and removes 
    /// registry entries for validators that have left the validator set.
    ///
    /// The number of rounds in each epoch may vary due to network asynchrony, so we must record the
    /// number of rounds served in previous epochs to be able to ensure that a banned validator serves its
    /// full ban period when its ban span multiple epochs.
    public(friend) fun on_new_epoch() acquires BanRegistry, LatestView {
        if (!exists<LatestView>(@supra_framework)) { return };
        if (!exists<BanRegistry>(@supra_framework)) { return };
        let latest_view = borrow_global<LatestView>(@supra_framework);
        let ban_registry = borrow_global_mut<BanRegistry>(@supra_framework);

        // The pool addresses of the validators for the new epoch.
        let new_committee_pool_addresses = stake::get_committee_pool_addresses();
        // The pool addresses of the validators that have left the committee.
        let retired_validators = vector::empty();
        vector::for_each_mut(
            &mut ban_registry.bans,
            |v| {
                let v: &mut ValidatorBans = v;
                if (vector::contains(&new_committee_pool_addresses, &v.pool_address)) {
                    if (latest_view.epoch > v.active.epoch_earned) {
                        v.active.rounds_served_in_previous_epochs = v.active.rounds_served_in_previous_epochs
                            + latest_view.round;
                    } else if (latest_view.epoch == v.active.epoch_earned && latest_view.round > v.active.round_earned) {
                        v.active.rounds_served_in_previous_epochs = latest_view.round - v.active.round_earned;
                    };
                    // else: The ban hasn't started yet.
                } else {
                    vector::push_back(&mut retired_validators, v.pool_address)
                }
            }
        );

        vector::for_each_ref(
            &retired_validators,
            |p| {
                let (is_banned, index) = vector::find(
                    &ban_registry.bans,
                    |v| {
                        let v: &ValidatorBans = v;
                        &v.pool_address == p
                    }
                );
                if (is_banned) {
                    vector::swap_remove(&mut ban_registry.bans, index);

                    if (features::module_event_enabled()) {
                        event::emit(
                            Reinstated {
                                pool_address: *p,
                                epoch: latest_view.epoch,
                                round: latest_view.round
                            }
                        );
                    }
                }
            }
        );
    }

    /// Calculate the number of rounds remaining in a given ban (not including probation).
    fun remaining_ban_duration(
        ban: &ValidatorBans, latest_view: &LatestView
    ): u64 {
        let initial_ban_duration = get_initial_ban_duration();
        let max_ban_duration = get_max_ban_duration();
        let duration = initial_ban_duration * pow(2, (ban.consecutive_bans as u64));
        let duration = min(duration, max_ban_duration);
        let rounds_served =
            if (latest_view.epoch > ban.active.epoch_earned) {
                ban.active.rounds_served_in_previous_epochs + latest_view.round
            } else if (latest_view.epoch == ban.active.epoch_earned && latest_view.round > ban.active.round_earned) {
                latest_view.round - ban.active.round_earned
            } else {
                // The ban hasn't started yet.
                0
            };
        if (duration > rounds_served) {
            duration - rounds_served
        } else { 0 }
    }

    /// Calculate the number of rounds remaining in probation.
    /// Probation duration is constant and does not scale with consecutive bans.
    fun remaining_probation_duration(
        ban: &ValidatorBans, latest_view: &LatestView, probation_duration: u64
    ): u64 {
        let rounds_served =
            if (latest_view.epoch > ban.active.epoch_earned) {
                ban.active.rounds_served_in_previous_epochs + latest_view.round
            } else if (latest_view.epoch == ban.active.epoch_earned && latest_view.round > ban.active.round_earned) {
                latest_view.round - ban.active.round_earned
            } else {
                // The ban hasn't started yet.
                0
            };
        if (probation_duration > rounds_served) {
            probation_duration - rounds_served
        } else { 0 }
    }

    /// Returns true until ban registry size + minimum proposers required count less than committee size
    fun can_be_banned(ban_registry_len: u64): bool {
        let minimum_unbanned_proposers =
            leader_ban_registry_config::get_minimum_unbanned_proposers();
        let committee_size = stake::get_committee_size();
        committee_size > ban_registry_len + (minimum_unbanned_proposers as u64)
    }

    /// Validates registry initialised if not aborted with `EBAN_REGISTRY_NOT_INITIALIZED`
    fun assert_registry_initialized() {
        assert!(
            exists<BanRegistry>(@supra_framework),
            error::invalid_state(EBAN_REGISTRY_NOT_INITIALIZED)
        );
    }

    #[test_only]
    public fun get_pool_address_from_vp(
        validator_with_pool_addr: &ValidatorBans
    ): address {
        validator_with_pool_addr.pool_address
    }

    #[test_only]
    public fun get_consecutive_count_from_vp(
        validator_with_pool_addr: &ValidatorBans
    ): u32 {
        validator_with_pool_addr.consecutive_bans
    }

    #[test_only]
    public fun is_on_probation_from_vp(
        validator_with_pool_addr: &ValidatorBans
    ): bool {
        validator_with_pool_addr.active.on_probation
    }
}
