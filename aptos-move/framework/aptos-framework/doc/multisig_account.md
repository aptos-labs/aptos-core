
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
-  [Function `create_with_existing_account_call`](#0x1_multisig_account_create_with_existing_account_call)
-  [Function `create_with_existing_account`](#0x1_multisig_account_create_with_existing_account)
-  [Function `create_with_existing_account_and_revoke_auth_key_call`](#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key_call)
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


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_multisig_account_MultisigAccount"></a>

## Resource `MultisigAccount`

Represents a multisig account's configurations and transactions.
This will be stored in the multisig account (created as a resource account separate from any owner accounts).


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_signatures_required: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transactions: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>&gt;</code>
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
<code>signer_cap: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_owners_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_AddOwnersEvent">multisig_account::AddOwnersEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>remove_owners_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_RemoveOwnersEvent">multisig_account::RemoveOwnersEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_signature_required_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequiredEvent">multisig_account::UpdateSignaturesRequiredEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_transaction_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_CreateTransactionEvent">multisig_account::CreateTransactionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>vote_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_VoteEvent">multisig_account::VoteEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execute_rejected_transaction_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransactionEvent">multisig_account::ExecuteRejectedTransactionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>execute_transaction_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceededEvent">multisig_account::TransactionExecutionSucceededEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_execution_failed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailedEvent">multisig_account::TransactionExecutionFailedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>metadata_updated_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="multisig_account.md#0x1_multisig_account_MetadataUpdatedEvent">multisig_account::MetadataUpdatedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MultisigTransaction"></a>

## Struct `MultisigTransaction`

A transaction to be executed in a multisig account.
This must contain either the full transaction payload or its hash (stored as bytes).


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payload: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>payload_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>votes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, bool&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>creator: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_ExecutionError">ExecutionError</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>abort_location: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>error_type: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationMessage">MultisigAccountCreationMessage</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="chain_id.md#0x1_chain_id">chain_id</a>: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>account_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationWithAuthKeyRevocationMessage">MultisigAccountCreationWithAuthKeyRevocationMessage</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="chain_id.md#0x1_chain_id">chain_id</a>: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>account_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_AddOwnersEvent">AddOwnersEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners_added: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_AddOwners"></a>

## Struct `AddOwners`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_AddOwners">AddOwners</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>owners_added: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_RemoveOwnersEvent"></a>

## Struct `RemoveOwnersEvent`

Event emitted when new owners are removed from the multisig account.


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_RemoveOwnersEvent">RemoveOwnersEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owners_removed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_RemoveOwners"></a>

## Struct `RemoveOwners`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_RemoveOwners">RemoveOwners</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>owners_removed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_UpdateSignaturesRequiredEvent"></a>

## Struct `UpdateSignaturesRequiredEvent`

Event emitted when the number of signatures required is updated.


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequiredEvent">UpdateSignaturesRequiredEvent</a> <b>has</b> drop, store
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequired">UpdateSignaturesRequired</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_CreateTransactionEvent">CreateTransactionEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction: <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_CreateTransaction"></a>

## Struct `CreateTransaction`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_CreateTransaction">CreateTransaction</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction: <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_VoteEvent"></a>

## Struct `VoteEvent`

Event emitted when an owner approves or rejects a transaction.


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_VoteEvent">VoteEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner: <b>address</b></code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_Vote">Vote</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>owner: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransactionEvent">ExecuteRejectedTransactionEvent</a> <b>has</b> drop, store
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
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_ExecuteRejectedTransaction"></a>

## Struct `ExecuteRejectedTransaction`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransaction">ExecuteRejectedTransaction</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
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
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionSucceededEvent"></a>

## Struct `TransactionExecutionSucceededEvent`

Event emitted when a transaction is executed.


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceededEvent">TransactionExecutionSucceededEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceeded">TransactionExecutionSucceeded</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailedEvent">TransactionExecutionFailedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_error: <a href="multisig_account.md#0x1_multisig_account_ExecutionError">multisig_account::ExecutionError</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_TransactionExecutionFailed"></a>

## Struct `TransactionExecutionFailed`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailed">TransactionExecutionFailed</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>executor: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_approvals: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>execution_error: <a href="multisig_account.md#0x1_multisig_account_ExecutionError">multisig_account::ExecutionError</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MetadataUpdatedEvent"></a>

## Struct `MetadataUpdatedEvent`

Event emitted when a transaction's metadata is updated.


<pre><code><b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MetadataUpdatedEvent">MetadataUpdatedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_multisig_account_MetadataUpdated"></a>

## Struct `MetadataUpdated`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="multisig_account.md#0x1_multisig_account_MetadataUpdated">MetadataUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_multisig_account_ZERO_AUTH_KEY"></a>



<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_multisig_account_DOMAIN_SEPARATOR"></a>

The salt used to create a resource account during multisig account creation.
This is used to avoid conflicts with other modules that also create resource accounts with the same owner
account.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_DOMAIN_SEPARATOR">DOMAIN_SEPARATOR</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 109, 117, 108, 116, 105, 115, 105, 103, 95, 97, 99, 99, 111, 117, 110, 116];
</code></pre>



<a id="0x1_multisig_account_EACCOUNT_NOT_MULTISIG"></a>

Specified account is not a multisig account.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EACCOUNT_NOT_MULTISIG">EACCOUNT_NOT_MULTISIG</a>: u64 = 2002;
</code></pre>



<a id="0x1_multisig_account_EDUPLICATE_METADATA_KEY"></a>

The specified metadata contains duplicate attributes (keys).


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EDUPLICATE_METADATA_KEY">EDUPLICATE_METADATA_KEY</a>: u64 = 16;
</code></pre>



<a id="0x1_multisig_account_EDUPLICATE_OWNER"></a>

Owner list cannot contain the same address more than once.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EDUPLICATE_OWNER">EDUPLICATE_OWNER</a>: u64 = 1;
</code></pre>



<a id="0x1_multisig_account_EINVALID_PAYLOAD_HASH"></a>

Payload hash must be exactly 32 bytes (sha3-256).


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EINVALID_PAYLOAD_HASH">EINVALID_PAYLOAD_HASH</a>: u64 = 12;
</code></pre>



<a id="0x1_multisig_account_EINVALID_SEQUENCE_NUMBER"></a>

The sequence number provided is invalid. It must be between [1, next pending transaction - 1].


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>: u64 = 17;
</code></pre>



<a id="0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED"></a>

Number of signatures required must be more than zero and at most the total number of owners.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED">EINVALID_SIGNATURES_REQUIRED</a>: u64 = 11;
</code></pre>



<a id="0x1_multisig_account_EMAX_PENDING_TRANSACTIONS_EXCEEDED"></a>

The number of pending transactions has exceeded the maximum allowed.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EMAX_PENDING_TRANSACTIONS_EXCEEDED">EMAX_PENDING_TRANSACTIONS_EXCEEDED</a>: u64 = 19;
</code></pre>



<a id="0x1_multisig_account_EMULTISIG_ACCOUNTS_NOT_ENABLED_YET"></a>

Multisig accounts has not been enabled on this current network yet.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EMULTISIG_ACCOUNTS_NOT_ENABLED_YET">EMULTISIG_ACCOUNTS_NOT_ENABLED_YET</a>: u64 = 14;
</code></pre>



<a id="0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED"></a>

The multisig v2 enhancement feature is not enabled.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED">EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED</a>: u64 = 20;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_APPROVALS"></a>

Transaction has not received enough approvals to be executed.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_APPROVALS">ENOT_ENOUGH_APPROVALS</a>: u64 = 2009;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_OWNERS"></a>

Multisig account must have at least one owner.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_OWNERS">ENOT_ENOUGH_OWNERS</a>: u64 = 5;
</code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_REJECTIONS"></a>

Transaction has not received enough rejections to be officially rejected.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_REJECTIONS">ENOT_ENOUGH_REJECTIONS</a>: u64 = 10;
</code></pre>



<a id="0x1_multisig_account_ENOT_OWNER"></a>

Account executing this operation is not an owner of the multisig account.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ENOT_OWNER">ENOT_OWNER</a>: u64 = 2003;
</code></pre>



<a id="0x1_multisig_account_ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH"></a>

The number of metadata keys and values don't match.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH">ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH</a>: u64 = 15;
</code></pre>



<a id="0x1_multisig_account_EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP"></a>

Provided owners to remove and new owners overlap.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP">EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP</a>: u64 = 18;
</code></pre>



<a id="0x1_multisig_account_EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF"></a>

The multisig account itself cannot be an owner.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF">EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF</a>: u64 = 13;
</code></pre>



<a id="0x1_multisig_account_EPAYLOAD_CANNOT_BE_EMPTY"></a>

Transaction payload cannot be empty.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_CANNOT_BE_EMPTY">EPAYLOAD_CANNOT_BE_EMPTY</a>: u64 = 4;
</code></pre>



<a id="0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH"></a>

Provided target function does not match the payload stored in the on-chain transaction.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH">EPAYLOAD_DOES_NOT_MATCH</a>: u64 = 2010;
</code></pre>



<a id="0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH_HASH"></a>

Provided target function does not match the hash stored in the on-chain transaction.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH_HASH">EPAYLOAD_DOES_NOT_MATCH_HASH</a>: u64 = 2008;
</code></pre>



<a id="0x1_multisig_account_ETRANSACTION_NOT_FOUND"></a>

Transaction with specified id cannot be found.


<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_ETRANSACTION_NOT_FOUND">ETRANSACTION_NOT_FOUND</a>: u64 = 2006;
</code></pre>



<a id="0x1_multisig_account_MAX_PENDING_TRANSACTIONS"></a>



<pre><code><b>const</b> <a href="multisig_account.md#0x1_multisig_account_MAX_PENDING_TRANSACTIONS">MAX_PENDING_TRANSACTIONS</a>: u64 = 20;
</code></pre>



<a id="0x1_multisig_account_metadata"></a>

## Function `metadata`

Return the multisig account's metadata.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_metadata">metadata</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_metadata">metadata</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): SimpleMap&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).metadata
}
</code></pre>



</details>

<a id="0x1_multisig_account_num_signatures_required"></a>

## Function `num_signatures_required`

Return the number of signatures required to execute or execute-reject a transaction in the provided
multisig account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64 <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).num_signatures_required
}
</code></pre>



