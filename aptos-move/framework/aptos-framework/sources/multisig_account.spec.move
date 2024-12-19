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
        pragma aborts_if_is_partial;
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
        ensures option::spec_is_none(transaction.payload) ==> result == provided_payload;
    }

    spec get_next_multisig_account_address(creator: address): address {
        aborts_if !exists<account::Account>(creator);
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

}
