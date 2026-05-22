spec aptos_framework::multisig_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: For every multi-signature account, the range of required signatures should always be in the range of
    /// one to the total number of owners.
    /// Criticality: Critical
    /// Implementation: While creating a MultisigAccount, the function create_with_owners_internal checks that
    /// num_signatures_required is in the span from 1 to total count of owners.
    /// Enforcement: This has been audited.
    ///
    /// No.: 2
    /// Requirement: The list of owners for a multi-signature account should not contain any duplicate owners, and the
    /// multi-signature account itself cannot be listed as one of its owners.
    /// Criticality: Critical
    /// Implementation: The function validate_owners validates the owner vector that no duplicate entries exists.
    /// Enforcement: This has been audited.
    ///
    /// No.: 3
    /// Requirement: The current value of the next sequence number should not be present in the transaction table, until
    /// the next sequence number gets increased.
    /// Criticality: Medium
    /// Implementation: The add_transaction function increases the next sequence number and only then adds the
    /// transaction with the old next sequence number to the transaction table.
    /// Enforcement: This has been audited.
    ///
    /// No.: 4
    /// Requirement: When the last executed sequence number is smaller than the next sequence number by only one unit, no
    /// transactions should exist in the multi-signature account's transactions list.
    /// Criticality: High
    /// Implementation: The get_pending_transactions function retrieves pending transactions by iterating through the
    /// transactions table, starting from the last_executed_sequence_number + 1 to the next_sequence_number.
    /// Enforcement: Audited that MultisigAccount.transactions is empty when
    /// last_executed_sequence_number == next_sequence_number -1
    ///
    /// No.: 5
    /// Requirement: The last executed sequence number is always smaller than the next sequence number.
    /// Criticality: Medium
    /// Implementation: When creating a new MultisigAccount, the last_executed_sequence_number and next_sequence_number
    /// are assigned with 0 and 1 respectively, and from there both these values increase monotonically when a
    /// transaction is executed and removed from the table and when new transaction are added respectively.
    /// Enforcement: This has been audited.
    ///
    /// No.: 6
    /// Requirement: The number of pending transactions should be equal to the difference between the next sequence number
    /// and the last executed sequence number.
    /// Criticality: High
    /// Implementation: When a transaction is added, next_sequence_number is incremented. And when a transaction is
    /// removed after execution, last_executed_sequence_number is incremented.
    /// Enforcement: This has been audited.
    ///
    /// No.: 7
    /// Requirement: Only transactions with valid sequence number should be fetched.
    /// Criticality: Medium
    /// Implementation: Functions such as: 1. get_transaction 2. can_be_executed 3. can_be_rejected 4. vote always
    /// validate the given sequence number and only then fetch the associated transaction.
    /// Enforcement: Audited that it aborts if the sequence number is not valid.
    ///
    /// No.: 8
    /// Requirement: The execution or rejection of a transaction should enforce that the minimum number of required
    /// signatures is less or equal to the total number of approvals.
    /// Criticality: Critical
    /// Implementation: The functions can_be_executed and can_be_rejected perform validation on the number of votes
    /// required for execution or rejection.
    /// Enforcement: Audited that these functions return the correct value.
    ///
    /// No.: 9
    /// Requirement: The creation of a multi-signature account properly initializes the resources and then it gets
    /// published under the corresponding account.
    /// Criticality: Medium
    /// Implementation: When creating a MultisigAccount via one of the functions: create_with_existing_account,
    /// create_with_existing_account_and_revoke_auth_key, create_with_owners, create, the MultisigAccount data is
    /// initialized properly and published to the multisig_account (new or existing).
    /// Enforcement: Audited that the MultisigAccount is initialized properly.
    ///
    /// No.: 10
    /// Requirement: Creation of a multi-signature account on top of an existing account should revoke auth key and any
    /// previous offered capabilities or control.
    /// Criticality: Critical
    /// Implementation: The function create_with_existing_account_and_revoke_auth_key, after successfully creating the
    /// MultisigAccount, rotates the account to ZeroAuthKey and revokes any offered capabilities of that account.
    /// Enforcement: Audited that the account's auth key and the offered capabilities are revoked.
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
    /// Enforcement: Audited it aborts if the caller is not in the owner's list of the account.
    /// (create_transaction_with_hash, create_transaction) Audited that the transaction is successfully stored in the
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
    /// Enforcement: Audited that it aborts if the caller is not in the owner's list (approve_transaction,
    /// reject_transaction, vote_transaction, assert_is_owner). Audited that it aborts if the transaction with the given
    /// sequence number doesn't exist in the account (approve_transaction, reject_transaction, vote_transaction).
    /// Audited that the vote is recorded as intended.
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
    /// Enforcement: Audited that it aborts if the caller is not in the owner's list (execute_rejected_transaction,
    /// validate_multisig_transaction). Audited that it aborts if the transaction with the given sequence number doesn't
    /// exist in the account (execute_rejected_transaction, validate_multisig_transaction). Audited that it aborts if
    /// the votes (approvals or rejections) are less than num_signatures_required (execute_rejected_transaction,
    /// validate_multisig_transaction). Audited that the transaction is removed from the MultisigAccount
    /// (execute_rejected_transaction, remove_executed_transaction, successful_transaction_execution_cleanup,
    /// failed_transaction_execution_cleanup).
    ///
    /// No.: 16
    /// Requirement: Removing an executed transaction from the transactions list should increase the last sequence number
    /// monotonically.
    /// Criticality: High
    /// Implementation: When transactions are removed via remove_executed_transaction (maybe called by VM cleanup or
    /// execute_rejected_transaction), the last_executed_sequence_number increases by 1.
    /// Enforcement: Audited that last_executed_sequence_number is incremented.
    ///
    /// No.: 17
    /// Requirement: The voting and transaction creation operations should only be available if a multi-signature account
    /// exists.
    /// Criticality: Low
    /// Implementation: The function assert_multisig_account_exists validates the existence of MultisigAccount under the
    /// account.
    /// Enforcement: Audited that it aborts if the MultisigAccount doesn't exist on the account.
    /// </high-level-req>

    spec module {
        /// A `MultisigAccountTimeLock` can never exist without its corresponding `MultisigAccount`.
        /// This is enforced at write time by `upsert_timelock_internal`, which asserts the
        /// multisig account exists before publishing the timelock resource.
        invariant forall a: address where exists<MultisigAccountTimeLock>(a):
            exists<MultisigAccount>(a);
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
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if sequence_number == 0 || sequence_number >= multisig_account_resource.next_sequence_number;
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
        ensures result == table::spec_get(multisig_account_resource.transactions, sequence_number);
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
        //pragma aborts_if_is_partial;
        //aborts_if !exists<account::Account>(creator);
        let owner_nonce = global<account::Account>(creator).sequence_number;
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
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if sequence_number == 0 || sequence_number >= multisig_account_resource.next_sequence_number;
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
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
        pragma aborts_if_is_partial;
        let multisig_account_resource = global<MultisigAccount>(multisig_account);
        // assert_valid_sequence_number borrows the resource and validates the range.
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if sequence_number == 0 || sequence_number >= multisig_account_resource.next_sequence_number;
        // The owner-side checks must hold whenever the function returns true. The timelock and
        // approval-count parts of the conjunction depend on transaction state and the optional
        // MultisigAccountTimeLock resource — those remain abstract here.
        ensures result ==>
            vector::spec_contains(multisig_account_resource.owners, owner) &&
            sequence_number == multisig_account_resource.last_executed_sequence_number + 1;
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
        pragma aborts_if_is_partial;
        let multisig_address = signer::address_of(multisig_account);
        let timelock_enabled = features::spec_multisig_timelock_enabled();
        // Feature flag must be enabled.
        aborts_if !features::spec_multisig_timelock_enabled();
        // Must be a multisig account.
        aborts_if !exists<MultisigAccount>(multisig_address);
        // Timelock must be enabled
        ensures timelock_enabled;
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
        // Frame condition: the MultisigAccount resource itself is not mutated. This pins the
        // invariant that upsert_timelock cannot perturb owners, num_signatures_required, the
        // transaction queue, metadata, or any event handles.
        ensures global<MultisigAccount>(multisig_address) == old(global<MultisigAccount>(multisig_address));
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
        pragma aborts_if_is_partial;
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

    ////////////////////////// Core entry function specs ///////////////////////////////

    spec create_transaction(owner: &signer, multisig_account: address, payload: vector<u8>) {
        use std::signer;
        pragma aborts_if_is_partial;
        // Payload cannot be empty.
        aborts_if len(payload) == 0;
        // Must be a multisig account.
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec create_transaction_with_hash(owner: &signer, multisig_account: address, payload_hash: vector<u8>) {
        use std::signer;
        pragma aborts_if_is_partial;
        // Hash must be exactly 32 bytes.
        aborts_if len(payload_hash) != 32;
        // Must be a multisig account.
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec approve_transaction(owner: &signer, multisig_account: address, sequence_number: u64) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec reject_transaction(owner: &signer, multisig_account: address, sequence_number: u64) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
    }

    spec vote_transanction(owner: &signer, multisig_account: address, sequence_number: u64, approved: bool) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        // Caller must be an owner.
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        // Transaction must exist.
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec vote_transaction(owner: &signer, multisig_account: address, sequence_number: u64, approved: bool) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
        aborts_if !table::spec_contains(global<MultisigAccount>(multisig_account).transactions, sequence_number);
    }

    spec execute_rejected_transaction(owner: &signer, multisig_account: address) {
        use std::signer;
        pragma aborts_if_is_partial;
        aborts_if !exists<MultisigAccount>(multisig_account);
        aborts_if !vector::spec_contains(global<MultisigAccount>(multisig_account).owners, signer::address_of(owner));
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
        aborts_if !vector::spec_contains(multisig_account_resource.owners, owner_addr);
        // Sequence-number arithmetic must not overflow.
        aborts_if multisig_account_resource.last_executed_sequence_number + 1 > MAX_U64;
        // The pending transaction must exist at the next sequence number
        // (assert_transaction_exists).
        aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
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
        pragma aborts_if_is_partial;
        // num_signatures_required must be in [1, len(owners)].
        aborts_if num_signatures_required == 0 || num_signatures_required > len(owners);
        // After creation, MultisigAccount resource exists.
        ensures exists<MultisigAccount>(signer::address_of(multisig_account));
        let post multisig = global<MultisigAccount>(signer::address_of(multisig_account));
        // num_signatures_required is set correctly.
        ensures multisig.num_signatures_required == num_signatures_required;
        // Sequence numbers are initialized correctly.
        ensures multisig.last_executed_sequence_number == 0;
        ensures multisig.next_sequence_number == 1;
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
        ensures multisig_account_resource.last_executed_sequence_number ==
            old(multisig_account_resource).last_executed_sequence_number + 1;
    }
}