</details>

<a id="0x1_multisig_account_owners"></a>

## Function `owners`

Return a vector of all of the provided multisig account's owners.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_owners">owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_owners">owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).owners
}
</code></pre>



</details>

<a id="0x1_multisig_account_is_owner"></a>

## Function `is_owner`

Return true if the provided owner is an owner of the provided multisig account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_is_owner">is_owner</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_is_owner">is_owner</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&<b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).owners, &owner)
}
</code></pre>



</details>

<a id="0x1_multisig_account_get_transaction"></a>

## Function `get_transaction`

Return the transaction with the given transaction id.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_transaction">get_transaction</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_transaction">get_transaction</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    sequence_number: u64,
): <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a> <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>assert</b>!(
        sequence_number &gt; 0 && sequence_number &lt; multisig_account_resource.next_sequence_number,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>),
    );
    *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&multisig_account_resource.transactions, sequence_number)
}
</code></pre>



</details>

<a id="0x1_multisig_account_get_pending_transactions"></a>

## Function `get_pending_transactions`

Return all pending transactions.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_pending_transactions">get_pending_transactions</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_pending_transactions">get_pending_transactions</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a>&gt; <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> pending_transactions: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a>&gt; = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> <a href="multisig_account.md#0x1_multisig_account">multisig_account</a> = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> i = <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>.last_executed_sequence_number + 1;
    <b>let</b> next_sequence_number = <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>.next_sequence_number;
    <b>while</b> (i &lt; next_sequence_number) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> pending_transactions, *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>.transactions, i));
        i = i + 1;
    };
    pending_transactions
}
</code></pre>



</details>

<a id="0x1_multisig_account_get_next_transaction_payload"></a>

## Function `get_next_transaction_payload`

Return the payload for the next transaction in the queue.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_transaction_payload">get_next_transaction_payload</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, provided_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_transaction_payload">get_next_transaction_payload</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, provided_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&multisig_account_resource.transactions, sequence_number);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&transaction.payload)) {
        *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&transaction.payload)
    } <b>else</b> {
        provided_payload
    }
}
</code></pre>



</details>

<a id="0x1_multisig_account_can_be_executed"></a>

## Function `can_be_executed`

Return true if the transaction with given transaction id can be executed now.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_be_executed">can_be_executed</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_be_executed">can_be_executed</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>let</b> (num_approvals, _) = <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    sequence_number == <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1 &&
        num_approvals &gt;= <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>)
}
</code></pre>



</details>

<a id="0x1_multisig_account_can_execute"></a>

## Function `can_execute`

Return true if the owner can execute the transaction with given transaction id now.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_execute">can_execute</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_execute">can_execute</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>let</b> (num_approvals, _) = <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>if</b> (!<a href="multisig_account.md#0x1_multisig_account_has_voted_for_approval">has_voted_for_approval</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, owner)) {
        num_approvals = num_approvals + 1;
    };
    <a href="multisig_account.md#0x1_multisig_account_is_owner">is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) &&
        sequence_number == <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1 &&
        num_approvals &gt;= <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>)
}
</code></pre>



</details>

<a id="0x1_multisig_account_can_be_rejected"></a>

## Function `can_be_rejected`

Return true if the transaction with given transaction id can be officially rejected.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_be_rejected">can_be_rejected</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_be_rejected">can_be_rejected</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>let</b> (_, num_rejections) = <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    sequence_number == <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1 &&
        num_rejections &gt;= <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>)
}
</code></pre>



</details>

<a id="0x1_multisig_account_can_reject"></a>

## Function `can_reject`

Return true if the owner can execute the "rejected" transaction with given transaction id now.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_reject">can_reject</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_can_reject">can_reject</a>(owner: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>let</b> (_, num_rejections) = <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
    <b>if</b> (!<a href="multisig_account.md#0x1_multisig_account_has_voted_for_rejection">has_voted_for_rejection</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, owner)) {
        num_rejections = num_rejections + 1;
    };
    <a href="multisig_account.md#0x1_multisig_account_is_owner">is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) &&
        sequence_number == <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1 &&
        num_rejections &gt;= <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>)
}
</code></pre>



</details>

<a id="0x1_multisig_account_get_next_multisig_account_address"></a>

## Function `get_next_multisig_account_address`

Return the predicted address for the next multisig account if created from the given creator address.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_multisig_account_address">get_next_multisig_account_address</a>(creator: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_multisig_account_address">get_next_multisig_account_address</a>(creator: <b>address</b>): <b>address</b> {
    <b>let</b> owner_nonce = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(creator);
    create_resource_address(&creator, <a href="multisig_account.md#0x1_multisig_account_create_multisig_account_seed">create_multisig_account_seed</a>(to_bytes(&owner_nonce)))
}
</code></pre>



</details>

<a id="0x1_multisig_account_last_resolved_sequence_number"></a>

