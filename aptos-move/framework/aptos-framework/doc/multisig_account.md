
<a id="0x1_multisig_account"></a>

# Module `0x1::multisig_account`

Enhanced multisig account standard on Aptos. This is different from the native multisig scheme support enforced via
the account's auth key.

This module allows creating a flexible and powerful multisig account with seamless support for updating owners
without changing the auth key. Users can choose to store transaction payloads waiting for owner signatures on chain
or off chain (primary consideration is decentralization/transparency vs gas cost).

The multisig account is a resource account underneath. By default, it has no auth key and can only be controlled via
the special multisig transaction flow. However, owners can create a transaction to change the auth key to match a
private key off chain if so desired.

Transactions need to be executed in order of creation, similar to transactions for a normal Aptos account (enforced
with account nonce).

The flow is like below:
1. Owners can create a new multisig account by calling create (signer is default single owner) or with
create_with_owners where multiple initial owner addresses can be specified. This is different (and easier) from
the native multisig scheme where the owners' public keys have to be specified. Here, only addresses are needed.
2. Owners can be added/removed any time by calling add_owners or remove_owners. The transactions to do still need
to follow the k-of-n scheme specified for the multisig account.
3. To create a new transaction, an owner can call create_transaction with the transaction payload. This will store
the full transaction payload on chain, which adds decentralization (censorship is not possible as the data is
available on chain) and makes it easier to fetch all transactions waiting for execution. If saving gas is desired,
an owner can alternatively call create_transaction_with_hash where only the payload hash is stored. Later execution
will be verified using the hash. Only owners can create transactions and a transaction id (incremeting id) will be
assigned.
4. To approve or reject a transaction, other owners can call approve() or reject() with the transaction id.
5. If there are enough approvals, any owner can execute the transaction using the special MultisigTransaction type
with the transaction id if the full payload is already stored on chain or with the transaction payload if only a
hash is stored. Transaction execution will first check with this module that the transaction payload has gotten
enough signatures. If so, it will be executed as the multisig account. The owner who executes will pay for gas.
6. If there are enough rejections, any owner can finalize the rejection by calling execute_rejected_transaction().

Note that this multisig account model is not designed to use with a large number of owners. The more owners there
are, the more expensive voting on transactions will become. If a large number of owners is designed, such as in a
flat governance structure, clients are encouraged to write their own modules on top of this multisig account module
and implement the governance voting logic on top.


