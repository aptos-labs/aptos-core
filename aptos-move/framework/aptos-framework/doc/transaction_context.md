
<a id="0x1_transaction_context"></a>

# Module `0x1::transaction_context`



-  [Struct `AUID`](#0x1_transaction_context_AUID)
-  [Struct `EntryFunctionPayload`](#0x1_transaction_context_EntryFunctionPayload)
-  [Struct `MultisigPayload`](#0x1_transaction_context_MultisigPayload)
-  [Constants](#@Constants_0)
-  [Function `get_txn_hash`](#0x1_transaction_context_get_txn_hash)
-  [Function `get_transaction_hash`](#0x1_transaction_context_get_transaction_hash)
-  [Function `generate_unique_address`](#0x1_transaction_context_generate_unique_address)
-  [Function `generate_auid_address`](#0x1_transaction_context_generate_auid_address)
-  [Function `get_script_hash`](#0x1_transaction_context_get_script_hash)
-  [Function `generate_auid`](#0x1_transaction_context_generate_auid)
-  [Function `auid_address`](#0x1_transaction_context_auid_address)
-  [Function `sender`](#0x1_transaction_context_sender)
-  [Function `sender_internal`](#0x1_transaction_context_sender_internal)
-  [Function `secondary_signers`](#0x1_transaction_context_secondary_signers)
-  [Function `secondary_signers_internal`](#0x1_transaction_context_secondary_signers_internal)
-  [Function `gas_payer`](#0x1_transaction_context_gas_payer)
-  [Function `gas_payer_internal`](#0x1_transaction_context_gas_payer_internal)
-  [Function `max_gas_amount`](#0x1_transaction_context_max_gas_amount)
-  [Function `max_gas_amount_internal`](#0x1_transaction_context_max_gas_amount_internal)
-  [Function `gas_unit_price`](#0x1_transaction_context_gas_unit_price)
-  [Function `gas_unit_price_internal`](#0x1_transaction_context_gas_unit_price_internal)
-  [Function `chain_id`](#0x1_transaction_context_chain_id)
-  [Function `chain_id_internal`](#0x1_transaction_context_chain_id_internal)
-  [Function `entry_function_payload`](#0x1_transaction_context_entry_function_payload)
-  [Function `entry_function_payload_internal`](#0x1_transaction_context_entry_function_payload_internal)
-  [Function `account_address`](#0x1_transaction_context_account_address)
-  [Function `module_name`](#0x1_transaction_context_module_name)
-  [Function `function_name`](#0x1_transaction_context_function_name)
-  [Function `type_arg_names`](#0x1_transaction_context_type_arg_names)
-  [Function `args`](#0x1_transaction_context_args)
-  [Function `multisig_payload`](#0x1_transaction_context_multisig_payload)
-  [Function `multisig_payload_internal`](#0x1_transaction_context_multisig_payload_internal)
-  [Function `multisig_address`](#0x1_transaction_context_multisig_address)
-  [Function `inner_entry_function_payload`](#0x1_transaction_context_inner_entry_function_payload)
-  [Specification](#@Specification_1)
    -  [Function `get_txn_hash`](#@Specification_1_get_txn_hash)
    -  [Function `get_transaction_hash`](#@Specification_1_get_transaction_hash)
    -  [Function `generate_unique_address`](#@Specification_1_generate_unique_address)
    -  [Function `generate_auid_address`](#@Specification_1_generate_auid_address)
    -  [Function `get_script_hash`](#@Specification_1_get_script_hash)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `auid_address`](#@Specification_1_auid_address)
    -  [Function `sender_internal`](#@Specification_1_sender_internal)
    -  [Function `secondary_signers_internal`](#@Specification_1_secondary_signers_internal)
    -  [Function `gas_payer_internal`](#@Specification_1_gas_payer_internal)
    -  [Function `max_gas_amount_internal`](#@Specification_1_max_gas_amount_internal)
    -  [Function `gas_unit_price_internal`](#@Specification_1_gas_unit_price_internal)
    -  [Function `chain_id_internal`](#@Specification_1_chain_id_internal)
    -  [Function `entry_function_payload_internal`](#@Specification_1_entry_function_payload_internal)
    -  [Function `multisig_payload_internal`](#@Specification_1_multisig_payload_internal)


<pre><code>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_transaction_context_AUID"></a>

## Struct `AUID`

A wrapper denoting aptos unique identifer (AUID)
for storing an address


<pre><code>struct AUID has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>unique_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_context_EntryFunctionPayload"></a>

## Struct `EntryFunctionPayload`

Represents the entry function payload.


<pre><code>struct EntryFunctionPayload has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>function_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>ty_args_names: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>args: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_context_MultisigPayload"></a>

## Struct `MultisigPayload`

Represents the multisig payload.


<pre><code>struct MultisigPayload has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multisig_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>entry_function_payload: option::Option&lt;transaction_context::EntryFunctionPayload&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_context_ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED"></a>

The transaction context extension feature is not enabled.


<pre><code>const ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_transaction_context_ETRANSACTION_CONTEXT_NOT_AVAILABLE"></a>

Transaction context is only available in the user transaction prologue, execution, or epilogue phases.


<pre><code>const ETRANSACTION_CONTEXT_NOT_AVAILABLE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_transaction_context_get_txn_hash"></a>

## Function `get_txn_hash`

Returns the transaction hash of the current transaction.


<pre><code>fun get_txn_hash(): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun get_txn_hash(): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_transaction_context_get_transaction_hash"></a>

## Function `get_transaction_hash`

Returns the transaction hash of the current transaction.
Internally calls the private function <code>get_txn_hash</code>.
This function is created for to feature gate the <code>get_txn_hash</code> function.


<pre><code>public fun get_transaction_hash(): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_transaction_hash(): vector&lt;u8&gt; &#123;<br/>    get_txn_hash()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_generate_unique_address"></a>

## Function `generate_unique_address`

Returns a universally unique identifier (of type address) generated
by hashing the transaction hash of this transaction and a sequence number
specific to this transaction. This function can be called any
number of times inside a single transaction. Each such call increments
the sequence number and generates a new unique address.
Uses Scheme in types/src/transaction/authenticator.rs for domain separation
from other ways of generating unique addresses.


<pre><code>fun generate_unique_address(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun generate_unique_address(): address;<br/></code></pre>



</details>

<a id="0x1_transaction_context_generate_auid_address"></a>

## Function `generate_auid_address`

Returns a aptos unique identifier. Internally calls
the private function <code>generate_unique_address</code>. This function is
created for to feature gate the <code>generate_unique_address</code> function.


<pre><code>public fun generate_auid_address(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_auid_address(): address &#123;<br/>    generate_unique_address()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_get_script_hash"></a>

## Function `get_script_hash`

Returns the script hash of the current entry function.


<pre><code>public fun get_script_hash(): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public native fun get_script_hash(): vector&lt;u8&gt;;<br/></code></pre>



</details>

<a id="0x1_transaction_context_generate_auid"></a>

## Function `generate_auid`

This method runs <code>generate_unique_address</code> native function and returns
the generated unique address wrapped in the AUID class.


<pre><code>public fun generate_auid(): transaction_context::AUID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun generate_auid(): AUID &#123;<br/>    return AUID &#123;<br/>        unique_address: generate_unique_address()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_auid_address"></a>

## Function `auid_address`

Returns the unique address wrapped in the given AUID struct.


<pre><code>public fun auid_address(auid: &amp;transaction_context::AUID): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun auid_address(auid: &amp;AUID): address &#123;<br/>    auid.unique_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_sender"></a>

## Function `sender`

Returns the sender's address for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun sender(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sender(): address &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    sender_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_sender_internal"></a>

## Function `sender_internal`



<pre><code>fun sender_internal(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun sender_internal(): address;<br/></code></pre>



</details>

<a id="0x1_transaction_context_secondary_signers"></a>

## Function `secondary_signers`

Returns the list of the secondary signers for the current transaction.
If the current transaction has no secondary signers, this function returns an empty vector.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun secondary_signers(): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun secondary_signers(): vector&lt;address&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    secondary_signers_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_secondary_signers_internal"></a>

## Function `secondary_signers_internal`



<pre><code>fun secondary_signers_internal(): vector&lt;address&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun secondary_signers_internal(): vector&lt;address&gt;;<br/></code></pre>



</details>

<a id="0x1_transaction_context_gas_payer"></a>

## Function `gas_payer`

Returns the gas payer address for the current transaction.
It is either the sender's address if no separate gas fee payer is specified for the current transaction,
or the address of the separate gas fee payer if one is specified.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun gas_payer(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun gas_payer(): address &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    gas_payer_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_gas_payer_internal"></a>

## Function `gas_payer_internal`



<pre><code>fun gas_payer_internal(): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun gas_payer_internal(): address;<br/></code></pre>



</details>

<a id="0x1_transaction_context_max_gas_amount"></a>

## Function `max_gas_amount`

Returns the max gas amount in units which is specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun max_gas_amount(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max_gas_amount(): u64 &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    max_gas_amount_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_max_gas_amount_internal"></a>

## Function `max_gas_amount_internal`



<pre><code>fun max_gas_amount_internal(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun max_gas_amount_internal(): u64;<br/></code></pre>



</details>

<a id="0x1_transaction_context_gas_unit_price"></a>

## Function `gas_unit_price`

Returns the gas unit price in Octas which is specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun gas_unit_price(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun gas_unit_price(): u64 &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    gas_unit_price_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_gas_unit_price_internal"></a>

## Function `gas_unit_price_internal`



<pre><code>fun gas_unit_price_internal(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun gas_unit_price_internal(): u64;<br/></code></pre>



</details>

<a id="0x1_transaction_context_chain_id"></a>

## Function `chain_id`

Returns the chain ID specified for the current transaction.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun chain_id(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun chain_id(): u8 &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    chain_id_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_chain_id_internal"></a>

## Function `chain_id_internal`



<pre><code>fun chain_id_internal(): u8<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun chain_id_internal(): u8;<br/></code></pre>



</details>

<a id="0x1_transaction_context_entry_function_payload"></a>

## Function `entry_function_payload`

Returns the entry function payload if the current transaction has such a payload. Otherwise, return <code>None</code>.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun entry_function_payload(): option::Option&lt;transaction_context::EntryFunctionPayload&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun entry_function_payload(): Option&lt;EntryFunctionPayload&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    entry_function_payload_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_entry_function_payload_internal"></a>

## Function `entry_function_payload_internal`



<pre><code>fun entry_function_payload_internal(): option::Option&lt;transaction_context::EntryFunctionPayload&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun entry_function_payload_internal(): Option&lt;EntryFunctionPayload&gt;;<br/></code></pre>



</details>

<a id="0x1_transaction_context_account_address"></a>

## Function `account_address`

Returns the account address of the entry function payload.


<pre><code>public fun account_address(payload: &amp;transaction_context::EntryFunctionPayload): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun account_address(payload: &amp;EntryFunctionPayload): address &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.account_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_module_name"></a>

## Function `module_name`

Returns the module name of the entry function payload.


<pre><code>public fun module_name(payload: &amp;transaction_context::EntryFunctionPayload): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun module_name(payload: &amp;EntryFunctionPayload): String &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.module_name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_function_name"></a>

## Function `function_name`

Returns the function name of the entry function payload.


<pre><code>public fun function_name(payload: &amp;transaction_context::EntryFunctionPayload): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun function_name(payload: &amp;EntryFunctionPayload): String &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.function_name<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_type_arg_names"></a>

## Function `type_arg_names`

Returns the type arguments names of the entry function payload.


<pre><code>public fun type_arg_names(payload: &amp;transaction_context::EntryFunctionPayload): vector&lt;string::String&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun type_arg_names(payload: &amp;EntryFunctionPayload): vector&lt;String&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.ty_args_names<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_args"></a>

## Function `args`

Returns the arguments of the entry function payload.


<pre><code>public fun args(payload: &amp;transaction_context::EntryFunctionPayload): vector&lt;vector&lt;u8&gt;&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun args(payload: &amp;EntryFunctionPayload): vector&lt;vector&lt;u8&gt;&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.args<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_multisig_payload"></a>

## Function `multisig_payload`

Returns the multisig payload if the current transaction has such a payload. Otherwise, return <code>None</code>.
This function aborts if called outside of the transaction prologue, execution, or epilogue phases.


<pre><code>public fun multisig_payload(): option::Option&lt;transaction_context::MultisigPayload&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multisig_payload(): Option&lt;MultisigPayload&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    multisig_payload_internal()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_multisig_payload_internal"></a>

## Function `multisig_payload_internal`



<pre><code>fun multisig_payload_internal(): option::Option&lt;transaction_context::MultisigPayload&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun multisig_payload_internal(): Option&lt;MultisigPayload&gt;;<br/></code></pre>



</details>

<a id="0x1_transaction_context_multisig_address"></a>

## Function `multisig_address`

Returns the multisig account address of the multisig payload.


<pre><code>public fun multisig_address(payload: &amp;transaction_context::MultisigPayload): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multisig_address(payload: &amp;MultisigPayload): address &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.multisig_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_context_inner_entry_function_payload"></a>

## Function `inner_entry_function_payload`

Returns the inner entry function payload of the multisig payload.


<pre><code>public fun inner_entry_function_payload(payload: &amp;transaction_context::MultisigPayload): option::Option&lt;transaction_context::EntryFunctionPayload&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun inner_entry_function_payload(payload: &amp;MultisigPayload): Option&lt;EntryFunctionPayload&gt; &#123;<br/>    assert!(features::transaction_context_extension_enabled(), error::invalid_state(ETRANSACTION_CONTEXT_EXTENSION_NOT_ENABLED));<br/>    payload.entry_function_payload<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_get_txn_hash"></a>

### Function `get_txn_hash`


<pre><code>fun get_txn_hash(): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_get_txn_hash();<br/></code></pre>




<a id="0x1_transaction_context_spec_get_txn_hash"></a>


<pre><code>fun spec_get_txn_hash(): vector&lt;u8&gt;;<br/></code></pre>



<a id="@Specification_1_get_transaction_hash"></a>

### Function `get_transaction_hash`


<pre><code>public fun get_transaction_hash(): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures result &#61;&#61; spec_get_txn_hash();<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
ensures [abstract] len(result) &#61;&#61; 32;<br/></code></pre>



<a id="@Specification_1_generate_unique_address"></a>

### Function `generate_unique_address`


<pre><code>fun generate_unique_address(): address<br/></code></pre>




<pre><code>pragma opaque;<br/>ensures [abstract] result &#61;&#61; spec_generate_unique_address();<br/></code></pre>




<a id="0x1_transaction_context_spec_generate_unique_address"></a>


<pre><code>fun spec_generate_unique_address(): address;<br/></code></pre>



<a id="@Specification_1_generate_auid_address"></a>

### Function `generate_auid_address`


<pre><code>public fun generate_auid_address(): address<br/></code></pre>




<pre><code>pragma opaque;<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
ensures [abstract] result &#61;&#61; spec_generate_unique_address();<br/></code></pre>



<a id="@Specification_1_get_script_hash"></a>

### Function `get_script_hash`


<pre><code>public fun get_script_hash(): vector&lt;u8&gt;<br/></code></pre>





<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Fetching the transaction hash should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The get_transaction_hash function calls the native function get_txn_hash, which fetches the NativeTransactionContext struct and returns the txn_hash field.</td>
<td>Audited that the native function returns the txn hash, whose size is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-1">get_txn_hash</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Fetching the unique address should never abort.</td>
<td>Low</td>
<td>The function auid_address returns the unique address from a supplied AUID resource.</td>
<td>Formally verified via <a href="#high-level-req-2">auid_address</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Generating the unique address should return a vector with 32 bytes.</td>
<td>Medium</td>
<td>The generate_auid_address function checks calls the native function generate_unique_address which fetches the NativeTransactionContext struct, increments the auid_counter by one, and then creates a new authentication key from a preimage, which is then returned.</td>
<td>Audited that the native function returns an address, and the length of an address is 32 bytes. This has been modeled as the abstract postcondition that the returned vector is of length 32. Formally verified via <a href="#high-level-req-3">generate_auid_address</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Fetching the script hash of the current entry function should never fail and should return a vector with 32 bytes if the transaction payload is a script, otherwise an empty vector.</td>
<td>Low</td>
<td>The native function get_script_hash returns the NativeTransactionContext.script_hash field.</td>
<td>Audited that the native function holds the required property. This has been modeled as the abstract spec. Formally verified via <a href="#high-level-req-4">get_script_hash</a>.</td>
</tr>

</table>



<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma opaque;<br/>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_get_script_hash();<br/>ensures [abstract] len(result) &#61;&#61; 32;<br/></code></pre>




<a id="0x1_transaction_context_spec_get_script_hash"></a>


<pre><code>fun spec_get_script_hash(): vector&lt;u8&gt;;<br/></code></pre>



<a id="@Specification_1_auid_address"></a>

### Function `auid_address`


<pre><code>public fun auid_address(auid: &amp;transaction_context::AUID): address<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if false;<br/></code></pre>



<a id="@Specification_1_sender_internal"></a>

### Function `sender_internal`


<pre><code>fun sender_internal(): address<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_secondary_signers_internal"></a>

### Function `secondary_signers_internal`


<pre><code>fun secondary_signers_internal(): vector&lt;address&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_gas_payer_internal"></a>

### Function `gas_payer_internal`


<pre><code>fun gas_payer_internal(): address<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_max_gas_amount_internal"></a>

### Function `max_gas_amount_internal`


<pre><code>fun max_gas_amount_internal(): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_gas_unit_price_internal"></a>

### Function `gas_unit_price_internal`


<pre><code>fun gas_unit_price_internal(): u64<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_chain_id_internal"></a>

### Function `chain_id_internal`


<pre><code>fun chain_id_internal(): u8<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_entry_function_payload_internal"></a>

### Function `entry_function_payload_internal`


<pre><code>fun entry_function_payload_internal(): option::Option&lt;transaction_context::EntryFunctionPayload&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_multisig_payload_internal"></a>

### Function `multisig_payload_internal`


<pre><code>fun multisig_payload_internal(): option::Option&lt;transaction_context::MultisigPayload&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
