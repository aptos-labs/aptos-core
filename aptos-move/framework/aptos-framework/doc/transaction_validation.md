
<a id="0x1_transaction_validation"></a>

# Module `0x1::transaction_validation`



-  [Resource `TransactionValidation`](#0x1_transaction_validation_TransactionValidation)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_transaction_validation_initialize)
-  [Function `collect_deposit`](#0x1_transaction_validation_collect_deposit)
-  [Function `return_deposit`](#0x1_transaction_validation_return_deposit)
-  [Function `prologue_common`](#0x1_transaction_validation_prologue_common)
-  [Function `script_prologue`](#0x1_transaction_validation_script_prologue)
-  [Function `script_prologue_collect_deposit`](#0x1_transaction_validation_script_prologue_collect_deposit)
-  [Function `multi_agent_script_prologue`](#0x1_transaction_validation_multi_agent_script_prologue)
-  [Function `multi_agent_common_prologue`](#0x1_transaction_validation_multi_agent_common_prologue)
-  [Function `fee_payer_script_prologue`](#0x1_transaction_validation_fee_payer_script_prologue)
-  [Function `fee_payer_script_prologue_collect_deposit`](#0x1_transaction_validation_fee_payer_script_prologue_collect_deposit)
-  [Function `epilogue`](#0x1_transaction_validation_epilogue)
-  [Function `epilogue_return_deposit`](#0x1_transaction_validation_epilogue_return_deposit)
-  [Function `epilogue_gas_payer`](#0x1_transaction_validation_epilogue_gas_payer)
-  [Function `epilogue_gas_payer_return_deposit`](#0x1_transaction_validation_epilogue_gas_payer_return_deposit)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `collect_deposit`](#@Specification_1_collect_deposit)
    -  [Function `return_deposit`](#@Specification_1_return_deposit)
    -  [Function `prologue_common`](#@Specification_1_prologue_common)
    -  [Function `script_prologue`](#@Specification_1_script_prologue)
    -  [Function `script_prologue_collect_deposit`](#@Specification_1_script_prologue_collect_deposit)
    -  [Function `multi_agent_script_prologue`](#@Specification_1_multi_agent_script_prologue)
    -  [Function `multi_agent_common_prologue`](#@Specification_1_multi_agent_common_prologue)
    -  [Function `fee_payer_script_prologue`](#@Specification_1_fee_payer_script_prologue)
    -  [Function `fee_payer_script_prologue_collect_deposit`](#@Specification_1_fee_payer_script_prologue_collect_deposit)
    -  [Function `epilogue`](#@Specification_1_epilogue)
    -  [Function `epilogue_return_deposit`](#@Specification_1_epilogue_return_deposit)
    -  [Function `epilogue_gas_payer`](#@Specification_1_epilogue_gas_payer)
    -  [Function `epilogue_gas_payer_return_deposit`](#@Specification_1_epilogue_gas_payer_return_deposit)


<pre><code>use 0x1::account;<br/>use 0x1::aptos_coin;<br/>use 0x1::bcs;<br/>use 0x1::chain_id;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/>use 0x1::transaction_fee;<br/></code></pre>



<a id="0x1_transaction_validation_TransactionValidation"></a>

## Resource `TransactionValidation`

This holds information that will be picked up by the VM to call the<br/> correct chain&#45;specific prologue and epilogue functions


<pre><code>struct TransactionValidation has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>module_addr: address</code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>script_prologue_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>module_prologue_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>multi_agent_prologue_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>user_epilogue_name: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_validation_MAX_U64"></a>

MSB is used to indicate a gas payer tx


<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_transaction_validation_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code>const EOUT_OF_GAS: u64 &#61; 6;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code>const PROLOGUE_EACCOUNT_DOES_NOT_EXIST: u64 &#61; 1004;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID"></a>



<pre><code>const PROLOGUE_EBAD_CHAIN_ID: u64 &#61; 1007;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT"></a>



<pre><code>const PROLOGUE_ECANT_PAY_GAS_DEPOSIT: u64 &#61; 1005;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED"></a>



<pre><code>const PROLOGUE_EFEE_PAYER_NOT_ENABLED: u64 &#61; 1010;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT"></a>



<pre><code>const PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT: u64 &#61; 1011;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY"></a>

Prologue errors. These are separated out from the other errors in this<br/> module since they are mapped separately to major VM statuses, and are<br/> important to the semantics of the system.


<pre><code>const PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY: u64 &#61; 1001;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH"></a>



<pre><code>const PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 &#61; 1009;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG: u64 &#61; 1008;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW: u64 &#61; 1003;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD: u64 &#61; 1002;<br/></code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED"></a>



<pre><code>const PROLOGUE_ETRANSACTION_EXPIRED: u64 &#61; 1006;<br/></code></pre>



<a id="0x1_transaction_validation_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, script_prologue_name: vector&lt;u8&gt;, module_prologue_name: vector&lt;u8&gt;, multi_agent_prologue_name: vector&lt;u8&gt;, user_epilogue_name: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(<br/>    aptos_framework: &amp;signer,<br/>    script_prologue_name: vector&lt;u8&gt;,<br/>    // module_prologue_name is deprecated and not used.<br/>    module_prologue_name: vector&lt;u8&gt;,<br/>    multi_agent_prologue_name: vector&lt;u8&gt;,<br/>    user_epilogue_name: vector&lt;u8&gt;,<br/>) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    move_to(aptos_framework, TransactionValidation &#123;<br/>        module_addr: @aptos_framework,<br/>        module_name: b&quot;transaction_validation&quot;,<br/>        script_prologue_name,<br/>        // module_prologue_name is deprecated and not used.<br/>        module_prologue_name,<br/>        multi_agent_prologue_name,<br/>        user_epilogue_name,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_collect_deposit"></a>

## Function `collect_deposit`

Called in prologue to optionally hold some amount for special txns (e.g. randomness txns).<br/> <code>return_deposit()</code> should be invoked in the corresponding epilogue with the same arguments.


<pre><code>fun collect_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun collect_deposit(gas_payer: address, amount: Option&lt;u64&gt;) &#123;<br/>    if (option::is_some(&amp;amount)) &#123;<br/>        let amount &#61; option::extract(&amp;mut amount);<br/>        let balance &#61; coin::balance&lt;AptosCoin&gt;(gas_payer);<br/>        assert!(balance &gt;&#61; amount, error::invalid_state(PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT));<br/>        transaction_fee::burn_fee(gas_payer, amount);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_return_deposit"></a>

## Function `return_deposit`

Called in epilogue to optionally released the amount held in prologue for special txns (e.g. randomness txns).


<pre><code>fun return_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun return_deposit(gas_payer: address, amount: Option&lt;u64&gt;) &#123;<br/>    if (option::is_some(&amp;amount)) &#123;<br/>        let amount &#61; option::extract(&amp;mut amount);<br/>        transaction_fee::mint_and_refund(gas_payer, amount);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code>fun prologue_common(sender: signer, gas_payer: address, txn_sequence_number: u64, txn_authentication_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun prologue_common(<br/>    sender: signer,<br/>    gas_payer: address,<br/>    txn_sequence_number: u64,<br/>    txn_authentication_key: vector&lt;u8&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>) &#123;<br/>    assert!(<br/>        timestamp::now_seconds() &lt; txn_expiration_time,<br/>        error::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),<br/>    );<br/>    assert!(chain_id::get() &#61;&#61; chain_id, error::invalid_argument(PROLOGUE_EBAD_CHAIN_ID));<br/><br/>    let transaction_sender &#61; signer::address_of(&amp;sender);<br/><br/>    if (<br/>        transaction_sender &#61;&#61; gas_payer<br/>        &#124;&#124; account::exists_at(transaction_sender)<br/>        &#124;&#124; !features::sponsored_automatic_account_creation_enabled()<br/>        &#124;&#124; txn_sequence_number &gt; 0<br/>    ) &#123;<br/>        assert!(account::exists_at(transaction_sender), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));<br/>        assert!(<br/>            txn_authentication_key &#61;&#61; account::get_authentication_key(transaction_sender),<br/>            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),<br/>        );<br/><br/>        let account_sequence_number &#61; account::get_sequence_number(transaction_sender);<br/>        assert!(<br/>            txn_sequence_number &lt; (1u64 &lt;&lt; 63),<br/>            error::out_of_range(PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG)<br/>        );<br/><br/>        assert!(<br/>            txn_sequence_number &gt;&#61; account_sequence_number,<br/>            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD)<br/>        );<br/><br/>        assert!(<br/>            txn_sequence_number &#61;&#61; account_sequence_number,<br/>            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)<br/>        );<br/>    &#125; else &#123;<br/>        // In this case, the transaction is sponsored and the account does not exist, so ensure<br/>        // the default values match.<br/>        assert!(<br/>            txn_sequence_number &#61;&#61; 0,<br/>            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)<br/>        );<br/><br/>        assert!(<br/>            txn_authentication_key &#61;&#61; bcs::to_bytes(&amp;transaction_sender),<br/>            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),<br/>        );<br/>    &#125;;<br/><br/>    let max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;<br/>    assert!(<br/>        coin::is_account_registered&lt;AptosCoin&gt;(gas_payer),<br/>        error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),<br/>    );<br/>    assert!(<br/>        coin::is_balance_at_least&lt;AptosCoin&gt;(gas_payer, max_transaction_fee),<br/>        error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT)<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue"></a>

## Function `script_prologue`



<pre><code>fun script_prologue(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun script_prologue(<br/>    sender: signer,<br/>    txn_sequence_number: u64,<br/>    txn_public_key: vector&lt;u8&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>    _script_hash: vector&lt;u8&gt;,<br/>) &#123;<br/>    let gas_payer &#61; signer::address_of(&amp;sender);<br/>    prologue_common(sender, gas_payer, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue_collect_deposit"></a>

## Function `script_prologue_collect_deposit`

<code>script_prologue()</code> then collect an optional deposit depending on the txn.<br/><br/> Deposit collection goes last so <code>script_prologue()</code> doesn&apos;t have to be aware of the deposit logic.


<pre><code>fun script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, script_hash: vector&lt;u8&gt;, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun script_prologue_collect_deposit(<br/>    sender: signer,<br/>    txn_sequence_number: u64,<br/>    txn_public_key: vector&lt;u8&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>    script_hash: vector&lt;u8&gt;,<br/>    required_deposit: Option&lt;u64&gt;,<br/>) &#123;<br/>    let gas_payer &#61; signer::address_of(&amp;sender);<br/>    script_prologue(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id, script_hash);<br/>    collect_deposit(gas_payer, required_deposit);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code>fun multi_agent_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun multi_agent_script_prologue(<br/>    sender: signer,<br/>    txn_sequence_number: u64,<br/>    txn_sender_public_key: vector&lt;u8&gt;,<br/>    secondary_signer_addresses: vector&lt;address&gt;,<br/>    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>) &#123;<br/>    let sender_addr &#61; signer::address_of(&amp;sender);<br/>    prologue_common(<br/>        sender,<br/>        sender_addr,<br/>        txn_sequence_number,<br/>        txn_sender_public_key,<br/>        txn_gas_price,<br/>        txn_max_gas_units,<br/>        txn_expiration_time,<br/>        chain_id,<br/>    );<br/>    multi_agent_common_prologue(secondary_signer_addresses, secondary_signer_public_key_hashes);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code>fun multi_agent_common_prologue(secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun multi_agent_common_prologue(<br/>    secondary_signer_addresses: vector&lt;address&gt;,<br/>    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,<br/>) &#123;<br/>    let num_secondary_signers &#61; vector::length(&amp;secondary_signer_addresses);<br/>    assert!(<br/>        vector::length(&amp;secondary_signer_public_key_hashes) &#61;&#61; num_secondary_signers,<br/>        error::invalid_argument(PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH),<br/>    );<br/><br/>    let i &#61; 0;<br/>    while (&#123;<br/>        spec &#123;<br/>            invariant i &lt;&#61; num_secondary_signers;<br/>            invariant forall j in 0..i:<br/>                account::exists_at(secondary_signer_addresses[j])<br/>                &amp;&amp; secondary_signer_public_key_hashes[j]<br/>                   &#61;&#61; account::get_authentication_key(secondary_signer_addresses[j]);<br/>        &#125;;<br/>        (i &lt; num_secondary_signers)<br/>    &#125;) &#123;<br/>        let secondary_address &#61; &#42;vector::borrow(&amp;secondary_signer_addresses, i);<br/>        assert!(account::exists_at(secondary_address), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));<br/><br/>        let signer_public_key_hash &#61; &#42;vector::borrow(&amp;secondary_signer_public_key_hashes, i);<br/>        assert!(<br/>            signer_public_key_hash &#61;&#61; account::get_authentication_key(secondary_address),<br/>            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),<br/>        );<br/>        i &#61; i &#43; 1;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue"></a>

## Function `fee_payer_script_prologue`



<pre><code>fun fee_payer_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fee_payer_script_prologue(<br/>    sender: signer,<br/>    txn_sequence_number: u64,<br/>    txn_sender_public_key: vector&lt;u8&gt;,<br/>    secondary_signer_addresses: vector&lt;address&gt;,<br/>    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,<br/>    fee_payer_address: address,<br/>    fee_payer_public_key_hash: vector&lt;u8&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>) &#123;<br/>    assert!(features::fee_payer_enabled(), error::invalid_state(PROLOGUE_EFEE_PAYER_NOT_ENABLED));<br/>    prologue_common(<br/>        sender,<br/>        fee_payer_address,<br/>        txn_sequence_number,<br/>        txn_sender_public_key,<br/>        txn_gas_price,<br/>        txn_max_gas_units,<br/>        txn_expiration_time,<br/>        chain_id,<br/>    );<br/>    multi_agent_common_prologue(secondary_signer_addresses, secondary_signer_public_key_hashes);<br/>    assert!(<br/>        fee_payer_public_key_hash &#61;&#61; account::get_authentication_key(fee_payer_address),<br/>        error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue_collect_deposit"></a>

## Function `fee_payer_script_prologue_collect_deposit`

<code>fee_payer_script_prologue()</code> then collect an optional deposit depending on the txn.<br/><br/> Deposit collection goes last so <code>fee_payer_script_prologue()</code> doesn&apos;t have to be aware of the deposit logic.


<pre><code>fun fee_payer_script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fee_payer_script_prologue_collect_deposit(<br/>    sender: signer,<br/>    txn_sequence_number: u64,<br/>    txn_sender_public_key: vector&lt;u8&gt;,<br/>    secondary_signer_addresses: vector&lt;address&gt;,<br/>    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,<br/>    fee_payer_address: address,<br/>    fee_payer_public_key_hash: vector&lt;u8&gt;,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    txn_expiration_time: u64,<br/>    chain_id: u8,<br/>    required_deposit: Option&lt;u64&gt;,<br/>) &#123;<br/>    fee_payer_script_prologue(<br/>        sender,<br/>        txn_sequence_number,<br/>        txn_sender_public_key,<br/>        secondary_signer_addresses,<br/>        secondary_signer_public_key_hashes,<br/>        fee_payer_address,<br/>        fee_payer_public_key_hash,<br/>        txn_gas_price,<br/>        txn_max_gas_units,<br/>        txn_expiration_time,<br/>        chain_id,<br/>    );<br/>    collect_deposit(fee_payer_address, required_deposit);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue"></a>

## Function `epilogue`

Epilogue function is run after a transaction is successfully executed.<br/> Called by the Adapter


<pre><code>fun epilogue(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue(<br/>    account: signer,<br/>    storage_fee_refunded: u64,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    gas_units_remaining: u64<br/>) &#123;<br/>    let addr &#61; signer::address_of(&amp;account);<br/>    epilogue_gas_payer(account, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_return_deposit"></a>

## Function `epilogue_return_deposit`

Return the deposit held in prologue, then <code>epilogue()</code>.<br/><br/> Deposit return goes first so <code>epilogue()</code> doesn&apos;t have to be aware of this change.


<pre><code>fun epilogue_return_deposit(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_return_deposit(<br/>    account: signer,<br/>    storage_fee_refunded: u64,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    gas_units_remaining: u64,<br/>    required_deposit: Option&lt;u64&gt;,<br/>) &#123;<br/>    let gas_payer &#61; signer::address_of(&amp;account);<br/>    return_deposit(gas_payer, required_deposit);<br/>    epilogue(<br/>        account,<br/>        storage_fee_refunded,<br/>        txn_gas_price,<br/>        txn_max_gas_units,<br/>        gas_units_remaining,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer"></a>

## Function `epilogue_gas_payer`

Epilogue function with explicit gas payer specified, is run after a transaction is successfully executed.<br/> Called by the Adapter


<pre><code>fun epilogue_gas_payer(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_gas_payer(<br/>    account: signer,<br/>    gas_payer: address,<br/>    storage_fee_refunded: u64,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    gas_units_remaining: u64<br/>) &#123;<br/>    assert!(txn_max_gas_units &gt;&#61; gas_units_remaining, error::invalid_argument(EOUT_OF_GAS));<br/>    let gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;<br/><br/>    assert!(<br/>        (txn_gas_price as u128) &#42; (gas_used as u128) &lt;&#61; MAX_U64,<br/>        error::out_of_range(EOUT_OF_GAS)<br/>    );<br/>    let transaction_fee_amount &#61; txn_gas_price &#42; gas_used;<br/>    // it&apos;s important to maintain the error code consistent with vm<br/>    // to do failed transaction cleanup.<br/>    assert!(<br/>        coin::is_balance_at_least&lt;AptosCoin&gt;(gas_payer, transaction_fee_amount),<br/>        error::out_of_range(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),<br/>    );<br/><br/>    let amount_to_burn &#61; if (features::collect_and_distribute_gas_fees()) &#123;<br/>        // TODO(gas): We might want to distinguish the refundable part of the charge and burn it or track<br/>        // it separately, so that we don&apos;t increase the total supply by refunding.<br/><br/>        // If transaction fees are redistributed to validators, collect them here for<br/>        // later redistribution.<br/>        transaction_fee::collect_fee(gas_payer, transaction_fee_amount);<br/>        0<br/>    &#125; else &#123;<br/>        // Otherwise, just burn the fee.<br/>        // TODO: this branch should be removed completely when transaction fee collection<br/>        // is tested and is fully proven to work well.<br/>        transaction_fee_amount<br/>    &#125;;<br/><br/>    if (amount_to_burn &gt; storage_fee_refunded) &#123;<br/>        let burn_amount &#61; amount_to_burn &#45; storage_fee_refunded;<br/>        transaction_fee::burn_fee(gas_payer, burn_amount);<br/>    &#125; else if (amount_to_burn &lt; storage_fee_refunded) &#123;<br/>        let mint_amount &#61; storage_fee_refunded &#45; amount_to_burn;<br/>        transaction_fee::mint_and_refund(gas_payer, mint_amount)<br/>    &#125;;<br/><br/>    // Increment sequence number<br/>    let addr &#61; signer::address_of(&amp;account);<br/>    account::increment_sequence_number(addr);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_return_deposit"></a>

## Function `epilogue_gas_payer_return_deposit`

Return the deposit held in prologue to the gas payer, then <code>epilogue_gas_payer()</code>.<br/><br/> Deposit return should go first so <code>epilogue_gas_payer()</code> doesn&apos;t have to be aware of this change.


<pre><code>fun epilogue_gas_payer_return_deposit(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_gas_payer_return_deposit(<br/>    account: signer,<br/>    gas_payer: address,<br/>    storage_fee_refunded: u64,<br/>    txn_gas_price: u64,<br/>    txn_max_gas_units: u64,<br/>    gas_units_remaining: u64,<br/>    required_deposit: Option&lt;u64&gt;,<br/>) &#123;<br/>    return_deposit(gas_payer, required_deposit);<br/>    epilogue_gas_payer(<br/>        account,<br/>        gas_payer,<br/>        storage_fee_refunded,<br/>        txn_gas_price,<br/>        txn_max_gas_units,<br/>        gas_units_remaining,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The sender of a transaction should have sufficient coin balance to pay the transaction fee.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The prologue_common function asserts that the transaction sender has enough coin balance to be paid as the max_transaction_fee.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;PrologueCommonAbortsIf&lt;/a&gt;. Moreover, the native transaction validation patterns have been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;All secondary signer addresses are verified to be authentic through a validation process.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The function multi_agent_script_prologue ensures that each secondary signer address undergoes authentication validation, including verification of account existence and authentication key matching, confirming their authenticity.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;multi_agent_script_prologue&lt;/a&gt;. Moreover, the native transaction validation patterns have been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;After successful execution, base the transaction fee on the configuration set by the features library.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The epilogue function collects the transaction fee for either redistribution or burning based on the feature::collect_and_distribute_gas_fees result.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;epilogue&lt;/a&gt;. Moreover, the native transaction validation patterns have been manually audited.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, script_prologue_name: vector&lt;u8&gt;, module_prologue_name: vector&lt;u8&gt;, multi_agent_prologue_name: vector&lt;u8&gt;, user_epilogue_name: vector&lt;u8&gt;)<br/></code></pre>


Ensure caller is <code>aptos_framework</code>.<br/> Aborts if TransactionValidation already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(addr);<br/>aborts_if exists&lt;TransactionValidation&gt;(addr);<br/>ensures exists&lt;TransactionValidation&gt;(addr);<br/></code></pre>


Create a schema to reuse some code.<br/> Give some constraints that may abort according to the conditions.


<a id="0x1_transaction_validation_PrologueCommonAbortsIf"></a>


<pre><code>schema PrologueCommonAbortsIf &#123;<br/>sender: signer;<br/>gas_payer: address;<br/>txn_sequence_number: u64;<br/>txn_authentication_key: vector&lt;u8&gt;;<br/>txn_gas_price: u64;<br/>txn_max_gas_units: u64;<br/>txn_expiration_time: u64;<br/>chain_id: u8;<br/>aborts_if !exists&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if !(timestamp::now_seconds() &lt; txn_expiration_time);<br/>aborts_if !exists&lt;ChainId&gt;(@aptos_framework);<br/>aborts_if !(chain_id::get() &#61;&#61; chain_id);<br/>let transaction_sender &#61; signer::address_of(sender);<br/>aborts_if (<br/>    !features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION)<br/>    &#124;&#124; account::exists_at(transaction_sender)<br/>    &#124;&#124; transaction_sender &#61;&#61; gas_payer<br/>    &#124;&#124; txn_sequence_number &gt; 0<br/>) &amp;&amp; (<br/>    !(txn_sequence_number &gt;&#61; global&lt;Account&gt;(transaction_sender).sequence_number)<br/>    &#124;&#124; !(txn_authentication_key &#61;&#61; global&lt;Account&gt;(transaction_sender).authentication_key)<br/>    &#124;&#124; !account::exists_at(transaction_sender)<br/>    &#124;&#124; !(txn_sequence_number &#61;&#61; global&lt;Account&gt;(transaction_sender).sequence_number)<br/>);<br/>aborts_if features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION)<br/>    &amp;&amp; transaction_sender !&#61; gas_payer<br/>    &amp;&amp; txn_sequence_number &#61;&#61; 0<br/>    &amp;&amp; !account::exists_at(transaction_sender)<br/>    &amp;&amp; txn_authentication_key !&#61; bcs::to_bytes(transaction_sender);<br/>aborts_if !(txn_sequence_number &lt; (1u64 &lt;&lt; 63));<br/>let max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;<br/>aborts_if max_transaction_fee &gt; MAX_U64;<br/>aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
    aborts_if !(global&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer).coin.value &gt;&#61; max_transaction_fee);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_collect_deposit"></a>

### Function `collect_deposit`


<pre><code>fun collect_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_return_deposit"></a>

### Function `return_deposit`


<pre><code>fun return_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_prologue_common"></a>

### Function `prologue_common`


<pre><code>fun prologue_common(sender: signer, gas_payer: address, txn_sequence_number: u64, txn_authentication_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include PrologueCommonAbortsIf;<br/></code></pre>



<a id="@Specification_1_script_prologue"></a>

### Function `script_prologue`


<pre><code>fun script_prologue(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>include PrologueCommonAbortsIf &#123;<br/>    gas_payer: signer::address_of(sender),<br/>    txn_authentication_key: txn_public_key<br/>&#125;;<br/></code></pre>




<a id="0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf"></a>


<pre><code>schema MultiAgentPrologueCommonAbortsIf &#123;<br/>secondary_signer_addresses: vector&lt;address&gt;;<br/>secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;;<br/>let num_secondary_signers &#61; len(secondary_signer_addresses);<br/>aborts_if len(secondary_signer_public_key_hashes) !&#61; num_secondary_signers;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
    aborts_if exists i in 0..num_secondary_signers:<br/>    !account::exists_at(secondary_signer_addresses[i])<br/>        &#124;&#124; secondary_signer_public_key_hashes[i] !&#61;<br/>        account::get_authentication_key(secondary_signer_addresses[i]);<br/>ensures forall i in 0..num_secondary_signers:<br/>    account::exists_at(secondary_signer_addresses[i])<br/>        &amp;&amp; secondary_signer_public_key_hashes[i] &#61;&#61;<br/>            account::get_authentication_key(secondary_signer_addresses[i]);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_script_prologue_collect_deposit"></a>

### Function `script_prologue_collect_deposit`


<pre><code>fun script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, script_hash: vector&lt;u8&gt;, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_multi_agent_script_prologue"></a>

### Function `multi_agent_script_prologue`


<pre><code>fun multi_agent_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>


Aborts if length of public key hashed vector<br/> not equal the number of singers.


<pre><code>pragma verify_duration_estimate &#61; 120;<br/>let gas_payer &#61; signer::address_of(sender);<br/>pragma verify &#61; false;<br/>include PrologueCommonAbortsIf &#123;<br/>    gas_payer,<br/>    txn_sequence_number,<br/>    txn_authentication_key: txn_sender_public_key,<br/>&#125;;<br/>include MultiAgentPrologueCommonAbortsIf &#123;<br/>    secondary_signer_addresses,<br/>    secondary_signer_public_key_hashes,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_multi_agent_common_prologue"></a>

### Function `multi_agent_common_prologue`


<pre><code>fun multi_agent_common_prologue(secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>




<pre><code>include MultiAgentPrologueCommonAbortsIf &#123;<br/>    secondary_signer_addresses,<br/>    secondary_signer_public_key_hashes,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_fee_payer_script_prologue"></a>

### Function `fee_payer_script_prologue`


<pre><code>fun fee_payer_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>aborts_if !features::spec_is_enabled(features::FEE_PAYER_ENABLED);<br/>let gas_payer &#61; fee_payer_address;<br/>include PrologueCommonAbortsIf &#123;<br/>    gas_payer,<br/>    txn_sequence_number,<br/>    txn_authentication_key: txn_sender_public_key,<br/>&#125;;<br/>include MultiAgentPrologueCommonAbortsIf &#123;<br/>    secondary_signer_addresses,<br/>    secondary_signer_public_key_hashes,<br/>&#125;;<br/>aborts_if !account::exists_at(gas_payer);<br/>aborts_if !(fee_payer_public_key_hash &#61;&#61; account::get_authentication_key(gas_payer));<br/>aborts_if !features::spec_fee_payer_enabled();<br/></code></pre>



<a id="@Specification_1_fee_payer_script_prologue_collect_deposit"></a>

### Function `fee_payer_script_prologue_collect_deposit`


<pre><code>fun fee_payer_script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_epilogue"></a>

### Function `epilogue`


<pre><code>fun epilogue(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br/></code></pre>


Abort according to the conditions.<br/> <code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exists.<br/> Skip transaction_fee::burn_fee verification.


<pre><code>pragma verify &#61; false;<br/>include EpilogueGasPayerAbortsIf &#123; gas_payer: signer::address_of(account) &#125;;<br/></code></pre>



<a id="@Specification_1_epilogue_return_deposit"></a>

### Function `epilogue_return_deposit`


<pre><code>fun epilogue_return_deposit(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_epilogue_gas_payer"></a>

### Function `epilogue_gas_payer`


<pre><code>fun epilogue_gas_payer(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)<br/></code></pre>


Abort according to the conditions.<br/> <code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exist.<br/> Skip transaction_fee::burn_fee verification.


<pre><code>pragma verify &#61; false;<br/>include EpilogueGasPayerAbortsIf;<br/></code></pre>




<a id="0x1_transaction_validation_EpilogueGasPayerAbortsIf"></a>


<pre><code>schema EpilogueGasPayerAbortsIf &#123;<br/>account: signer;<br/>gas_payer: address;<br/>storage_fee_refunded: u64;<br/>txn_gas_price: u64;<br/>txn_max_gas_units: u64;<br/>gas_units_remaining: u64;<br/>aborts_if !(txn_max_gas_units &gt;&#61; gas_units_remaining);<br/>let gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;<br/>aborts_if !(txn_gas_price &#42; gas_used &lt;&#61; MAX_U64);<br/>let transaction_fee_amount &#61; txn_gas_price &#42; gas_used;<br/>let addr &#61; signer::address_of(account);<br/>let pre_account &#61; global&lt;account::Account&gt;(addr);<br/>let post account &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if !(global&lt;Account&gt;(addr).sequence_number &lt; MAX_U64);<br/>ensures account.sequence_number &#61;&#61; pre_account.sequence_number &#43; 1;<br/>let collect_fee_enabled &#61; features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES);<br/>let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;<br/>let aggr &#61; collected_fees.value;<br/>let aggr_val &#61; aggregator::spec_aggregator_get_val(aggr);<br/>let aggr_lim &#61; aggregator::spec_get_limit(aggr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
    aborts_if collect_fee_enabled &amp;&amp; !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>aborts_if collect_fee_enabled &amp;&amp; transaction_fee_amount &gt; 0 &amp;&amp; aggr_val &#43; transaction_fee_amount &gt; aggr_lim;<br/>let amount_to_burn&#61; if (collect_fee_enabled) &#123;<br/>    0<br/>&#125; else &#123;<br/>    transaction_fee_amount &#45; storage_fee_refunded<br/>&#125;;<br/>let apt_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>let maybe_apt_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;<br/>let total_supply_enabled &#61; option::spec_is_some(maybe_apt_supply);<br/>let apt_supply &#61; option::spec_borrow(maybe_apt_supply);<br/>let apt_supply_value &#61; optional_aggregator::optional_aggregator_value(apt_supply);<br/>let post post_maybe_apt_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;<br/>let post post_apt_supply &#61; option::spec_borrow(post_maybe_apt_supply);<br/>let post post_apt_supply_value &#61; optional_aggregator::optional_aggregator_value(post_apt_supply);<br/>aborts_if amount_to_burn &gt; 0 &amp;&amp; !exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>aborts_if amount_to_burn &gt; 0 &amp;&amp; !exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr);<br/>aborts_if amount_to_burn &gt; 0 &amp;&amp; total_supply_enabled &amp;&amp; apt_supply_value &lt; amount_to_burn;<br/>ensures total_supply_enabled &#61;&#61;&gt; apt_supply_value &#45; amount_to_burn &#61;&#61; post_apt_supply_value;<br/>let amount_to_mint &#61; if (collect_fee_enabled) &#123;<br/>    storage_fee_refunded<br/>&#125; else &#123;<br/>    storage_fee_refunded &#45; transaction_fee_amount<br/>&#125;;<br/>let total_supply &#61; coin::supply&lt;AptosCoin&gt;;<br/>let post post_total_supply &#61; coin::supply&lt;AptosCoin&gt;;<br/>aborts_if amount_to_mint &gt; 0 &amp;&amp; !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(addr);<br/>aborts_if amount_to_mint &gt; 0 &amp;&amp; !exists&lt;AptosCoinMintCapability&gt;(@aptos_framework);<br/>aborts_if amount_to_mint &gt; 0 &amp;&amp; total_supply &#43; amount_to_mint &gt; MAX_U128;<br/>ensures amount_to_mint &gt; 0 &#61;&#61;&gt; post_total_supply &#61;&#61; total_supply &#43; amount_to_mint;<br/>let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>aborts_if (amount_to_mint !&#61; 0) &amp;&amp; !exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr);<br/>include coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: amount_to_mint &#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_epilogue_gas_payer_return_deposit"></a>

### Function `epilogue_gas_payer_return_deposit`


<pre><code>fun epilogue_gas_payer_return_deposit(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
