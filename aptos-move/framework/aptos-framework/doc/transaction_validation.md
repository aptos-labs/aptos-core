
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


<pre><code>use 0x1::account;
use 0x1::aptos_coin;
use 0x1::bcs;
use 0x1::chain_id;
use 0x1::coin;
use 0x1::error;
use 0x1::features;
use 0x1::option;
use 0x1::signer;
use 0x1::system_addresses;
use 0x1::timestamp;
use 0x1::transaction_fee;
</code></pre>



<a id="0x1_transaction_validation_TransactionValidation"></a>

## Resource `TransactionValidation`

This holds information that will be picked up by the VM to call the
correct chain-specific prologue and epilogue functions


<pre><code>struct TransactionValidation has key
</code></pre>



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


<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_transaction_validation_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code>const EOUT_OF_GAS: u64 &#61; 6;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EACCOUNT_DOES_NOT_EXIST"></a>



<pre><code>const PROLOGUE_EACCOUNT_DOES_NOT_EXIST: u64 &#61; 1004;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EBAD_CHAIN_ID"></a>



<pre><code>const PROLOGUE_EBAD_CHAIN_ID: u64 &#61; 1007;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ECANT_PAY_GAS_DEPOSIT"></a>



<pre><code>const PROLOGUE_ECANT_PAY_GAS_DEPOSIT: u64 &#61; 1005;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EFEE_PAYER_NOT_ENABLED"></a>



<pre><code>const PROLOGUE_EFEE_PAYER_NOT_ENABLED: u64 &#61; 1010;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT"></a>



<pre><code>const PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT: u64 &#61; 1011;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY"></a>

Prologue errors. These are separated out from the other errors in this
module since they are mapped separately to major VM statuses, and are
important to the semantics of the system.


<pre><code>const PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY: u64 &#61; 1001;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH"></a>



<pre><code>const PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 &#61; 1009;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG: u64 &#61; 1008;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW: u64 &#61; 1003;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD"></a>



<pre><code>const PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD: u64 &#61; 1002;
</code></pre>



<a id="0x1_transaction_validation_PROLOGUE_ETRANSACTION_EXPIRED"></a>



<pre><code>const PROLOGUE_ETRANSACTION_EXPIRED: u64 &#61; 1006;
</code></pre>