## Function `last_resolved_sequence_number`

Return the id of the last transaction that was executed (successful or failed) or removed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64 <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    multisig_account_resource.last_executed_sequence_number
}
</code></pre>



</details>

<a id="0x1_multisig_account_next_sequence_number"></a>

## Function `next_sequence_number`

Return the id of the next transaction created.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_next_sequence_number">next_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_next_sequence_number">next_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64 <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    multisig_account_resource.next_sequence_number
}
</code></pre>



</details>

<a id="0x1_multisig_account_vote"></a>

## Function `vote`

Return a bool tuple indicating whether an owner has voted and if so, whether they voted yes or no.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote">vote</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): (bool, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote">vote</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): (bool, bool) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>assert</b>!(
        sequence_number &gt; 0 && sequence_number &lt; multisig_account_resource.next_sequence_number,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>),
    );
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&multisig_account_resource.transactions, sequence_number);
    <b>let</b> votes = &transaction.votes;
    <b>let</b> voted = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(votes, &owner);
    <b>let</b> vote = voted && *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(votes, &owner);
    (voted, vote)
}
</code></pre>



</details>

<a id="0x1_multisig_account_available_transaction_queue_capacity"></a>

## Function `available_transaction_queue_capacity`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_available_transaction_queue_capacity">available_transaction_queue_capacity</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_available_transaction_queue_capacity">available_transaction_queue_capacity</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64 <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> num_pending_transactions = multisig_account_resource.next_sequence_number - multisig_account_resource.last_executed_sequence_number - 1;
    <b>if</b> (num_pending_transactions &gt; <a href="multisig_account.md#0x1_multisig_account_MAX_PENDING_TRANSACTIONS">MAX_PENDING_TRANSACTIONS</a>) {
        0
    } <b>else</b> {
        <a href="multisig_account.md#0x1_multisig_account_MAX_PENDING_TRANSACTIONS">MAX_PENDING_TRANSACTIONS</a> - num_pending_transactions
    }
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account_call"></a>

## Function `create_with_existing_account_call`

Private entry function that creates a new multisig account on top of an existing account.

This offers a migration path for an existing account with any type of auth key.

Note that this does not revoke auth key-based control over the account. Owners should separately rotate the auth
key after they are fully migrated to the new multisig account. Alternatively, they can call
create_with_existing_account_and_revoke_auth_key_call instead.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_call">create_with_existing_account_call</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_call">create_with_existing_account_call</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
        <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
        owners,
        num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );
}
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


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account">create_with_existing_account</a>(multisig_address: <b>address</b>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, create_multisig_account_signed_message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account">create_with_existing_account</a>(
    multisig_address: <b>address</b>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    account_scheme: u8,
    account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    create_multisig_account_signed_message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    // Verify that the `<a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationMessage">MultisigAccountCreationMessage</a>` <b>has</b> the right information and is signed by the <a href="account.md#0x1_account">account</a>
    // owner's key.
    <b>let</b> proof_challenge = <a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationMessage">MultisigAccountCreationMessage</a> {
        <a href="chain_id.md#0x1_chain_id">chain_id</a>: <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(),
        account_address: multisig_address,
        sequence_number: <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(multisig_address),
        owners,
        num_signatures_required,
    };
    <a href="account.md#0x1_account_verify_signed_message">account::verify_signed_message</a>(
        multisig_address,
        account_scheme,
        account_public_key,
        create_multisig_account_signed_message,
        proof_challenge,
    );

    // We create the <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> for the multisig <a href="account.md#0x1_account">account</a> here since this is required <b>to</b> add the <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> resource
    // This should be safe and authorized because we have verified the signed message from the existing <a href="account.md#0x1_account">account</a>
    // that authorizes creating a multisig <a href="account.md#0x1_account">account</a> <b>with</b> the specified owners and signature threshold.
    <b>let</b> <a href="multisig_account.md#0x1_multisig_account">multisig_account</a> = &<a href="create_signer.md#0x1_create_signer">create_signer</a>(multisig_address);
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
        <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
        owners,
        num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account_and_revoke_auth_key_call"></a>

## Function `create_with_existing_account_and_revoke_auth_key_call`

Private entry function that creates a new multisig account on top of an existing account and immediately rotate
the origin auth key to 0x0.

