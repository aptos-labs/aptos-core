
<a id="0x1_multisig_account"></a>

# Module `0x1::multisig_account`

Enhanced multisig account standard on Aptos. This is different from the native multisig scheme support enforced via<br/> the account&apos;s auth key.<br/><br/> This module allows creating a flexible and powerful multisig account with seamless support for updating owners<br/> without changing the auth key. Users can choose to store transaction payloads waiting for owner signatures on chain<br/> or off chain (primary consideration is decentralization/transparency vs gas cost).<br/><br/> The multisig account is a resource account underneath. By default, it has no auth key and can only be controlled via<br/> the special multisig transaction flow. However, owners can create a transaction to change the auth key to match a<br/> private key off chain if so desired.<br/><br/> Transactions need to be executed in order of creation, similar to transactions for a normal Aptos account (enforced<br/> with account nonce).<br/><br/> The flow is like below:<br/> 1. Owners can create a new multisig account by calling create (signer is default single owner) or with<br/> create_with_owners where multiple initial owner addresses can be specified. This is different (and easier) from<br/> the native multisig scheme where the owners&apos; public keys have to be specified. Here, only addresses are needed.<br/> 2. Owners can be added/removed any time by calling add_owners or remove_owners. The transactions to do still need<br/> to follow the k&#45;of&#45;n scheme specified for the multisig account.<br/> 3. To create a new transaction, an owner can call create_transaction with the transaction payload. This will store<br/> the full transaction payload on chain, which adds decentralization (censorship is not possible as the data is<br/> available on chain) and makes it easier to fetch all transactions waiting for execution. If saving gas is desired,<br/> an owner can alternatively call create_transaction_with_hash where only the payload hash is stored. Later execution<br/> will be verified using the hash. Only owners can create transactions and a transaction id (incremeting id) will be<br/> assigned.<br/> 4. To approve or reject a transaction, other owners can call approve() or reject() with the transaction id.<br/> 5. If there are enough approvals, any owner can execute the transaction using the special MultisigTransaction type<br/> with the transaction id if the full payload is already stored on chain or with the transaction payload if only a<br/> hash is stored. Transaction execution will first check with this module that the transaction payload has gotten<br/> enough signatures. If so, it will be executed as the multisig account. The owner who executes will pay for gas.<br/> 6. If there are enough rejections, any owner can finalize the rejection by calling execute_rejected_transaction().<br/><br/> Note that this multisig account model is not designed to use with a large number of owners. The more owners there<br/> are, the more expensive voting on transactions will become. If a large number of owners is designed, such as in a<br/> flat governance structure, clients are encouraged to write their own modules on top of this multisig account module<br/> and implement the governance voting logic on top.


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


<pre><code>use 0x1::account;<br/>use 0x1::aptos_coin;<br/>use 0x1::bcs;<br/>use 0x1::chain_id;<br/>use 0x1::coin;<br/>use 0x1::create_signer;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::hash;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::simple_map;<br/>use 0x1::string;<br/>use 0x1::table;<br/>use 0x1::timestamp;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_multisig_account_MultisigAccount"></a>

## Resource `MultisigAccount`

Represents a multisig account&apos;s configurations and transactions.<br/> This will be stored in the multisig account (created as a resource account separate from any owner accounts).


<pre><code>struct MultisigAccount has key<br/></code></pre>



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

A transaction to be executed in a multisig account.<br/> This must contain either the full transaction payload or its hash (stored as bytes).


<pre><code>struct MultisigTransaction has copy, drop, store<br/></code></pre>



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


<pre><code>struct ExecutionError has copy, drop, store<br/></code></pre>



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


<pre><code>struct MultisigAccountCreationMessage has copy, drop<br/></code></pre>



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


<pre><code>struct MultisigAccountCreationWithAuthKeyRevocationMessage has copy, drop<br/></code></pre>



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


<pre><code>struct AddOwnersEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct AddOwners has drop, store<br/></code></pre>



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


<pre><code>struct RemoveOwnersEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct RemoveOwners has drop, store<br/></code></pre>



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


<pre><code>struct UpdateSignaturesRequiredEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct UpdateSignaturesRequired has drop, store<br/></code></pre>



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


<pre><code>struct CreateTransactionEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct CreateTransaction has drop, store<br/></code></pre>



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


<pre><code>struct VoteEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct Vote has drop, store<br/></code></pre>



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

Event emitted when a transaction is officially rejected because the number of rejections has reached the<br/> number of signatures required.


<pre><code>struct ExecuteRejectedTransactionEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct ExecuteRejectedTransaction has drop, store<br/></code></pre>



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


<pre><code>struct TransactionExecutionSucceededEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TransactionExecutionSucceeded has drop, store<br/></code></pre>



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

Event emitted when a transaction&apos;s execution failed.


<pre><code>struct TransactionExecutionFailedEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct TransactionExecutionFailed has drop, store<br/></code></pre>



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

Event emitted when a transaction&apos;s metadata is updated.


<pre><code>struct MetadataUpdatedEvent has drop, store<br/></code></pre>



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



<pre><code>&#35;[event]<br/>struct MetadataUpdated has drop, store<br/></code></pre>



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



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];<br/></code></pre>



<a id="0x1_multisig_account_DOMAIN_SEPARATOR"></a>

The salt used to create a resource account during multisig account creation.<br/> This is used to avoid conflicts with other modules that also create resource accounts with the same owner<br/> account.


<pre><code>const DOMAIN_SEPARATOR: vector&lt;u8&gt; &#61; [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 109, 117, 108, 116, 105, 115, 105, 103, 95, 97, 99, 99, 111, 117, 110, 116];<br/></code></pre>



<a id="0x1_multisig_account_EACCOUNT_NOT_MULTISIG"></a>

Specified account is not a multisig account.


<pre><code>const EACCOUNT_NOT_MULTISIG: u64 &#61; 2002;<br/></code></pre>



<a id="0x1_multisig_account_EDUPLICATE_METADATA_KEY"></a>

The specified metadata contains duplicate attributes (keys).


<pre><code>const EDUPLICATE_METADATA_KEY: u64 &#61; 16;<br/></code></pre>



<a id="0x1_multisig_account_EDUPLICATE_OWNER"></a>

Owner list cannot contain the same address more than once.


<pre><code>const EDUPLICATE_OWNER: u64 &#61; 1;<br/></code></pre>



<a id="0x1_multisig_account_EINVALID_PAYLOAD_HASH"></a>