-  [Resource `MultisigAccount`](#0x1_multisig_account_MultisigAccount)
-  [Struct `MultisigTransaction`](#0x1_multisig_account_MultisigTransaction)
-  [Struct `ExecutionError`](#0x1_multisig_account_ExecutionError)
-  [Struct `MultisigAccountCreationMessage`](#0x1_multisig_account_MultisigAccountCreationMessage)
-  [Struct `MultisigAccountCreationWithAuthKeyRevocationMessage`](#0x1_multisig_account_MultisigAccountCreationWithAuthKeyRevocationMessage)
-  [Struct `AddOwnersEvent`](#0x1_multisig_account_AddOwnersEvent)
-  [Struct `AddOwners`](#0x1_multisig_account_AddOwners)
-  [Struct `RemoveOwnersEvent`](#0x1_multisig_account_RemoveOwnersEvent)
-  [Struct `RemoveOwners`](#0x1_multisig_account_RemoveOwners)
-  [Struct `UpdateSignaturesRequiredEvent`](#0x1_multisig_account_UpdateSignaturesRequiredEvent)
-  [Struct `UpdateSignaturesRequired`](#0x1_multisig_account_UpdateSignaturesRequired)
-  [Struct `CreateTransactionEvent`](#0x1_multisig_account_CreateTransactionEvent)
-  [Struct `CreateTransaction`](#0x1_multisig_account_CreateTransaction)
-  [Struct `VoteEvent`](#0x1_multisig_account_VoteEvent)
-  [Struct `Vote`](#0x1_multisig_account_Vote)
-  [Struct `ExecuteRejectedTransactionEvent`](#0x1_multisig_account_ExecuteRejectedTransactionEvent)
-  [Struct `ExecuteRejectedTransaction`](#0x1_multisig_account_ExecuteRejectedTransaction)
-  [Struct `TransactionExecutionSucceededEvent`](#0x1_multisig_account_TransactionExecutionSucceededEvent)
-  [Struct `TransactionExecutionSucceeded`](#0x1_multisig_account_TransactionExecutionSucceeded)
-  [Struct `TransactionExecutionFailedEvent`](#0x1_multisig_account_TransactionExecutionFailedEvent)
-  [Struct `TransactionExecutionFailed`](#0x1_multisig_account_TransactionExecutionFailed)
-  [Struct `MetadataUpdatedEvent`](#0x1_multisig_account_MetadataUpdatedEvent)
-  [Struct `MetadataUpdated`](#0x1_multisig_account_MetadataUpdated)
-  [Constants](#@Constants_0)
-  [Function `metadata`](#0x1_multisig_account_metadata)
-  [Function `num_signatures_required`](#0x1_multisig_account_num_signatures_required)
-  [Function `owners`](#0x1_multisig_account_owners)
-  [Function `is_owner`](#0x1_multisig_account_is_owner)
-  [Function `get_transaction`](#0x1_multisig_account_get_transaction)
-  [Function `get_pending_transactions`](#0x1_multisig_account_get_pending_transactions)
-  [Function `get_next_transaction_payload`](#0x1_multisig_account_get_next_transaction_payload)
-  [Function `can_be_executed`](#0x1_multisig_account_can_be_executed)
-  [Function `can_execute`](#0x1_multisig_account_can_execute)
-  [Function `can_be_rejected`](#0x1_multisig_account_can_be_rejected)
-  [Function `can_reject`](#0x1_multisig_account_can_reject)
-  [Function `get_next_multisig_account_address`](#0x1_multisig_account_get_next_multisig_account_address)
-  [Function `last_resolved_sequence_number`](#0x1_multisig_account_last_resolved_sequence_number)
-  [Function `next_sequence_number`](#0x1_multisig_account_next_sequence_number)
-  [Function `vote`](#0x1_multisig_account_vote)
-  [Function `available_transaction_queue_capacity`](#0x1_multisig_account_available_transaction_queue_capacity)
-  [Function `create_with_existing_account`](#0x1_multisig_account_create_with_existing_account)
-  [Function `create_with_existing_account_and_revoke_auth_key`](#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key)
-  [Function `create`](#0x1_multisig_account_create)
-  [Function `create_with_owners`](#0x1_multisig_account_create_with_owners)
-  [Function `create_with_owners_then_remove_bootstrapper`](#0x1_multisig_account_create_with_owners_then_remove_bootstrapper)
-  [Function `create_with_owners_internal`](#0x1_multisig_account_create_with_owners_internal)
-  [Function `add_owner`](#0x1_multisig_account_add_owner)
-  [Function `add_owners`](#0x1_multisig_account_add_owners)
-  [Function `add_owners_and_update_signatures_required`](#0x1_multisig_account_add_owners_and_update_signatures_required)
-  [Function `remove_owner`](#0x1_multisig_account_remove_owner)
-  [Function `remove_owners`](#0x1_multisig_account_remove_owners)
-  [Function `swap_owner`](#0x1_multisig_account_swap_owner)
-  [Function `swap_owners`](#0x1_multisig_account_swap_owners)
-  [Function `swap_owners_and_update_signatures_required`](#0x1_multisig_account_swap_owners_and_update_signatures_required)
-  [Function `update_signatures_required`](#0x1_multisig_account_update_signatures_required)
-  [Function `update_metadata`](#0x1_multisig_account_update_metadata)
-  [Function `update_metadata_internal`](#0x1_multisig_account_update_metadata_internal)
-  [Function `create_transaction`](#0x1_multisig_account_create_transaction)
-  [Function `create_transaction_with_hash`](#0x1_multisig_account_create_transaction_with_hash)
-  [Function `approve_transaction`](#0x1_multisig_account_approve_transaction)
-  [Function `reject_transaction`](#0x1_multisig_account_reject_transaction)
-  [Function `vote_transanction`](#0x1_multisig_account_vote_transanction)
-  [Function `vote_transaction`](#0x1_multisig_account_vote_transaction)
-  [Function `vote_transactions`](#0x1_multisig_account_vote_transactions)
-  [Function `execute_rejected_transaction`](#0x1_multisig_account_execute_rejected_transaction)
-  [Function `execute_rejected_transactions`](#0x1_multisig_account_execute_rejected_transactions)
-  [Function `validate_multisig_transaction`](#0x1_multisig_account_validate_multisig_transaction)
-  [Function `successful_transaction_execution_cleanup`](#0x1_multisig_account_successful_transaction_execution_cleanup)
-  [Function `failed_transaction_execution_cleanup`](#0x1_multisig_account_failed_transaction_execution_cleanup)
-  [Function `transaction_execution_cleanup_common`](#0x1_multisig_account_transaction_execution_cleanup_common)
-  [Function `remove_executed_transaction`](#0x1_multisig_account_remove_executed_transaction)
-  [Function `add_transaction`](#0x1_multisig_account_add_transaction)
-  [Function `create_multisig_account`](#0x1_multisig_account_create_multisig_account)
-  [Function `create_multisig_account_seed`](#0x1_multisig_account_create_multisig_account_seed)
-  [Function `validate_owners`](#0x1_multisig_account_validate_owners)
-  [Function `assert_is_owner_internal`](#0x1_multisig_account_assert_is_owner_internal)
-  [Function `assert_is_owner`](#0x1_multisig_account_assert_is_owner)
-  [Function `num_approvals_and_rejections_internal`](#0x1_multisig_account_num_approvals_and_rejections_internal)
-  [Function `num_approvals_and_rejections`](#0x1_multisig_account_num_approvals_and_rejections)
-  [Function `has_voted_for_approval`](#0x1_multisig_account_has_voted_for_approval)
-  [Function `has_voted_for_rejection`](#0x1_multisig_account_has_voted_for_rejection)
-  [Function `assert_multisig_account_exists`](#0x1_multisig_account_assert_multisig_account_exists)
-  [Function `assert_valid_sequence_number`](#0x1_multisig_account_assert_valid_sequence_number)
-  [Function `assert_transaction_exists`](#0x1_multisig_account_assert_transaction_exists)
-  [Function `update_owner_schema`](#0x1_multisig_account_update_owner_schema)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `metadata`](#@Specification_1_metadata)
    -  [Function `num_signatures_required`](#@Specification_1_num_signatures_required)
    -  [Function `owners`](#@Specification_1_owners)
    -  [Function `get_transaction`](#@Specification_1_get_transaction)
    -  [Function `get_next_transaction_payload`](#@Specification_1_get_next_transaction_payload)
    -  [Function `get_next_multisig_account_address`](#@Specification_1_get_next_multisig_account_address)
    -  [Function `last_resolved_sequence_number`](#@Specification_1_last_resolved_sequence_number)
    -  [Function `next_sequence_number`](#@Specification_1_next_sequence_number)
    -  [Function `vote`](#@Specification_1_vote)


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::bcs;
use 0x1::chain_id;
use 0x1::coin;
use 0x1::create_signer;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::hash;
use 0x1::option;
use 0x1::signer;
use 0x1::simple_map;
use 0x1::string;
use 0x1::table;
use 0x1::timestamp;
use 0x1::vector;
</code></pre>



<a id="0x1_multisig_account_MultisigAccount"></a>

## Resource `MultisigAccount`

Represents a multisig account's configurations and transactions.
This will be stored in the multisig account (created as a resource account separate from any owner accounts).


<pre><code>struct MultisigAccount has key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_signatures_required: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transactions: table::Table&lt;u64, multisig_account::MultisigTransaction&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>last_executed_sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>next_sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_cap: option::Option&lt;account::SignerCapability&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_owners_events: event::EventHandle&lt;multisig_account::AddOwnersEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>remove_owners_events: event::EventHandle&lt;multisig_account::RemoveOwnersEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_signature_required_events: event::EventHandle&lt;multisig_account::UpdateSignaturesRequiredEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_transaction_events: event::EventHandle&lt;multisig_account::CreateTransactionEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: event::EventHandle&lt;multisig_account::VoteEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execute_rejected_transaction_events: event::EventHandle&lt;multisig_account::ExecuteRejectedTransactionEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execute_transaction_events: event::EventHandle&lt;multisig_account::TransactionExecutionSucceededEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_execution_failed_events: event::EventHandle&lt;multisig_account::TransactionExecutionFailedEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata_updated_events: event::EventHandle&lt;multisig_account::MetadataUpdatedEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MultisigTransaction"></a>

## Struct `MultisigTransaction`

A transaction to be executed in a multisig account.
This must contain either the full transaction payload or its hash (stored as bytes).


<pre><code>struct MultisigTransaction has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payload: option::Option&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>payload_hash: option::Option&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>votes: simple_map::SimpleMap&lt;address, bool&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>creation_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_ExecutionError"></a>

## Struct `ExecutionError`

Contains information about execution failure.


<pre><code>struct ExecutionError has copy, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>abort_location: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>error_type: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>error_code: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MultisigAccountCreationMessage"></a>

## Struct `MultisigAccountCreationMessage`

Used only for verifying multisig account creation on top of existing accounts.


<pre><code>struct MultisigAccountCreationMessage has copy, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chain_id: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>account_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owners: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_signatures_required: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MultisigAccountCreationWithAuthKeyRevocationMessage"></a>

## Struct `MultisigAccountCreationWithAuthKeyRevocationMessage`

Used only for verifying multisig account creation on top of existing accounts and rotating the auth key to 0x0.


<pre><code>struct MultisigAccountCreationWithAuthKeyRevocationMessage has copy, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chain_id: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>account_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owners: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_signatures_required: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_AddOwnersEvent"></a>

## Struct `AddOwnersEvent`

Event emitted when new owners are added to the multisig account.


<pre><code>struct AddOwnersEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners_added: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_AddOwners"></a>

## Struct `AddOwners`



<pre><code>&#35;[event]
struct AddOwners has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>owners_added: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_RemoveOwnersEvent"></a>

## Struct `RemoveOwnersEvent`

Event emitted when new owners are removed from the multisig account.


<pre><code>struct RemoveOwnersEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners_removed: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_RemoveOwners"></a>

## Struct `RemoveOwners`



<pre><code>&#35;[event]
struct RemoveOwners has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>owners_removed: vector&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_UpdateSignaturesRequiredEvent"></a>

## Struct `UpdateSignaturesRequiredEvent`

Event emitted when the number of signatures required is updated.


<pre><code>struct UpdateSignaturesRequiredEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_num_signatures_required: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_num_signatures_required: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_UpdateSignaturesRequired"></a>

## Struct `UpdateSignaturesRequired`



<pre><code>&#35;[event]
struct UpdateSignaturesRequired has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_num_signatures_required: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_num_signatures_required: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_CreateTransactionEvent"></a>

## Struct `CreateTransactionEvent`

Event emitted when a transaction is created.


<pre><code>struct CreateTransactionEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction: multisig_account::MultisigTransaction</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_CreateTransaction"></a>

## Struct `CreateTransaction`



<pre><code>&#35;[event]
struct CreateTransaction has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction: multisig_account::MultisigTransaction</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_VoteEvent"></a>

## Struct `VoteEvent`

Event emitted when an owner approves or rejects a transaction.


<pre><code>struct VoteEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>approved: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_Vote"></a>

## Struct `Vote`



<pre><code>&#35;[event]
struct Vote has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>owner: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>approved: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_ExecuteRejectedTransactionEvent"></a>

## Struct `ExecuteRejectedTransactionEvent`

Event emitted when a transaction is officially rejected because the number of rejections has reached the
number of signatures required.


<pre><code>struct ExecuteRejectedTransactionEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>num_rejections: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_ExecuteRejectedTransaction"></a>

## Struct `ExecuteRejectedTransaction`



<pre><code>&#35;[event]
struct ExecuteRejectedTransaction has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>num_rejections: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionSucceededEvent"></a>

## Struct `TransactionExecutionSucceededEvent`

Event emitted when a transaction is executed.


<pre><code>struct TransactionExecutionSucceededEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionSucceeded"></a>

## Struct `TransactionExecutionSucceeded`



<pre><code>&#35;[event]
struct TransactionExecutionSucceeded has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionFailedEvent"></a>

## Struct `TransactionExecutionFailedEvent`

Event emitted when a transaction's execution failed.


<pre><code>struct TransactionExecutionFailedEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_error: multisig_account::ExecutionError</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionFailed"></a>

## Struct `TransactionExecutionFailed`



<pre><code>&#35;[event]
struct TransactionExecutionFailed has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>executor: address</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_error: multisig_account::ExecutionError</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MetadataUpdatedEvent"></a>

## Struct `MetadataUpdatedEvent`

Event emitted when a transaction's metadata is updated.


<pre><code>struct MetadataUpdatedEvent has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MetadataUpdated"></a>

## Struct `MetadataUpdated`



<pre><code>&#35;[event]
struct MetadataUpdated has drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_metadata: simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_multisig_account_ZERO_AUTH_KEY"></a>



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_multisig_account_DOMAIN_SEPARATOR"></a>

The salt used to create a resource account during multisig account creation.
This is used to avoid conflicts with other modules that also create resource accounts with the same owner
account.


<pre><code>const DOMAIN_SEPARATOR: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 109, 117, 108, 116, 105, 115, 105, 103, 95, 97, 99, 99, 111, 117, 110, 116];
</code></pre>



<a id="0x1_multisig_account_EACCOUNT_NOT_MULTISIG"></a>

Specified account is not a multisig account.


<pre><code>const EACCOUNT_NOT_MULTISIG: u64 &#61; 2002;
</code></pre>



<a id="0x1_multisig_account_EDUPLICATE_METADATA_KEY"></a>

The specified metadata contains duplicate attributes (keys).


<pre><code>const EDUPLICATE_METADATA_KEY: u64 &#61; 16;
</code></pre>



<a id="0x1_multisig_account_EDUPLICATE_OWNER"></a>

Owner list cannot contain the same address more than once.


<pre><code>const EDUPLICATE_OWNER: u64 &#61; 1;
</code></pre>



<a id="0x1_multisig_account_EINVALID_PAYLOAD_HASH"></a>

Payload hash must be exactly 32 bytes (sha3-256).


<pre><code>const EINVALID_PAYLOAD_HASH: u64 &#61; 12;
</code></pre>



<a id="0x1_multisig_account_EINVALID_SEQUENCE_NUMBER"></a>

The sequence number provided is invalid. It must be between [1, next pending transaction - 1].


<pre><code>const EINVALID_SEQUENCE_NUMBER: u64 &#61; 17;
</code></pre>



<a id="0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED"></a>

Number of signatures required must be more than zero and at most the total number of owners.


<pre><code>const EINVALID_SIGNATURES_REQUIRED: u64 &#61; 11;
</code></pre>



<a id="0x1_multisig_account_EMAX_PENDING_TRANSACTIONS_EXCEEDED"></a>

The number of pending transactions has exceeded the maximum allowed.


<pre><code>const EMAX_PENDING_TRANSACTIONS_EXCEEDED: u64 &#61; 19;
</code></pre>



<a id="0x1_multisig_account_EMULTISIG_ACCOUNTS_NOT_ENABLED_YET"></a>

Multisig accounts has not been enabled on this current network yet.


<pre><code>const EMULTISIG_ACCOUNTS_NOT_ENABLED_YET: u64 &#61; 14;
</code></pre>



<a id="0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED"></a>

The multisig v2 enhancement feature is not enabled.


<pre><code>const EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED: u64 &#61; 20;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_APPROVALS"></a>

Transaction has not received enough approvals to be executed.


<pre><code>const ENOT_ENOUGH_APPROVALS: u64 &#61; 2009;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_OWNERS"></a>

Multisig account must have at least one owner.


<pre><code>const ENOT_ENOUGH_OWNERS: u64 &#61; 5;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_REJECTIONS"></a>

Transaction has not received enough rejections to be officially rejected.


<pre><code>const ENOT_ENOUGH_REJECTIONS: u64 &#61; 10;
</code></pre>



<a id="0x1_multisig_account_ENOT_OWNER"></a>

Account executing this operation is not an owner of the multisig account.


<pre><code>const ENOT_OWNER: u64 &#61; 2003;
</code></pre>



<a id="0x1_multisig_account_ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH"></a>

The number of metadata keys and values don't match.


<pre><code>const ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH: u64 &#61; 15;
</code></pre>



<a id="0x1_multisig_account_EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP"></a>

Provided owners to remove and new owners overlap.


<pre><code>const EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP: u64 &#61; 18;
</code></pre>



<a id="0x1_multisig_account_EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF"></a>

The multisig account itself cannot be an owner.


<pre><code>const EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF: u64 &#61; 13;
</code></pre>



<a id="0x1_multisig_account_EPAYLOAD_CANNOT_BE_EMPTY"></a>

Transaction payload cannot be empty.


<pre><code>const EPAYLOAD_CANNOT_BE_EMPTY: u64 &#61; 4;
</code></pre>



<a id="0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH_HASH"></a>

Provided target function does not match the hash stored in the on-chain transaction.


<pre><code>const EPAYLOAD_DOES_NOT_MATCH_HASH: u64 &#61; 2008;
</code></pre>



<a id="0x1_multisig_account_ETRANSACTION_NOT_FOUND"></a>

Transaction with specified id cannot be found.


<pre><code>const ETRANSACTION_NOT_FOUND: u64 &#61; 2006;
</code></pre>



<a id="0x1_multisig_account_MAX_PENDING_TRANSACTIONS"></a>



<pre><code>const MAX_PENDING_TRANSACTIONS: u64 &#61; 20;
</code></pre>



<a id="0x1_multisig_account_metadata"></a>

## Function `metadata`

Return the multisig account's metadata.


<pre><code>&#35;[view]
public fun metadata(multisig_account: address): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun metadata(multisig_account: address): SimpleMap&lt;String, vector&lt;u8&gt;&gt; acquires MultisigAccount &#123;
    borrow_global&lt;MultisigAccount&gt;(multisig_account).metadata
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_num_signatures_required"></a>

## Function `num_signatures_required`

Return the number of signatures required to execute or execute-reject a transaction in the provided
multisig account.


<pre><code>&#35;[view]
public fun num_signatures_required(multisig_account: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun num_signatures_required(multisig_account: address): u64 acquires MultisigAccount &#123;
    borrow_global&lt;MultisigAccount&gt;(multisig_account).num_signatures_required
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_owners"></a>

## Function `owners`

Return a vector of all of the provided multisig account's owners.


<pre><code>&#35;[view]
public fun owners(multisig_account: address): vector&lt;address&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owners(multisig_account: address): vector&lt;address&gt; acquires MultisigAccount &#123;
    borrow_global&lt;MultisigAccount&gt;(multisig_account).owners
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_is_owner"></a>

## Function `is_owner`

Return true if the provided owner is an owner of the provided multisig account.


<pre><code>&#35;[view]
public fun is_owner(owner: address, multisig_account: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_owner(owner: address, multisig_account: address): bool acquires MultisigAccount &#123;
    vector::contains(&amp;borrow_global&lt;MultisigAccount&gt;(multisig_account).owners, &amp;owner)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_get_transaction"></a>

## Function `get_transaction`

Return the transaction with the given transaction id.


<pre><code>&#35;[view]
public fun get_transaction(multisig_account: address, sequence_number: u64): multisig_account::MultisigTransaction
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_transaction(
    multisig_account: address,
    sequence_number: u64,
): MultisigTransaction acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    assert!(
        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,
        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),
    );
    &#42;table::borrow(&amp;multisig_account_resource.transactions, sequence_number)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_get_pending_transactions"></a>

## Function `get_pending_transactions`

Return all pending transactions.


<pre><code>&#35;[view]
public fun get_pending_transactions(multisig_account: address): vector&lt;multisig_account::MultisigTransaction&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_pending_transactions(
    multisig_account: address
): vector&lt;MultisigTransaction&gt; acquires MultisigAccount &#123;
    let pending_transactions: vector&lt;MultisigTransaction&gt; &#61; vector[];
    let multisig_account &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    let i &#61; multisig_account.last_executed_sequence_number &#43; 1;
    let next_sequence_number &#61; multisig_account.next_sequence_number;
    while (i &lt; next_sequence_number) &#123;
        vector::push_back(&amp;mut pending_transactions, &#42;table::borrow(&amp;multisig_account.transactions, i));
        i &#61; i &#43; 1;
    &#125;;
    pending_transactions
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_get_next_transaction_payload"></a>

## Function `get_next_transaction_payload`

Return the payload for the next transaction in the queue.


<pre><code>&#35;[view]
public fun get_next_transaction_payload(multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_next_transaction_payload(
    multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt; acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;
    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);

    if (option::is_some(&amp;transaction.payload)) &#123;
        &#42;option::borrow(&amp;transaction.payload)
    &#125; else &#123;
        provided_payload
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_can_be_executed"></a>

## Function `can_be_executed`

Return true if the transaction with given transaction id can be executed now.


<pre><code>&#35;[view]
public fun can_be_executed(multisig_account: address, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_executed(multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;
    assert_valid_sequence_number(multisig_account, sequence_number);
    let (num_approvals, _) &#61; num_approvals_and_rejections(multisig_account, sequence_number);
    sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;
        num_approvals &gt;&#61; num_signatures_required(multisig_account)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_can_execute"></a>

## Function `can_execute`

Return true if the owner can execute the transaction with given transaction id now.


<pre><code>&#35;[view]
public fun can_execute(owner: address, multisig_account: address, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_execute(owner: address, multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;
    assert_valid_sequence_number(multisig_account, sequence_number);
    let (num_approvals, _) &#61; num_approvals_and_rejections(multisig_account, sequence_number);
    if (!has_voted_for_approval(multisig_account, sequence_number, owner)) &#123;
        num_approvals &#61; num_approvals &#43; 1;
    &#125;;
    is_owner(owner, multisig_account) &amp;&amp;
        sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;
        num_approvals &gt;&#61; num_signatures_required(multisig_account)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_can_be_rejected"></a>

## Function `can_be_rejected`

Return true if the transaction with given transaction id can be officially rejected.


<pre><code>&#35;[view]
public fun can_be_rejected(multisig_account: address, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_rejected(multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;
    assert_valid_sequence_number(multisig_account, sequence_number);
    let (_, num_rejections) &#61; num_approvals_and_rejections(multisig_account, sequence_number);
    sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;
        num_rejections &gt;&#61; num_signatures_required(multisig_account)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_can_reject"></a>

## Function `can_reject`

Return true if the owner can execute the "rejected" transaction with given transaction id now.


<pre><code>&#35;[view]
public fun can_reject(owner: address, multisig_account: address, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_reject(owner: address, multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;
    assert_valid_sequence_number(multisig_account, sequence_number);
    let (_, num_rejections) &#61; num_approvals_and_rejections(multisig_account, sequence_number);
    if (!has_voted_for_rejection(multisig_account, sequence_number, owner)) &#123;
        num_rejections &#61; num_rejections &#43; 1;
    &#125;;
    is_owner(owner, multisig_account) &amp;&amp;
        sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;
        num_rejections &gt;&#61; num_signatures_required(multisig_account)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_get_next_multisig_account_address"></a>

## Function `get_next_multisig_account_address`

Return the predicted address for the next multisig account if created from the given creator address.


<pre><code>&#35;[view]
public fun get_next_multisig_account_address(creator: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_next_multisig_account_address(creator: address): address &#123;
    let owner_nonce &#61; account::get_sequence_number(creator);
    create_resource_address(&amp;creator, create_multisig_account_seed(to_bytes(&amp;owner_nonce)))
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_last_resolved_sequence_number"></a>

## Function `last_resolved_sequence_number`

Return the id of the last transaction that was executed (successful or failed) or removed.


<pre><code>&#35;[view]
public fun last_resolved_sequence_number(multisig_account: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun last_resolved_sequence_number(multisig_account: address): u64 acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    multisig_account_resource.last_executed_sequence_number
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_next_sequence_number"></a>

## Function `next_sequence_number`

Return the id of the next transaction created.


<pre><code>&#35;[view]
public fun next_sequence_number(multisig_account: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_sequence_number(multisig_account: address): u64 acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    multisig_account_resource.next_sequence_number
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_vote"></a>

## Function `vote`

Return a bool tuple indicating whether an owner has voted and if so, whether they voted yes or no.


<pre><code>&#35;[view]
public fun vote(multisig_account: address, sequence_number: u64, owner: address): (bool, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vote(
    multisig_account: address, sequence_number: u64, owner: address): (bool, bool) acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    assert!(
        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,
        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),
    );
    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);
    let votes &#61; &amp;transaction.votes;
    let voted &#61; simple_map::contains_key(votes, &amp;owner);
    let vote &#61; voted &amp;&amp; &#42;simple_map::borrow(votes, &amp;owner);
    (voted, vote)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_available_transaction_queue_capacity"></a>

## Function `available_transaction_queue_capacity`



<pre><code>&#35;[view]
public fun available_transaction_queue_capacity(multisig_account: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun available_transaction_queue_capacity(multisig_account: address): u64 acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    let num_pending_transactions &#61; multisig_account_resource.next_sequence_number &#45; multisig_account_resource.last_executed_sequence_number &#45; 1;
    if (num_pending_transactions &gt; MAX_PENDING_TRANSACTIONS) &#123;
        0
    &#125; else &#123;
        MAX_PENDING_TRANSACTIONS &#45; num_pending_transactions
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account"></a>

## Function `create_with_existing_account`

Creates a new multisig account on top of an existing account.

This offers a migration path for an existing account with a multi-ed25519 auth key (native multisig account).
In order to ensure a malicious module cannot obtain backdoor control over an existing account, a signed message
with a valid signature from the account's auth key is required.

Note that this does not revoke auth key-based control over the account. Owners should separately rotate the auth
key after they are fully migrated to the new multisig account. Alternatively, they can call
create_with_existing_account_and_revoke_auth_key instead.


<pre><code>public entry fun create_with_existing_account(multisig_address: address, owners: vector&lt;address&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: vector&lt;u8&gt;, create_multisig_account_signed_message: vector&lt;u8&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_existing_account(
    multisig_address: address,
    owners: vector&lt;address&gt;,
    num_signatures_required: u64,
    account_scheme: u8,
    account_public_key: vector&lt;u8&gt;,
    create_multisig_account_signed_message: vector&lt;u8&gt;,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    // Verify that the `MultisigAccountCreationMessage` has the right information and is signed by the account
    // owner&apos;s key.
    let proof_challenge &#61; MultisigAccountCreationMessage &#123;
        chain_id: chain_id::get(),
        account_address: multisig_address,
        sequence_number: account::get_sequence_number(multisig_address),
        owners,
        num_signatures_required,
    &#125;;
    account::verify_signed_message(
        multisig_address,
        account_scheme,
        account_public_key,
        create_multisig_account_signed_message,
        proof_challenge,
    );

    // We create the signer for the multisig account here since this is required to add the MultisigAccount resource
    // This should be safe and authorized because we have verified the signed message from the existing account
    // that authorizes creating a multisig account with the specified owners and signature threshold.
    let multisig_account &#61; &amp;create_signer(multisig_address);
    create_with_owners_internal(
        multisig_account,
        owners,
        num_signatures_required,
        option::none&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account_and_revoke_auth_key"></a>

## Function `create_with_existing_account_and_revoke_auth_key`

Creates a new multisig account on top of an existing account and immediately rotate the origin auth key to 0x0.

Note: If the original account is a resource account, this does not revoke all control over it as if any
SignerCapability of the resource account still exists, it can still be used to generate the signer for the
account.


<pre><code>public entry fun create_with_existing_account_and_revoke_auth_key(multisig_address: address, owners: vector&lt;address&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: vector&lt;u8&gt;, create_multisig_account_signed_message: vector&lt;u8&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_existing_account_and_revoke_auth_key(
    multisig_address: address,
    owners: vector&lt;address&gt;,
    num_signatures_required: u64,
    account_scheme: u8,
    account_public_key: vector&lt;u8&gt;,
    create_multisig_account_signed_message: vector&lt;u8&gt;,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    // Verify that the `MultisigAccountCreationMessage` has the right information and is signed by the account
    // owner&apos;s key.
    let proof_challenge &#61; MultisigAccountCreationWithAuthKeyRevocationMessage &#123;
        chain_id: chain_id::get(),
        account_address: multisig_address,
        sequence_number: account::get_sequence_number(multisig_address),
        owners,
        num_signatures_required,
    &#125;;
    account::verify_signed_message(
        multisig_address,
        account_scheme,
        account_public_key,
        create_multisig_account_signed_message,
        proof_challenge,
    );

    // We create the signer for the multisig account here since this is required to add the MultisigAccount resource
    // This should be safe and authorized because we have verified the signed message from the existing account
    // that authorizes creating a multisig account with the specified owners and signature threshold.
    let multisig_account &#61; &amp;create_signer(multisig_address);
    create_with_owners_internal(
        multisig_account,
        owners,
        num_signatures_required,
        option::none&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );

    // Rotate the account&apos;s auth key to 0x0, which effectively revokes control via auth key.
    let multisig_address &#61; address_of(multisig_account);
    account::rotate_authentication_key_internal(multisig_account, ZERO_AUTH_KEY);
    // This also needs to revoke any signer capability or rotation capability that exists for the account to
    // completely remove all access to the account.
    if (account::is_signer_capability_offered(multisig_address)) &#123;
        account::revoke_any_signer_capability(multisig_account);
    &#125;;
    if (account::is_rotation_capability_offered(multisig_address)) &#123;
        account::revoke_any_rotation_capability(multisig_account);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create"></a>

## Function `create`

Creates a new multisig account and add the signer as a single owner.


<pre><code>public entry fun create(owner: &amp;signer, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create(
    owner: &amp;signer,
    num_signatures_required: u64,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    create_with_owners(owner, vector[], num_signatures_required, metadata_keys, metadata_values);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners"></a>

## Function `create_with_owners`

Creates a new multisig account with the specified additional owner list and signatures required.

@param additional_owners The owner account who calls this function cannot be in the additional_owners and there
cannot be any duplicate owners in the list.
@param num_signatures_required The number of signatures required to execute a transaction. Must be at least 1 and
at most the total number of owners.


<pre><code>public entry fun create_with_owners(owner: &amp;signer, additional_owners: vector&lt;address&gt;, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_owners(
    owner: &amp;signer,
    additional_owners: vector&lt;address&gt;,
    num_signatures_required: u64,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    let (multisig_account, multisig_signer_cap) &#61; create_multisig_account(owner);
    vector::push_back(&amp;mut additional_owners, address_of(owner));
    create_with_owners_internal(
        &amp;multisig_account,
        additional_owners,
        num_signatures_required,
        option::some(multisig_signer_cap),
        metadata_keys,
        metadata_values,
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_then_remove_bootstrapper"></a>

## Function `create_with_owners_then_remove_bootstrapper`

Like <code>create_with_owners</code>, but removes the calling account after creation.

This is for creating a vanity multisig account from a bootstrapping account that should not
be an owner after the vanity multisig address has been secured.


<pre><code>public entry fun create_with_owners_then_remove_bootstrapper(bootstrapper: &amp;signer, owners: vector&lt;address&gt;, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_owners_then_remove_bootstrapper(
    bootstrapper: &amp;signer,
    owners: vector&lt;address&gt;,
    num_signatures_required: u64,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    let bootstrapper_address &#61; address_of(bootstrapper);
    create_with_owners(
        bootstrapper,
        owners,
        num_signatures_required,
        metadata_keys,
        metadata_values
    );
    update_owner_schema(
        get_next_multisig_account_address(bootstrapper_address),
        vector[],
        vector[bootstrapper_address],
        option::none()
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_internal"></a>

## Function `create_with_owners_internal`



<pre><code>fun create_with_owners_internal(multisig_account: &amp;signer, owners: vector&lt;address&gt;, num_signatures_required: u64, multisig_account_signer_cap: option::Option&lt;account::SignerCapability&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_with_owners_internal(
    multisig_account: &amp;signer,
    owners: vector&lt;address&gt;,
    num_signatures_required: u64,
    multisig_account_signer_cap: Option&lt;SignerCapability&gt;,
    metadata_keys: vector&lt;String&gt;,
    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,
) acquires MultisigAccount &#123;
    assert!(features::multisig_accounts_enabled(), error::unavailable(EMULTISIG_ACCOUNTS_NOT_ENABLED_YET));
    assert!(
        num_signatures_required &gt; 0 &amp;&amp; num_signatures_required &lt;&#61; vector::length(&amp;owners),
        error::invalid_argument(EINVALID_SIGNATURES_REQUIRED),
    );

    let multisig_address &#61; address_of(multisig_account);
    validate_owners(&amp;owners, multisig_address);
    move_to(multisig_account, MultisigAccount &#123;
        owners,
        num_signatures_required,
        transactions: table::new&lt;u64, MultisigTransaction&gt;(),
        metadata: simple_map::create&lt;String, vector&lt;u8&gt;&gt;(),
        // First transaction will start at id 1 instead of 0.
        last_executed_sequence_number: 0,
        next_sequence_number: 1,
        signer_cap: multisig_account_signer_cap,
        add_owners_events: new_event_handle&lt;AddOwnersEvent&gt;(multisig_account),
        remove_owners_events: new_event_handle&lt;RemoveOwnersEvent&gt;(multisig_account),
        update_signature_required_events: new_event_handle&lt;UpdateSignaturesRequiredEvent&gt;(multisig_account),
        create_transaction_events: new_event_handle&lt;CreateTransactionEvent&gt;(multisig_account),
        vote_events: new_event_handle&lt;VoteEvent&gt;(multisig_account),
        execute_rejected_transaction_events: new_event_handle&lt;ExecuteRejectedTransactionEvent&gt;(multisig_account),
        execute_transaction_events: new_event_handle&lt;TransactionExecutionSucceededEvent&gt;(multisig_account),
        transaction_execution_failed_events: new_event_handle&lt;TransactionExecutionFailedEvent&gt;(multisig_account),
        metadata_updated_events: new_event_handle&lt;MetadataUpdatedEvent&gt;(multisig_account),
    &#125;);

    update_metadata_internal(multisig_account, metadata_keys, metadata_values, false);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_add_owner"></a>

## Function `add_owner`

Similar to add_owners, but only allow adding one owner.


<pre><code>entry fun add_owner(multisig_account: &amp;signer, new_owner: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owner(multisig_account: &amp;signer, new_owner: address) acquires MultisigAccount &#123;
    add_owners(multisig_account, vector[new_owner]);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_add_owners"></a>

## Function `add_owners`

Add new owners to the multisig account. This can only be invoked by the multisig account itself, through the
proposal flow.

Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the owners list.


<pre><code>entry fun add_owners(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owners(
    multisig_account: &amp;signer, new_owners: vector&lt;address&gt;) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        new_owners,
        vector[],
        option::none()
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_add_owners_and_update_signatures_required"></a>

## Function `add_owners_and_update_signatures_required`

Add owners then update number of signatures required, in a single operation.


<pre><code>entry fun add_owners_and_update_signatures_required(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owners_and_update_signatures_required(
    multisig_account: &amp;signer,
    new_owners: vector&lt;address&gt;,
    new_num_signatures_required: u64
) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        new_owners,
        vector[],
        option::some(new_num_signatures_required)
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_remove_owner"></a>

## Function `remove_owner`

Similar to remove_owners, but only allow removing one owner.


<pre><code>entry fun remove_owner(multisig_account: &amp;signer, owner_to_remove: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun remove_owner(
    multisig_account: &amp;signer, owner_to_remove: address) acquires MultisigAccount &#123;
    remove_owners(multisig_account, vector[owner_to_remove]);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_remove_owners"></a>

## Function `remove_owners`

Remove owners from the multisig account. This can only be invoked by the multisig account itself, through the
proposal flow.

This function skips any owners who are not in the multisig account's list of owners.
Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the owners list.


<pre><code>entry fun remove_owners(multisig_account: &amp;signer, owners_to_remove: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun remove_owners(
    multisig_account: &amp;signer, owners_to_remove: vector&lt;address&gt;) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        vector[],
        owners_to_remove,
        option::none()
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owner"></a>

## Function `swap_owner`

Swap an owner in for an old one, without changing required signatures.


<pre><code>entry fun swap_owner(multisig_account: &amp;signer, to_swap_in: address, to_swap_out: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owner(
    multisig_account: &amp;signer,
    to_swap_in: address,
    to_swap_out: address
) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        vector[to_swap_in],
        vector[to_swap_out],
        option::none()
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owners"></a>

## Function `swap_owners`

Swap owners in and out, without changing required signatures.


<pre><code>entry fun swap_owners(multisig_account: &amp;signer, to_swap_in: vector&lt;address&gt;, to_swap_out: vector&lt;address&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owners(
    multisig_account: &amp;signer,
    to_swap_in: vector&lt;address&gt;,
    to_swap_out: vector&lt;address&gt;
) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        to_swap_in,
        to_swap_out,
        option::none()
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owners_and_update_signatures_required"></a>

## Function `swap_owners_and_update_signatures_required`

Swap owners in and out, updating number of required signatures.


<pre><code>entry fun swap_owners_and_update_signatures_required(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;, owners_to_remove: vector&lt;address&gt;, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owners_and_update_signatures_required(
    multisig_account: &amp;signer,
    new_owners: vector&lt;address&gt;,
    owners_to_remove: vector&lt;address&gt;,
    new_num_signatures_required: u64
) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        new_owners,
        owners_to_remove,
        option::some(new_num_signatures_required)
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_update_signatures_required"></a>

## Function `update_signatures_required`

Update the number of signatures required to execute transaction in the specified multisig account.

This can only be invoked by the multisig account itself, through the proposal flow.
Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the number of signatures required.


<pre><code>entry fun update_signatures_required(multisig_account: &amp;signer, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun update_signatures_required(
    multisig_account: &amp;signer, new_num_signatures_required: u64) acquires MultisigAccount &#123;
    update_owner_schema(
        address_of(multisig_account),
        vector[],
        vector[],
        option::some(new_num_signatures_required)
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_update_metadata"></a>

## Function `update_metadata`

Allow the multisig account to update its own metadata. Note that this overrides the entire existing metadata.
If any attributes are not specified in the metadata, they will be removed!

This can only be invoked by the multisig account itself, through the proposal flow.
Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the number of signatures required.


<pre><code>entry fun update_metadata(multisig_account: &amp;signer, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun update_metadata(
    multisig_account: &amp;signer, keys: vector&lt;String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;) acquires MultisigAccount &#123;
    update_metadata_internal(multisig_account, keys, values, true);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_update_metadata_internal"></a>

## Function `update_metadata_internal`



<pre><code>fun update_metadata_internal(multisig_account: &amp;signer, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, emit_event: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_metadata_internal(
    multisig_account: &amp;signer,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    emit_event: bool,
) acquires MultisigAccount &#123;
    let num_attributes &#61; vector::length(&amp;keys);
    assert!(
        num_attributes &#61;&#61; vector::length(&amp;values),
        error::invalid_argument(ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH),
    );

    let multisig_address &#61; address_of(multisig_account);
    assert_multisig_account_exists(multisig_address);
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_address);
    let old_metadata &#61; multisig_account_resource.metadata;
    multisig_account_resource.metadata &#61; simple_map::create&lt;String, vector&lt;u8&gt;&gt;();
    let metadata &#61; &amp;mut multisig_account_resource.metadata;
    let i &#61; 0;
    while (i &lt; num_attributes) &#123;
        let key &#61; &#42;vector::borrow(&amp;keys, i);
        let value &#61; &#42;vector::borrow(&amp;values, i);
        assert!(
            !simple_map::contains_key(metadata, &amp;key),
            error::invalid_argument(EDUPLICATE_METADATA_KEY),
        );

        simple_map::add(metadata, key, value);
        i &#61; i &#43; 1;
    &#125;;

    if (emit_event) &#123;
        if (std::features::module_event_migration_enabled()) &#123;
            emit(
                MetadataUpdated &#123;
                    multisig_account: multisig_address,
                    old_metadata,
                    new_metadata: multisig_account_resource.metadata,
                &#125;
            )
        &#125;;
        emit_event(
            &amp;mut multisig_account_resource.metadata_updated_events,
            MetadataUpdatedEvent &#123;
                old_metadata,
                new_metadata: multisig_account_resource.metadata,
            &#125;
        );
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_transaction"></a>

## Function `create_transaction`

Create a multisig transaction, which will have one approval initially (from the creator).


<pre><code>public entry fun create_transaction(owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_transaction(
    owner: &amp;signer,
    multisig_account: address,
    payload: vector&lt;u8&gt;,
) acquires MultisigAccount &#123;
    assert!(vector::length(&amp;payload) &gt; 0, error::invalid_argument(EPAYLOAD_CANNOT_BE_EMPTY));

    assert_multisig_account_exists(multisig_account);
    assert_is_owner(owner, multisig_account);

    let creator &#61; address_of(owner);
    let transaction &#61; MultisigTransaction &#123;
        payload: option::some(payload),
        payload_hash: option::none&lt;vector&lt;u8&gt;&gt;(),
        votes: simple_map::create&lt;address, bool&gt;(),
        creator,
        creation_time_secs: now_seconds(),
    &#125;;
    add_transaction(creator, multisig_account, transaction);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_transaction_with_hash"></a>

## Function `create_transaction_with_hash`

Create a multisig transaction with a transaction hash instead of the full payload.
This means the payload will be stored off chain for gas saving. Later, during execution, the executor will need
to provide the full payload, which will be validated against the hash stored on-chain.


<pre><code>public entry fun create_transaction_with_hash(owner: &amp;signer, multisig_account: address, payload_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_transaction_with_hash(
    owner: &amp;signer,
    multisig_account: address,
    payload_hash: vector&lt;u8&gt;,
) acquires MultisigAccount &#123;
    // Payload hash is a sha3&#45;256 hash, so it must be exactly 32 bytes.
    assert!(vector::length(&amp;payload_hash) &#61;&#61; 32, error::invalid_argument(EINVALID_PAYLOAD_HASH));

    assert_multisig_account_exists(multisig_account);
    assert_is_owner(owner, multisig_account);

    let creator &#61; address_of(owner);
    let transaction &#61; MultisigTransaction &#123;
        payload: option::none&lt;vector&lt;u8&gt;&gt;(),
        payload_hash: option::some(payload_hash),
        votes: simple_map::create&lt;address, bool&gt;(),
        creator,
        creation_time_secs: now_seconds(),
    &#125;;
    add_transaction(creator, multisig_account, transaction);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_approve_transaction"></a>

## Function `approve_transaction`

Approve a multisig transaction.


<pre><code>public entry fun approve_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun approve_transaction(
    owner: &amp;signer, multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;
    vote_transanction(owner, multisig_account, sequence_number, true);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_reject_transaction"></a>

## Function `reject_transaction`

Reject a multisig transaction.


<pre><code>public entry fun reject_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reject_transaction(
    owner: &amp;signer, multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;
    vote_transanction(owner, multisig_account, sequence_number, false);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transanction"></a>

## Function `vote_transanction`

Generic function that can be used to either approve or reject a multisig transaction
Retained for backward compatibility: the function with the typographical error in its name
will continue to be an accessible entry point.


<pre><code>public entry fun vote_transanction(owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transanction(
    owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool) acquires MultisigAccount &#123;
    assert_multisig_account_exists(multisig_account);
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    assert_is_owner_internal(owner, multisig_account_resource);

    assert!(
        table::contains(&amp;multisig_account_resource.transactions, sequence_number),
        error::not_found(ETRANSACTION_NOT_FOUND),
    );
    let transaction &#61; table::borrow_mut(&amp;mut multisig_account_resource.transactions, sequence_number);
    let votes &#61; &amp;mut transaction.votes;
    let owner_addr &#61; address_of(owner);

    if (simple_map::contains_key(votes, &amp;owner_addr)) &#123;
        &#42;simple_map::borrow_mut(votes, &amp;owner_addr) &#61; approved;
    &#125; else &#123;
        simple_map::add(votes, owner_addr, approved);
    &#125;;

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            Vote &#123;
                multisig_account,
                owner: owner_addr,
                sequence_number,
                approved,
            &#125;
        );
    &#125;;
    emit_event(
        &amp;mut multisig_account_resource.vote_events,
        VoteEvent &#123;
            owner: owner_addr,
            sequence_number,
            approved,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transaction"></a>

## Function `vote_transaction`

Generic function that can be used to either approve or reject a multisig transaction


<pre><code>public entry fun vote_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transaction(
    owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool) acquires MultisigAccount &#123;
    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));
    vote_transanction(owner, multisig_account, sequence_number, approved);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transactions"></a>

## Function `vote_transactions`

Generic function that can be used to either approve or reject a batch of transactions within a specified range.


<pre><code>public entry fun vote_transactions(owner: &amp;signer, multisig_account: address, starting_sequence_number: u64, final_sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transactions(
    owner: &amp;signer, multisig_account: address, starting_sequence_number: u64, final_sequence_number: u64, approved: bool) acquires MultisigAccount &#123;
    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));
    let sequence_number &#61; starting_sequence_number;
    while(sequence_number &lt;&#61; final_sequence_number) &#123;
        vote_transanction(owner, multisig_account, sequence_number, approved);
        sequence_number &#61; sequence_number &#43; 1;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transaction"></a>

## Function `execute_rejected_transaction`

Remove the next transaction if it has sufficient owner rejections.


<pre><code>public entry fun execute_rejected_transaction(owner: &amp;signer, multisig_account: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun execute_rejected_transaction(
    owner: &amp;signer,
    multisig_account: address,
) acquires MultisigAccount &#123;
    assert_multisig_account_exists(multisig_account);
    assert_is_owner(owner, multisig_account);

    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;
    let owner_addr &#61; address_of(owner);
    if(features::multisig_v2_enhancement_feature_enabled()) &#123;
        // Implicitly vote for rejection if the owner has not voted for rejection yet.
        if (!has_voted_for_rejection(multisig_account, sequence_number, owner_addr)) &#123;
            reject_transaction(owner, multisig_account, sequence_number);
        &#125;
    &#125;;

    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    let (_, num_rejections) &#61; remove_executed_transaction(multisig_account_resource);
    assert!(
        num_rejections &gt;&#61; multisig_account_resource.num_signatures_required,
        error::invalid_state(ENOT_ENOUGH_REJECTIONS),
    );

    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            ExecuteRejectedTransaction &#123;
                multisig_account,
                sequence_number,
                num_rejections,
                executor: address_of(owner),
            &#125;
        );
    &#125;;
    emit_event(
        &amp;mut multisig_account_resource.execute_rejected_transaction_events,
        ExecuteRejectedTransactionEvent &#123;
            sequence_number,
            num_rejections,
            executor: owner_addr,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transactions"></a>

## Function `execute_rejected_transactions`

Remove the next transactions until the final_sequence_number if they have sufficient owner rejections.


<pre><code>public entry fun execute_rejected_transactions(owner: &amp;signer, multisig_account: address, final_sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun execute_rejected_transactions(
    owner: &amp;signer,
    multisig_account: address,
    final_sequence_number: u64,
) acquires MultisigAccount &#123;
    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));
    assert!(last_resolved_sequence_number(multisig_account) &lt; final_sequence_number, error::invalid_argument(EINVALID_SEQUENCE_NUMBER));
    assert!(final_sequence_number &lt; next_sequence_number(multisig_account), error::invalid_argument(EINVALID_SEQUENCE_NUMBER));
    while(last_resolved_sequence_number(multisig_account) &lt; final_sequence_number) &#123;
        execute_rejected_transaction(owner, multisig_account);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_validate_multisig_transaction"></a>

## Function `validate_multisig_transaction`

Called by the VM as part of transaction prologue, which is invoked during mempool transaction validation and as
the first step of transaction execution.

Transaction payload is optional if it's already stored on chain for the transaction.


<pre><code>fun validate_multisig_transaction(owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_multisig_transaction(
    owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;) acquires MultisigAccount &#123;
    assert_multisig_account_exists(multisig_account);
    assert_is_owner(owner, multisig_account);
    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;
    assert_transaction_exists(multisig_account, sequence_number);

    if(features::multisig_v2_enhancement_feature_enabled()) &#123;
        assert!(
            can_execute(address_of(owner), multisig_account, sequence_number),
            error::invalid_argument(ENOT_ENOUGH_APPROVALS),
        );
    &#125;
    else &#123;
        assert!(
            can_be_executed(multisig_account, sequence_number),
            error::invalid_argument(ENOT_ENOUGH_APPROVALS),
        );
    &#125;;

    // If the transaction payload is not stored on chain, verify that the provided payload matches the hashes stored
    // on chain.
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);
    if (option::is_some(&amp;transaction.payload_hash)) &#123;
        let payload_hash &#61; option::borrow(&amp;transaction.payload_hash);
        assert!(
            sha3_256(payload) &#61;&#61; &#42;payload_hash,
            error::invalid_argument(EPAYLOAD_DOES_NOT_MATCH_HASH),
        );
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_successful_transaction_execution_cleanup"></a>

## Function `successful_transaction_execution_cleanup`

Post-execution cleanup for a successful multisig transaction execution.
This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code>fun successful_transaction_execution_cleanup(executor: address, multisig_account: address, transaction_payload: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun successful_transaction_execution_cleanup(
    executor: address,
    multisig_account: address,
    transaction_payload: vector&lt;u8&gt;,
) acquires MultisigAccount &#123;
    let num_approvals &#61; transaction_execution_cleanup_common(executor, multisig_account);
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            TransactionExecutionSucceeded &#123;
                multisig_account,
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                executor,
            &#125;
        );
    &#125;;
    emit_event(
        &amp;mut multisig_account_resource.execute_transaction_events,
        TransactionExecutionSucceededEvent &#123;
            sequence_number: multisig_account_resource.last_executed_sequence_number,
            transaction_payload,
            num_approvals,
            executor,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_failed_transaction_execution_cleanup"></a>

## Function `failed_transaction_execution_cleanup`

Post-execution cleanup for a failed multisig transaction execution.
This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code>fun failed_transaction_execution_cleanup(executor: address, multisig_account: address, transaction_payload: vector&lt;u8&gt;, execution_error: multisig_account::ExecutionError)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun failed_transaction_execution_cleanup(
    executor: address,
    multisig_account: address,
    transaction_payload: vector&lt;u8&gt;,
    execution_error: ExecutionError,
) acquires MultisigAccount &#123;
    let num_approvals &#61; transaction_execution_cleanup_common(executor, multisig_account);
    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            TransactionExecutionFailed &#123;
                multisig_account,
                executor,
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                execution_error,
            &#125;
        );
    &#125;;
    emit_event(
        &amp;mut multisig_account_resource.transaction_execution_failed_events,
        TransactionExecutionFailedEvent &#123;
            executor,
            sequence_number: multisig_account_resource.last_executed_sequence_number,
            transaction_payload,
            num_approvals,
            execution_error,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_transaction_execution_cleanup_common"></a>

## Function `transaction_execution_cleanup_common`



<pre><code>fun transaction_execution_cleanup_common(executor: address, multisig_account: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun transaction_execution_cleanup_common(executor: address, multisig_account: address): u64 acquires MultisigAccount &#123;
    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;
    let implicit_approval &#61; !has_voted_for_approval(multisig_account, sequence_number, executor);

    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);
    let (num_approvals, _) &#61; remove_executed_transaction(multisig_account_resource);

    if(features::multisig_v2_enhancement_feature_enabled() &amp;&amp; implicit_approval) &#123;
        if (std::features::module_event_migration_enabled()) &#123;
            emit(
                Vote &#123;
                    multisig_account,
                    owner: executor,
                    sequence_number,
                    approved: true,
                &#125;
            );
        &#125;;
        num_approvals &#61; num_approvals &#43; 1;
        emit_event(
            &amp;mut multisig_account_resource.vote_events,
            VoteEvent &#123;
                owner: executor,
                sequence_number,
                approved: true,
            &#125;
        );
    &#125;;

    num_approvals
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_remove_executed_transaction"></a>

## Function `remove_executed_transaction`



<pre><code>fun remove_executed_transaction(multisig_account_resource: &amp;mut multisig_account::MultisigAccount): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_executed_transaction(multisig_account_resource: &amp;mut MultisigAccount): (u64, u64) &#123;
    let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;
    let transaction &#61; table::remove(&amp;mut multisig_account_resource.transactions, sequence_number);
    multisig_account_resource.last_executed_sequence_number &#61; sequence_number;
    num_approvals_and_rejections_internal(&amp;multisig_account_resource.owners, &amp;transaction)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_add_transaction"></a>

## Function `add_transaction`



<pre><code>fun add_transaction(creator: address, multisig_account: address, transaction: multisig_account::MultisigTransaction)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun add_transaction(
    creator: address,
    multisig_account: address,
    transaction: MultisigTransaction
) &#123;
    if(features::multisig_v2_enhancement_feature_enabled()) &#123;
        assert!(
            available_transaction_queue_capacity(multisig_account) &gt; 0,
            error::invalid_state(EMAX_PENDING_TRANSACTIONS_EXCEEDED)
        );
    &#125;;

    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);

    // The transaction creator also automatically votes for the transaction.
    simple_map::add(&amp;mut transaction.votes, creator, true);

    let sequence_number &#61; multisig_account_resource.next_sequence_number;
    multisig_account_resource.next_sequence_number &#61; sequence_number &#43; 1;
    table::add(&amp;mut multisig_account_resource.transactions, sequence_number, transaction);
    if (std::features::module_event_migration_enabled()) &#123;
        emit(
            CreateTransaction &#123; multisig_account: multisig_account, creator, sequence_number, transaction &#125;
        );
    &#125;;
    emit_event(
        &amp;mut multisig_account_resource.create_transaction_events,
        CreateTransactionEvent &#123; creator, sequence_number, transaction &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account"></a>

## Function `create_multisig_account`



<pre><code>fun create_multisig_account(owner: &amp;signer): (signer, account::SignerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_multisig_account(owner: &amp;signer): (signer, SignerCapability) &#123;
    let owner_nonce &#61; account::get_sequence_number(address_of(owner));
    let (multisig_signer, multisig_signer_cap) &#61;
        account::create_resource_account(owner, create_multisig_account_seed(to_bytes(&amp;owner_nonce)));
    // Register the account to receive APT as this is not done by default as part of the resource account creation
    // flow.
    if (!coin::is_account_registered&lt;AptosCoin&gt;(address_of(&amp;multisig_signer))) &#123;
        coin::register&lt;AptosCoin&gt;(&amp;multisig_signer);
    &#125;;

    (multisig_signer, multisig_signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account_seed"></a>

## Function `create_multisig_account_seed`



<pre><code>fun create_multisig_account_seed(seed: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_multisig_account_seed(seed: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;
    // Generate a seed that will be used to create the resource account that hosts the multisig account.
    let multisig_account_seed &#61; vector::empty&lt;u8&gt;();
    vector::append(&amp;mut multisig_account_seed, DOMAIN_SEPARATOR);
    vector::append(&amp;mut multisig_account_seed, seed);

    multisig_account_seed
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_validate_owners"></a>

## Function `validate_owners`



<pre><code>fun validate_owners(owners: &amp;vector&lt;address&gt;, multisig_account: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_owners(owners: &amp;vector&lt;address&gt;, multisig_account: address) &#123;
    let distinct_owners: vector&lt;address&gt; &#61; vector[];
    vector::for_each_ref(owners, &#124;owner&#124; &#123;
        let owner &#61; &#42;owner;
        assert!(owner !&#61; multisig_account, error::invalid_argument(EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF));
        let (found, _) &#61; vector::index_of(&amp;distinct_owners, &amp;owner);
        assert!(!found, error::invalid_argument(EDUPLICATE_OWNER));
        vector::push_back(&amp;mut distinct_owners, owner);
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner_internal"></a>

## Function `assert_is_owner_internal`



<pre><code>fun assert_is_owner_internal(owner: &amp;signer, multisig_account: &amp;multisig_account::MultisigAccount)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_is_owner_internal(owner: &amp;signer, multisig_account: &amp;MultisigAccount) &#123;
    assert!(
        vector::contains(&amp;multisig_account.owners, &amp;address_of(owner)),
        error::permission_denied(ENOT_OWNER),
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner"></a>

## Function `assert_is_owner`



<pre><code>fun assert_is_owner(owner: &amp;signer, multisig_account: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_is_owner(owner: &amp;signer, multisig_account: address) acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    assert_is_owner_internal(owner, multisig_account_resource);
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections_internal"></a>

## Function `num_approvals_and_rejections_internal`



<pre><code>fun num_approvals_and_rejections_internal(owners: &amp;vector&lt;address&gt;, transaction: &amp;multisig_account::MultisigTransaction): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun num_approvals_and_rejections_internal(owners: &amp;vector&lt;address&gt;, transaction: &amp;MultisigTransaction): (u64, u64) &#123;
    let num_approvals &#61; 0;
    let num_rejections &#61; 0;

    let votes &#61; &amp;transaction.votes;
    vector::for_each_ref(owners, &#124;owner&#124; &#123;
        if (simple_map::contains_key(votes, owner)) &#123;
            if (&#42;simple_map::borrow(votes, owner)) &#123;
                num_approvals &#61; num_approvals &#43; 1;
            &#125; else &#123;
                num_rejections &#61; num_rejections &#43; 1;
            &#125;;
        &#125;
    &#125;);

    (num_approvals, num_rejections)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections"></a>

## Function `num_approvals_and_rejections`



<pre><code>fun num_approvals_and_rejections(multisig_account: address, sequence_number: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun num_approvals_and_rejections(multisig_account: address, sequence_number: u64): (u64, u64) acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);
    num_approvals_and_rejections_internal(&amp;multisig_account_resource.owners, transaction)
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_approval"></a>

## Function `has_voted_for_approval`



<pre><code>fun has_voted_for_approval(multisig_account: address, sequence_number: u64, owner: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun has_voted_for_approval(multisig_account: address, sequence_number: u64, owner: address): bool acquires MultisigAccount &#123;
    let (voted, vote) &#61; vote(multisig_account, sequence_number, owner);
    voted &amp;&amp; vote
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_rejection"></a>

## Function `has_voted_for_rejection`



<pre><code>fun has_voted_for_rejection(multisig_account: address, sequence_number: u64, owner: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun has_voted_for_rejection(multisig_account: address, sequence_number: u64, owner: address): bool acquires MultisigAccount &#123;
    let (voted, vote) &#61; vote(multisig_account, sequence_number, owner);
    voted &amp;&amp; !vote
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_assert_multisig_account_exists"></a>

## Function `assert_multisig_account_exists`



<pre><code>fun assert_multisig_account_exists(multisig_account: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_multisig_account_exists(multisig_account: address) &#123;
    assert!(exists&lt;MultisigAccount&gt;(multisig_account), error::invalid_state(EACCOUNT_NOT_MULTISIG));
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_assert_valid_sequence_number"></a>

## Function `assert_valid_sequence_number`



<pre><code>fun assert_valid_sequence_number(multisig_account: address, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_valid_sequence_number(multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    assert!(
        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,
        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_assert_transaction_exists"></a>

## Function `assert_transaction_exists`



<pre><code>fun assert_transaction_exists(multisig_account: address, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_transaction_exists(multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;
    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);
    assert!(
        table::contains(&amp;multisig_account_resource.transactions, sequence_number),
        error::not_found(ETRANSACTION_NOT_FOUND),
    );
&#125;
</code></pre>



</details>

<a id="0x1_multisig_account_update_owner_schema"></a>

## Function `update_owner_schema`

Add new owners, remove owners to remove, update signatures required.


<pre><code>fun update_owner_schema(multisig_address: address, new_owners: vector&lt;address&gt;, owners_to_remove: vector&lt;address&gt;, optional_new_num_signatures_required: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_owner_schema(
    multisig_address: address,
    new_owners: vector&lt;address&gt;,
    owners_to_remove: vector&lt;address&gt;,
    optional_new_num_signatures_required: Option&lt;u64&gt;,
) acquires MultisigAccount &#123;
    assert_multisig_account_exists(multisig_address);
    let multisig_account_ref_mut &#61;
        borrow_global_mut&lt;MultisigAccount&gt;(multisig_address);
    // Verify no overlap between new owners and owners to remove.
    vector::for_each_ref(&amp;new_owners, &#124;new_owner_ref&#124; &#123;
        assert!(
            !vector::contains(&amp;owners_to_remove, new_owner_ref),
            error::invalid_argument(EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP)
        )
    &#125;);
    // If new owners provided, try to add them and emit an event.
    if (vector::length(&amp;new_owners) &gt; 0) &#123;
        vector::append(&amp;mut multisig_account_ref_mut.owners, new_owners);
        validate_owners(
            &amp;multisig_account_ref_mut.owners,
            multisig_address
        );
        if (std::features::module_event_migration_enabled()) &#123;
            emit(AddOwners &#123; multisig_account: multisig_address, owners_added: new_owners &#125;);
        &#125;;
        emit_event(
            &amp;mut multisig_account_ref_mut.add_owners_events,
            AddOwnersEvent &#123; owners_added: new_owners &#125;
        );
    &#125;;
    // If owners to remove provided, try to remove them.
    if (vector::length(&amp;owners_to_remove) &gt; 0) &#123;
        let owners_ref_mut &#61; &amp;mut multisig_account_ref_mut.owners;
        let owners_removed &#61; vector[];
        vector::for_each_ref(&amp;owners_to_remove, &#124;owner_to_remove_ref&#124; &#123;
            let (found, index) &#61;
                vector::index_of(owners_ref_mut, owner_to_remove_ref);
            if (found) &#123;
                vector::push_back(
                    &amp;mut owners_removed,
                    vector::swap_remove(owners_ref_mut, index)
                );
            &#125;
        &#125;);
        // Only emit event if owner(s) actually removed.
        if (vector::length(&amp;owners_removed) &gt; 0) &#123;
            if (std::features::module_event_migration_enabled()) &#123;
                emit(
                    RemoveOwners &#123; multisig_account: multisig_address, owners_removed &#125;
                );
            &#125;;
            emit_event(
                &amp;mut multisig_account_ref_mut.remove_owners_events,
                RemoveOwnersEvent &#123; owners_removed &#125;
            );
        &#125;
    &#125;;
    // If new signature count provided, try to update count.
    if (option::is_some(&amp;optional_new_num_signatures_required)) &#123;
        let new_num_signatures_required &#61;
            option::extract(&amp;mut optional_new_num_signatures_required);
        assert!(
            new_num_signatures_required &gt; 0,
            error::invalid_argument(EINVALID_SIGNATURES_REQUIRED)
        );
        let old_num_signatures_required &#61;
            multisig_account_ref_mut.num_signatures_required;
        // Only apply update and emit event if a change indicated.
        if (new_num_signatures_required !&#61; old_num_signatures_required) &#123;
            multisig_account_ref_mut.num_signatures_required &#61;
                new_num_signatures_required;
            if (std::features::module_event_migration_enabled()) &#123;
                emit(
                    UpdateSignaturesRequired &#123;
                        multisig_account: multisig_address,
                        old_num_signatures_required,
                        new_num_signatures_required,
                    &#125;
                );
            &#125;;
            emit_event(
                &amp;mut multisig_account_ref_mut.update_signature_required_events,
                UpdateSignaturesRequiredEvent &#123;
                    old_num_signatures_required,
                    new_num_signatures_required,
                &#125;
            );
        &#125;
    &#125;;
    // Verify number of owners.
    let num_owners &#61; vector::length(&amp;multisig_account_ref_mut.owners);
    assert!(
        num_owners &gt;&#61; multisig_account_ref_mut.num_signatures_required,
        error::invalid_state(ENOT_ENOUGH_OWNERS)
    );
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>For every multi-signature account, the range of required signatures should always be in the range of one to the total number of owners.</td>
<td>Critical</td>
<td>While creating a MultisigAccount, the function create_with_owners_internal checks that num_signatures_required is in the span from 1 to total count of owners.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>2</td>
<td>The list of owners for a multi-signature account should not contain any duplicate owners, and the multi-signature account itself cannot be listed as one of its owners.</td>
<td>Critical</td>
<td>The function validate_owners validates the owner vector that no duplicate entries exists.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>3</td>
<td>The current value of the next sequence number should not be present in the transaction table, until the next sequence number gets increased.</td>
<td>Medium</td>
<td>The add_transaction function increases the next sequence number and only then adds the transaction with the old next sequence number to the transaction table.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>4</td>
<td>When the last executed sequence number is smaller than the next sequence number by only one unit, no transactions should exist in the multi-signature account's transactions list.</td>
<td>High</td>
<td>The get_pending_transactions function retrieves pending transactions by iterating through the transactions table, starting from the last_executed_sequence_number + 1 to the next_sequence_number.</td>
<td>Audited that MultisigAccount.transactions is empty when last_executed_sequence_number == next_sequence_number -1</td>
</tr>

<tr>
<td>5</td>
<td>The last executed sequence number is always smaller than the next sequence number.</td>
<td>Medium</td>
<td>When creating a new MultisigAccount, the last_executed_sequence_number and next_sequence_number are assigned with 0 and 1 respectively, and from there both these values increase monotonically when a transaction is executed and removed from the table and when new transaction are added respectively.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>6</td>
<td>The number of pending transactions should be equal to the difference between the next sequence number and the last executed sequence number.</td>
<td>High</td>
<td>When a transaction is added, next_sequence_number is incremented. And when a transaction is removed after execution, last_executed_sequence_number is incremented.</td>
<td>This has been audited.</td>
</tr>

<tr>
<td>7</td>
<td>Only transactions with valid sequence number should be fetched.</td>
<td>Medium</td>
<td>Functions such as: 1. get_transaction 2. can_be_executed 3. can_be_rejected 4. vote always validate the given sequence number and only then fetch the associated transaction.</td>
<td>Audited that it aborts if the sequence number is not valid.</td>
</tr>

<tr>
<td>8</td>
<td>The execution or rejection of a transaction should enforce that the minimum number of required signatures is less or equal to the total number of approvals.</td>
<td>Critical</td>
<td>The functions can_be_executed and can_be_rejected perform validation on the number of votes required for execution or rejection.</td>
<td>Audited that these functions return the correct value.</td>
</tr>

<tr>
<td>9</td>
<td>The creation of a multi-signature account properly initializes the resources and then it gets published under the corresponding account.</td>
<td>Medium</td>
<td>When creating a MultisigAccount via one of the functions: create_with_existing_account, create_with_existing_account_and_revoke_auth_key, create_with_owners, create, the MultisigAccount data is initialized properly and published to the multisig_account (new or existing).</td>
<td>Audited that the MultisigAccount is initialized properly.</td>
</tr>

<tr>
<td>10</td>
<td>Creation of a multi-signature account on top of an existing account should revoke auth key and any previous offered capabilities or control.</td>
<td>Critical</td>
<td>The function create_with_existing_account_and_revoke_auth_key, after successfully creating the MultisigAccount, rotates the account to ZeroAuthKey and revokes any offered capabilities of that account.</td>
<td>Audited that the account's auth key and the offered capabilities are revoked.</td>
</tr>

<tr>
<td>11</td>
<td>Upon the creation of a multi-signature account from a bootstrapping account, the ownership of the resultant account should not pertain to the bootstrapping account.</td>
<td>High</td>
<td>In create_with_owners_then_remove_bootstrapper function after successful creation of the account the bootstrapping account is removed from the owner vector of the account.</td>
<td>Audited that the bootstrapping account is not in the owners list.</td>
</tr>

<tr>
<td>12</td>
<td>Performing any changes on the list of owners such as adding new owners, removing owners, swapping owners should ensure that the number of required signature, for the multi-signature account remains valid.</td>
<td>Critical</td>
<td>The following function as used to modify the owners list and the required signature of the account: add_owner, add_owners, add_owners_and_update_signatures_required, remove_owner, remove_owners, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_signatures_required. All of these functions use update_owner_schema function to process these changes, the function validates the owner list while adding and verifies that the account has enough required signatures and updates the owner's schema.</td>
<td>Audited that the owners are added successfully. (add_owner, add_owners, add_owners_and_update_signatures_required, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_owner_schema) Audited that the owners are removed successfully. (remove_owner, remove_owners, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_owner_schema) Audited that the num_signatures_required is updated successfully. (add_owners_and_update_signatures_required, swap_owners_and_update_signatures_required, update_signatures_required, update_owner_schema)</td>
</tr>

<tr>
<td>13</td>
<td>The creation of a transaction should be limited to an account owner, which should be automatically considered a voter; additionally, the account's sequence should increase monotonically.</td>
<td>Critical</td>
<td>The following functions can only be called by the owners of the account and create a transaction and uses add_transaction function to gives approval on behalf of the creator and increments the next_sequence_number and finally adds the transaction to the MultsigAccount: create_transaction_with_hash, create_transaction.</td>
<td>Audited it aborts if the caller is not in the owner's list of the account. (create_transaction_with_hash, create_transaction) Audited that the transaction is successfully stored in the MultisigAccount.(create_transaction_with_hash, create_transaction, add_transaction) Audited that the creators voted to approve the transaction. (create_transaction_with_hash, create_transaction, add_transaction) Audited that the next_sequence_number increases monotonically. (create_transaction_with_hash, create_transaction, add_transaction)</td>
</tr>

<tr>
<td>14</td>
<td>Only owners are allowed to vote for a valid transaction.</td>
<td>Critical</td>
<td>Any owner of the MultisigAccount can either approve (approve_transaction) or reject (reject_transaction) a transaction. Both these functions use a generic function to vote for the transaction which validates the caller and the transaction id and adds/updates the vote.</td>
<td>Audited that it aborts if the caller is not in the owner's list (approve_transaction, reject_transaction, vote_transaction, assert_is_owner). Audited that it aborts if the transaction with the given sequence number doesn't exist in the account (approve_transaction, reject_transaction, vote_transaction). Audited that the vote is recorded as intended.</td>
</tr>

<tr>
<td>15</td>
<td>Only owners are allowed to execute a valid transaction, if the number of approvals meets the k-of-n criteria, finally the executed transaction should be removed.</td>
<td>Critical</td>
<td>Functions execute_rejected_transaction and validate_multisig_transaction can only be called by the owner which validates the transaction and based on the number of approvals and rejections it proceeds to execute the transactions. For rejected transaction, the transactions are immediately removed from the MultisigAccount via remove_executed_transaction. VM validates the transaction via validate_multisig_transaction and cleans up the transaction via successful_transaction_execution_cleanup and failed_transaction_execution_cleanup.</td>
<td>Audited that it aborts if the caller is not in the owner's list (execute_rejected_transaction, validate_multisig_transaction). Audited that it aborts if the transaction with the given sequence number doesn't exist in the account (execute_rejected_transaction, validate_multisig_transaction). Audited that it aborts if the votes (approvals or rejections) are less than num_signatures_required (execute_rejected_transaction, validate_multisig_transaction). Audited that the transaction is removed from the MultisigAccount (execute_rejected_transaction, remove_executed_transaction, successful_transaction_execution_cleanup, failed_transaction_execution_cleanup).</td>
</tr>

<tr>
<td>16</td>
<td>Removing an executed transaction from the transactions list should increase the last sequence number monotonically.</td>
<td>High</td>
<td>When transactions are removed via remove_executed_transaction (maybe called by VM cleanup or execute_rejected_transaction), the last_executed_sequence_number increases by 1.</td>
<td>Audited that last_executed_sequence_number is incremented.</td>
</tr>

<tr>
<td>17</td>
<td>The voting and transaction creation operations should only be available if a multi-signature account exists.</td>
<td>Low</td>
<td>The function assert_multisig_account_exists validates the existence of MultisigAccount under the account.</td>
<td>Audited that it aborts if the MultisigAccount doesn't exist on the account.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<a id="@Specification_1_metadata"></a>

### Function `metadata`


<pre><code>&#35;[view]
public fun metadata(multisig_account: address): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).metadata;
</code></pre>



<a id="@Specification_1_num_signatures_required"></a>

### Function `num_signatures_required`


<pre><code>&#35;[view]
public fun num_signatures_required(multisig_account: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).num_signatures_required;
</code></pre>



<a id="@Specification_1_owners"></a>

### Function `owners`


<pre><code>&#35;[view]
public fun owners(multisig_account: address): vector&lt;address&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).owners;
</code></pre>



<a id="@Specification_1_get_transaction"></a>

### Function `get_transaction`


<pre><code>&#35;[view]
public fun get_transaction(multisig_account: address, sequence_number: u64): multisig_account::MultisigTransaction
</code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);
aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
aborts_if sequence_number &#61;&#61; 0 &#124;&#124; sequence_number &gt;&#61; multisig_account_resource.next_sequence_number;
aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
ensures result &#61;&#61; table::spec_get(multisig_account_resource.transactions, sequence_number);
</code></pre>



<a id="@Specification_1_get_next_transaction_payload"></a>

### Function `get_next_transaction_payload`


<pre><code>&#35;[view]
public fun get_next_transaction_payload(multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt;
</code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);
let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;
let transaction &#61; table::spec_get(multisig_account_resource.transactions, sequence_number);
aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
aborts_if multisig_account_resource.last_executed_sequence_number &#43; 1 &gt; MAX_U64;
aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
ensures option::spec_is_none(transaction.payload) &#61;&#61;&gt; result &#61;&#61; provided_payload;
</code></pre>



<a id="@Specification_1_get_next_multisig_account_address"></a>

### Function `get_next_multisig_account_address`


<pre><code>&#35;[view]
public fun get_next_multisig_account_address(creator: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;account::Account&gt;(creator);
let owner_nonce &#61; global&lt;account::Account&gt;(creator).sequence_number;
</code></pre>



<a id="@Specification_1_last_resolved_sequence_number"></a>

### Function `last_resolved_sequence_number`


<pre><code>&#35;[view]
public fun last_resolved_sequence_number(multisig_account: address): u64
</code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);
aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
ensures result &#61;&#61; multisig_account_resource.last_executed_sequence_number;
</code></pre>



<a id="@Specification_1_next_sequence_number"></a>

### Function `next_sequence_number`


<pre><code>&#35;[view]
public fun next_sequence_number(multisig_account: address): u64
</code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);
aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
ensures result &#61;&#61; multisig_account_resource.next_sequence_number;
</code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>&#35;[view]
public fun vote(multisig_account: address, sequence_number: u64, owner: address): (bool, bool)
</code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);
aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);
aborts_if sequence_number &#61;&#61; 0 &#124;&#124; sequence_number &gt;&#61; multisig_account_resource.next_sequence_number;
aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);
let transaction &#61; table::spec_get(multisig_account_resource.transactions, sequence_number);
let votes &#61; transaction.votes;
let voted &#61; simple_map::spec_contains_key(votes, owner);
let vote &#61; voted &amp;&amp; simple_map::spec_get(votes, owner);
ensures result_1 &#61;&#61; voted;
ensures result_2 &#61;&#61; vote;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
