spec aptos_framework::multisig_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: For every multi-signature account, the range of required signatures should always be in the range of
    /// one to the total number of owners.
    /// Criticality: Critical
    /// Implementation: While creating a MultisigAccount, the function create_with_owners_internal checks that
    /// num_signatures_required is in the span from 1 to total count of owners.
    /// Enforcement: Formally verified via [high-level-req-1](global invariant) for every MultisigAccount, at creation
    /// and across all owner and threshold updates.
    ///
    /// No.: 2
    /// Requirement: The list of owners for a multi-signature account should not contain any duplicate owners, and the
    /// multi-signature account itself cannot be listed as one of its owners.
    /// Criticality: Critical
    /// Implementation: The function validate_owners validates the owner vector that no duplicate entries exists.
    /// Enforcement: Formally verified at creation via the aborts conditions of create_with_owners_internal. Audited
    /// that removing owners preserves it.
    ///
    /// No.: 3
    /// Requirement: The current value of the next sequence number should not be present in the transaction table, until
    /// the next sequence number gets increased.
    /// Criticality: Medium
    /// Implementation: The add_transaction function increases the next sequence number and only then adds the
    /// transaction with the old next sequence number to the transaction table.
    /// Enforcement: Formally verified via [high-level-req-3](MultisigAccount) as a data invariant: the table
    /// contains exactly the sequence numbers strictly between last_executed_sequence_number and
    /// next_sequence_number.
    ///
    /// No.: 4
    /// Requirement: When the last executed sequence number is smaller than the next sequence number by only one unit, no
    /// transactions should exist in the multi-signature account's transactions list.
    /// Criticality: High
    /// Implementation: The get_pending_transactions function retrieves pending transactions by iterating through the
    /// transactions table, starting from the last_executed_sequence_number + 1 to the next_sequence_number.
    /// Enforcement: Formally verified via [high-level-req-3](MultisigAccount), whose interval is empty when
    /// last_executed_sequence_number == next_sequence_number - 1.
    ///
    /// No.: 5
    /// Requirement: The last executed sequence number is always smaller than the next sequence number.
    /// Criticality: Medium
    /// Implementation: When creating a new MultisigAccount, the last_executed_sequence_number and next_sequence_number
    /// are assigned with 0 and 1 respectively, and from there both these values increase monotonically when a
    /// transaction is executed and removed from the table and when new transaction are added respectively.
    /// Enforcement: Formally verified via [high-level-req-5](MultisigAccount) as a data invariant.
    ///
    /// No.: 6
    /// Requirement: The number of pending transactions should be equal to the difference between the next sequence number
    /// and the last executed sequence number.
    /// Criticality: High
    /// Implementation: When a transaction is added, next_sequence_number is incremented. And when a transaction is
    /// removed after execution, last_executed_sequence_number is incremented.
    /// Enforcement: Formally verified via [high-level-req-6.1](MultisigAccount) that the stored transactions are
    /// exactly those numbered last_executed_sequence_number + 1 to next_sequence_number - 1, and via
    /// [high-level-req-6.2](get_pending_transactions) that it returns next_sequence_number -
    /// last_executed_sequence_number - 1 transactions.
    ///
    /// No.: 7
    /// Requirement: Only transactions with valid sequence number should be fetched.
    /// Criticality: Medium
    /// Implementation: Functions such as: 1. get_transaction 2. can_be_executed 3. can_be_rejected 4. vote always
    /// validate the given sequence number and only then fetch the associated transaction.
    /// Enforcement: Formally verified via [high-level-req-7.1](ValidSequenceNumberAbortsIf) and
    /// [high-level-req-7.2](ValidSequenceNumberAbortsIf), included by get_transaction, vote and the can_* functions,
    /// that it aborts if the sequence number is not valid.
    ///
    /// No.: 8
    /// Requirement: The execution or rejection of a transaction should enforce that the minimum number of required
    /// signatures is less or equal to the total number of approvals.
    /// Criticality: Critical
    /// Implementation: The functions can_be_executed and can_be_rejected perform validation on the number of votes
    /// required for execution or rejection.
    /// Enforcement: Formally verified via [high-level-req-8.1](can_be_executed), [high-level-req-8.2](can_be_rejected),
    /// [high-level-req-8.3](can_execute) and [high-level-req-8.4](can_reject) that the result is exactly: the
    /// transaction is the next one to resolve, and the approvals (resp. rejections) cast by current owners, counting
    /// the given owner's vote as cast for can_execute and can_reject, reach num_signatures_required, and for execution
    /// the timelock admits it. The abort conditions of these functions are only partly specified.
    ///
    /// No.: 9
    /// Requirement: The creation of a multi-signature account properly initializes the resources and then it gets
    /// published under the corresponding account.
    /// Criticality: Medium
    /// Implementation: When creating a MultisigAccount via one of the functions: create_with_existing_account,
    /// create_with_existing_account_and_revoke_auth_key, create_with_owners, create, the MultisigAccount data is
    /// initialized properly and published to the multisig_account (new or existing).
    /// Enforcement: Formally verified via [high-level-req-9.1](CreateMultisigAccountAbortsIfAndEnsures), included by
    /// create_with_owners and the create_with_existing_account variants, that the MultisigAccount is published with
    /// the given owners and number of signatures required, and via [high-level-req-9.2](create_with_owners_internal)
    /// that its sequence numbers start at 0 and 1. Audited that the metadata, the transaction table and the event
    /// handles are initialized properly.
    ///
    /// No.: 10
    /// Requirement: Creation of a multi-signature account on top of an existing account should revoke auth key and any
    /// previous offered capabilities or control.
    /// Criticality: Critical
    /// Implementation: The function create_with_existing_account_and_revoke_auth_key, after successfully creating the
    /// MultisigAccount, rotates the account to ZeroAuthKey and revokes any offered capabilities of that account.
    /// Enforcement: Formally verified via [high-level-req-10.1](create_with_existing_account_and_revoke_auth_key),
    /// [high-level-req-10.2](create_with_existing_account_and_revoke_auth_key) and
    /// [high-level-req-10.3](create_with_existing_account_and_revoke_auth_key) that the account's auth key and the
    /// offered capabilities are revoked; create_with_existing_account_and_revoke_auth_key_call has the same conditions.
    ///
    /// No.: 11
    /// Requirement: Upon the creation of a multi-signature account from a bootstrapping account, the ownership of the
    /// resultant account should not pertain to the bootstrapping account.
    /// Criticality: High
    /// Implementation: In create_with_owners_then_remove_bootstrapper function after successful creation of the account
    /// the bootstrapping account is removed from the owner vector of the account.
    /// Enforcement: Audited that the bootstrapping account is not in the owners list.
    ///
    /// No.: 12
    /// Requirement: Performing any changes on the list of owners such as adding new owners, removing owners, swapping
    /// owners should ensure that the number of required signature, for the multi-signature account remains valid.
    /// Criticality: Critical
    /// Implementation: The following function as used to modify the owners list and the required signature of the
    /// account: add_owner, add_owners, add_owners_and_update_signatures_required, remove_owner, remove_owners,
    /// swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_signatures_required. All of these
    /// functions use update_owner_schema function to process these changes, the function validates the owner list while
    /// adding and verifies that the account has enough required signatures and updates the owner's schema.
    /// Enforcement: Audited that the owners are added successfully. (add_owner, add_owners,
    /// add_owners_and_update_signatures_required, swap_owner, swap_owners, swap_owners_and_update_signatures_required,
    /// update_owner_schema) Audited that the owners are removed successfully. (remove_owner, remove_owners, swap_owner,
    /// swap_owners, swap_owners_and_update_signatures_required, update_owner_schema) Audited that the
    /// num_signatures_required is updated successfully. (add_owners_and_update_signatures_required,
    /// swap_owners_and_update_signatures_required, update_signatures_required, update_owner_schema)
    ///
    /// No.: 13
    /// Requirement: The creation of a transaction should be limited to an account owner, which should be automatically
    /// considered a voter; additionally, the account's sequence should increase monotonically.
    /// Criticality: Critical
    /// Implementation: The following functions can only be called by the owners of the account and create a transaction
    /// and uses add_transaction function to gives approval on behalf of the creator and increments the
    /// next_sequence_number and finally adds the transaction to the MultsigAccount: create_transaction_with_hash,
    /// create_transaction.
    /// Enforcement: Formally verified via [high-level-req-13.1](create_transaction) and
    /// [high-level-req-13.2](create_transaction_with_hash) that it aborts if the caller is not in the owner's list of
    /// the account. Audited that the transaction is successfully stored in the
    /// MultisigAccount.(create_transaction_with_hash, create_transaction, add_transaction) Audited that the creators
    /// voted to approve the transaction. (create_transaction_with_hash, create_transaction, add_transaction) Audited
    /// that the next_sequence_number increases monotonically. (create_transaction_with_hash, create_transaction,
    /// add_transaction)
    ///
    /// No.: 14
    /// Requirement: Only owners are allowed to vote for a valid transaction.
    /// Criticality: Critical
    /// Implementation: Any owner of the MultisigAccount can either approve (approve_transaction) or reject
    /// (reject_transaction) a transaction. Both these functions use a generic function to vote for the transaction
    /// which validates the caller and the transaction id and adds/updates the vote.
    /// Enforcement: Formally verified via [high-level-req-14.1](approve_transaction),
    /// [high-level-req-14.2](reject_transaction) and [high-level-req-14.3](vote_transaction) that it aborts if the
    /// caller is not in the owner's list, and via [high-level-req-14.4](vote_transaction) that vote_transaction aborts
    /// if the transaction with the given sequence number doesn't exist in the account. Audited the latter for
    /// approve_transaction and reject_transaction, and that the vote is recorded as intended.
    ///
    /// No.: 15
    /// Requirement: Only owners are allowed to execute a valid transaction, if the number of approvals meets the k-of-n
    /// criteria, finally the executed transaction should be removed.
    /// Criticality: Critical
    /// Implementation: Functions execute_rejected_transaction and validate_multisig_transaction can only be called by
    /// the owner which validates the transaction and based on the number of approvals and rejections it proceeds to
    /// execute the transactions. For rejected transaction, the transactions are immediately removed from the
    /// MultisigAccount via remove_executed_transaction. VM validates the transaction via validate_multisig_transaction
    /// and cleans up the transaction via successful_transaction_execution_cleanup and
    /// failed_transaction_execution_cleanup.
    /// Enforcement: Formally verified via [high-level-req-15.1](execute_rejected_transaction) and
    /// [high-level-req-15.2](validate_multisig_transaction) that it aborts if the caller is not in the owner's list,
    /// via [high-level-req-15.3](validate_multisig_transaction) that validate_multisig_transaction aborts if the next
    /// transaction doesn't exist in the account, and via [high-level-req-15.4](TransactionExecutionCleanup), included by
    /// successful_transaction_execution_cleanup and failed_transaction_execution_cleanup, that the executed transaction
    /// is removed. Audited that it aborts if the votes (approvals or rejections) are less than num_signatures_required,
    /// that execute_rejected_transaction aborts if the transaction doesn't exist, and that it removes the transaction.
    ///
    /// No.: 16
    /// Requirement: Removing an executed transaction from the transactions list should increase the last sequence number
    /// monotonically.
    /// Criticality: High
    /// Implementation: When transactions are removed via remove_executed_transaction (maybe called by VM cleanup or
    /// execute_rejected_transaction), the last_executed_sequence_number increases by 1.
    /// Enforcement: Formally verified via [high-level-req-16.1](remove_executed_transaction) and
    /// [high-level-req-16.2](TransactionExecutionCleanup) that last_executed_sequence_number is incremented by one.
    ///
    /// No.: 17
    /// Requirement: The voting and transaction creation operations should only be available if a multi-signature account
    /// exists.
    /// Criticality: Low
    /// Implementation: The function assert_multisig_account_exists validates the existence of MultisigAccount under the
    /// account.
    /// Enforcement: Formally verified via [high-level-req-17.1](create_transaction),
    /// [high-level-req-17.2](create_transaction_with_hash), [high-level-req-17.3](approve_transaction),
    /// [high-level-req-17.4](reject_transaction) and [high-level-req-17.5](vote_transaction) that it aborts if the
    /// MultisigAccount doesn't exist on the account.
    /// </high-level-req>

    spec module {
        /// A `MultisigAccountTimeLock` can never exist without its corresponding `MultisigAccount`.
        /// This is enforced at write time by `upsert_timelock_internal`, which asserts the
        /// multisig account exists before publishing the timelock resource.
        invariant forall a: address where exists<MultisigAccountTimeLock>(a):
            exists<MultisigAccount>(a);
    }

    spec module {
        /// [high-level-req-1]
        invariant [suspendable] forall a: address {exists<MultisigAccount>(a)} where exists<MultisigAccount>(a):
            1 <= global<MultisigAccount>(a).num_signatures_required
                && global<MultisigAccount>(a).num_signatures_required <= len(global<MultisigAccount>(a).owners);
    }

    spec MultisigAccount {
        /// [high-level-req-5]
        invariant last_executed_sequence_number < next_sequence_number;
        /// [high-level-req-3]
        /// [high-level-req-6.1]
        invariant forall i: u64 {table::spec_contains(transactions, i)}:
            table::spec_contains(transactions, i) <==>
                last_executed_sequence_number < i && i < next_sequence_number;
    }

    spec metadata(multisig_account: address): SimpleMap<String, vector<u8>> {
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == global<MultisigAccount>(multisig_account).metadata;
    }

    spec num_signatures_required(multisig_account: address): u64 {
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == global<MultisigAccount>(multisig_account).num_signatures_required;
    }

    spec owners(multisig_account: address): vector<address> {
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == global<MultisigAccount>(multisig_account).owners;
    }

    spec is_owner(owner: address, multisig_account: address): bool {
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == vector::spec_contains(global<MultisigAccount>(multisig_account).owners, owner);
    }

    spec get_transaction(
        multisig_account: address,
        sequence_number: u64,
    ): MultisigTransaction {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        ensures result == table::spec_get(multisig_account_resource.transactions, sequence_number);
    }

    spec get_pending_transactions(multisig_account: address): vector<MultisigTransaction> {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        let last = multisig_account_resource.last_executed_sequence_number;
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-6.2]
        ensures len(result) == multisig_account_resource.next_sequence_number - last - 1;
    }

    /// The account exists, `sequence_number` is in its queue range and the transaction is stored.
    spec schema ValidSequenceNumberAbortsIf {
        multisig_account: address;
        sequence_number: u64;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-7.1]
        aborts_if sequence_number == 0 || sequence_number >= multisig_account_resource.next_sequence_number;
        /// [high-level-req-7.2]
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
    }

    spec get_next_transaction_payload(
    multisig_account: address, provided_payload: vector<u8>
    ): vector<u8> {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        let sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
        let transaction = table::spec_get(multisig_account_resource.transactions, sequence_number);
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
        ensures option::is_none(transaction.payload) ==> result == provided_payload;
    }

    spec get_next_multisig_account_address(creator: address): address {
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        aborts_if !exists<account::Account>(creator) && !features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        ensures result == spec_next_multisig_account_address(creator);
    }

    spec last_resolved_sequence_number(multisig_account: address): u64 {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == multisig_account_resource.last_executed_sequence_number;
    }

    spec next_sequence_number(multisig_account: address): u64 {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        aborts_if !exists<MultisigAccount>(multisig_account);
        ensures result == multisig_account_resource.next_sequence_number;
    }

    spec vote(
        multisig_account: address,
        sequence_number: u64,
        owner: address
    ): (bool, bool) {
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        let transaction = table::spec_get(multisig_account_resource.transactions, sequence_number);
        let votes = transaction.votes;
        let voted = simple_map::spec_contains_key(votes, owner);
        let vote = voted && simple_map::spec_get(votes, owner);
        ensures result_1 == voted;
        ensures result_2 == vote;
    }

    ////////////////////////// Timelock view function specs ///////////////////////////////

    /// Note: `can_execute_with_timelock` is an `inline` helper, so the Prover sees it expanded
    /// at every call site. Its preconditions and result are therefore captured here via the
    /// `can_execute` spec rather than via a standalone spec block.
    spec can_execute(owner: address, multisig_account: address, sequence_number: u64): bool {
        // Opaque, so that callers use the vote-count summary instead of the counting loop.
        pragma opaque;
        // Partial: the vote-count and timelock aborts are not specified.
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        let votes = table::spec_get(multisig_account_resource.transactions, sequence_number).votes;
        // The owner's own approval is counted as if cast now.
        let num_approvals = spec_num_approvals(multisig_account_resource.owners, votes)
            + if (simple_map::spec_contains_key(votes, owner) && simple_map::spec_get(votes, owner)) 0 else 1;
        /// [high-level-req-8.3]
        ensures result == (vector::spec_contains(multisig_account_resource.owners, owner)
            && sequence_number == multisig_account_resource.last_executed_sequence_number + 1
            && num_approvals >= multisig_account_resource.num_signatures_required
            && spec_timelock_admits(multisig_account, sequence_number, num_approvals));
    }

    spec can_be_executed(multisig_account: address, sequence_number: u64): bool {
        // Opaque, so that callers use the vote-count summary instead of the counting loop.
        pragma opaque;
        // Partial: the vote-count and timelock aborts are not specified.
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        let votes = table::spec_get(multisig_account_resource.transactions, sequence_number).votes;
        let num_approvals = spec_num_approvals(multisig_account_resource.owners, votes);
        /// [high-level-req-8.1]
        ensures result == (sequence_number == multisig_account_resource.last_executed_sequence_number + 1
            && num_approvals >= multisig_account_resource.num_signatures_required
            && spec_timelock_admits(multisig_account, sequence_number, num_approvals));
    }

    /// The number of approvals cast by `owners`.
    spec fun spec_num_approvals(owners: vector<address>, votes: SimpleMap<address, bool>): u64 {
        vector::spec_fold<address, u64>(
            |acc, owner| if (simple_map::spec_contains_key(votes, owner) && simple_map::spec_get(votes, owner)) {
                acc + 1u64
            } else acc,
            owners, 0u64, len(owners))
    }

    /// Whether the timelock of `multisig_account`, if any, admits executing transaction `sequence_number` with
    /// `num_approvals` approvals now.
    spec fun spec_timelock_admits(multisig_account: address, sequence_number: u64, num_approvals: u64): bool {
        if (exists<MultisigAccountTimeLock>(multisig_account)) {
            let timelock = global<MultisigAccountTimeLock>(multisig_account);
            let creation_time_secs = table::spec_get(
                global<MultisigAccount>(multisig_account).transactions, sequence_number).creation_time_secs;
            (option::spec_is_some(timelock.override_threshold)
                && num_approvals >= option::spec_borrow(timelock.override_threshold))
                || aptos_framework::timestamp::spec_now_seconds() - creation_time_secs >= timelock.timelock_period
        } else {
            true
        }
    }

    /// The number of rejections cast by `owners`.
    spec fun spec_num_rejections(owners: vector<address>, votes: SimpleMap<address, bool>): u64 {
        vector::spec_fold<address, u64>(
            |acc, owner| if (simple_map::spec_contains_key(votes, owner)) {
                if (simple_map::spec_get(votes, owner)) acc else acc + 1u64
            } else acc,
            owners, 0u64, len(owners))
    }

    spec can_be_rejected(multisig_account: address, sequence_number: u64): bool {
        // Partial: the vote-count aborts are not specified.
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        let votes = table::spec_get(multisig_account_resource.transactions, sequence_number).votes;
        /// [high-level-req-8.2]
        ensures result == (sequence_number == multisig_account_resource.last_executed_sequence_number + 1
            && spec_num_rejections(multisig_account_resource.owners, votes)
                >= multisig_account_resource.num_signatures_required);
    }

    spec can_reject(owner: address, multisig_account: address, sequence_number: u64): bool {
        // Partial: the vote-count aborts are not specified.
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        include ValidSequenceNumberAbortsIf;
        aborts_if vector::spec_contains(multisig_account_resource.owners, owner)
            && multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        ensures result ==>
            vector::spec_contains(multisig_account_resource.owners, owner) &&
            sequence_number == multisig_account_resource.last_executed_sequence_number + 1;
        let votes = table::spec_get(multisig_account_resource.transactions, sequence_number).votes;
        // The owner's own rejection is counted as if cast now.
        let num_rejections = spec_num_rejections(multisig_account_resource.owners, votes)
            + if (simple_map::spec_contains_key(votes, owner) && !simple_map::spec_get(votes, owner)) 0 else 1;
        /// [high-level-req-8.4]
        ensures result == (vector::spec_contains(multisig_account_resource.owners, owner)
            && sequence_number == multisig_account_resource.last_executed_sequence_number + 1
            && num_rejections >= multisig_account_resource.num_signatures_required);
    }

    spec timelock_period(multisig_account: address): u64 {
        aborts_if false;
        ensures !exists<MultisigAccountTimeLock>(multisig_account) ==> result == 0;
        ensures exists<MultisigAccountTimeLock>(multisig_account) ==>
            result == global<MultisigAccountTimeLock>(multisig_account).timelock_period;
    }

    spec timelock_override_threshold(multisig_account: address): Option<u64> {
        aborts_if false;
        ensures !exists<MultisigAccountTimeLock>(multisig_account) ==>
            option::is_none(result);
        ensures exists<MultisigAccountTimeLock>(multisig_account) ==>
            result == global<MultisigAccountTimeLock>(multisig_account).override_threshold;
    }

    ////////////////////////// Timelock entry function specs ///////////////////////////////

    spec upsert_timelock(multisig_account: &signer, timelock_period: u64, override_threshold: Option<u64>) {
        use std::signer;
        use std::features;
        let multisig_address = signer::address_of(multisig_account);
        let timelock_enabled = features::spec_multisig_timelock_enabled();
        // Feature flag must be enabled.
        aborts_if !features::spec_multisig_timelock_enabled();
        // Timelock must be enabled
        ensures timelock_enabled;
        include UpsertTimelockAbortsIfAndEnsures;
        // Frame condition: the MultisigAccount resource itself is not mutated. This pins the
        // invariant that upsert_timelock cannot perturb owners, num_signatures_required, the
        // transaction queue, metadata, or any event handles.
        ensures global<MultisigAccount>(multisig_address) == old(global<MultisigAccount>(multisig_address));
    }

    spec upsert_timelock_internal(multisig_account: &signer, timelock_period: u64, override_threshold: Option<u64>) {
        use std::signer;
        pragma opaque;
        let multisig_address = signer::address_of(multisig_account);
        include UpsertTimelockAbortsIfAndEnsures;
        modifies global<MultisigAccountTimeLock>(multisig_address);
    }

    spec schema UpsertTimelockAbortsIfAndEnsures {
        multisig_address: address;
        timelock_period: u64;
        override_threshold: Option<u64>;
        // Must be a multisig account.
        aborts_if !exists<MultisigAccount>(multisig_address);
        // Timelock period must be within valid range.
        aborts_if timelock_period < MIN_TIMELOCK_PERIOD || timelock_period > MAX_TIMELOCK_PERIOD;
        // Override threshold must be > num_signatures_required (if provided).
        aborts_if override_threshold.is_some() &&
            override_threshold.borrow() <= global<MultisigAccount>(multisig_address).num_signatures_required;
        // Override threshold must be <= number of owners (if provided).
        aborts_if override_threshold.is_some() &&
            override_threshold.borrow() > len(global<MultisigAccount>(multisig_address).owners);
        // After upsert, the timelock resource exists.
        ensures exists<MultisigAccountTimeLock>(multisig_address);
        // The stored values match the provided values.
        ensures global<MultisigAccountTimeLock>(multisig_address).timelock_period == timelock_period;
        ensures global<MultisigAccountTimeLock>(multisig_address).override_threshold == override_threshold;
    }

    spec create_with_owners_and_timelock(
        owner: &signer,
        additional_owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
        timelock_period: Option<u64>,
        override_threshold: Option<u64>,
    ) {
        use std::features;
        include CreateWithOwnersAbortsIf;
        // The creator is appended to the owner set before create_with_owners_internal runs.
        let total_owners = len(additional_owners) + 1;
        // Feature flag must be enabled.
        aborts_if !features::spec_multisig_timelock_enabled();
        // override_threshold has no meaning without a timelock_period.
        aborts_if option::is_some(override_threshold) && option::is_none(timelock_period);
        // num_signatures_required must be in range once the creator joins the owner set.
        aborts_if num_signatures_required == 0 || num_signatures_required > total_owners;
        // Timelock period, when configured, must be within valid range.
        aborts_if option::is_some(timelock_period) &&
            (option::spec_borrow(timelock_period) < MIN_TIMELOCK_PERIOD ||
                option::spec_borrow(timelock_period) > MAX_TIMELOCK_PERIOD);
        // Override threshold, when configured, must be stronger than the normal quorum and fit
        // in the owner set.
        aborts_if option::is_some(override_threshold) &&
            option::spec_borrow(override_threshold) <= num_signatures_required;
        aborts_if option::is_some(override_threshold) &&
            option::spec_borrow(override_threshold) > total_owners;
    }

    spec remove_timelock(multisig_account: &signer) {
        use std::signer;
        let multisig_address = signer::address_of(multisig_account);
        aborts_if !exists<MultisigAccount>(multisig_address);
        // Aborts if no timelock exists — removal is no longer silent.
        aborts_if !exists<MultisigAccountTimeLock>(multisig_address);
        // After removal, the timelock resource no longer exists.
        ensures !exists<MultisigAccountTimeLock>(multisig_address);
        // Frame condition: the MultisigAccount resource itself is not mutated.
        ensures global<MultisigAccount>(multisig_address) == old(global<MultisigAccount>(multisig_address));
    }

    ////////////////////////// Creation function specs ///////////////////////////////

    spec schema CreateMultisigAccountAbortsIf {
        multisig_address: address;
        owners: vector<address>;
        num_signatures_required: u64;
        aborts_if num_signatures_required == 0 || num_signatures_required > len(owners);
        aborts_if exists<MultisigAccount>(multisig_address);
    }

    spec schema CreateMultisigAccountAbortsIfAndEnsures {
        multisig_address: address;
        owners: vector<address>;
        num_signatures_required: u64;
        include CreateMultisigAccountAbortsIf;
        let post multisig = global<MultisigAccount>(multisig_address);
        /// [high-level-req-9.1]
        ensures exists<MultisigAccount>(multisig_address);
        ensures multisig.owners == owners;
        ensures multisig.num_signatures_required == num_signatures_required;
    }

    spec create_with_owners(
        owner: &signer,
        additional_owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
    ) {
        use std::signer;
        let owner_addr = signer::address_of(owner);
        let owners = concat(additional_owners, vec(owner_addr));
        let multisig_address = spec_next_multisig_account_address(owner_addr);
        include CreateMultisigAccountAbortsIfAndEnsures;
        include CreateWithOwnersAbortsIf;
        let post multisig = global<MultisigAccount>(multisig_address);
        ensures multisig.last_executed_sequence_number == 0;
        ensures multisig.next_sequence_number == 1;
        ensures option::is_some(multisig.signer_cap) && option::spec_borrow(multisig.signer_cap).account == multisig_address;
    }

    /// The aborts of creating a multisig account for `owner` and the other owners beyond the signature count.
    spec schema CreateWithOwnersAbortsIf {
        use std::signer;
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        use aptos_std::type_info;
        owner: &signer;
        additional_owners: vector<address>;
        metadata_keys: vector<String>;
        metadata_values: vector<vector<u8>>;
        let owner_addr = signer::address_of(owner);
        let owners = concat(additional_owners, vec(owner_addr));
        let multisig_address = spec_next_multisig_account_address(owner_addr);
        aborts_if exists i in 0..len(owners), j in 0..len(owners):
            i < j && owners[i] == owners[j];
        aborts_if exists i in 0..len(owners): owners[i] == multisig_address;
        aborts_if exists<account::Account>(multisig_address);
        // Creating the resource account.
        aborts_if !exists<account::Account>(owner_addr) && !features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        include account::EnsureResourceExistsAbortsIf { addr: multisig_address };
        aborts_if len(account::ZERO_AUTH_KEY) != 32;
        aborts_if !exists<coin::CoinInfo<AptosCoin>>(type_info::type_of<AptosCoin>().account_address);
        // Creating the multisig account.
        aborts_if !features::spec_is_enabled(features::MULTISIG_ACCOUNTS);
        aborts_if exists<MultisigAccount>(multisig_address);
        aborts_if len(metadata_keys) != len(metadata_values);
        aborts_if exists m in 0..len(metadata_keys), n in 0..len(metadata_keys):
            m < n && metadata_keys[m] == metadata_keys[n];
    }

    spec create_with_owners_then_remove_bootstrapper(
        bootstrapper: &signer,
        owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
    ) {
        use std::signer;
        // Partial: the aborts of creating the resource account and of the metadata are not specified; removing
        // the bootstrapper goes through a closure loop, so the final owner list is not specified.
        pragma aborts_if_is_partial;
        let bootstrapper_addr = signer::address_of(bootstrapper);
        let all_owners = concat(owners, vec(bootstrapper_addr));
        let multisig_address = spec_next_multisig_account_address(bootstrapper_addr);
        include CreateMultisigAccountAbortsIf { owners: all_owners };
        aborts_if exists i in 0..len(all_owners), j in 0..len(all_owners):
            i < j && all_owners[i] == all_owners[j];
        aborts_if exists<account::Account>(multisig_address);
        let post multisig = global<MultisigAccount>(multisig_address);
        ensures exists<MultisigAccount>(multisig_address);
        ensures multisig.num_signatures_required == num_signatures_required;
        ensures len(multisig.owners) >= num_signatures_required;
        ensures multisig.next_sequence_number == 1;
    }

    /// The address `create_multisig_account` uses; the seed mirrors `create_multisig_account_seed`.
    spec fun spec_next_multisig_account_address(creator: address): address {
        let nonce = if (exists<account::Account>(creator)) {
            global<account::Account>(creator).sequence_number
        } else { 0 };
        account::spec_create_resource_address(creator, concat(concat(vec<u8>(), DOMAIN_SEPARATOR), std::bcs::to_bytes((nonce as u64))))
    }

    spec create_with_existing_account(
        multisig_address: address,
        owners: vector<address>,
        num_signatures_required: u64,
        account_scheme: u8,
        account_public_key: vector<u8>,
        create_multisig_account_signed_message: vector<u8>,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
    ) {
        // Partial: the signature-verification aborts are not specified.
        pragma aborts_if_is_partial;
        // The creation proof is rejected where it could be replayed.
        aborts_if transaction_context::spec_is_orderless_txn() || transaction_context::spec_is_multisig_payload_txn();
        include CreateMultisigAccountAbortsIfAndEnsures;
    }

    spec create_with_existing_account_and_revoke_auth_key(
        multisig_address: address,
        owners: vector<address>,
        num_signatures_required: u64,
        account_scheme: u8,
        account_public_key: vector<u8>,
        create_multisig_account_signed_message: vector<u8>,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
    ) {
        // Partial: the signature-verification aborts are not specified.
        pragma aborts_if_is_partial;
        // The creation proof is rejected where it could be replayed.
        aborts_if transaction_context::spec_is_orderless_txn() || transaction_context::spec_is_multisig_payload_txn();
        include CreateMultisigAccountAbortsIfAndEnsures;
        // The auth key is rotated to zero and any signer or rotation capability offer is revoked.
        let post account_resource = global<account::Account>(multisig_address);
        /// [high-level-req-10.1]
        ensures account_resource.authentication_key == ZERO_AUTH_KEY;
        /// [high-level-req-10.2]
        ensures option::is_none(account_resource.signer_capability_offer.for);
        /// [high-level-req-10.3]
        ensures option::is_none(account_resource.rotation_capability_offer.for);
    }

    spec create_with_existing_account_and_revoke_auth_key_call(
        multisig_account: &signer,
        owners: vector<address>,
        num_signatures_required: u64,
        metadata_keys: vector<String>,
        metadata_values:vector<vector<u8>>,
    ) {
        use std::signer;
        // Owner validation, metadata and account-resource aborts are not specified.
        pragma aborts_if_is_partial;
        let multisig_address = signer::address_of(multisig_account);
        include CreateMultisigAccountAbortsIfAndEnsures;
        // The auth key is rotated to zero and any signer or rotation capability offer is revoked.
        let post account_resource = global<account::Account>(multisig_address);
        ensures account_resource.authentication_key == ZERO_AUTH_KEY;
        ensures option::is_none(account_resource.signer_capability_offer.for);
        ensures option::is_none(account_resource.rotation_capability_offer.for);
    }

    ////////////////////////// Core entry function specs ///////////////////////////////

    spec create_transaction(owner: &signer, multisig_account: address, payload: vector<u8>) {
        use std::signer;
        include AddTransactionAbortsIf;
        // Payload cannot be empty.
        aborts_if len(payload) == 0;
        // Must be a multisig account.
        /// [high-level-req-17.1]
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        /// [high-level-req-13.1]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec create_transaction_with_hash(owner: &signer, multisig_account: address, payload_hash: vector<u8>) {
        use std::signer;
        include AddTransactionAbortsIf;
        // Hash must be exactly 32 bytes.
        aborts_if len(payload_hash) != 32;
        // Must be a multisig account.
        /// [high-level-req-17.2]
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        /// [high-level-req-13.2]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    /// Queueing a transaction needs queue capacity, a fresh sequence number and the clock.
    spec schema AddTransactionAbortsIf {
        use std::features;
        multisig_account: address;
        let res = global<MultisigAccount>(multisig_account);
        let num_pending = res.next_sequence_number - res.last_executed_sequence_number - 1;
        aborts_if exists<MultisigAccount>(multisig_account)
            && features::spec_is_enabled(features::MULTISIG_V2_ENHANCEMENT)
            && num_pending >= MAX_PENDING_TRANSACTIONS;
        aborts_if exists<MultisigAccount>(multisig_account) && res.next_sequence_number + 1 > MAX_U64;
        aborts_if !exists<aptos_framework::timestamp::CurrentTimeMicroseconds>(@aptos_framework);
    }

    spec approve_transaction(owner: &signer, multisig_account: address, sequence_number: u64) {
        use std::signer;
        /// [high-level-req-17.3]
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-14.1]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec reject_transaction(owner: &signer, multisig_account: address, sequence_number: u64) {
        use std::signer;
        /// [high-level-req-17.4]
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-14.2]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec vote_transanction(owner: &signer, multisig_account: address, sequence_number: u64, approved: bool) {
        use std::signer;
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        // Transaction must exist.
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec vote_transaction(owner: &signer, multisig_account: address, sequence_number: u64, approved: bool) {
        use std::signer;
        use std::features;
        aborts_if !features::spec_is_enabled(features::MULTISIG_V2_ENHANCEMENT);
        /// [high-level-req-17.5]
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-14.3]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        /// [high-level-req-14.4]
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec execute_rejected_transaction(owner: &signer, multisig_account: address) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        /// [high-level-req-15.1]
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec vote_transactions(
        owner: &signer,
        multisig_account: address,
        starting_sequence_number: u64,
        final_sequence_number: u64,
        approved: bool
    ) {
        use std::signer;
        use std::features;
        // Partial: the per-transaction aborts after the first vote are not specified.
        pragma aborts_if_is_partial;
        let pre = global<MultisigAccount>(multisig_account);
        let post post_r = global<MultisigAccount>(multisig_account);
        let voting = starting_sequence_number <= final_sequence_number;
        aborts_if !features::spec_is_enabled(features::MULTISIG_V2_ENHANCEMENT);
        aborts_if voting && !exists<MultisigAccount>(multisig_account);
        aborts_if voting && !vector::spec_contains(pre.owners, signer::address_of(owner));
        // Only votes change.
        ensures post_r.owners == pre.owners;
        ensures post_r.num_signatures_required == pre.num_signatures_required;
        ensures post_r.last_executed_sequence_number == pre.last_executed_sequence_number;
        ensures post_r.next_sequence_number == pre.next_sequence_number;
        ensures post_r.metadata == pre.metadata;
    }

    spec execute_rejected_transactions(
        owner: &signer,
        multisig_account: address,
        final_sequence_number: u64,
    ) {
        use std::signer;
        use std::features;
        // Partial: the rejection-count aborts of each removal are not specified.
        pragma aborts_if_is_partial;
        let pre = global<MultisigAccount>(multisig_account);
        let post post_r = global<MultisigAccount>(multisig_account);
        aborts_if !features::spec_is_enabled(features::MULTISIG_V2_ENHANCEMENT);
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if pre.last_executed_sequence_number >= final_sequence_number;
        aborts_if final_sequence_number >= pre.next_sequence_number;
        aborts_if !vector::spec_contains(pre.owners, signer::address_of(owner));
        // All transactions up to `final_sequence_number` are resolved; the owner set is unchanged.
        ensures post_r.last_executed_sequence_number == final_sequence_number;
        ensures post_r.next_sequence_number == pre.next_sequence_number;
        ensures post_r.owners == pre.owners;
        ensures post_r.num_signatures_required == pre.num_signatures_required;
        ensures post_r.metadata == pre.metadata;
    }

    ////////////////////////// VM prologue specs ///////////////////////////////

    /// `validate_multisig_transaction` is called by the VM as part of the transaction prologue.
    /// The timelock change added `ETIMELOCK_NOT_EXPIRED` as a new abort path, separate from the
    /// existing `ENOT_ENOUGH_APPROVALS` quorum failure. This spec documents the verifiable
    /// abort vectors; the quorum, timelock, and payload-match aborts depend on transaction-
    /// level state (and, for timelock, an inline helper plus the optional
    /// `MultisigAccountTimeLock` resource) and are left to `pragma aborts_if_is_partial`.
    spec validate_multisig_transaction(owner: &signer, multisig_account: address, payload: vector<u8>) {
        use std::signer;
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        let sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
        let owner_addr = signer::address_of(owner);

        // The multisig account must exist (assert_multisig_account_exists).
        aborts_if !exists<MultisigAccount>(multisig_account);
        // The signer must be an owner of the multisig account (assert_is_owner).
        /// [high-level-req-15.2]
        aborts_if !vector::spec_contains(multisig_account_resource.owners, owner_addr);
        // Sequence-number arithmetic must not overflow.
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        // The pending transaction must exist at the next sequence number
        // (assert_transaction_exists).
        /// [high-level-req-15.3]
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
    }

    spec successful_transaction_execution_cleanup(
        executor: address,
        multisig_account: address,
        transaction_payload: vector<u8>,
    ) {
        // Vote-counting arithmetic aborts are not specified.
        pragma aborts_if_is_partial;
        include TransactionExecutionCleanup;
    }

    spec failed_transaction_execution_cleanup(
        executor: address,
        multisig_account: address,
        transaction_payload: vector<u8>,
        execution_error: ExecutionError,
    ) {
        // Vote-counting arithmetic aborts are not specified.
        pragma aborts_if_is_partial;
        include TransactionExecutionCleanup;
    }

    /// The next pending transaction is removed from the queue and marked as resolved; the owner
    /// set and signature threshold are unchanged.
    spec schema TransactionExecutionCleanup {
        multisig_account: address;
        let pre = global<MultisigAccount>(multisig_account);
        let post post_r = global<MultisigAccount>(multisig_account);
        let sequence_number = pre.last_executed_sequence_number + 1;
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if sequence_number > MAX_U64;
        aborts_if sequence_number >= pre.next_sequence_number;
        aborts_if !table::spec_contains(pre.transactions, sequence_number);
        /// [high-level-req-16.2]
        ensures post_r.last_executed_sequence_number == sequence_number;
        /// [high-level-req-15.4]
        ensures !table::spec_contains(post_r.transactions, sequence_number);
        ensures post_r.next_sequence_number == pre.next_sequence_number;
        ensures post_r.owners == pre.owners;
        ensures post_r.num_signatures_required == pre.num_signatures_required;
    }

    ////////////////////////// Internal function specs ///////////////////////////////

    spec create_with_owners_internal(
        multisig_account: &signer,
        owners: vector<address>,
        num_signatures_required: u64,
        multisig_account_signer_cap: Option<SignerCapability>,
        metadata_keys: vector<String>,
        metadata_values: vector<vector<u8>>,
    ) {
        use std::signer;
        use std::features;
        pragma verify = true;
        let multisig_address = signer::address_of(multisig_account);
        aborts_if !features::spec_is_enabled(features::MULTISIG_ACCOUNTS);
        aborts_if num_signatures_required == 0 || num_signatures_required > len(owners);
        aborts_if exists<MultisigAccount>(multisig_address);
        aborts_if exists i in 0..len(owners): owners[i] == multisig_address;
        aborts_if exists i in 0..len(owners), j in 0..len(owners):
            i < j && owners[i] == owners[j];
        // The account holds nine event handles of the multisig account.
        include account::NewEventHandleAbortsIf { account: multisig_account };
        aborts_if exists<account::Account>(multisig_address)
            && global<account::Account>(multisig_address).guid_creation_num + 9 >= account::MAX_GUID_CREATION_NUM;
        aborts_if len(metadata_keys) != len(metadata_values);
        aborts_if exists m in 0..len(metadata_keys), n in 0..len(metadata_keys):
            m < n && metadata_keys[m] == metadata_keys[n];
        ensures exists<MultisigAccount>(multisig_address);
        let post multisig = global<MultisigAccount>(multisig_address);
        ensures multisig.owners == owners;
        ensures multisig.num_signatures_required == num_signatures_required;
        /// [high-level-req-9.2]
        ensures multisig.last_executed_sequence_number == 0;
        ensures multisig.next_sequence_number == 1;
    }

    spec update_metadata_internal(
        multisig_account: &signer,
        keys: vector<String>,
        values: vector<vector<u8>>,
        emit_event: bool,
    ) {
        use std::signer;
        pragma opaque;
        let multisig_address = signer::address_of(multisig_account);
        aborts_if len(keys) != len(values);
        aborts_if !exists<MultisigAccount>(multisig_address);
        aborts_if exists m in 0..len(keys), n in 0..len(keys): m < n && keys[m] == keys[n];
        modifies global<MultisigAccount>(multisig_address);
        let pre = global<MultisigAccount>(multisig_address);
        let post post_r = global<MultisigAccount>(multisig_address);
        ensures exists<MultisigAccount>(multisig_address);
        ensures post_r.owners == pre.owners;
        ensures post_r.num_signatures_required == pre.num_signatures_required;
        ensures post_r.transactions == pre.transactions;
        ensures post_r.last_executed_sequence_number == pre.last_executed_sequence_number;
        ensures post_r.next_sequence_number == pre.next_sequence_number;
        ensures post_r.signer_cap == pre.signer_cap;
    }

    spec validate_owners(owners: &vector<address>, multisig_account: address) {
        pragma opaque;
        aborts_if [abstract] exists i in 0..len(owners): owners[i] == multisig_account;
        aborts_if [abstract] exists i in 0..len(owners), j in 0..len(owners) where i < j:
            owners[i] == owners[j];
    }

    spec create_multisig_account(owner: &signer): (signer, SignerCapability) {
        use std::signer;
        use std::features::{Self, DEFAULT_ACCOUNT_RESOURCE};
        use aptos_std::type_info;
        let owner_addr = signer::address_of(owner);
        let multisig_address = spec_next_multisig_account_address(owner_addr);
        aborts_if !exists<account::Account>(owner_addr) && !features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);
        aborts_if exists<account::Account>(multisig_address);
        include account::EnsureResourceExistsAbortsIf { addr: multisig_address };
        aborts_if len(account::ZERO_AUTH_KEY) != 32;
        aborts_if !exists<coin::CoinInfo<AptosCoin>>(type_info::type_of<AptosCoin>().account_address);
        ensures signer::address_of(result_1) == multisig_address;
        ensures result_2.account == multisig_address;
        ensures exists<account::Account>(multisig_address);
    }

    spec remove_executed_transaction(multisig_account_resource: &mut MultisigAccount): (u64, u64) {
        pragma aborts_if_is_partial;
        // last_executed_sequence_number must not overflow.
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        // The transaction to remove must exist.
        aborts_if !table::spec_contains(
            multisig_account_resource.transactions,
            multisig_account_resource.last_executed_sequence_number + 1
        );
        // After removal, last_executed_sequence_number increments by 1.
        /// [high-level-req-16.1]
        ensures multisig_account_resource.last_executed_sequence_number ==
            old(multisig_account_resource).last_executed_sequence_number + 1;
    }

    spec update_owner_schema(
        multisig_address: address,
        new_owners: vector<address>,
        owners_to_remove: vector<address>,
        optional_new_num_signatures_required: Option<u64>,
    ) {
        // The owner list and threshold may be invalid between the update and the checks that abort on it.
        pragma disable_invariants_in_body;
    }
}