Payload hash must be exactly 32 bytes (sha3&#45;256).


<pre><code>const EINVALID_PAYLOAD_HASH: u64 &#61; 12;<br/></code></pre>



<a id="0x1_multisig_account_EINVALID_SEQUENCE_NUMBER"></a>

The sequence number provided is invalid. It must be between [1, next pending transaction &#45; 1].


<pre><code>const EINVALID_SEQUENCE_NUMBER: u64 &#61; 17;<br/></code></pre>



<a id="0x1_multisig_account_EINVALID_SIGNATURES_REQUIRED"></a>

Number of signatures required must be more than zero and at most the total number of owners.


<pre><code>const EINVALID_SIGNATURES_REQUIRED: u64 &#61; 11;<br/></code></pre>



<a id="0x1_multisig_account_EMAX_PENDING_TRANSACTIONS_EXCEEDED"></a>

The number of pending transactions has exceeded the maximum allowed.


<pre><code>const EMAX_PENDING_TRANSACTIONS_EXCEEDED: u64 &#61; 19;<br/></code></pre>



<a id="0x1_multisig_account_EMULTISIG_ACCOUNTS_NOT_ENABLED_YET"></a>

Multisig accounts has not been enabled on this current network yet.


<pre><code>const EMULTISIG_ACCOUNTS_NOT_ENABLED_YET: u64 &#61; 14;<br/></code></pre>



<a id="0x1_multisig_account_EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED"></a>

The multisig v2 enhancement feature is not enabled.


<pre><code>const EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED: u64 &#61; 20;<br/></code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_APPROVALS"></a>

Transaction has not received enough approvals to be executed.


<pre><code>const ENOT_ENOUGH_APPROVALS: u64 &#61; 2009;<br/></code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_OWNERS"></a>

Multisig account must have at least one owner.


<pre><code>const ENOT_ENOUGH_OWNERS: u64 &#61; 5;<br/></code></pre>



<a id="0x1_multisig_account_ENOT_ENOUGH_REJECTIONS"></a>

Transaction has not received enough rejections to be officially rejected.


<pre><code>const ENOT_ENOUGH_REJECTIONS: u64 &#61; 10;<br/></code></pre>



<a id="0x1_multisig_account_ENOT_OWNER"></a>

Account executing this operation is not an owner of the multisig account.


<pre><code>const ENOT_OWNER: u64 &#61; 2003;<br/></code></pre>



<a id="0x1_multisig_account_ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH"></a>

The number of metadata keys and values don&apos;t match.


<pre><code>const ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH: u64 &#61; 15;<br/></code></pre>



<a id="0x1_multisig_account_EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP"></a>

Provided owners to remove and new owners overlap.


<pre><code>const EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP: u64 &#61; 18;<br/></code></pre>



<a id="0x1_multisig_account_EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF"></a>

The multisig account itself cannot be an owner.


<pre><code>const EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF: u64 &#61; 13;<br/></code></pre>



<a id="0x1_multisig_account_EPAYLOAD_CANNOT_BE_EMPTY"></a>

Transaction payload cannot be empty.


<pre><code>const EPAYLOAD_CANNOT_BE_EMPTY: u64 &#61; 4;<br/></code></pre>



<a id="0x1_multisig_account_EPAYLOAD_DOES_NOT_MATCH_HASH"></a>

Provided target function does not match the hash stored in the on&#45;chain transaction.


<pre><code>const EPAYLOAD_DOES_NOT_MATCH_HASH: u64 &#61; 2008;<br/></code></pre>



<a id="0x1_multisig_account_ETRANSACTION_NOT_FOUND"></a>

Transaction with specified id cannot be found.


<pre><code>const ETRANSACTION_NOT_FOUND: u64 &#61; 2006;<br/></code></pre>



<a id="0x1_multisig_account_MAX_PENDING_TRANSACTIONS"></a>



<pre><code>const MAX_PENDING_TRANSACTIONS: u64 &#61; 20;<br/></code></pre>



<a id="0x1_multisig_account_metadata"></a>

## Function `metadata`

Return the multisig account&apos;s metadata.


<pre><code>&#35;[view]<br/>public fun metadata(multisig_account: address): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun metadata(multisig_account: address): SimpleMap&lt;String, vector&lt;u8&gt;&gt; acquires MultisigAccount &#123;<br/>    borrow_global&lt;MultisigAccount&gt;(multisig_account).metadata<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_num_signatures_required"></a>

## Function `num_signatures_required`

Return the number of signatures required to execute or execute&#45;reject a transaction in the provided<br/> multisig account.


<pre><code>&#35;[view]<br/>public fun num_signatures_required(multisig_account: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun num_signatures_required(multisig_account: address): u64 acquires MultisigAccount &#123;<br/>    borrow_global&lt;MultisigAccount&gt;(multisig_account).num_signatures_required<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_owners"></a>

## Function `owners`

Return a vector of all of the provided multisig account&apos;s owners.


<pre><code>&#35;[view]<br/>public fun owners(multisig_account: address): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun owners(multisig_account: address): vector&lt;address&gt; acquires MultisigAccount &#123;<br/>    borrow_global&lt;MultisigAccount&gt;(multisig_account).owners<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_is_owner"></a>

## Function `is_owner`

Return true if the provided owner is an owner of the provided multisig account.


<pre><code>&#35;[view]<br/>public fun is_owner(owner: address, multisig_account: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_owner(owner: address, multisig_account: address): bool acquires MultisigAccount &#123;<br/>    vector::contains(&amp;borrow_global&lt;MultisigAccount&gt;(multisig_account).owners, &amp;owner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_get_transaction"></a>

## Function `get_transaction`

Return the transaction with the given transaction id.


<pre><code>&#35;[view]<br/>public fun get_transaction(multisig_account: address, sequence_number: u64): multisig_account::MultisigTransaction<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_transaction(<br/>    multisig_account: address,<br/>    sequence_number: u64,<br/>): MultisigTransaction acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    assert!(<br/>        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,<br/>        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),<br/>    );<br/>    &#42;table::borrow(&amp;multisig_account_resource.transactions, sequence_number)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_get_pending_transactions"></a>

## Function `get_pending_transactions`

Return all pending transactions.


<pre><code>&#35;[view]<br/>public fun get_pending_transactions(multisig_account: address): vector&lt;multisig_account::MultisigTransaction&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_pending_transactions(<br/>    multisig_account: address<br/>): vector&lt;MultisigTransaction&gt; acquires MultisigAccount &#123;<br/>    let pending_transactions: vector&lt;MultisigTransaction&gt; &#61; vector[];<br/>    let multisig_account &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    let i &#61; multisig_account.last_executed_sequence_number &#43; 1;<br/>    let next_sequence_number &#61; multisig_account.next_sequence_number;<br/>    while (i &lt; next_sequence_number) &#123;<br/>        vector::push_back(&amp;mut pending_transactions, &#42;table::borrow(&amp;multisig_account.transactions, i));<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    pending_transactions<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_get_next_transaction_payload"></a>

## Function `get_next_transaction_payload`

Return the payload for the next transaction in the queue.


<pre><code>&#35;[view]<br/>public fun get_next_transaction_payload(multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_next_transaction_payload(<br/>    multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt; acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;<br/>    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);<br/><br/>    if (option::is_some(&amp;transaction.payload)) &#123;<br/>        &#42;option::borrow(&amp;transaction.payload)<br/>    &#125; else &#123;<br/>        provided_payload<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_can_be_executed"></a>

## Function `can_be_executed`

Return true if the transaction with given transaction id can be executed now.


<pre><code>&#35;[view]<br/>public fun can_be_executed(multisig_account: address, sequence_number: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_executed(multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;<br/>    assert_valid_sequence_number(multisig_account, sequence_number);<br/>    let (num_approvals, _) &#61; num_approvals_and_rejections(multisig_account, sequence_number);<br/>    sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;<br/>        num_approvals &gt;&#61; num_signatures_required(multisig_account)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_can_execute"></a>

## Function `can_execute`

Return true if the owner can execute the transaction with given transaction id now.


<pre><code>&#35;[view]<br/>public fun can_execute(owner: address, multisig_account: address, sequence_number: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_execute(owner: address, multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;<br/>    assert_valid_sequence_number(multisig_account, sequence_number);<br/>    let (num_approvals, _) &#61; num_approvals_and_rejections(multisig_account, sequence_number);<br/>    if (!has_voted_for_approval(multisig_account, sequence_number, owner)) &#123;<br/>        num_approvals &#61; num_approvals &#43; 1;<br/>    &#125;;<br/>    is_owner(owner, multisig_account) &amp;&amp;<br/>        sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;<br/>        num_approvals &gt;&#61; num_signatures_required(multisig_account)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_can_be_rejected"></a>

## Function `can_be_rejected`

Return true if the transaction with given transaction id can be officially rejected.


<pre><code>&#35;[view]<br/>public fun can_be_rejected(multisig_account: address, sequence_number: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_be_rejected(multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;<br/>    assert_valid_sequence_number(multisig_account, sequence_number);<br/>    let (_, num_rejections) &#61; num_approvals_and_rejections(multisig_account, sequence_number);<br/>    sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;<br/>        num_rejections &gt;&#61; num_signatures_required(multisig_account)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_can_reject"></a>

## Function `can_reject`

Return true if the owner can execute the &quot;rejected&quot; transaction with given transaction id now.


<pre><code>&#35;[view]<br/>public fun can_reject(owner: address, multisig_account: address, sequence_number: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_reject(owner: address, multisig_account: address, sequence_number: u64): bool acquires MultisigAccount &#123;<br/>    assert_valid_sequence_number(multisig_account, sequence_number);<br/>    let (_, num_rejections) &#61; num_approvals_and_rejections(multisig_account, sequence_number);<br/>    if (!has_voted_for_rejection(multisig_account, sequence_number, owner)) &#123;<br/>        num_rejections &#61; num_rejections &#43; 1;<br/>    &#125;;<br/>    is_owner(owner, multisig_account) &amp;&amp;<br/>        sequence_number &#61;&#61; last_resolved_sequence_number(multisig_account) &#43; 1 &amp;&amp;<br/>        num_rejections &gt;&#61; num_signatures_required(multisig_account)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_get_next_multisig_account_address"></a>

## Function `get_next_multisig_account_address`

Return the predicted address for the next multisig account if created from the given creator address.


<pre><code>&#35;[view]<br/>public fun get_next_multisig_account_address(creator: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_next_multisig_account_address(creator: address): address &#123;<br/>    let owner_nonce &#61; account::get_sequence_number(creator);<br/>    create_resource_address(&amp;creator, create_multisig_account_seed(to_bytes(&amp;owner_nonce)))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_last_resolved_sequence_number"></a>

## Function `last_resolved_sequence_number`

Return the id of the last transaction that was executed (successful or failed) or removed.


<pre><code>&#35;[view]<br/>public fun last_resolved_sequence_number(multisig_account: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun last_resolved_sequence_number(multisig_account: address): u64 acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    multisig_account_resource.last_executed_sequence_number<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_next_sequence_number"></a>

## Function `next_sequence_number`

Return the id of the next transaction created.


<pre><code>&#35;[view]<br/>public fun next_sequence_number(multisig_account: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun next_sequence_number(multisig_account: address): u64 acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    multisig_account_resource.next_sequence_number<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_vote"></a>

## Function `vote`

Return a bool tuple indicating whether an owner has voted and if so, whether they voted yes or no.


<pre><code>&#35;[view]<br/>public fun vote(multisig_account: address, sequence_number: u64, owner: address): (bool, bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun vote(<br/>    multisig_account: address, sequence_number: u64, owner: address): (bool, bool) acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    assert!(<br/>        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,<br/>        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),<br/>    );<br/>    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);<br/>    let votes &#61; &amp;transaction.votes;<br/>    let voted &#61; simple_map::contains_key(votes, &amp;owner);<br/>    let vote &#61; voted &amp;&amp; &#42;simple_map::borrow(votes, &amp;owner);<br/>    (voted, vote)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_available_transaction_queue_capacity"></a>

## Function `available_transaction_queue_capacity`



<pre><code>&#35;[view]<br/>public fun available_transaction_queue_capacity(multisig_account: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun available_transaction_queue_capacity(multisig_account: address): u64 acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    let num_pending_transactions &#61; multisig_account_resource.next_sequence_number &#45; multisig_account_resource.last_executed_sequence_number &#45; 1;<br/>    if (num_pending_transactions &gt; MAX_PENDING_TRANSACTIONS) &#123;<br/>        0<br/>    &#125; else &#123;<br/>        MAX_PENDING_TRANSACTIONS &#45; num_pending_transactions<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account"></a>

## Function `create_with_existing_account`

Creates a new multisig account on top of an existing account.<br/><br/> This offers a migration path for an existing account with a multi&#45;ed25519 auth key (native multisig account).<br/> In order to ensure a malicious module cannot obtain backdoor control over an existing account, a signed message<br/> with a valid signature from the account&apos;s auth key is required.<br/><br/> Note that this does not revoke auth key&#45;based control over the account. Owners should separately rotate the auth<br/> key after they are fully migrated to the new multisig account. Alternatively, they can call<br/> create_with_existing_account_and_revoke_auth_key instead.


<pre><code>public entry fun create_with_existing_account(multisig_address: address, owners: vector&lt;address&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: vector&lt;u8&gt;, create_multisig_account_signed_message: vector&lt;u8&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_existing_account(<br/>    multisig_address: address,<br/>    owners: vector&lt;address&gt;,<br/>    num_signatures_required: u64,<br/>    account_scheme: u8,<br/>    account_public_key: vector&lt;u8&gt;,<br/>    create_multisig_account_signed_message: vector&lt;u8&gt;,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    // Verify that the `MultisigAccountCreationMessage` has the right information and is signed by the account<br/>    // owner&apos;s key.<br/>    let proof_challenge &#61; MultisigAccountCreationMessage &#123;<br/>        chain_id: chain_id::get(),<br/>        account_address: multisig_address,<br/>        sequence_number: account::get_sequence_number(multisig_address),<br/>        owners,<br/>        num_signatures_required,<br/>    &#125;;<br/>    account::verify_signed_message(<br/>        multisig_address,<br/>        account_scheme,<br/>        account_public_key,<br/>        create_multisig_account_signed_message,<br/>        proof_challenge,<br/>    );<br/><br/>    // We create the signer for the multisig account here since this is required to add the MultisigAccount resource<br/>    // This should be safe and authorized because we have verified the signed message from the existing account<br/>    // that authorizes creating a multisig account with the specified owners and signature threshold.<br/>    let multisig_account &#61; &amp;create_signer(multisig_address);<br/>    create_with_owners_internal(<br/>        multisig_account,<br/>        owners,<br/>        num_signatures_required,<br/>        option::none&lt;SignerCapability&gt;(),<br/>        metadata_keys,<br/>        metadata_values,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_with_existing_account_and_revoke_auth_key"></a>

## Function `create_with_existing_account_and_revoke_auth_key`

Creates a new multisig account on top of an existing account and immediately rotate the origin auth key to 0x0.<br/><br/> Note: If the original account is a resource account, this does not revoke all control over it as if any<br/> SignerCapability of the resource account still exists, it can still be used to generate the signer for the<br/> account.


<pre><code>public entry fun create_with_existing_account_and_revoke_auth_key(multisig_address: address, owners: vector&lt;address&gt;, num_signatures_required: u64, account_scheme: u8, account_public_key: vector&lt;u8&gt;, create_multisig_account_signed_message: vector&lt;u8&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_existing_account_and_revoke_auth_key(<br/>    multisig_address: address,<br/>    owners: vector&lt;address&gt;,<br/>    num_signatures_required: u64,<br/>    account_scheme: u8,<br/>    account_public_key: vector&lt;u8&gt;,<br/>    create_multisig_account_signed_message: vector&lt;u8&gt;,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    // Verify that the `MultisigAccountCreationMessage` has the right information and is signed by the account<br/>    // owner&apos;s key.<br/>    let proof_challenge &#61; MultisigAccountCreationWithAuthKeyRevocationMessage &#123;<br/>        chain_id: chain_id::get(),<br/>        account_address: multisig_address,<br/>        sequence_number: account::get_sequence_number(multisig_address),<br/>        owners,<br/>        num_signatures_required,<br/>    &#125;;<br/>    account::verify_signed_message(<br/>        multisig_address,<br/>        account_scheme,<br/>        account_public_key,<br/>        create_multisig_account_signed_message,<br/>        proof_challenge,<br/>    );<br/><br/>    // We create the signer for the multisig account here since this is required to add the MultisigAccount resource<br/>    // This should be safe and authorized because we have verified the signed message from the existing account<br/>    // that authorizes creating a multisig account with the specified owners and signature threshold.<br/>    let multisig_account &#61; &amp;create_signer(multisig_address);<br/>    create_with_owners_internal(<br/>        multisig_account,<br/>        owners,<br/>        num_signatures_required,<br/>        option::none&lt;SignerCapability&gt;(),<br/>        metadata_keys,<br/>        metadata_values,<br/>    );<br/><br/>    // Rotate the account&apos;s auth key to 0x0, which effectively revokes control via auth key.<br/>    let multisig_address &#61; address_of(multisig_account);<br/>    account::rotate_authentication_key_internal(multisig_account, ZERO_AUTH_KEY);<br/>    // This also needs to revoke any signer capability or rotation capability that exists for the account to<br/>    // completely remove all access to the account.<br/>    if (account::is_signer_capability_offered(multisig_address)) &#123;<br/>        account::revoke_any_signer_capability(multisig_account);<br/>    &#125;;<br/>    if (account::is_rotation_capability_offered(multisig_address)) &#123;<br/>        account::revoke_any_rotation_capability(multisig_account);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create"></a>

## Function `create`

Creates a new multisig account and add the signer as a single owner.


<pre><code>public entry fun create(owner: &amp;signer, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create(<br/>    owner: &amp;signer,<br/>    num_signatures_required: u64,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    create_with_owners(owner, vector[], num_signatures_required, metadata_keys, metadata_values);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners"></a>

## Function `create_with_owners`

Creates a new multisig account with the specified additional owner list and signatures required.<br/><br/> @param additional_owners The owner account who calls this function cannot be in the additional_owners and there<br/> cannot be any duplicate owners in the list.<br/> @param num_signatures_required The number of signatures required to execute a transaction. Must be at least 1 and<br/> at most the total number of owners.


<pre><code>public entry fun create_with_owners(owner: &amp;signer, additional_owners: vector&lt;address&gt;, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_owners(<br/>    owner: &amp;signer,<br/>    additional_owners: vector&lt;address&gt;,<br/>    num_signatures_required: u64,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    let (multisig_account, multisig_signer_cap) &#61; create_multisig_account(owner);<br/>    vector::push_back(&amp;mut additional_owners, address_of(owner));<br/>    create_with_owners_internal(<br/>        &amp;multisig_account,<br/>        additional_owners,<br/>        num_signatures_required,<br/>        option::some(multisig_signer_cap),<br/>        metadata_keys,<br/>        metadata_values,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_then_remove_bootstrapper"></a>

## Function `create_with_owners_then_remove_bootstrapper`

Like <code>create_with_owners</code>, but removes the calling account after creation.<br/><br/> This is for creating a vanity multisig account from a bootstrapping account that should not<br/> be an owner after the vanity multisig address has been secured.


<pre><code>public entry fun create_with_owners_then_remove_bootstrapper(bootstrapper: &amp;signer, owners: vector&lt;address&gt;, num_signatures_required: u64, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_with_owners_then_remove_bootstrapper(<br/>    bootstrapper: &amp;signer,<br/>    owners: vector&lt;address&gt;,<br/>    num_signatures_required: u64,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    let bootstrapper_address &#61; address_of(bootstrapper);<br/>    create_with_owners(<br/>        bootstrapper,<br/>        owners,<br/>        num_signatures_required,<br/>        metadata_keys,<br/>        metadata_values<br/>    );<br/>    update_owner_schema(<br/>        get_next_multisig_account_address(bootstrapper_address),<br/>        vector[],<br/>        vector[bootstrapper_address],<br/>        option::none()<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_with_owners_internal"></a>

## Function `create_with_owners_internal`



<pre><code>fun create_with_owners_internal(multisig_account: &amp;signer, owners: vector&lt;address&gt;, num_signatures_required: u64, multisig_account_signer_cap: option::Option&lt;account::SignerCapability&gt;, metadata_keys: vector&lt;string::String&gt;, metadata_values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_with_owners_internal(<br/>    multisig_account: &amp;signer,<br/>    owners: vector&lt;address&gt;,<br/>    num_signatures_required: u64,<br/>    multisig_account_signer_cap: Option&lt;SignerCapability&gt;,<br/>    metadata_keys: vector&lt;String&gt;,<br/>    metadata_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>) acquires MultisigAccount &#123;<br/>    assert!(features::multisig_accounts_enabled(), error::unavailable(EMULTISIG_ACCOUNTS_NOT_ENABLED_YET));<br/>    assert!(<br/>        num_signatures_required &gt; 0 &amp;&amp; num_signatures_required &lt;&#61; vector::length(&amp;owners),<br/>        error::invalid_argument(EINVALID_SIGNATURES_REQUIRED),<br/>    );<br/><br/>    let multisig_address &#61; address_of(multisig_account);<br/>    validate_owners(&amp;owners, multisig_address);<br/>    move_to(multisig_account, MultisigAccount &#123;<br/>        owners,<br/>        num_signatures_required,<br/>        transactions: table::new&lt;u64, MultisigTransaction&gt;(),<br/>        metadata: simple_map::create&lt;String, vector&lt;u8&gt;&gt;(),<br/>        // First transaction will start at id 1 instead of 0.<br/>        last_executed_sequence_number: 0,<br/>        next_sequence_number: 1,<br/>        signer_cap: multisig_account_signer_cap,<br/>        add_owners_events: new_event_handle&lt;AddOwnersEvent&gt;(multisig_account),<br/>        remove_owners_events: new_event_handle&lt;RemoveOwnersEvent&gt;(multisig_account),<br/>        update_signature_required_events: new_event_handle&lt;UpdateSignaturesRequiredEvent&gt;(multisig_account),<br/>        create_transaction_events: new_event_handle&lt;CreateTransactionEvent&gt;(multisig_account),<br/>        vote_events: new_event_handle&lt;VoteEvent&gt;(multisig_account),<br/>        execute_rejected_transaction_events: new_event_handle&lt;ExecuteRejectedTransactionEvent&gt;(multisig_account),<br/>        execute_transaction_events: new_event_handle&lt;TransactionExecutionSucceededEvent&gt;(multisig_account),<br/>        transaction_execution_failed_events: new_event_handle&lt;TransactionExecutionFailedEvent&gt;(multisig_account),<br/>        metadata_updated_events: new_event_handle&lt;MetadataUpdatedEvent&gt;(multisig_account),<br/>    &#125;);<br/><br/>    update_metadata_internal(multisig_account, metadata_keys, metadata_values, false);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_add_owner"></a>

## Function `add_owner`

Similar to add_owners, but only allow adding one owner.


<pre><code>entry fun add_owner(multisig_account: &amp;signer, new_owner: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owner(multisig_account: &amp;signer, new_owner: address) acquires MultisigAccount &#123;<br/>    add_owners(multisig_account, vector[new_owner]);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_add_owners"></a>

## Function `add_owners`

Add new owners to the multisig account. This can only be invoked by the multisig account itself, through the<br/> proposal flow.<br/><br/> Note that this function is not public so it can only be invoked directly instead of via a module or script. This<br/> ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to<br/> maliciously alter the owners list.


<pre><code>entry fun add_owners(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owners(<br/>    multisig_account: &amp;signer, new_owners: vector&lt;address&gt;) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        new_owners,<br/>        vector[],<br/>        option::none()<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_add_owners_and_update_signatures_required"></a>

## Function `add_owners_and_update_signatures_required`

Add owners then update number of signatures required, in a single operation.


<pre><code>entry fun add_owners_and_update_signatures_required(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;, new_num_signatures_required: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun add_owners_and_update_signatures_required(<br/>    multisig_account: &amp;signer,<br/>    new_owners: vector&lt;address&gt;,<br/>    new_num_signatures_required: u64<br/>) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        new_owners,<br/>        vector[],<br/>        option::some(new_num_signatures_required)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_remove_owner"></a>

## Function `remove_owner`

Similar to remove_owners, but only allow removing one owner.


<pre><code>entry fun remove_owner(multisig_account: &amp;signer, owner_to_remove: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun remove_owner(<br/>    multisig_account: &amp;signer, owner_to_remove: address) acquires MultisigAccount &#123;<br/>    remove_owners(multisig_account, vector[owner_to_remove]);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_remove_owners"></a>

## Function `remove_owners`

Remove owners from the multisig account. This can only be invoked by the multisig account itself, through the<br/> proposal flow.<br/><br/> This function skips any owners who are not in the multisig account&apos;s list of owners.<br/> Note that this function is not public so it can only be invoked directly instead of via a module or script. This<br/> ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to<br/> maliciously alter the owners list.


<pre><code>entry fun remove_owners(multisig_account: &amp;signer, owners_to_remove: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun remove_owners(<br/>    multisig_account: &amp;signer, owners_to_remove: vector&lt;address&gt;) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        vector[],<br/>        owners_to_remove,<br/>        option::none()<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_swap_owner"></a>

## Function `swap_owner`

Swap an owner in for an old one, without changing required signatures.


<pre><code>entry fun swap_owner(multisig_account: &amp;signer, to_swap_in: address, to_swap_out: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owner(<br/>    multisig_account: &amp;signer,<br/>    to_swap_in: address,<br/>    to_swap_out: address<br/>) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        vector[to_swap_in],<br/>        vector[to_swap_out],<br/>        option::none()<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_swap_owners"></a>

## Function `swap_owners`

Swap owners in and out, without changing required signatures.


<pre><code>entry fun swap_owners(multisig_account: &amp;signer, to_swap_in: vector&lt;address&gt;, to_swap_out: vector&lt;address&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owners(<br/>    multisig_account: &amp;signer,<br/>    to_swap_in: vector&lt;address&gt;,<br/>    to_swap_out: vector&lt;address&gt;<br/>) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        to_swap_in,<br/>        to_swap_out,<br/>        option::none()<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_swap_owners_and_update_signatures_required"></a>

## Function `swap_owners_and_update_signatures_required`

Swap owners in and out, updating number of required signatures.


<pre><code>entry fun swap_owners_and_update_signatures_required(multisig_account: &amp;signer, new_owners: vector&lt;address&gt;, owners_to_remove: vector&lt;address&gt;, new_num_signatures_required: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun swap_owners_and_update_signatures_required(<br/>    multisig_account: &amp;signer,<br/>    new_owners: vector&lt;address&gt;,<br/>    owners_to_remove: vector&lt;address&gt;,<br/>    new_num_signatures_required: u64<br/>) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        new_owners,<br/>        owners_to_remove,<br/>        option::some(new_num_signatures_required)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_update_signatures_required"></a>

## Function `update_signatures_required`

Update the number of signatures required to execute transaction in the specified multisig account.<br/><br/> This can only be invoked by the multisig account itself, through the proposal flow.<br/> Note that this function is not public so it can only be invoked directly instead of via a module or script. This<br/> ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to<br/> maliciously alter the number of signatures required.


<pre><code>entry fun update_signatures_required(multisig_account: &amp;signer, new_num_signatures_required: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun update_signatures_required(<br/>    multisig_account: &amp;signer, new_num_signatures_required: u64) acquires MultisigAccount &#123;<br/>    update_owner_schema(<br/>        address_of(multisig_account),<br/>        vector[],<br/>        vector[],<br/>        option::some(new_num_signatures_required)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_update_metadata"></a>

## Function `update_metadata`

Allow the multisig account to update its own metadata. Note that this overrides the entire existing metadata.<br/> If any attributes are not specified in the metadata, they will be removed!<br/><br/> This can only be invoked by the multisig account itself, through the proposal flow.<br/> Note that this function is not public so it can only be invoked directly instead of via a module or script. This<br/> ensures that a multisig transaction cannot lead to another module obtaining the multisig signer and using it to<br/> maliciously alter the number of signatures required.


<pre><code>entry fun update_metadata(multisig_account: &amp;signer, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun update_metadata(<br/>    multisig_account: &amp;signer, keys: vector&lt;String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;) acquires MultisigAccount &#123;<br/>    update_metadata_internal(multisig_account, keys, values, true);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_update_metadata_internal"></a>

## Function `update_metadata_internal`



<pre><code>fun update_metadata_internal(multisig_account: &amp;signer, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, emit_event: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_metadata_internal(<br/>    multisig_account: &amp;signer,<br/>    keys: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    emit_event: bool,<br/>) acquires MultisigAccount &#123;<br/>    let num_attributes &#61; vector::length(&amp;keys);<br/>    assert!(<br/>        num_attributes &#61;&#61; vector::length(&amp;values),<br/>        error::invalid_argument(ENUMBER_OF_METADATA_KEYS_AND_VALUES_DONT_MATCH),<br/>    );<br/><br/>    let multisig_address &#61; address_of(multisig_account);<br/>    assert_multisig_account_exists(multisig_address);<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_address);<br/>    let old_metadata &#61; multisig_account_resource.metadata;<br/>    multisig_account_resource.metadata &#61; simple_map::create&lt;String, vector&lt;u8&gt;&gt;();<br/>    let metadata &#61; &amp;mut multisig_account_resource.metadata;<br/>    let i &#61; 0;<br/>    while (i &lt; num_attributes) &#123;<br/>        let key &#61; &#42;vector::borrow(&amp;keys, i);<br/>        let value &#61; &#42;vector::borrow(&amp;values, i);<br/>        assert!(<br/>            !simple_map::contains_key(metadata, &amp;key),<br/>            error::invalid_argument(EDUPLICATE_METADATA_KEY),<br/>        );<br/><br/>        simple_map::add(metadata, key, value);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/><br/>    if (emit_event) &#123;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(<br/>                MetadataUpdated &#123;<br/>                    multisig_account: multisig_address,<br/>                    old_metadata,<br/>                    new_metadata: multisig_account_resource.metadata,<br/>                &#125;<br/>            )<br/>        &#125;;<br/>        emit_event(<br/>            &amp;mut multisig_account_resource.metadata_updated_events,<br/>            MetadataUpdatedEvent &#123;<br/>                old_metadata,<br/>                new_metadata: multisig_account_resource.metadata,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_transaction"></a>

## Function `create_transaction`

Create a multisig transaction, which will have one approval initially (from the creator).


<pre><code>public entry fun create_transaction(owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_transaction(<br/>    owner: &amp;signer,<br/>    multisig_account: address,<br/>    payload: vector&lt;u8&gt;,<br/>) acquires MultisigAccount &#123;<br/>    assert!(vector::length(&amp;payload) &gt; 0, error::invalid_argument(EPAYLOAD_CANNOT_BE_EMPTY));<br/><br/>    assert_multisig_account_exists(multisig_account);<br/>    assert_is_owner(owner, multisig_account);<br/><br/>    let creator &#61; address_of(owner);<br/>    let transaction &#61; MultisigTransaction &#123;<br/>        payload: option::some(payload),<br/>        payload_hash: option::none&lt;vector&lt;u8&gt;&gt;(),<br/>        votes: simple_map::create&lt;address, bool&gt;(),<br/>        creator,<br/>        creation_time_secs: now_seconds(),<br/>    &#125;;<br/>    add_transaction(creator, multisig_account, transaction);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_transaction_with_hash"></a>

## Function `create_transaction_with_hash`

Create a multisig transaction with a transaction hash instead of the full payload.<br/> This means the payload will be stored off chain for gas saving. Later, during execution, the executor will need<br/> to provide the full payload, which will be validated against the hash stored on&#45;chain.


<pre><code>public entry fun create_transaction_with_hash(owner: &amp;signer, multisig_account: address, payload_hash: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_transaction_with_hash(<br/>    owner: &amp;signer,<br/>    multisig_account: address,<br/>    payload_hash: vector&lt;u8&gt;,<br/>) acquires MultisigAccount &#123;<br/>    // Payload hash is a sha3&#45;256 hash, so it must be exactly 32 bytes.<br/>    assert!(vector::length(&amp;payload_hash) &#61;&#61; 32, error::invalid_argument(EINVALID_PAYLOAD_HASH));<br/><br/>    assert_multisig_account_exists(multisig_account);<br/>    assert_is_owner(owner, multisig_account);<br/><br/>    let creator &#61; address_of(owner);<br/>    let transaction &#61; MultisigTransaction &#123;<br/>        payload: option::none&lt;vector&lt;u8&gt;&gt;(),<br/>        payload_hash: option::some(payload_hash),<br/>        votes: simple_map::create&lt;address, bool&gt;(),<br/>        creator,<br/>        creation_time_secs: now_seconds(),<br/>    &#125;;<br/>    add_transaction(creator, multisig_account, transaction);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_approve_transaction"></a>

## Function `approve_transaction`

Approve a multisig transaction.


<pre><code>public entry fun approve_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun approve_transaction(<br/>    owner: &amp;signer, multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;<br/>    vote_transanction(owner, multisig_account, sequence_number, true);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_reject_transaction"></a>

## Function `reject_transaction`

Reject a multisig transaction.


<pre><code>public entry fun reject_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun reject_transaction(<br/>    owner: &amp;signer, multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;<br/>    vote_transanction(owner, multisig_account, sequence_number, false);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_vote_transanction"></a>

## Function `vote_transanction`

Generic function that can be used to either approve or reject a multisig transaction<br/> Retained for backward compatibility: the function with the typographical error in its name<br/> will continue to be an accessible entry point.


<pre><code>public entry fun vote_transanction(owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transanction(<br/>    owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool) acquires MultisigAccount &#123;<br/>    assert_multisig_account_exists(multisig_account);<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    assert_is_owner_internal(owner, multisig_account_resource);<br/><br/>    assert!(<br/>        table::contains(&amp;multisig_account_resource.transactions, sequence_number),<br/>        error::not_found(ETRANSACTION_NOT_FOUND),<br/>    );<br/>    let transaction &#61; table::borrow_mut(&amp;mut multisig_account_resource.transactions, sequence_number);<br/>    let votes &#61; &amp;mut transaction.votes;<br/>    let owner_addr &#61; address_of(owner);<br/><br/>    if (simple_map::contains_key(votes, &amp;owner_addr)) &#123;<br/>        &#42;simple_map::borrow_mut(votes, &amp;owner_addr) &#61; approved;<br/>    &#125; else &#123;<br/>        simple_map::add(votes, owner_addr, approved);<br/>    &#125;;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            Vote &#123;<br/>                multisig_account,<br/>                owner: owner_addr,<br/>                sequence_number,<br/>                approved,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut multisig_account_resource.vote_events,<br/>        VoteEvent &#123;<br/>            owner: owner_addr,<br/>            sequence_number,<br/>            approved,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_vote_transaction"></a>

## Function `vote_transaction`

Generic function that can be used to either approve or reject a multisig transaction


<pre><code>public entry fun vote_transaction(owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transaction(<br/>    owner: &amp;signer, multisig_account: address, sequence_number: u64, approved: bool) acquires MultisigAccount &#123;<br/>    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));<br/>    vote_transanction(owner, multisig_account, sequence_number, approved);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_vote_transactions"></a>

## Function `vote_transactions`

Generic function that can be used to either approve or reject a batch of transactions within a specified range.


<pre><code>public entry fun vote_transactions(owner: &amp;signer, multisig_account: address, starting_sequence_number: u64, final_sequence_number: u64, approved: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun vote_transactions(<br/>    owner: &amp;signer, multisig_account: address, starting_sequence_number: u64, final_sequence_number: u64, approved: bool) acquires MultisigAccount &#123;<br/>    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));<br/>    let sequence_number &#61; starting_sequence_number;<br/>    while(sequence_number &lt;&#61; final_sequence_number) &#123;<br/>        vote_transanction(owner, multisig_account, sequence_number, approved);<br/>        sequence_number &#61; sequence_number &#43; 1;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transaction"></a>

## Function `execute_rejected_transaction`

Remove the next transaction if it has sufficient owner rejections.


<pre><code>public entry fun execute_rejected_transaction(owner: &amp;signer, multisig_account: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun execute_rejected_transaction(<br/>    owner: &amp;signer,<br/>    multisig_account: address,<br/>) acquires MultisigAccount &#123;<br/>    assert_multisig_account_exists(multisig_account);<br/>    assert_is_owner(owner, multisig_account);<br/><br/>    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;<br/>    let owner_addr &#61; address_of(owner);<br/>    if(features::multisig_v2_enhancement_feature_enabled()) &#123;<br/>        // Implicitly vote for rejection if the owner has not voted for rejection yet.<br/>        if (!has_voted_for_rejection(multisig_account, sequence_number, owner_addr)) &#123;<br/>            reject_transaction(owner, multisig_account, sequence_number);<br/>        &#125;<br/>    &#125;;<br/><br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    let (_, num_rejections) &#61; remove_executed_transaction(multisig_account_resource);<br/>    assert!(<br/>        num_rejections &gt;&#61; multisig_account_resource.num_signatures_required,<br/>        error::invalid_state(ENOT_ENOUGH_REJECTIONS),<br/>    );<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            ExecuteRejectedTransaction &#123;<br/>                multisig_account,<br/>                sequence_number,<br/>                num_rejections,<br/>                executor: address_of(owner),<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut multisig_account_resource.execute_rejected_transaction_events,<br/>        ExecuteRejectedTransactionEvent &#123;<br/>            sequence_number,<br/>            num_rejections,<br/>            executor: owner_addr,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_execute_rejected_transactions"></a>

## Function `execute_rejected_transactions`

Remove the next transactions until the final_sequence_number if they have sufficient owner rejections.


<pre><code>public entry fun execute_rejected_transactions(owner: &amp;signer, multisig_account: address, final_sequence_number: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun execute_rejected_transactions(<br/>    owner: &amp;signer,<br/>    multisig_account: address,<br/>    final_sequence_number: u64,<br/>) acquires MultisigAccount &#123;<br/>    assert!(features::multisig_v2_enhancement_feature_enabled(), error::invalid_state(EMULTISIG_V2_ENHANCEMENT_NOT_ENABLED));<br/>    assert!(last_resolved_sequence_number(multisig_account) &lt; final_sequence_number, error::invalid_argument(EINVALID_SEQUENCE_NUMBER));<br/>    assert!(final_sequence_number &lt; next_sequence_number(multisig_account), error::invalid_argument(EINVALID_SEQUENCE_NUMBER));<br/>    while(last_resolved_sequence_number(multisig_account) &lt; final_sequence_number) &#123;<br/>        execute_rejected_transaction(owner, multisig_account);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_validate_multisig_transaction"></a>

## Function `validate_multisig_transaction`

Called by the VM as part of transaction prologue, which is invoked during mempool transaction validation and as<br/> the first step of transaction execution.<br/><br/> Transaction payload is optional if it&apos;s already stored on chain for the transaction.


<pre><code>fun validate_multisig_transaction(owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_multisig_transaction(<br/>    owner: &amp;signer, multisig_account: address, payload: vector&lt;u8&gt;) acquires MultisigAccount &#123;<br/>    assert_multisig_account_exists(multisig_account);<br/>    assert_is_owner(owner, multisig_account);<br/>    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;<br/>    assert_transaction_exists(multisig_account, sequence_number);<br/><br/>    if(features::multisig_v2_enhancement_feature_enabled()) &#123;<br/>        assert!(<br/>            can_execute(address_of(owner), multisig_account, sequence_number),<br/>            error::invalid_argument(ENOT_ENOUGH_APPROVALS),<br/>        );<br/>    &#125;<br/>    else &#123;<br/>        assert!(<br/>            can_be_executed(multisig_account, sequence_number),<br/>            error::invalid_argument(ENOT_ENOUGH_APPROVALS),<br/>        );<br/>    &#125;;<br/><br/>    // If the transaction payload is not stored on chain, verify that the provided payload matches the hashes stored<br/>    // on chain.<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);<br/>    if (option::is_some(&amp;transaction.payload_hash)) &#123;<br/>        let payload_hash &#61; option::borrow(&amp;transaction.payload_hash);<br/>        assert!(<br/>            sha3_256(payload) &#61;&#61; &#42;payload_hash,<br/>            error::invalid_argument(EPAYLOAD_DOES_NOT_MATCH_HASH),<br/>        );<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_successful_transaction_execution_cleanup"></a>

## Function `successful_transaction_execution_cleanup`

Post&#45;execution cleanup for a successful multisig transaction execution.<br/> This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code>fun successful_transaction_execution_cleanup(executor: address, multisig_account: address, transaction_payload: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun successful_transaction_execution_cleanup(<br/>    executor: address,<br/>    multisig_account: address,<br/>    transaction_payload: vector&lt;u8&gt;,<br/>) acquires MultisigAccount &#123;<br/>    let num_approvals &#61; transaction_execution_cleanup_common(executor, multisig_account);<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            TransactionExecutionSucceeded &#123;<br/>                multisig_account,<br/>                sequence_number: multisig_account_resource.last_executed_sequence_number,<br/>                transaction_payload,<br/>                num_approvals,<br/>                executor,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut multisig_account_resource.execute_transaction_events,<br/>        TransactionExecutionSucceededEvent &#123;<br/>            sequence_number: multisig_account_resource.last_executed_sequence_number,<br/>            transaction_payload,<br/>            num_approvals,<br/>            executor,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_failed_transaction_execution_cleanup"></a>

## Function `failed_transaction_execution_cleanup`

Post&#45;execution cleanup for a failed multisig transaction execution.<br/> This function is private so no other code can call this beside the VM itself as part of MultisigTransaction.


<pre><code>fun failed_transaction_execution_cleanup(executor: address, multisig_account: address, transaction_payload: vector&lt;u8&gt;, execution_error: multisig_account::ExecutionError)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun failed_transaction_execution_cleanup(<br/>    executor: address,<br/>    multisig_account: address,<br/>    transaction_payload: vector&lt;u8&gt;,<br/>    execution_error: ExecutionError,<br/>) acquires MultisigAccount &#123;<br/>    let num_approvals &#61; transaction_execution_cleanup_common(executor, multisig_account);<br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            TransactionExecutionFailed &#123;<br/>                multisig_account,<br/>                executor,<br/>                sequence_number: multisig_account_resource.last_executed_sequence_number,<br/>                transaction_payload,<br/>                num_approvals,<br/>                execution_error,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut multisig_account_resource.transaction_execution_failed_events,<br/>        TransactionExecutionFailedEvent &#123;<br/>            executor,<br/>            sequence_number: multisig_account_resource.last_executed_sequence_number,<br/>            transaction_payload,<br/>            num_approvals,<br/>            execution_error,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_transaction_execution_cleanup_common"></a>

## Function `transaction_execution_cleanup_common`



<pre><code>fun transaction_execution_cleanup_common(executor: address, multisig_account: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun transaction_execution_cleanup_common(executor: address, multisig_account: address): u64 acquires MultisigAccount &#123;<br/>    let sequence_number &#61; last_resolved_sequence_number(multisig_account) &#43; 1;<br/>    let implicit_approval &#61; !has_voted_for_approval(multisig_account, sequence_number, executor);<br/><br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/>    let (num_approvals, _) &#61; remove_executed_transaction(multisig_account_resource);<br/><br/>    if(features::multisig_v2_enhancement_feature_enabled() &amp;&amp; implicit_approval) &#123;<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(<br/>                Vote &#123;<br/>                    multisig_account,<br/>                    owner: executor,<br/>                    sequence_number,<br/>                    approved: true,<br/>                &#125;<br/>            );<br/>        &#125;;<br/>        num_approvals &#61; num_approvals &#43; 1;<br/>        emit_event(<br/>            &amp;mut multisig_account_resource.vote_events,<br/>            VoteEvent &#123;<br/>                owner: executor,<br/>                sequence_number,<br/>                approved: true,<br/>            &#125;<br/>        );<br/>    &#125;;<br/><br/>    num_approvals<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_remove_executed_transaction"></a>

## Function `remove_executed_transaction`



<pre><code>fun remove_executed_transaction(multisig_account_resource: &amp;mut multisig_account::MultisigAccount): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun remove_executed_transaction(multisig_account_resource: &amp;mut MultisigAccount): (u64, u64) &#123;<br/>    let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;<br/>    let transaction &#61; table::remove(&amp;mut multisig_account_resource.transactions, sequence_number);<br/>    multisig_account_resource.last_executed_sequence_number &#61; sequence_number;<br/>    num_approvals_and_rejections_internal(&amp;multisig_account_resource.owners, &amp;transaction)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_add_transaction"></a>

## Function `add_transaction`



<pre><code>fun add_transaction(creator: address, multisig_account: address, transaction: multisig_account::MultisigTransaction)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun add_transaction(<br/>    creator: address,<br/>    multisig_account: address,<br/>    transaction: MultisigTransaction<br/>) &#123;<br/>    if(features::multisig_v2_enhancement_feature_enabled()) &#123;<br/>        assert!(<br/>            available_transaction_queue_capacity(multisig_account) &gt; 0,<br/>            error::invalid_state(EMAX_PENDING_TRANSACTIONS_EXCEEDED)<br/>        );<br/>    &#125;;<br/><br/>    let multisig_account_resource &#61; borrow_global_mut&lt;MultisigAccount&gt;(multisig_account);<br/><br/>    // The transaction creator also automatically votes for the transaction.<br/>    simple_map::add(&amp;mut transaction.votes, creator, true);<br/><br/>    let sequence_number &#61; multisig_account_resource.next_sequence_number;<br/>    multisig_account_resource.next_sequence_number &#61; sequence_number &#43; 1;<br/>    table::add(&amp;mut multisig_account_resource.transactions, sequence_number, transaction);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        emit(<br/>            CreateTransaction &#123; multisig_account: multisig_account, creator, sequence_number, transaction &#125;<br/>        );<br/>    &#125;;<br/>    emit_event(<br/>        &amp;mut multisig_account_resource.create_transaction_events,<br/>        CreateTransactionEvent &#123; creator, sequence_number, transaction &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account"></a>

## Function `create_multisig_account`



<pre><code>fun create_multisig_account(owner: &amp;signer): (signer, account::SignerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_multisig_account(owner: &amp;signer): (signer, SignerCapability) &#123;<br/>    let owner_nonce &#61; account::get_sequence_number(address_of(owner));<br/>    let (multisig_signer, multisig_signer_cap) &#61;<br/>        account::create_resource_account(owner, create_multisig_account_seed(to_bytes(&amp;owner_nonce)));<br/>    // Register the account to receive APT as this is not done by default as part of the resource account creation<br/>    // flow.<br/>    if (!coin::is_account_registered&lt;AptosCoin&gt;(address_of(&amp;multisig_signer))) &#123;<br/>        coin::register&lt;AptosCoin&gt;(&amp;multisig_signer);<br/>    &#125;;<br/><br/>    (multisig_signer, multisig_signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_create_multisig_account_seed"></a>

## Function `create_multisig_account_seed`



<pre><code>fun create_multisig_account_seed(seed: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_multisig_account_seed(seed: vector&lt;u8&gt;): vector&lt;u8&gt; &#123;<br/>    // Generate a seed that will be used to create the resource account that hosts the multisig account.<br/>    let multisig_account_seed &#61; vector::empty&lt;u8&gt;();<br/>    vector::append(&amp;mut multisig_account_seed, DOMAIN_SEPARATOR);<br/>    vector::append(&amp;mut multisig_account_seed, seed);<br/><br/>    multisig_account_seed<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_validate_owners"></a>

## Function `validate_owners`



<pre><code>fun validate_owners(owners: &amp;vector&lt;address&gt;, multisig_account: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_owners(owners: &amp;vector&lt;address&gt;, multisig_account: address) &#123;<br/>    let distinct_owners: vector&lt;address&gt; &#61; vector[];<br/>    vector::for_each_ref(owners, &#124;owner&#124; &#123;<br/>        let owner &#61; &#42;owner;<br/>        assert!(owner !&#61; multisig_account, error::invalid_argument(EOWNER_CANNOT_BE_MULTISIG_ACCOUNT_ITSELF));<br/>        let (found, _) &#61; vector::index_of(&amp;distinct_owners, &amp;owner);<br/>        assert!(!found, error::invalid_argument(EDUPLICATE_OWNER));<br/>        vector::push_back(&amp;mut distinct_owners, owner);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner_internal"></a>

## Function `assert_is_owner_internal`



<pre><code>fun assert_is_owner_internal(owner: &amp;signer, multisig_account: &amp;multisig_account::MultisigAccount)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_is_owner_internal(owner: &amp;signer, multisig_account: &amp;MultisigAccount) &#123;<br/>    assert!(<br/>        vector::contains(&amp;multisig_account.owners, &amp;address_of(owner)),<br/>        error::permission_denied(ENOT_OWNER),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_assert_is_owner"></a>

## Function `assert_is_owner`



<pre><code>fun assert_is_owner(owner: &amp;signer, multisig_account: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_is_owner(owner: &amp;signer, multisig_account: address) acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    assert_is_owner_internal(owner, multisig_account_resource);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections_internal"></a>

## Function `num_approvals_and_rejections_internal`



<pre><code>fun num_approvals_and_rejections_internal(owners: &amp;vector&lt;address&gt;, transaction: &amp;multisig_account::MultisigTransaction): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun num_approvals_and_rejections_internal(owners: &amp;vector&lt;address&gt;, transaction: &amp;MultisigTransaction): (u64, u64) &#123;<br/>    let num_approvals &#61; 0;<br/>    let num_rejections &#61; 0;<br/><br/>    let votes &#61; &amp;transaction.votes;<br/>    vector::for_each_ref(owners, &#124;owner&#124; &#123;<br/>        if (simple_map::contains_key(votes, owner)) &#123;<br/>            if (&#42;simple_map::borrow(votes, owner)) &#123;<br/>                num_approvals &#61; num_approvals &#43; 1;<br/>            &#125; else &#123;<br/>                num_rejections &#61; num_rejections &#43; 1;<br/>            &#125;;<br/>        &#125;<br/>    &#125;);<br/><br/>    (num_approvals, num_rejections)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_num_approvals_and_rejections"></a>

## Function `num_approvals_and_rejections`



<pre><code>fun num_approvals_and_rejections(multisig_account: address, sequence_number: u64): (u64, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun num_approvals_and_rejections(multisig_account: address, sequence_number: u64): (u64, u64) acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    let transaction &#61; table::borrow(&amp;multisig_account_resource.transactions, sequence_number);<br/>    num_approvals_and_rejections_internal(&amp;multisig_account_resource.owners, transaction)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_approval"></a>

## Function `has_voted_for_approval`



<pre><code>fun has_voted_for_approval(multisig_account: address, sequence_number: u64, owner: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun has_voted_for_approval(multisig_account: address, sequence_number: u64, owner: address): bool acquires MultisigAccount &#123;<br/>    let (voted, vote) &#61; vote(multisig_account, sequence_number, owner);<br/>    voted &amp;&amp; vote<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_has_voted_for_rejection"></a>

## Function `has_voted_for_rejection`



<pre><code>fun has_voted_for_rejection(multisig_account: address, sequence_number: u64, owner: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun has_voted_for_rejection(multisig_account: address, sequence_number: u64, owner: address): bool acquires MultisigAccount &#123;<br/>    let (voted, vote) &#61; vote(multisig_account, sequence_number, owner);<br/>    voted &amp;&amp; !vote<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_assert_multisig_account_exists"></a>

## Function `assert_multisig_account_exists`



<pre><code>fun assert_multisig_account_exists(multisig_account: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_multisig_account_exists(multisig_account: address) &#123;<br/>    assert!(exists&lt;MultisigAccount&gt;(multisig_account), error::invalid_state(EACCOUNT_NOT_MULTISIG));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_assert_valid_sequence_number"></a>

## Function `assert_valid_sequence_number`



<pre><code>fun assert_valid_sequence_number(multisig_account: address, sequence_number: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_valid_sequence_number(multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    assert!(<br/>        sequence_number &gt; 0 &amp;&amp; sequence_number &lt; multisig_account_resource.next_sequence_number,<br/>        error::invalid_argument(EINVALID_SEQUENCE_NUMBER),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_assert_transaction_exists"></a>

## Function `assert_transaction_exists`



<pre><code>fun assert_transaction_exists(multisig_account: address, sequence_number: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline fun assert_transaction_exists(multisig_account: address, sequence_number: u64) acquires MultisigAccount &#123;<br/>    let multisig_account_resource &#61; borrow_global&lt;MultisigAccount&gt;(multisig_account);<br/>    assert!(<br/>        table::contains(&amp;multisig_account_resource.transactions, sequence_number),<br/>        error::not_found(ETRANSACTION_NOT_FOUND),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_multisig_account_update_owner_schema"></a>

## Function `update_owner_schema`

Add new owners, remove owners to remove, update signatures required.


<pre><code>fun update_owner_schema(multisig_address: address, new_owners: vector&lt;address&gt;, owners_to_remove: vector&lt;address&gt;, optional_new_num_signatures_required: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_owner_schema(<br/>    multisig_address: address,<br/>    new_owners: vector&lt;address&gt;,<br/>    owners_to_remove: vector&lt;address&gt;,<br/>    optional_new_num_signatures_required: Option&lt;u64&gt;,<br/>) acquires MultisigAccount &#123;<br/>    assert_multisig_account_exists(multisig_address);<br/>    let multisig_account_ref_mut &#61;<br/>        borrow_global_mut&lt;MultisigAccount&gt;(multisig_address);<br/>    // Verify no overlap between new owners and owners to remove.<br/>    vector::for_each_ref(&amp;new_owners, &#124;new_owner_ref&#124; &#123;<br/>        assert!(<br/>            !vector::contains(&amp;owners_to_remove, new_owner_ref),<br/>            error::invalid_argument(EOWNERS_TO_REMOVE_NEW_OWNERS_OVERLAP)<br/>        )<br/>    &#125;);<br/>    // If new owners provided, try to add them and emit an event.<br/>    if (vector::length(&amp;new_owners) &gt; 0) &#123;<br/>        vector::append(&amp;mut multisig_account_ref_mut.owners, new_owners);<br/>        validate_owners(<br/>            &amp;multisig_account_ref_mut.owners,<br/>            multisig_address<br/>        );<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            emit(AddOwners &#123; multisig_account: multisig_address, owners_added: new_owners &#125;);<br/>        &#125;;<br/>        emit_event(<br/>            &amp;mut multisig_account_ref_mut.add_owners_events,<br/>            AddOwnersEvent &#123; owners_added: new_owners &#125;<br/>        );<br/>    &#125;;<br/>    // If owners to remove provided, try to remove them.<br/>    if (vector::length(&amp;owners_to_remove) &gt; 0) &#123;<br/>        let owners_ref_mut &#61; &amp;mut multisig_account_ref_mut.owners;<br/>        let owners_removed &#61; vector[];<br/>        vector::for_each_ref(&amp;owners_to_remove, &#124;owner_to_remove_ref&#124; &#123;<br/>            let (found, index) &#61;<br/>                vector::index_of(owners_ref_mut, owner_to_remove_ref);<br/>            if (found) &#123;<br/>                vector::push_back(<br/>                    &amp;mut owners_removed,<br/>                    vector::swap_remove(owners_ref_mut, index)<br/>                );<br/>            &#125;<br/>        &#125;);<br/>        // Only emit event if owner(s) actually removed.<br/>        if (vector::length(&amp;owners_removed) &gt; 0) &#123;<br/>            if (std::features::module_event_migration_enabled()) &#123;<br/>                emit(<br/>                    RemoveOwners &#123; multisig_account: multisig_address, owners_removed &#125;<br/>                );<br/>            &#125;;<br/>            emit_event(<br/>                &amp;mut multisig_account_ref_mut.remove_owners_events,<br/>                RemoveOwnersEvent &#123; owners_removed &#125;<br/>            );<br/>        &#125;<br/>    &#125;;<br/>    // If new signature count provided, try to update count.<br/>    if (option::is_some(&amp;optional_new_num_signatures_required)) &#123;<br/>        let new_num_signatures_required &#61;<br/>            option::extract(&amp;mut optional_new_num_signatures_required);<br/>        assert!(<br/>            new_num_signatures_required &gt; 0,<br/>            error::invalid_argument(EINVALID_SIGNATURES_REQUIRED)<br/>        );<br/>        let old_num_signatures_required &#61;<br/>            multisig_account_ref_mut.num_signatures_required;<br/>        // Only apply update and emit event if a change indicated.<br/>        if (new_num_signatures_required !&#61; old_num_signatures_required) &#123;<br/>            multisig_account_ref_mut.num_signatures_required &#61;<br/>                new_num_signatures_required;<br/>            if (std::features::module_event_migration_enabled()) &#123;<br/>                emit(<br/>                    UpdateSignaturesRequired &#123;<br/>                        multisig_account: multisig_address,<br/>                        old_num_signatures_required,<br/>                        new_num_signatures_required,<br/>                    &#125;<br/>                );<br/>            &#125;;<br/>            emit_event(<br/>                &amp;mut multisig_account_ref_mut.update_signature_required_events,<br/>                UpdateSignaturesRequiredEvent &#123;<br/>                    old_num_signatures_required,<br/>                    new_num_signatures_required,<br/>                &#125;<br/>            );<br/>        &#125;<br/>    &#125;;<br/>    // Verify number of owners.<br/>    let num_owners &#61; vector::length(&amp;multisig_account_ref_mut.owners);<br/>    assert!(<br/>        num_owners &gt;&#61; multisig_account_ref_mut.num_signatures_required,<br/>        error::invalid_state(ENOT_ENOUGH_OWNERS)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;For every multi&#45;signature account, the range of required signatures should always be in the range of one to the total number of owners.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;While creating a MultisigAccount, the function create_with_owners_internal checks that num_signatures_required is in the span from 1 to total count of owners.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The list of owners for a multi&#45;signature account should not contain any duplicate owners, and the multi&#45;signature account itself cannot be listed as one of its owners.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The function validate_owners validates the owner vector that no duplicate entries exists.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;The current value of the next sequence number should not be present in the transaction table, until the next sequence number gets increased.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The add_transaction function increases the next sequence number and only then adds the transaction with the old next sequence number to the transaction table.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;When the last executed sequence number is smaller than the next sequence number by only one unit, no transactions should exist in the multi&#45;signature account&apos;s transactions list.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The get_pending_transactions function retrieves pending transactions by iterating through the transactions table, starting from the last_executed_sequence_number &#43; 1 to the next_sequence_number.&lt;/td&gt;<br/>&lt;td&gt;Audited that MultisigAccount.transactions is empty when last_executed_sequence_number &#61;&#61; next_sequence_number &#45;1&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;The last executed sequence number is always smaller than the next sequence number.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;When creating a new MultisigAccount, the last_executed_sequence_number and next_sequence_number are assigned with 0 and 1 respectively, and from there both these values increase monotonically when a transaction is executed and removed from the table and when new transaction are added respectively.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;The number of pending transactions should be equal to the difference between the next sequence number and the last executed sequence number.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;When a transaction is added, next_sequence_number is incremented. And when a transaction is removed after execution, last_executed_sequence_number is incremented.&lt;/td&gt;<br/>&lt;td&gt;This has been audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;Only transactions with valid sequence number should be fetched.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;Functions such as: 1. get_transaction 2. can_be_executed 3. can_be_rejected 4. vote always validate the given sequence number and only then fetch the associated transaction.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the sequence number is not valid.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;The execution or rejection of a transaction should enforce that the minimum number of required signatures is less or equal to the total number of approvals.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The functions can_be_executed and can_be_rejected perform validation on the number of votes required for execution or rejection.&lt;/td&gt;<br/>&lt;td&gt;Audited that these functions return the correct value.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;9&lt;/td&gt;<br/>&lt;td&gt;The creation of a multi&#45;signature account properly initializes the resources and then it gets published under the corresponding account.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;When creating a MultisigAccount via one of the functions: create_with_existing_account, create_with_existing_account_and_revoke_auth_key, create_with_owners, create, the MultisigAccount data is initialized properly and published to the multisig_account (new or existing).&lt;/td&gt;<br/>&lt;td&gt;Audited that the MultisigAccount is initialized properly.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;10&lt;/td&gt;<br/>&lt;td&gt;Creation of a multi&#45;signature account on top of an existing account should revoke auth key and any previous offered capabilities or control.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The function create_with_existing_account_and_revoke_auth_key, after successfully creating the MultisigAccount, rotates the account to ZeroAuthKey and revokes any offered capabilities of that account.&lt;/td&gt;<br/>&lt;td&gt;Audited that the account&apos;s auth key and the offered capabilities are revoked.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;11&lt;/td&gt;<br/>&lt;td&gt;Upon the creation of a multi&#45;signature account from a bootstrapping account, the ownership of the resultant account should not pertain to the bootstrapping account.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;In create_with_owners_then_remove_bootstrapper function after successful creation of the account the bootstrapping account is removed from the owner vector of the account.&lt;/td&gt;<br/>&lt;td&gt;Audited that the bootstrapping account is not in the owners list.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;12&lt;/td&gt;<br/>&lt;td&gt;Performing any changes on the list of owners such as adding new owners, removing owners, swapping owners should ensure that the number of required signature, for the multi&#45;signature account remains valid.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The following function as used to modify the owners list and the required signature of the account: add_owner, add_owners, add_owners_and_update_signatures_required, remove_owner, remove_owners, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_signatures_required. All of these functions use update_owner_schema function to process these changes, the function validates the owner list while adding and verifies that the account has enough required signatures and updates the owner&apos;s schema.&lt;/td&gt;<br/>&lt;td&gt;Audited that the owners are added successfully. (add_owner, add_owners, add_owners_and_update_signatures_required, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_owner_schema) Audited that the owners are removed successfully. (remove_owner, remove_owners, swap_owner, swap_owners, swap_owners_and_update_signatures_required, update_owner_schema) Audited that the num_signatures_required is updated successfully. (add_owners_and_update_signatures_required, swap_owners_and_update_signatures_required, update_signatures_required, update_owner_schema)&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;13&lt;/td&gt;<br/>&lt;td&gt;The creation of a transaction should be limited to an account owner, which should be automatically considered a voter; additionally, the account&apos;s sequence should increase monotonically.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The following functions can only be called by the owners of the account and create a transaction and uses add_transaction function to gives approval on behalf of the creator and increments the next_sequence_number and finally adds the transaction to the MultsigAccount: create_transaction_with_hash, create_transaction.&lt;/td&gt;<br/>&lt;td&gt;Audited it aborts if the caller is not in the owner&apos;s list of the account. (create_transaction_with_hash, create_transaction) Audited that the transaction is successfully stored in the MultisigAccount.(create_transaction_with_hash, create_transaction, add_transaction) Audited that the creators voted to approve the transaction. (create_transaction_with_hash, create_transaction, add_transaction) Audited that the next_sequence_number increases monotonically. (create_transaction_with_hash, create_transaction, add_transaction)&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;14&lt;/td&gt;<br/>&lt;td&gt;Only owners are allowed to vote for a valid transaction.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Any owner of the MultisigAccount can either approve (approve_transaction) or reject (reject_transaction) a transaction. Both these functions use a generic function to vote for the transaction which validates the caller and the transaction id and adds/updates the vote.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the caller is not in the owner&apos;s list (approve_transaction, reject_transaction, vote_transaction, assert_is_owner). Audited that it aborts if the transaction with the given sequence number doesn&apos;t exist in the account (approve_transaction, reject_transaction, vote_transaction). Audited that the vote is recorded as intended.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;15&lt;/td&gt;<br/>&lt;td&gt;Only owners are allowed to execute a valid transaction, if the number of approvals meets the k&#45;of&#45;n criteria, finally the executed transaction should be removed.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Functions execute_rejected_transaction and validate_multisig_transaction can only be called by the owner which validates the transaction and based on the number of approvals and rejections it proceeds to execute the transactions. For rejected transaction, the transactions are immediately removed from the MultisigAccount via remove_executed_transaction. VM validates the transaction via validate_multisig_transaction and cleans up the transaction via successful_transaction_execution_cleanup and failed_transaction_execution_cleanup.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the caller is not in the owner&apos;s list (execute_rejected_transaction, validate_multisig_transaction). Audited that it aborts if the transaction with the given sequence number doesn&apos;t exist in the account (execute_rejected_transaction, validate_multisig_transaction). Audited that it aborts if the votes (approvals or rejections) are less than num_signatures_required (execute_rejected_transaction, validate_multisig_transaction). Audited that the transaction is removed from the MultisigAccount (execute_rejected_transaction, remove_executed_transaction, successful_transaction_execution_cleanup, failed_transaction_execution_cleanup).&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;16&lt;/td&gt;<br/>&lt;td&gt;Removing an executed transaction from the transactions list should increase the last sequence number monotonically.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;When transactions are removed via remove_executed_transaction (maybe called by VM cleanup or execute_rejected_transaction), the last_executed_sequence_number increases by 1.&lt;/td&gt;<br/>&lt;td&gt;Audited that last_executed_sequence_number is incremented.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;17&lt;/td&gt;<br/>&lt;td&gt;The voting and transaction creation operations should only be available if a multi&#45;signature account exists.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The function assert_multisig_account_exists validates the existence of MultisigAccount under the account.&lt;/td&gt;<br/>&lt;td&gt;Audited that it aborts if the MultisigAccount doesn&apos;t exist on the account.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>



<a id="module-level-spec"></a>

### Module-level Specification


<a id="@Specification_1_metadata"></a>

### Function `metadata`


<pre><code>&#35;[view]<br/>public fun metadata(multisig_account: address): simple_map::SimpleMap&lt;string::String, vector&lt;u8&gt;&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).metadata;<br/></code></pre>



<a id="@Specification_1_num_signatures_required"></a>

### Function `num_signatures_required`


<pre><code>&#35;[view]<br/>public fun num_signatures_required(multisig_account: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).num_signatures_required;<br/></code></pre>



<a id="@Specification_1_owners"></a>

### Function `owners`


<pre><code>&#35;[view]<br/>public fun owners(multisig_account: address): vector&lt;address&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>ensures result &#61;&#61; global&lt;MultisigAccount&gt;(multisig_account).owners;<br/></code></pre>



<a id="@Specification_1_get_transaction"></a>

### Function `get_transaction`


<pre><code>&#35;[view]<br/>public fun get_transaction(multisig_account: address, sequence_number: u64): multisig_account::MultisigTransaction<br/></code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if sequence_number &#61;&#61; 0 &#124;&#124; sequence_number &gt;&#61; multisig_account_resource.next_sequence_number;<br/>aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);<br/>ensures result &#61;&#61; table::spec_get(multisig_account_resource.transactions, sequence_number);<br/></code></pre>



<a id="@Specification_1_get_next_transaction_payload"></a>

### Function `get_next_transaction_payload`


<pre><code>&#35;[view]<br/>public fun get_next_transaction_payload(multisig_account: address, provided_payload: vector&lt;u8&gt;): vector&lt;u8&gt;<br/></code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);<br/>let sequence_number &#61; multisig_account_resource.last_executed_sequence_number &#43; 1;<br/>let transaction &#61; table::spec_get(multisig_account_resource.transactions, sequence_number);<br/>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if multisig_account_resource.last_executed_sequence_number &#43; 1 &gt; MAX_U64;<br/>aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);<br/>ensures option::spec_is_none(transaction.payload) &#61;&#61;&gt; result &#61;&#61; provided_payload;<br/></code></pre>



<a id="@Specification_1_get_next_multisig_account_address"></a>

### Function `get_next_multisig_account_address`


<pre><code>&#35;[view]<br/>public fun get_next_multisig_account_address(creator: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;account::Account&gt;(creator);<br/>let owner_nonce &#61; global&lt;account::Account&gt;(creator).sequence_number;<br/></code></pre>



<a id="@Specification_1_last_resolved_sequence_number"></a>

### Function `last_resolved_sequence_number`


<pre><code>&#35;[view]<br/>public fun last_resolved_sequence_number(multisig_account: address): u64<br/></code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>ensures result &#61;&#61; multisig_account_resource.last_executed_sequence_number;<br/></code></pre>



<a id="@Specification_1_next_sequence_number"></a>

### Function `next_sequence_number`


<pre><code>&#35;[view]<br/>public fun next_sequence_number(multisig_account: address): u64<br/></code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>ensures result &#61;&#61; multisig_account_resource.next_sequence_number;<br/></code></pre>



<a id="@Specification_1_vote"></a>

### Function `vote`


<pre><code>&#35;[view]<br/>public fun vote(multisig_account: address, sequence_number: u64, owner: address): (bool, bool)<br/></code></pre>




<pre><code>let multisig_account_resource &#61; global&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if !exists&lt;MultisigAccount&gt;(multisig_account);<br/>aborts_if sequence_number &#61;&#61; 0 &#124;&#124; sequence_number &gt;&#61; multisig_account_resource.next_sequence_number;<br/>aborts_if !table::spec_contains(multisig_account_resource.transactions, sequence_number);<br/>let transaction &#61; table::spec_get(multisig_account_resource.transactions, sequence_number);<br/>let votes &#61; transaction.votes;<br/>let voted &#61; simple_map::spec_contains_key(votes, owner);<br/>let vote &#61; voted &amp;&amp; simple_map::spec_get(votes, owner);<br/>ensures result_1 &#61;&#61; voted;<br/>ensures result_2 &#61;&#61; vote;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