<a id="0x1_transaction_validation_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, script_prologue_name: vector&lt;u8&gt;, module_prologue_name: vector&lt;u8&gt;, multi_agent_prologue_name: vector&lt;u8&gt;, user_epilogue_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(
    aptos_framework: &amp;signer,
    script_prologue_name: vector&lt;u8&gt;,
    // module_prologue_name is deprecated and not used.
    module_prologue_name: vector&lt;u8&gt;,
    multi_agent_prologue_name: vector&lt;u8&gt;,
    user_epilogue_name: vector&lt;u8&gt;,
) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);

    move_to(aptos_framework, TransactionValidation &#123;
        module_addr: @aptos_framework,
        module_name: b&quot;transaction_validation&quot;,
        script_prologue_name,
        // module_prologue_name is deprecated and not used.
        module_prologue_name,
        multi_agent_prologue_name,
        user_epilogue_name,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_collect_deposit"></a>

## Function `collect_deposit`

Called in prologue to optionally hold some amount for special txns (e.g. randomness txns).
<code>return_deposit()</code> should be invoked in the corresponding epilogue with the same arguments.


<pre><code>fun collect_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun collect_deposit(gas_payer: address, amount: Option&lt;u64&gt;) &#123;
    if (option::is_some(&amp;amount)) &#123;
        let amount &#61; option::extract(&amp;mut amount);
        let balance &#61; coin::balance&lt;AptosCoin&gt;(gas_payer);
        assert!(balance &gt;&#61; amount, error::invalid_state(PROLOGUE_EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT));
        transaction_fee::burn_fee(gas_payer, amount);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_return_deposit"></a>

## Function `return_deposit`

Called in epilogue to optionally released the amount held in prologue for special txns (e.g. randomness txns).


<pre><code>fun return_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun return_deposit(gas_payer: address, amount: Option&lt;u64&gt;) &#123;
    if (option::is_some(&amp;amount)) &#123;
        let amount &#61; option::extract(&amp;mut amount);
        transaction_fee::mint_and_refund(gas_payer, amount);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_prologue_common"></a>

## Function `prologue_common`



<pre><code>fun prologue_common(sender: signer, gas_payer: address, txn_sequence_number: u64, txn_authentication_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun prologue_common(
    sender: signer,
    gas_payer: address,
    txn_sequence_number: u64,
    txn_authentication_key: vector&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
) &#123;
    assert!(
        timestamp::now_seconds() &lt; txn_expiration_time,
        error::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),
    );
    assert!(chain_id::get() &#61;&#61; chain_id, error::invalid_argument(PROLOGUE_EBAD_CHAIN_ID));

    let transaction_sender &#61; signer::address_of(&amp;sender);

    if (
        transaction_sender &#61;&#61; gas_payer
        &#124;&#124; account::exists_at(transaction_sender)
        &#124;&#124; !features::sponsored_automatic_account_creation_enabled()
        &#124;&#124; txn_sequence_number &gt; 0
    ) &#123;
        assert!(account::exists_at(transaction_sender), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));
        assert!(
            txn_authentication_key &#61;&#61; account::get_authentication_key(transaction_sender),
            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );

        let account_sequence_number &#61; account::get_sequence_number(transaction_sender);
        assert!(
            txn_sequence_number &lt; (1u64 &lt;&lt; 63),
            error::out_of_range(PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG)
        );

        assert!(
            txn_sequence_number &gt;&#61; account_sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD)
        );

        assert!(
            txn_sequence_number &#61;&#61; account_sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)
        );
    &#125; else &#123;
        // In this case, the transaction is sponsored and the account does not exist, so ensure
        // the default values match.
        assert!(
            txn_sequence_number &#61;&#61; 0,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)
        );

        assert!(
            txn_authentication_key &#61;&#61; bcs::to_bytes(&amp;transaction_sender),
            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );
    &#125;;

    let max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;
    assert!(
        coin::is_account_registered&lt;AptosCoin&gt;(gas_payer),
        error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
    );
    assert!(
        coin::is_balance_at_least&lt;AptosCoin&gt;(gas_payer, max_transaction_fee),
        error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT)
    );
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue"></a>

## Function `script_prologue`



<pre><code>fun script_prologue(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun script_prologue(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
    _script_hash: vector&lt;u8&gt;,
) &#123;
    let gas_payer &#61; signer::address_of(&amp;sender);
    prologue_common(sender, gas_payer, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_script_prologue_collect_deposit"></a>

## Function `script_prologue_collect_deposit`

<code>script_prologue()</code> then collect an optional deposit depending on the txn.

Deposit collection goes last so <code>script_prologue()</code> doesn't have to be aware of the deposit logic.


<pre><code>fun script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, script_hash: vector&lt;u8&gt;, required_deposit: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun script_prologue_collect_deposit(
    sender: signer,
    txn_sequence_number: u64,
    txn_public_key: vector&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
    script_hash: vector&lt;u8&gt;,
    required_deposit: Option&lt;u64&gt;,
) &#123;
    let gas_payer &#61; signer::address_of(&amp;sender);
    script_prologue(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id, script_hash);
    collect_deposit(gas_payer, required_deposit);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_script_prologue"></a>

## Function `multi_agent_script_prologue`



<pre><code>fun multi_agent_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun multi_agent_script_prologue(
    sender: signer,
    txn_sequence_number: u64,
    txn_sender_public_key: vector&lt;u8&gt;,
    secondary_signer_addresses: vector&lt;address&gt;,
    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
) &#123;
    let sender_addr &#61; signer::address_of(&amp;sender);
    prologue_common(
        sender,
        sender_addr,
        txn_sequence_number,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        chain_id,
    );
    multi_agent_common_prologue(secondary_signer_addresses, secondary_signer_public_key_hashes);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_multi_agent_common_prologue"></a>

## Function `multi_agent_common_prologue`



<pre><code>fun multi_agent_common_prologue(secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun multi_agent_common_prologue(
    secondary_signer_addresses: vector&lt;address&gt;,
    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,
) &#123;
    let num_secondary_signers &#61; vector::length(&amp;secondary_signer_addresses);
    assert!(
        vector::length(&amp;secondary_signer_public_key_hashes) &#61;&#61; num_secondary_signers,
        error::invalid_argument(PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH),
    );

    let i &#61; 0;
    while (&#123;
        spec &#123;
            invariant i &lt;&#61; num_secondary_signers;
            invariant forall j in 0..i:
                account::exists_at(secondary_signer_addresses[j])
                &amp;&amp; secondary_signer_public_key_hashes[j]
                   &#61;&#61; account::get_authentication_key(secondary_signer_addresses[j]);
        &#125;;
        (i &lt; num_secondary_signers)
    &#125;) &#123;
        let secondary_address &#61; &#42;vector::borrow(&amp;secondary_signer_addresses, i);
        assert!(account::exists_at(secondary_address), error::invalid_argument(PROLOGUE_EACCOUNT_DOES_NOT_EXIST));

        let signer_public_key_hash &#61; &#42;vector::borrow(&amp;secondary_signer_public_key_hashes, i);
        assert!(
            signer_public_key_hash &#61;&#61; account::get_authentication_key(secondary_address),
            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );
        i &#61; i &#43; 1;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue"></a>

## Function `fee_payer_script_prologue`



<pre><code>fun fee_payer_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fee_payer_script_prologue(
    sender: signer,
    txn_sequence_number: u64,
    txn_sender_public_key: vector&lt;u8&gt;,
    secondary_signer_addresses: vector&lt;address&gt;,
    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,
    fee_payer_address: address,
    fee_payer_public_key_hash: vector&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
) &#123;
    assert!(features::fee_payer_enabled(), error::invalid_state(PROLOGUE_EFEE_PAYER_NOT_ENABLED));
    prologue_common(
        sender,
        fee_payer_address,
        txn_sequence_number,
        txn_sender_public_key,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        chain_id,
    );
    multi_agent_common_prologue(secondary_signer_addresses, secondary_signer_public_key_hashes);
    assert!(
        fee_payer_public_key_hash &#61;&#61; account::get_authentication_key(fee_payer_address),
        error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
    );
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_fee_payer_script_prologue_collect_deposit"></a>

## Function `fee_payer_script_prologue_collect_deposit`

<code>fee_payer_script_prologue()</code> then collect an optional deposit depending on the txn.

Deposit collection goes last so <code>fee_payer_script_prologue()</code> doesn't have to be aware of the deposit logic.


<pre><code>fun fee_payer_script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, required_deposit: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun fee_payer_script_prologue_collect_deposit(
    sender: signer,
    txn_sequence_number: u64,
    txn_sender_public_key: vector&lt;u8&gt;,
    secondary_signer_addresses: vector&lt;address&gt;,
    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;,
    fee_payer_address: address,
    fee_payer_public_key_hash: vector&lt;u8&gt;,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    txn_expiration_time: u64,
    chain_id: u8,
    required_deposit: Option&lt;u64&gt;,
) &#123;
    fee_payer_script_prologue(
        sender,
        txn_sequence_number,
        txn_sender_public_key,
        secondary_signer_addresses,
        secondary_signer_public_key_hashes,
        fee_payer_address,
        fee_payer_public_key_hash,
        txn_gas_price,
        txn_max_gas_units,
        txn_expiration_time,
        chain_id,
    );
    collect_deposit(fee_payer_address, required_deposit);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue"></a>

## Function `epilogue`

Epilogue function is run after a transaction is successfully executed.
Called by the Adapter


<pre><code>fun epilogue(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue(
    account: signer,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) &#123;
    let addr &#61; signer::address_of(&amp;account);
    epilogue_gas_payer(account, addr, storage_fee_refunded, txn_gas_price, txn_max_gas_units, gas_units_remaining);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_return_deposit"></a>

## Function `epilogue_return_deposit`

Return the deposit held in prologue, then <code>epilogue()</code>.

Deposit return goes first so <code>epilogue()</code> doesn't have to be aware of this change.


<pre><code>fun epilogue_return_deposit(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_return_deposit(
    account: signer,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    required_deposit: Option&lt;u64&gt;,
) &#123;
    let gas_payer &#61; signer::address_of(&amp;account);
    return_deposit(gas_payer, required_deposit);
    epilogue(
        account,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
    );
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer"></a>

## Function `epilogue_gas_payer`

Epilogue function with explicit gas payer specified, is run after a transaction is successfully executed.
Called by the Adapter


<pre><code>fun epilogue_gas_payer(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_gas_payer(
    account: signer,
    gas_payer: address,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64
) &#123;
    assert!(txn_max_gas_units &gt;&#61; gas_units_remaining, error::invalid_argument(EOUT_OF_GAS));
    let gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;

    assert!(
        (txn_gas_price as u128) &#42; (gas_used as u128) &lt;&#61; MAX_U64,
        error::out_of_range(EOUT_OF_GAS)
    );
    let transaction_fee_amount &#61; txn_gas_price &#42; gas_used;
    // it&apos;s important to maintain the error code consistent with vm
    // to do failed transaction cleanup.
    assert!(
        coin::is_balance_at_least&lt;AptosCoin&gt;(gas_payer, transaction_fee_amount),
        error::out_of_range(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
    );

    let amount_to_burn &#61; if (features::collect_and_distribute_gas_fees()) &#123;
        // TODO(gas): We might want to distinguish the refundable part of the charge and burn it or track
        // it separately, so that we don&apos;t increase the total supply by refunding.

        // If transaction fees are redistributed to validators, collect them here for
        // later redistribution.
        transaction_fee::collect_fee(gas_payer, transaction_fee_amount);
        0
    &#125; else &#123;
        // Otherwise, just burn the fee.
        // TODO: this branch should be removed completely when transaction fee collection
        // is tested and is fully proven to work well.
        transaction_fee_amount
    &#125;;

    if (amount_to_burn &gt; storage_fee_refunded) &#123;
        let burn_amount &#61; amount_to_burn &#45; storage_fee_refunded;
        transaction_fee::burn_fee(gas_payer, burn_amount);
    &#125; else if (amount_to_burn &lt; storage_fee_refunded) &#123;
        let mint_amount &#61; storage_fee_refunded &#45; amount_to_burn;
        transaction_fee::mint_and_refund(gas_payer, mint_amount)
    &#125;;

    // Increment sequence number
    let addr &#61; signer::address_of(&amp;account);
    account::increment_sequence_number(addr);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_validation_epilogue_gas_payer_return_deposit"></a>

## Function `epilogue_gas_payer_return_deposit`

Return the deposit held in prologue to the gas payer, then <code>epilogue_gas_payer()</code>.

Deposit return should go first so <code>epilogue_gas_payer()</code> doesn't have to be aware of this change.


<pre><code>fun epilogue_gas_payer_return_deposit(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun epilogue_gas_payer_return_deposit(
    account: signer,
    gas_payer: address,
    storage_fee_refunded: u64,
    txn_gas_price: u64,
    txn_max_gas_units: u64,
    gas_units_remaining: u64,
    required_deposit: Option&lt;u64&gt;,
) &#123;
    return_deposit(gas_payer, required_deposit);
    epilogue_gas_payer(
        account,
        gas_payer,
        storage_fee_refunded,
        txn_gas_price,
        txn_max_gas_units,
        gas_units_remaining,
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
<td>The sender of a transaction should have sufficient coin balance to pay the transaction fee.</td>
<td>High</td>
<td>The prologue_common function asserts that the transaction sender has enough coin balance to be paid as the max_transaction_fee.</td>
<td>Formally verified via <a href="#high-level-req-1">PrologueCommonAbortsIf</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

<tr>
<td>2</td>
<td>All secondary signer addresses are verified to be authentic through a validation process.</td>
<td>Critical</td>
<td>The function multi_agent_script_prologue ensures that each secondary signer address undergoes authentication validation, including verification of account existence and authentication key matching, confirming their authenticity.</td>
<td>Formally verified via <a href="#high-level-req-2">multi_agent_script_prologue</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

<tr>
<td>3</td>
<td>After successful execution, base the transaction fee on the configuration set by the features library.</td>
<td>High</td>
<td>The epilogue function collects the transaction fee for either redistribution or burning based on the feature::collect_and_distribute_gas_fees result.</td>
<td>Formally Verified via <a href="#high-level-req-3">epilogue</a>. Moreover, the native transaction validation patterns have been manually audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, script_prologue_name: vector&lt;u8&gt;, module_prologue_name: vector&lt;u8&gt;, multi_agent_prologue_name: vector&lt;u8&gt;, user_epilogue_name: vector&lt;u8&gt;)
</code></pre>


Ensure caller is <code>aptos_framework</code>.
Aborts if TransactionValidation already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(addr);
aborts_if exists&lt;TransactionValidation&gt;(addr);
ensures exists&lt;TransactionValidation&gt;(addr);
</code></pre>


Create a schema to reuse some code.
Give some constraints that may abort according to the conditions.


<a id="0x1_transaction_validation_PrologueCommonAbortsIf"></a>


<pre><code>schema PrologueCommonAbortsIf &#123;
    sender: signer;
    gas_payer: address;
    txn_sequence_number: u64;
    txn_authentication_key: vector&lt;u8&gt;;
    txn_gas_price: u64;
    txn_max_gas_units: u64;
    txn_expiration_time: u64;
    chain_id: u8;
    aborts_if !exists&lt;CurrentTimeMicroseconds&gt;(@aptos_framework);
    aborts_if !(timestamp::now_seconds() &lt; txn_expiration_time);
    aborts_if !exists&lt;ChainId&gt;(@aptos_framework);
    aborts_if !(chain_id::get() &#61;&#61; chain_id);
    let transaction_sender &#61; signer::address_of(sender);
    aborts_if (
        !features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION)
        &#124;&#124; account::exists_at(transaction_sender)
        &#124;&#124; transaction_sender &#61;&#61; gas_payer
        &#124;&#124; txn_sequence_number &gt; 0
    ) &amp;&amp; (
        !(txn_sequence_number &gt;&#61; global&lt;Account&gt;(transaction_sender).sequence_number)
        &#124;&#124; !(txn_authentication_key &#61;&#61; global&lt;Account&gt;(transaction_sender).authentication_key)
        &#124;&#124; !account::exists_at(transaction_sender)
        &#124;&#124; !(txn_sequence_number &#61;&#61; global&lt;Account&gt;(transaction_sender).sequence_number)
    );
    aborts_if features::spec_is_enabled(features::SPONSORED_AUTOMATIC_ACCOUNT_CREATION)
        &amp;&amp; transaction_sender !&#61; gas_payer
        &amp;&amp; txn_sequence_number &#61;&#61; 0
        &amp;&amp; !account::exists_at(transaction_sender)
        &amp;&amp; txn_authentication_key !&#61; bcs::to_bytes(transaction_sender);
    aborts_if !(txn_sequence_number &lt; (1u64 &lt;&lt; 63));
    let max_transaction_fee &#61; txn_gas_price &#42; txn_max_gas_units;
    aborts_if max_transaction_fee &gt; MAX_U64;
    aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);
    // This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
    aborts_if !(global&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer).coin.value &gt;&#61; max_transaction_fee);
&#125;
</code></pre>



<a id="@Specification_1_collect_deposit"></a>

### Function `collect_deposit`


<pre><code>fun collect_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_return_deposit"></a>

### Function `return_deposit`


<pre><code>fun return_deposit(gas_payer: address, amount: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_prologue_common"></a>

### Function `prologue_common`


<pre><code>fun prologue_common(sender: signer, gas_payer: address, txn_sequence_number: u64, txn_authentication_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>




<pre><code>pragma verify &#61; false;
include PrologueCommonAbortsIf;
</code></pre>



<a id="@Specification_1_script_prologue"></a>

### Function `script_prologue`


<pre><code>fun script_prologue(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, _script_hash: vector&lt;u8&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
include PrologueCommonAbortsIf &#123;
    gas_payer: signer::address_of(sender),
    txn_authentication_key: txn_public_key
&#125;;
</code></pre>




<a id="0x1_transaction_validation_MultiAgentPrologueCommonAbortsIf"></a>


<pre><code>schema MultiAgentPrologueCommonAbortsIf &#123;
    secondary_signer_addresses: vector&lt;address&gt;;
    secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;;
    let num_secondary_signers &#61; len(secondary_signer_addresses);
    aborts_if len(secondary_signer_public_key_hashes) !&#61; num_secondary_signers;
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    aborts_if exists i in 0..num_secondary_signers:
        !account::exists_at(secondary_signer_addresses[i])
            &#124;&#124; secondary_signer_public_key_hashes[i] !&#61;
            account::get_authentication_key(secondary_signer_addresses[i]);
    ensures forall i in 0..num_secondary_signers:
        account::exists_at(secondary_signer_addresses[i])
            &amp;&amp; secondary_signer_public_key_hashes[i] &#61;&#61;
                account::get_authentication_key(secondary_signer_addresses[i]);
&#125;
</code></pre>



<a id="@Specification_1_script_prologue_collect_deposit"></a>

### Function `script_prologue_collect_deposit`


<pre><code>fun script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_public_key: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, script_hash: vector&lt;u8&gt;, required_deposit: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_multi_agent_script_prologue"></a>

### Function `multi_agent_script_prologue`


<pre><code>fun multi_agent_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>


Aborts if length of public key hashed vector
not equal the number of singers.


<pre><code>pragma verify_duration_estimate &#61; 120;
let gas_payer &#61; signer::address_of(sender);
pragma verify &#61; false;
include PrologueCommonAbortsIf &#123;
    gas_payer,
    txn_sequence_number,
    txn_authentication_key: txn_sender_public_key,
&#125;;
include MultiAgentPrologueCommonAbortsIf &#123;
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
&#125;;
</code></pre>



<a id="@Specification_1_multi_agent_common_prologue"></a>

### Function `multi_agent_common_prologue`


<pre><code>fun multi_agent_common_prologue(secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>




<pre><code>include MultiAgentPrologueCommonAbortsIf &#123;
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
&#125;;
</code></pre>



<a id="@Specification_1_fee_payer_script_prologue"></a>

### Function `fee_payer_script_prologue`


<pre><code>fun fee_payer_script_prologue(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;
aborts_if !features::spec_is_enabled(features::FEE_PAYER_ENABLED);
let gas_payer &#61; fee_payer_address;
include PrologueCommonAbortsIf &#123;
    gas_payer,
    txn_sequence_number,
    txn_authentication_key: txn_sender_public_key,
&#125;;
include MultiAgentPrologueCommonAbortsIf &#123;
    secondary_signer_addresses,
    secondary_signer_public_key_hashes,
&#125;;
aborts_if !account::exists_at(gas_payer);
aborts_if !(fee_payer_public_key_hash &#61;&#61; account::get_authentication_key(gas_payer));
aborts_if !features::spec_fee_payer_enabled();
</code></pre>



<a id="@Specification_1_fee_payer_script_prologue_collect_deposit"></a>

### Function `fee_payer_script_prologue_collect_deposit`


<pre><code>fun fee_payer_script_prologue_collect_deposit(sender: signer, txn_sequence_number: u64, txn_sender_public_key: vector&lt;u8&gt;, secondary_signer_addresses: vector&lt;address&gt;, secondary_signer_public_key_hashes: vector&lt;vector&lt;u8&gt;&gt;, fee_payer_address: address, fee_payer_public_key_hash: vector&lt;u8&gt;, txn_gas_price: u64, txn_max_gas_units: u64, txn_expiration_time: u64, chain_id: u8, required_deposit: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_epilogue"></a>

### Function `epilogue`


<pre><code>fun epilogue(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exists.
Skip transaction_fee::burn_fee verification.


<pre><code>pragma verify &#61; false;
include EpilogueGasPayerAbortsIf &#123; gas_payer: signer::address_of(account) &#125;;
</code></pre>



<a id="@Specification_1_epilogue_return_deposit"></a>

### Function `epilogue_return_deposit`


<pre><code>fun epilogue_return_deposit(account: signer, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_epilogue_gas_payer"></a>

### Function `epilogue_gas_payer`


<pre><code>fun epilogue_gas_payer(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64)
</code></pre>


Abort according to the conditions.
<code>AptosCoinCapabilities</code> and <code>CoinInfo</code> should exist.
Skip transaction_fee::burn_fee verification.


<pre><code>pragma verify &#61; false;
include EpilogueGasPayerAbortsIf;
</code></pre>




<a id="0x1_transaction_validation_EpilogueGasPayerAbortsIf"></a>


<pre><code>schema EpilogueGasPayerAbortsIf &#123;
    account: signer;
    gas_payer: address;
    storage_fee_refunded: u64;
    txn_gas_price: u64;
    txn_max_gas_units: u64;
    gas_units_remaining: u64;
    aborts_if !(txn_max_gas_units &gt;&#61; gas_units_remaining);
    let gas_used &#61; txn_max_gas_units &#45; gas_units_remaining;
    aborts_if !(txn_gas_price &#42; gas_used &lt;&#61; MAX_U64);
    let transaction_fee_amount &#61; txn_gas_price &#42; gas_used;
    let addr &#61; signer::address_of(account);
    let pre_account &#61; global&lt;account::Account&gt;(addr);
    let post account &#61; global&lt;account::Account&gt;(addr);
    aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(gas_payer);
    aborts_if !exists&lt;Account&gt;(addr);
    aborts_if !(global&lt;Account&gt;(addr).sequence_number &lt; MAX_U64);
    ensures account.sequence_number &#61;&#61; pre_account.sequence_number &#43; 1;
    let collect_fee_enabled &#61; features::spec_is_enabled(features::COLLECT_AND_DISTRIBUTE_GAS_FEES);
    let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;
    let aggr &#61; collected_fees.value;
    let aggr_val &#61; aggregator::spec_aggregator_get_val(aggr);
    let aggr_lim &#61; aggregator::spec_get_limit(aggr);
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    aborts_if collect_fee_enabled &amp;&amp; !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
    aborts_if collect_fee_enabled &amp;&amp; transaction_fee_amount &gt; 0 &amp;&amp; aggr_val &#43; transaction_fee_amount &gt; aggr_lim;
    let amount_to_burn&#61; if (collect_fee_enabled) &#123;
        0
    &#125; else &#123;
        transaction_fee_amount &#45; storage_fee_refunded
    &#125;;
    let apt_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
    let maybe_apt_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;
    let total_supply_enabled &#61; option::spec_is_some(maybe_apt_supply);
    let apt_supply &#61; option::spec_borrow(maybe_apt_supply);
    let apt_supply_value &#61; optional_aggregator::optional_aggregator_value(apt_supply);
    let post post_maybe_apt_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr).supply;
    let post post_apt_supply &#61; option::spec_borrow(post_maybe_apt_supply);
    let post post_apt_supply_value &#61; optional_aggregator::optional_aggregator_value(post_apt_supply);
    aborts_if amount_to_burn &gt; 0 &amp;&amp; !exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
    aborts_if amount_to_burn &gt; 0 &amp;&amp; !exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(apt_addr);
    aborts_if amount_to_burn &gt; 0 &amp;&amp; total_supply_enabled &amp;&amp; apt_supply_value &lt; amount_to_burn;
    ensures total_supply_enabled &#61;&#61;&gt; apt_supply_value &#45; amount_to_burn &#61;&#61; post_apt_supply_value;
    let amount_to_mint &#61; if (collect_fee_enabled) &#123;
        storage_fee_refunded
    &#125; else &#123;
        storage_fee_refunded &#45; transaction_fee_amount
    &#125;;
    let total_supply &#61; coin::supply&lt;AptosCoin&gt;;
    let post post_total_supply &#61; coin::supply&lt;AptosCoin&gt;;
    aborts_if amount_to_mint &gt; 0 &amp;&amp; !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(addr);
    aborts_if amount_to_mint &gt; 0 &amp;&amp; !exists&lt;AptosCoinMintCapability&gt;(@aptos_framework);
    aborts_if amount_to_mint &gt; 0 &amp;&amp; total_supply &#43; amount_to_mint &gt; MAX_U128;
    ensures amount_to_mint &gt; 0 &#61;&#61;&gt; post_total_supply &#61;&#61; total_supply &#43; amount_to_mint;
    let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
    aborts_if (amount_to_mint !&#61; 0) &amp;&amp; !exists&lt;coin::CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr);
    include coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: amount_to_mint &#125;;
&#125;
</code></pre>



<a id="@Specification_1_epilogue_gas_payer_return_deposit"></a>

### Function `epilogue_gas_payer_return_deposit`


<pre><code>fun epilogue_gas_payer_return_deposit(account: signer, gas_payer: address, storage_fee_refunded: u64, txn_gas_price: u64, txn_max_gas_units: u64, gas_units_remaining: u64, required_deposit: option::Option&lt;u64&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