Note: If the original account is a resource account, this does not revoke all control over it as if any
SignerCapability of the resource account still exists, it can still be used to generate the signer for the
account.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key_call">create_with_existing_account_and_revoke_auth_key_call</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key_call">create_with_existing_account_and_revoke_auth_key_call</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values:<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
        <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
        owners,
        num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );

    // Rotate the <a href="account.md#0x1_account">account</a>'s auth key <b>to</b> 0x0, which effectively revokes control via auth key.
    <b>let</b> multisig_address = address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="multisig_account.md#0x1_multisig_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);
    // This also needs <b>to</b> revoke <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> or rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> that <b>exists</b> for the <a href="account.md#0x1_account">account</a> <b>to</b>
    // completely remove all access <b>to</b> the <a href="account.md#0x1_account">account</a>.
    <b>if</b> (<a href="account.md#0x1_account_is_signer_capability_offered">account::is_signer_capability_offered</a>(multisig_address)) {
        <a href="account.md#0x1_account_revoke_any_signer_capability">account::revoke_any_signer_capability</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    };
    <b>if</b> (<a href="account.md#0x1_account_is_rotation_capability_offered">account::is_rotation_capability_offered</a>(multisig_address)) {
        <a href="account.md#0x1_account_revoke_any_rotation_capability">account::revoke_any_rotation_capability</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account_and_revoke_auth_key"></a>

## Function `create_with_existing_account_and_revoke_auth_key`

Creates a new multisig account on top of an existing account and immediately rotate the origin auth key to 0x0.

Note: If the original account is a resource account, this does not revoke all control over it as if any
SignerCapability of the resource account still exists, it can still be used to generate the signer for the
account.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key">create_with_existing_account_and_revoke_auth_key</a>(multisig_address: <b>address</b>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, create_multisig_account_signed_message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_existing_account_and_revoke_auth_key">create_with_existing_account_and_revoke_auth_key</a>(
    multisig_address: <b>address</b>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    account_scheme: u8,
    account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    create_multisig_account_signed_message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    // Verify that the `<a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationMessage">MultisigAccountCreationMessage</a>` <b>has</b> the right information and is signed by the <a href="account.md#0x1_account">account</a>
    // owner's key.
    <b>let</b> proof_challenge = <a href="multisig_account.md#0x1_multisig_account_MultisigAccountCreationWithAuthKeyRevocationMessage">MultisigAccountCreationWithAuthKeyRevocationMessage</a> {
        <a href="chain_id.md#0x1_chain_id">chain_id</a>: <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(),
        account_address: multisig_address,
        sequence_number: <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(multisig_address),
        owners,
        num_signatures_required,
    };
    <a href="account.md#0x1_account_verify_signed_message">account::verify_signed_message</a>(
        multisig_address,
        account_scheme,
        account_public_key,
        create_multisig_account_signed_message,
        proof_challenge,
    );

    // We create the <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> for the multisig <a href="account.md#0x1_account">account</a> here since this is required <b>to</b> add the <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> resource
    // This should be safe and authorized because we have verified the signed message from the existing <a href="account.md#0x1_account">account</a>
    // that authorizes creating a multisig <a href="account.md#0x1_account">account</a> <b>with</b> the specified owners and signature threshold.
    <b>let</b> <a href="multisig_account.md#0x1_multisig_account">multisig_account</a> = &<a href="create_signer.md#0x1_create_signer">create_signer</a>(multisig_address);
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
        <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
        owners,
        num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;SignerCapability&gt;(),
        metadata_keys,
        metadata_values,
    );

    // Rotate the <a href="account.md#0x1_account">account</a>'s auth key <b>to</b> 0x0, which effectively revokes control via auth key.
    <b>let</b> multisig_address = address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="account.md#0x1_account_rotate_authentication_key_internal">account::rotate_authentication_key_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="multisig_account.md#0x1_multisig_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);
    // This also needs <b>to</b> revoke <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> or rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> that <b>exists</b> for the <a href="account.md#0x1_account">account</a> <b>to</b>
    // completely remove all access <b>to</b> the <a href="account.md#0x1_account">account</a>.
    <b>if</b> (<a href="account.md#0x1_account_is_signer_capability_offered">account::is_signer_capability_offered</a>(multisig_address)) {
        <a href="account.md#0x1_account_revoke_any_signer_capability">account::revoke_any_signer_capability</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    };
    <b>if</b> (<a href="account.md#0x1_account_is_rotation_capability_offered">account::is_rotation_capability_offered</a>(multisig_address)) {
        <a href="account.md#0x1_account_revoke_any_rotation_capability">account::revoke_any_rotation_capability</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_create"></a>

## Function `create`

Creates a new multisig account and add the signer as a single owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create">create</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create">create</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners">create_with_owners</a>(owner, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], num_signatures_required, metadata_keys, metadata_values);
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners"></a>

## Function `create_with_owners`

Creates a new multisig account with the specified additional owner list and signatures required.

@param additional_owners The owner account who calls this function cannot be in the additional_owners and there
cannot be any duplicate owners in the list.
@param num_signatures_required The number of signatures required to execute a transaction. Must be at least 1 and
at most the total number of owners.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners">create_with_owners</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners">create_with_owners</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    additional_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> (<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, multisig_signer_cap) = <a href="multisig_account.md#0x1_multisig_account_create_multisig_account">create_multisig_account</a>(owner);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> additional_owners, address_of(owner));
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
        &<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
        additional_owners,
        num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(multisig_signer_cap),
        metadata_keys,
        metadata_values,
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_then_remove_bootstrapper"></a>

## Function `create_with_owners_then_remove_bootstrapper`

Like <code>create_with_owners</code>, but removes the calling account after creation.

This is for creating a vanity multisig account from a bootstrapping account that should not
be an owner after the vanity multisig address has been secured.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners_then_remove_bootstrapper">create_with_owners_then_remove_bootstrapper</a>(bootstrapper: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners_then_remove_bootstrapper">create_with_owners_then_remove_bootstrapper</a>(
    bootstrapper: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> bootstrapper_address = address_of(bootstrapper);
    <a href="multisig_account.md#0x1_multisig_account_create_with_owners">create_with_owners</a>(
        bootstrapper,
        owners,
        num_signatures_required,
        metadata_keys,
        metadata_values
    );
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        <a href="multisig_account.md#0x1_multisig_account_get_next_multisig_account_address">get_next_multisig_account_address</a>(bootstrapper_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[bootstrapper_address],
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_internal"></a>

## Function `create_with_owners_internal`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, num_signatures_required: u64, multisig_account_signer_cap: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>&gt;, metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_with_owners_internal">create_with_owners_internal</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    num_signatures_required: u64,
    multisig_account_signer_cap: Option&lt;SignerCapability&gt;,
    metadata_keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    metadata_values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_accounts_enabled">features::multisig_accounts_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="multisig_account.md#0x1_multisig_account_EMULTISIG_ACCOUNTS_NOT_ENABLED_YET">EMULTISIG_ACCOUNTS_NOT_ENABLED_YET</a>));
    <b>assert</b>!(
        num_signatures_required &gt; 0 && <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a> &lt;= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&owners),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED">EINVALID_SIGNATURES_REQUIRED</a>),
    );

    <b>let</b> multisig_address = address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_validate_owners">validate_owners</a>(&owners, multisig_address);
    <b>move_to</b>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
        owners,
        num_signatures_required,
        transactions: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;u64, <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a>&gt;(),
        metadata: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(),
        // First transaction will start at id 1 instead of 0.
        last_executed_sequence_number: 0,
        next_sequence_number: 1,
        signer_cap: multisig_account_signer_cap,
        add_owners_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_AddOwnersEvent">AddOwnersEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        remove_owners_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_RemoveOwnersEvent">RemoveOwnersEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        update_signature_required_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequiredEvent">UpdateSignaturesRequiredEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        create_transaction_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_CreateTransactionEvent">CreateTransactionEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        vote_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_VoteEvent">VoteEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        execute_rejected_transaction_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransactionEvent">ExecuteRejectedTransactionEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        execute_transaction_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceededEvent">TransactionExecutionSucceededEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        transaction_execution_failed_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailedEvent">TransactionExecutionFailedEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        metadata_updated_events: new_event_handle&lt;<a href="multisig_account.md#0x1_multisig_account_MetadataUpdatedEvent">MetadataUpdatedEvent</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
    });

    <a href="multisig_account.md#0x1_multisig_account_update_metadata_internal">update_metadata_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, metadata_keys, metadata_values, <b>false</b>);
}
</code></pre>



</details>

<a id="0x1_multisig_account_add_owner"></a>

## Function `add_owner`

Similar to add_owners, but only allow adding one owner.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owner">add_owner</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owner">add_owner</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owner: <b>address</b>) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_add_owners">add_owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[new_owner]);
}
</code></pre>



</details>

<a id="0x1_multisig_account_add_owners"></a>

## Function `add_owners`

Add new owners to the multisig account. This can only be invoked by the multisig account itself, through the
proposal flow.

Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the owners list.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owners">add_owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owners">add_owners</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        new_owners,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_add_owners_and_update_signatures_required"></a>

## Function `add_owners_and_update_signatures_required`

Add owners then update number of signatures required, in a single operation.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owners_and_update_signatures_required">add_owners_and_update_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_owners_and_update_signatures_required">add_owners_and_update_signatures_required</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    new_num_signatures_required: u64
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        new_owners,
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(new_num_signatures_required)
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_remove_owner"></a>

## Function `remove_owner`

Similar to remove_owners, but only allow removing one owner.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_owner">remove_owner</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_to_remove: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_owner">remove_owner</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner_to_remove: <b>address</b>) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_remove_owners">remove_owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[owner_to_remove]);
}
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


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_owners">remove_owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_owners">remove_owners</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        owners_to_remove,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owner"></a>

## Function `swap_owner`

Swap an owner in for an old one, without changing required signatures.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owner">swap_owner</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_swap_in: <b>address</b>, to_swap_out: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owner">swap_owner</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    to_swap_in: <b>address</b>,
    to_swap_out: <b>address</b>
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[to_swap_in],
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[to_swap_out],
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owners"></a>

## Function `swap_owners`

Swap owners in and out, without changing required signatures.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owners">swap_owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_swap_in: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, to_swap_out: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owners">swap_owners</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    to_swap_in: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    to_swap_out: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        to_swap_in,
        to_swap_out,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_swap_owners_and_update_signatures_required"></a>

## Function `swap_owners_and_update_signatures_required`

Swap owners in and out, updating number of required signatures.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owners_and_update_signatures_required">swap_owners_and_update_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_swap_owners_and_update_signatures_required">swap_owners_and_update_signatures_required</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    new_num_signatures_required: u64
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        new_owners,
        owners_to_remove,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(new_num_signatures_required)
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_update_signatures_required"></a>

## Function `update_signatures_required`

Update the number of signatures required to execute transaction in the specified multisig account.

