spec velor_framework::delegation_pool {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Every DelegationPool has only one corresponding StakePool stored at the same address.
    /// Criticality: Critical
    /// Implementation: Upon calling the initialize_delegation_pool function, a resource account is created from the
    /// "owner" signer to host the delegation pool resource and own the underlying stake pool.
    /// Enforcement: Audited that the address of StakePool equals address of DelegationPool and the data invariant on the DelegationPool.
    ///
    /// No.: 2
    /// Requirement: The signer capability within the delegation pool has an address equal to the address of the
    /// delegation pool.
    /// Criticality: Critical
    /// Implementation: The initialize_delegation_pool function moves the DelegationPool resource to the address
    /// associated with stake_pool_signer, which also possesses the signer capability.
    /// Enforcement: Audited that the address of signer cap equals address of DelegationPool.
    ///
    /// No.: 3
    /// Requirement: A delegator holds shares exclusively in one inactive shares pool, which could either be an already
    /// inactive pool or the pending_inactive pool.
    /// Criticality: High
    /// Implementation: The get_stake function returns the inactive stake owned by a delegator and checks which
    /// state the shares are in via the get_pending_withdrawal function.
    /// Enforcement: Audited that either inactive or pending_inactive stake after invoking the get_stake function is
    /// zero and both are never non-zero.
    ///
    /// No.: 4
    /// Requirement: The specific pool in which the delegator possesses inactive shares becomes designated as the
    /// pending withdrawal pool for that delegator.
    /// Criticality: Medium
    /// Implementation: The get_pending_withdrawal function checks if any pending withdrawal exists for a delegate
    /// address and if there is neither inactive nor pending_inactive stake, the pending_withdrawal_exists returns
    /// false.
    /// Enforcement: This has been audited.
    ///
    /// No.: 5
    /// Requirement: The existence of a pending withdrawal implies that it is associated with a pool where the
    /// delegator possesses inactive shares.
    /// Criticality: Medium
    /// Implementation: In the get_pending_withdrawal function, if withdrawal_exists is true, the function returns
    /// true and a non-zero amount
    /// Enforcement: get_pending_withdrawal has been audited.
    ///
    /// No.: 6
    /// Requirement: An inactive shares pool should have coins allocated to it; otherwise, it should become deleted.
    /// Criticality: Medium
    /// Implementation: The redeem_inactive_shares function has a check that destroys the inactive shares pool,
    /// given that it is empty.
    /// Enforcement: shares pools have been audited.
    ///
    /// No.: 7
    /// Requirement: The index of the pending withdrawal will not exceed the current OLC on DelegationPool.
    /// Criticality: High
    /// Implementation: The get_pending_withdrawal function has a check which ensures that withdrawal_olc.index <
    /// pool.observed_lockup_cycle.index.
    /// Enforcement: This has been audited.
    ///
    /// No.: 8
    /// Requirement: Slashing is not possible for inactive stakes.
    /// Criticality: Critical
    /// Implementation: The number of inactive staked coins must be greater than or equal to the
    /// total_coins_inactive of the pool.
    /// Enforcement: This has been audited.
    ///
    /// No.: 9
    /// Requirement: The delegator's active or pending inactive stake will always meet or exceed the minimum allowed
    /// value.
    /// Criticality: Medium
    /// Implementation: The add_stake, unlock and reactivate_stake functions ensure the active_shares or
    /// pending_inactive_shares balance for the delegator is greater than or equal to the MIN_COINS_ON_SHARES_POOL
    /// value.
    /// Enforcement: Audited the comparison of active_shares or inactive_shares balance for the delegator with the
    /// MIN_COINS_ON_SHARES_POOL value.
    ///
    /// No.: 10
    /// Requirement: The delegation pool exists at a given address.
    /// Criticality: Low
    /// Implementation: Functions that operate on the DelegationPool abort if there is no DelegationPool struct
    /// under the given pool_address.
    /// Enforcement: Audited that there is no DelegationPool structure assigned to the pool_address given as a
    /// parameter.
    ///
    /// No.: 11
    /// Requirement: The initialization of the delegation pool is contingent upon enabling the delegation pools
    /// feature.
    /// Criticality: Critical
    /// Implementation: The initialize_delegation_pool function should proceed if the DELEGATION_POOLS feature is
    /// enabled.
    /// Enforcement: This has been audited.
    /// </high-level-req>
    ///
    spec module {
        // TODO: verification disabled until this module is specified.
        pragma verify=false;
    }
}