This can only be invoked by the multisig account itself, through the proposal flow.
Note that this function is not public so it can only be invoked directly instead of via a module or script. This
ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to
maliciously alter the number of signatures required.


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_signatures_required">update_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_num_signatures_required: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_signatures_required">update_signatures_required</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_num_signatures_required: u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
        address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>),
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(new_num_signatures_required)
    );
}
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


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_metadata">update_metadata</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_metadata">update_metadata</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_update_metadata_internal">update_metadata_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, keys, values, <b>true</b>);
}
</code></pre>



</details>

<a id="0x1_multisig_account_update_metadata_internal"></a>

## Function `update_metadata_internal`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_metadata_internal">update_metadata_internal</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, emit_event: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_metadata_internal">update_metadata_internal</a>(
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    keys: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    emit_event: bool,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> num_attributes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&keys);
    <b>assert</b>!(
        num_attributes == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&values),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH">ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH</a>),
    );

    <b>let</b> multisig_address = address_of(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(multisig_address);
    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(multisig_address);
    <b>let</b> old_metadata = multisig_account_resource.metadata;
    multisig_account_resource.metadata = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;String, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
    <b>let</b> metadata = &<b>mut</b> multisig_account_resource.metadata;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_attributes) {
        <b>let</b> key = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&keys, i);
        <b>let</b> value = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&values, i);
        <b>assert</b>!(
            !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(metadata, &key),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EDUPLICATE_METADATA_KEY">EDUPLICATE_METADATA_KEY</a>),
        );

        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(metadata, key, value);
        i = i + 1;
    };

    <b>if</b> (emit_event) {
        <b>if</b> (std::features::module_event_migration_enabled()) {
            emit(
                <a href="multisig_account.md#0x1_multisig_account_MetadataUpdated">MetadataUpdated</a> {
                    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: multisig_address,
                    old_metadata,
                    new_metadata: multisig_account_resource.metadata,
                }
            )
        } <b>else</b> {
            emit_event(
                &<b>mut</b> multisig_account_resource.metadata_updated_events,
                <a href="multisig_account.md#0x1_multisig_account_MetadataUpdatedEvent">MetadataUpdatedEvent</a> {
                    old_metadata,
                    new_metadata: multisig_account_resource.metadata,
                }
            );
        };
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_transaction"></a>

## Function `create_transaction`

Create a multisig transaction, which will have one approval initially (from the creator).


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_transaction">create_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_transaction">create_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&payload) &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_CANNOT_BE_EMPTY">EPAYLOAD_CANNOT_BE_EMPTY</a>));

    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);

    <b>let</b> creator = address_of(owner);
    <b>let</b> transaction = <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a> {
        payload: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(payload),
        payload_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(),
        votes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, bool&gt;(),
        creator,
        creation_time_secs: now_seconds(),
    };
    <a href="multisig_account.md#0x1_multisig_account_add_transaction">add_transaction</a>(creator, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, transaction);
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_transaction_with_hash"></a>

## Function `create_transaction_with_hash`

Create a multisig transaction with a transaction hash instead of the full payload.
This means the payload will be stored off chain for gas saving. Later, during execution, the executor will need
to provide the full payload, which will be validated against the hash stored on-chain.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_transaction_with_hash">create_transaction_with_hash</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, payload_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_transaction_with_hash">create_transaction_with_hash</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    payload_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    // Payload <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> is a sha3-256 <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, so it must be exactly 32 bytes.
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&payload_hash) == 32, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_PAYLOAD_HASH">EINVALID_PAYLOAD_HASH</a>));

    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);

    <b>let</b> creator = address_of(owner);
    <b>let</b> transaction = <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a> {
        payload: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;(),
        payload_hash: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(payload_hash),
        votes: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, bool&gt;(),
        creator,
        creation_time_secs: now_seconds(),
    };
    <a href="multisig_account.md#0x1_multisig_account_add_transaction">add_transaction</a>(creator, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, transaction);
}
</code></pre>



</details>

<a id="0x1_multisig_account_approve_transaction"></a>

## Function `approve_transaction`

Approve a multisig transaction.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_approve_transaction">approve_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_approve_transaction">approve_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, <b>true</b>);
}
</code></pre>



</details>

<a id="0x1_multisig_account_reject_transaction"></a>

## Function `reject_transaction`

Reject a multisig transaction.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_reject_transaction">reject_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_reject_transaction">reject_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, <b>false</b>);
}
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transanction"></a>

## Function `vote_transanction`

Generic function that can be used to either approve or reject a multisig transaction
Retained for backward compatibility: the function with the typographical error in its name
will continue to be an accessible entry point.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, approved: bool) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner_internal">assert_is_owner_internal</a>(owner, multisig_account_resource);

    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&multisig_account_resource.transactions, sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="multisig_account.md#0x1_multisig_account_ETRANSACTION_NOT_FOUND">ETRANSACTION_NOT_FOUND</a>),
    );
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> multisig_account_resource.transactions, sequence_number);
    <b>let</b> votes = &<b>mut</b> transaction.votes;
    <b>let</b> owner_addr = address_of(owner);

    <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(votes, &owner_addr)) {
        *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(votes, &owner_addr) = approved;
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(votes, owner_addr, approved);
    };

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="multisig_account.md#0x1_multisig_account_Vote">Vote</a> {
                <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
                owner: owner_addr,
                sequence_number,
                approved,
            }
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> multisig_account_resource.vote_events,
            <a href="multisig_account.md#0x1_multisig_account_VoteEvent">VoteEvent</a> {
                owner: owner_addr,
                sequence_number,
                approved,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transaction"></a>

## Function `vote_transaction`

Generic function that can be used to either approve or reject a multisig transaction


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transaction">vote_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transaction">vote_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, approved: bool) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED">EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED</a>));
    <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, approved);
}
</code></pre>



</details>

<a id="0x1_multisig_account_vote_transactions"></a>

## Function `vote_transactions`

Generic function that can be used to either approve or reject a batch of transactions within a specified range.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transactions">vote_transactions</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, starting_sequence_number: u64, final_sequence_number: u64, approved: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote_transactions">vote_transactions</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, starting_sequence_number: u64, final_sequence_number: u64, approved: bool) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED">EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED</a>));
    <b>let</b> sequence_number = starting_sequence_number;
    <b>while</b>(sequence_number &lt;= final_sequence_number) {
        <a href="multisig_account.md#0x1_multisig_account_vote_transanction">vote_transanction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, approved);
        sequence_number = sequence_number + 1;
    }
}
</code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transaction"></a>

## Function `execute_rejected_transaction`

Remove the next transaction if it has sufficient owner rejections.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_execute_rejected_transaction">execute_rejected_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_execute_rejected_transaction">execute_rejected_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);

    <b>let</b> sequence_number = <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1;
    <b>let</b> owner_addr = address_of(owner);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>()) {
        // Implicitly vote for rejection <b>if</b> the owner <b>has</b> not voted for rejection yet.
        <b>if</b> (!<a href="multisig_account.md#0x1_multisig_account_has_voted_for_rejection">has_voted_for_rejection</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, owner_addr)) {
            <a href="multisig_account.md#0x1_multisig_account_reject_transaction">reject_transaction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);
        }
    };

    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> (_, num_rejections) = <a href="multisig_account.md#0x1_multisig_account_remove_executed_transaction">remove_executed_transaction</a>(multisig_account_resource);
    <b>assert</b>!(
        num_rejections &gt;= multisig_account_resource.num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_REJECTIONS">ENOT_ENOUGH_REJECTIONS</a>),
    );

    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransaction">ExecuteRejectedTransaction</a> {
                <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
                sequence_number,
                num_rejections,
                executor: address_of(owner),
            }
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> multisig_account_resource.execute_rejected_transaction_events,
            <a href="multisig_account.md#0x1_multisig_account_ExecuteRejectedTransactionEvent">ExecuteRejectedTransactionEvent</a> {
                sequence_number,
                num_rejections,
                executor: owner_addr,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transactions"></a>

## Function `execute_rejected_transactions`

Remove the next transactions until the final_sequence_number if they have sufficient owner rejections.


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_execute_rejected_transactions">execute_rejected_transactions</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, final_sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_execute_rejected_transactions">execute_rejected_transactions</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    final_sequence_number: u64,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED">EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED</a>));
    <b>assert</b>!(<a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) &lt; final_sequence_number, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>));
    <b>assert</b>!(final_sequence_number &lt; <a href="multisig_account.md#0x1_multisig_account_next_sequence_number">next_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>));
    <b>while</b>(<a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) &lt; final_sequence_number) {
        <a href="multisig_account.md#0x1_multisig_account_execute_rejected_transaction">execute_rejected_transaction</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    }
}
</code></pre>



</details>

<a id="0x1_multisig_account_validate_multisig_transaction"></a>

## Function `validate_multisig_transaction`

Called by the VM as part of transaction prologue, which is invoked during mempool transaction validation and as
the first step of transaction execution.

Transaction payload is optional if it's already stored on chain for the transaction.


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_validate_multisig_transaction">validate_multisig_transaction</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_validate_multisig_transaction">validate_multisig_transaction</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> sequence_number = <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1;
    <a href="multisig_account.md#0x1_multisig_account_assert_transaction_exists">assert_transaction_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>()) {
        <b>assert</b>!(
            <a href="multisig_account.md#0x1_multisig_account_can_execute">can_execute</a>(address_of(owner), <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_APPROVALS">ENOT_ENOUGH_APPROVALS</a>),
        );
    }
    <b>else</b> {
        <b>assert</b>!(
            <a href="multisig_account.md#0x1_multisig_account_can_be_executed">can_be_executed</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_APPROVALS">ENOT_ENOUGH_APPROVALS</a>),
        );
    };

    // If the transaction payload is not stored on chain, verify that the provided payload matches the hashes stored
    // on chain.
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&multisig_account_resource.transactions, sequence_number);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&transaction.payload_hash)) {
        <b>let</b> payload_hash = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&transaction.payload_hash);
        <b>assert</b>!(
            sha3_256(payload) == *payload_hash,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH_HASH">EPAYLOAD_DOES_NOT_MATCH_HASH</a>),
        );
    };

    // If the transaction payload is stored on chain and there is a provided payload,
    // verify that the provided payload matches the stored payload.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_abort_if_multisig_payload_mismatch_enabled">features::abort_if_multisig_payload_mismatch_enabled</a>()
        && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&transaction.payload)
        && !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&payload)
    ) {
        <b>let</b> stored_payload = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&transaction.payload);
        <b>assert</b>!(
            payload == *stored_payload,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH">EPAYLOAD_DOES_NOT_MATCH</a>),
        );
    }
}
</code></pre>



</details>

<a id="0x1_multisig_account_successful_transaction_execution_cleanup"></a>

## Function `successful_transaction_execution_cleanup`

Post-execution cleanup for a successful multisig transaction execution.
This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_successful_transaction_execution_cleanup">successful_transaction_execution_cleanup</a>(executor: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_successful_transaction_execution_cleanup">successful_transaction_execution_cleanup</a>(
    executor: <b>address</b>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> num_approvals = <a href="multisig_account.md#0x1_multisig_account_transaction_execution_cleanup_common">transaction_execution_cleanup_common</a>(executor, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceeded">TransactionExecutionSucceeded</a> {
                <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                executor,
            }
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> multisig_account_resource.execute_transaction_events,
            <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionSucceededEvent">TransactionExecutionSucceededEvent</a> {
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                executor,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_failed_transaction_execution_cleanup"></a>

## Function `failed_transaction_execution_cleanup`

Post-execution cleanup for a failed multisig transaction execution.
This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_failed_transaction_execution_cleanup">failed_transaction_execution_cleanup</a>(executor: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, execution_error: <a href="multisig_account.md#0x1_multisig_account_ExecutionError">multisig_account::ExecutionError</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_failed_transaction_execution_cleanup">failed_transaction_execution_cleanup</a>(
    executor: <b>address</b>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    transaction_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    execution_error: <a href="multisig_account.md#0x1_multisig_account_ExecutionError">ExecutionError</a>,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> num_approvals = <a href="multisig_account.md#0x1_multisig_account_transaction_execution_cleanup_common">transaction_execution_cleanup_common</a>(executor, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailed">TransactionExecutionFailed</a> {
                <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
                executor,
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                execution_error,
            }
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> multisig_account_resource.transaction_execution_failed_events,
            <a href="multisig_account.md#0x1_multisig_account_TransactionExecutionFailedEvent">TransactionExecutionFailedEvent</a> {
                executor,
                sequence_number: multisig_account_resource.last_executed_sequence_number,
                transaction_payload,
                num_approvals,
                execution_error,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_transaction_execution_cleanup_common"></a>

## Function `transaction_execution_cleanup_common`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_transaction_execution_cleanup_common">transaction_execution_cleanup_common</a>(executor: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_transaction_execution_cleanup_common">transaction_execution_cleanup_common</a>(executor: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64 <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> sequence_number = <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) + 1;
    <b>let</b> implicit_approval = !<a href="multisig_account.md#0x1_multisig_account_has_voted_for_approval">has_voted_for_approval</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, executor);

    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> (num_approvals, _) = <a href="multisig_account.md#0x1_multisig_account_remove_executed_transaction">remove_executed_transaction</a>(multisig_account_resource);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>() && implicit_approval) {
        <b>if</b> (std::features::module_event_migration_enabled()) {
            emit(
                <a href="multisig_account.md#0x1_multisig_account_Vote">Vote</a> {
                    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>,
                    owner: executor,
                    sequence_number,
                    approved: <b>true</b>,
                }
            );
        } <b>else</b> {
            emit_event(
                &<b>mut</b> multisig_account_resource.vote_events,
                <a href="multisig_account.md#0x1_multisig_account_VoteEvent">VoteEvent</a> {
                    owner: executor,
                    sequence_number,
                    approved: <b>true</b>,
                }
            );
        };
        num_approvals = num_approvals + 1;
    };

    num_approvals
}
</code></pre>



</details>

<a id="0x1_multisig_account_remove_executed_transaction"></a>

## Function `remove_executed_transaction`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_executed_transaction">remove_executed_transaction</a>(multisig_account_resource: &<b>mut</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">multisig_account::MultisigAccount</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_remove_executed_transaction">remove_executed_transaction</a>(multisig_account_resource: &<b>mut</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>): (u64, u64) {
    <b>let</b> sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(&<b>mut</b> multisig_account_resource.transactions, sequence_number);
    multisig_account_resource.last_executed_sequence_number = sequence_number;
    <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections_internal">num_approvals_and_rejections_internal</a>(&multisig_account_resource.owners, &transaction)
}
</code></pre>



</details>

<a id="0x1_multisig_account_add_transaction"></a>

## Function `add_transaction`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_transaction">add_transaction</a>(creator: <b>address</b>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, transaction: <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_add_transaction">add_transaction</a>(
    creator: <b>address</b>,
    <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>,
    transaction: <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a>
) {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_multisig_v2_enhancement_feature_enabled">features::multisig_v2_enhancement_feature_enabled</a>()) {
        <b>assert</b>!(
            <a href="multisig_account.md#0x1_multisig_account_available_transaction_queue_capacity">available_transaction_queue_capacity</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>) &gt; 0,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_EMAX_PENDING_TRANSACTIONS_EXCEEDED">EMAX_PENDING_TRANSACTIONS_EXCEEDED</a>)
        );
    };

    <b>let</b> multisig_account_resource = <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);

    // The transaction creator also automatically votes for the transaction.
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> transaction.votes, creator, <b>true</b>);

    <b>let</b> sequence_number = multisig_account_resource.next_sequence_number;
    multisig_account_resource.next_sequence_number = sequence_number + 1;
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> multisig_account_resource.transactions, sequence_number, transaction);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        emit(
            <a href="multisig_account.md#0x1_multisig_account_CreateTransaction">CreateTransaction</a> { <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, creator, sequence_number, transaction }
        );
    } <b>else</b> {
        emit_event(
            &<b>mut</b> multisig_account_resource.create_transaction_events,
            <a href="multisig_account.md#0x1_multisig_account_CreateTransactionEvent">CreateTransactionEvent</a> { creator, sequence_number, transaction },
        );
    };
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account"></a>

## Function `create_multisig_account`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_multisig_account">create_multisig_account</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_multisig_account">create_multisig_account</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, SignerCapability) {
    <b>let</b> owner_nonce = <a href="account.md#0x1_account_get_sequence_number">account::get_sequence_number</a>(address_of(owner));
    <b>let</b> (multisig_signer, multisig_signer_cap) =
        <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(owner, <a href="multisig_account.md#0x1_multisig_account_create_multisig_account_seed">create_multisig_account_seed</a>(to_bytes(&owner_nonce)));
    // Register the <a href="account.md#0x1_account">account</a> <b>to</b> receive APT <b>as</b> this is not done by default <b>as</b> part of the resource <a href="account.md#0x1_account">account</a> creation
    // flow.
    <b>if</b> (!<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(address_of(&multisig_signer))) {
        <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&multisig_signer);
    };

    (multisig_signer, multisig_signer_cap)
}
</code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account_seed"></a>

## Function `create_multisig_account_seed`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_multisig_account_seed">create_multisig_account_seed</a>(seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_create_multisig_account_seed">create_multisig_account_seed</a>(seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    // Generate a seed that will be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> that hosts the multisig <a href="account.md#0x1_account">account</a>.
    <b>let</b> multisig_account_seed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> multisig_account_seed, <a href="multisig_account.md#0x1_multisig_account_DOMAIN_SEPARATOR">DOMAIN_SEPARATOR</a>);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> multisig_account_seed, seed);

    multisig_account_seed
}
</code></pre>



</details>

<a id="0x1_multisig_account_validate_owners"></a>

## Function `validate_owners`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_validate_owners">validate_owners</a>(owners: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_validate_owners">validate_owners</a>(owners: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>) {
    <b>let</b> distinct_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(owners, |owner| {
        <b>let</b> owner = *owner;
        <b>assert</b>!(owner != <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF">EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF</a>));
        <b>let</b> (found, _) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&distinct_owners, &owner);
        <b>assert</b>!(!found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EDUPLICATE_OWNER">EDUPLICATE_OWNER</a>));
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> distinct_owners, owner);
    });
}
</code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner_internal"></a>

## Function `assert_is_owner_internal`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_is_owner_internal">assert_is_owner_internal</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">multisig_account::MultisigAccount</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_is_owner_internal">assert_is_owner_internal</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: &<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>) {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>.owners, &address_of(owner)),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="multisig_account.md#0x1_multisig_account_ENOT_OWNER">ENOT_OWNER</a>),
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner"></a>

## Function `assert_is_owner`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_is_owner">assert_is_owner</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <a href="multisig_account.md#0x1_multisig_account_assert_is_owner_internal">assert_is_owner_internal</a>(owner, multisig_account_resource);
}
</code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections_internal"></a>

## Function `num_approvals_and_rejections_internal`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections_internal">num_approvals_and_rejections_internal</a>(owners: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, transaction: &<a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections_internal">num_approvals_and_rejections_internal</a>(owners: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, transaction: &<a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">MultisigTransaction</a>): (u64, u64) {
    <b>let</b> num_approvals = 0;
    <b>let</b> num_rejections = 0;

    <b>let</b> votes = &transaction.votes;
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(owners, |owner| {
        <b>if</b> (<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(votes, owner)) {
            <b>if</b> (*<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(votes, owner)) {
                num_approvals = num_approvals + 1;
            } <b>else</b> {
                num_rejections = num_rejections + 1;
            };
        }
    });

    (num_approvals, num_rejections)
}
</code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections"></a>

## Function `num_approvals_and_rejections`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections">num_approvals_and_rejections</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): (u64, u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&multisig_account_resource.transactions, sequence_number);
    <a href="multisig_account.md#0x1_multisig_account_num_approvals_and_rejections_internal">num_approvals_and_rejections_internal</a>(&multisig_account_resource.owners, transaction)
}
</code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_approval"></a>

## Function `has_voted_for_approval`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_has_voted_for_approval">has_voted_for_approval</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_has_voted_for_approval">has_voted_for_approval</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> (voted, vote) = <a href="multisig_account.md#0x1_multisig_account_vote">vote</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, owner);
    voted && vote
}
</code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_rejection"></a>

## Function `has_voted_for_rejection`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_has_voted_for_rejection">has_voted_for_rejection</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_has_voted_for_rejection">has_voted_for_rejection</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): bool <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> (voted, vote) = <a href="multisig_account.md#0x1_multisig_account_vote">vote</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>, sequence_number, owner);
    voted && !vote
}
</code></pre>



</details>

<a id="0x1_multisig_account_assert_multisig_account_exists"></a>

## Function `assert_multisig_account_exists`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>) {
    <b>assert</b>!(<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_EACCOUNT_NOT_MULTISIG">EACCOUNT_NOT_MULTISIG</a>));
}
</code></pre>



</details>

<a id="0x1_multisig_account_assert_valid_sequence_number"></a>

## Function `assert_valid_sequence_number`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_valid_sequence_number">assert_valid_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>assert</b>!(
        sequence_number &gt; 0 && sequence_number &lt; multisig_account_resource.next_sequence_number,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SEQUENCE_NUMBER">EINVALID_SEQUENCE_NUMBER</a>),
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_assert_transaction_exists"></a>

## Function `assert_transaction_exists`



<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_transaction_exists">assert_transaction_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_assert_transaction_exists">assert_transaction_exists</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <b>let</b> multisig_account_resource = <b>borrow_global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&multisig_account_resource.transactions, sequence_number),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="multisig_account.md#0x1_multisig_account_ETRANSACTION_NOT_FOUND">ETRANSACTION_NOT_FOUND</a>),
    );
}
</code></pre>



</details>

<a id="0x1_multisig_account_update_owner_schema"></a>

## Function `update_owner_schema`

Add new owners, remove owners to remove, update signatures required.


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(multisig_address: <b>address</b>, new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, optional_new_num_signatures_required: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="multisig_account.md#0x1_multisig_account_update_owner_schema">update_owner_schema</a>(
    multisig_address: <b>address</b>,
    new_owners: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    owners_to_remove: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    optional_new_num_signatures_required: Option&lt;u64&gt;,
) <b>acquires</b> <a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a> {
    <a href="multisig_account.md#0x1_multisig_account_assert_multisig_account_exists">assert_multisig_account_exists</a>(multisig_address);
    <b>let</b> multisig_account_ref_mut =
        <b>borrow_global_mut</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(multisig_address);
    // Verify no overlap between new owners and owners <b>to</b> remove.
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&new_owners, |new_owner_ref| {
        <b>assert</b>!(
            !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&owners_to_remove, new_owner_ref),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP">EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP</a>)
        )
    });
    // If new owners provided, try <b>to</b> add them and emit an <a href="event.md#0x1_event">event</a>.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&new_owners) &gt; 0) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> multisig_account_ref_mut.owners, new_owners);
        <a href="multisig_account.md#0x1_multisig_account_validate_owners">validate_owners</a>(
            &multisig_account_ref_mut.owners,
            multisig_address
        );
        <b>if</b> (std::features::module_event_migration_enabled()) {
            emit(<a href="multisig_account.md#0x1_multisig_account_AddOwners">AddOwners</a> { <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: multisig_address, owners_added: new_owners });
        } <b>else</b> {
            emit_event(
                &<b>mut</b> multisig_account_ref_mut.add_owners_events,
                <a href="multisig_account.md#0x1_multisig_account_AddOwnersEvent">AddOwnersEvent</a> { owners_added: new_owners }
            );
        };
    };
    // If owners <b>to</b> remove provided, try <b>to</b> remove them.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&owners_to_remove) &gt; 0) {
        <b>let</b> owners_ref_mut = &<b>mut</b> multisig_account_ref_mut.owners;
        <b>let</b> owners_removed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&owners_to_remove, |owner_to_remove_ref| {
            <b>let</b> (found, index) =
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(owners_ref_mut, owner_to_remove_ref);
            <b>if</b> (found) {
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(
                    &<b>mut</b> owners_removed,
                    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(owners_ref_mut, index)
                );
            }
        });
        // Only emit <a href="event.md#0x1_event">event</a> <b>if</b> owner(s) actually removed.
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&owners_removed) &gt; 0) {
            <b>if</b> (std::features::module_event_migration_enabled()) {
                emit(
                    <a href="multisig_account.md#0x1_multisig_account_RemoveOwners">RemoveOwners</a> { <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: multisig_address, owners_removed }
                );
            } <b>else</b> {
                emit_event(
                    &<b>mut</b> multisig_account_ref_mut.remove_owners_events,
                    <a href="multisig_account.md#0x1_multisig_account_RemoveOwnersEvent">RemoveOwnersEvent</a> { owners_removed }
                );
            };
        }
    };
    // If new signature count provided, try <b>to</b> <b>update</b> count.
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&optional_new_num_signatures_required)) {
        <b>let</b> new_num_signatures_required =
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> optional_new_num_signatures_required);
        <b>assert</b>!(
            new_num_signatures_required &gt; 0,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="multisig_account.md#0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED">EINVALID_SIGNATURES_REQUIRED</a>)
        );
        <b>let</b> old_num_signatures_required =
            multisig_account_ref_mut.num_signatures_required;
        // Only <b>apply</b> <b>update</b> and emit <a href="event.md#0x1_event">event</a> <b>if</b> a change indicated.
        <b>if</b> (new_num_signatures_required != old_num_signatures_required) {
            multisig_account_ref_mut.num_signatures_required =
                new_num_signatures_required;
            <b>if</b> (std::features::module_event_migration_enabled()) {
                emit(
                    <a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequired">UpdateSignaturesRequired</a> {
                        <a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: multisig_address,
                        old_num_signatures_required,
                        new_num_signatures_required,
                    }
                );
            } <b>else</b> {
                emit_event(
                    &<b>mut</b> multisig_account_ref_mut.update_signature_required_events,
                    <a href="multisig_account.md#0x1_multisig_account_UpdateSignaturesRequiredEvent">UpdateSignaturesRequiredEvent</a> {
                        old_num_signatures_required,
                        new_num_signatures_required,
                    }
                );
            }
        }
    };
    // Verify number of owners.
    <b>let</b> num_owners = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&multisig_account_ref_mut.owners);
    <b>assert</b>!(
        num_owners &gt;= multisig_account_ref_mut.num_signatures_required,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="multisig_account.md#0x1_multisig_account_ENOT_ENOUGH_OWNERS">ENOT_ENOUGH_OWNERS</a>)
    );
}
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


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_metadata">metadata</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>ensures</b> result == <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).metadata;
</code></pre>



<a id="@Specification_1_num_signatures_required"></a>

### Function `num_signatures_required`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_num_signatures_required">num_signatures_required</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>ensures</b> result == <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).num_signatures_required;
</code></pre>



<a id="@Specification_1_owners"></a>

### Function `owners`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_owners">owners</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>ensures</b> result == <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>).owners;
</code></pre>



<a id="@Specification_1_get_transaction"></a>

### Function `get_transaction`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_transaction">get_transaction</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64): <a href="multisig_account.md#0x1_multisig_account_MultisigTransaction">multisig_account::MultisigTransaction</a>
</code></pre>




<pre><code><b>let</b> multisig_account_resource = <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> sequence_number == 0 || sequence_number &gt;= multisig_account_resource.next_sequence_number;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(multisig_account_resource.transactions, sequence_number);
<b>ensures</b> result == <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(multisig_account_resource.transactions, sequence_number);
</code></pre>



<a id="@Specification_1_get_next_transaction_payload"></a>

### Function `get_next_transaction_payload`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_transaction_payload">get_next_transaction_payload</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, provided_payload: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>let</b> multisig_account_resource = <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>let</b> sequence_number = multisig_account_resource.last_executed_sequence_number + 1;
<b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(multisig_account_resource.transactions, sequence_number);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> multisig_account_resource.last_executed_sequence_number + 1 &gt; MAX_U64;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(multisig_account_resource.transactions, sequence_number);
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(transaction.payload) ==&gt; result == provided_payload;
</code></pre>



<a id="@Specification_1_get_next_multisig_account_address"></a>

### Function `get_next_multisig_account_address`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_get_next_multisig_account_address">get_next_multisig_account_address</a>(creator: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(creator);
<b>let</b> owner_nonce = <b>global</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(creator).sequence_number;
</code></pre>



<a id="@Specification_1_last_resolved_sequence_number"></a>

### Function `last_resolved_sequence_number`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_last_resolved_sequence_number">last_resolved_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>




<pre><code><b>let</b> multisig_account_resource = <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>ensures</b> result == multisig_account_resource.last_executed_sequence_number;
</code></pre>



<a id="@Specification_1_next_sequence_number"></a>

### Function `next_sequence_number`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_next_sequence_number">next_sequence_number</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>): u64
</code></pre>




<pre><code><b>let</b> multisig_account_resource = <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>ensures</b> result == multisig_account_resource.next_sequence_number;
</code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="multisig_account.md#0x1_multisig_account_vote">vote</a>(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>: <b>address</b>, sequence_number: u64, owner: <b>address</b>): (bool, bool)
</code></pre>




<pre><code><b>let</b> multisig_account_resource = <b>global</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="multisig_account.md#0x1_multisig_account_MultisigAccount">MultisigAccount</a>&gt;(<a href="multisig_account.md#0x1_multisig_account">multisig_account</a>);
<b>aborts_if</b> sequence_number == 0 || sequence_number &gt;= multisig_account_resource.next_sequence_number;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(multisig_account_resource.transactions, sequence_number);
<b>let</b> transaction = <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(multisig_account_resource.transactions, sequence_number);
<b>let</b> votes = transaction.votes;
<b>let</b> voted = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(votes, owner);
<b>let</b> vote = voted && <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(votes, owner);
<b>ensures</b> result_1 == voted;
<b>ensures</b> result_2 == vote;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
