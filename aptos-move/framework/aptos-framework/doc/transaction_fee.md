
<a name="0x1_transaction_fee"></a>

# Module `0x1::transaction_fee`

This module provides an interface to burn or collect and redistribute transaction fees.


-  [Resource `AptosCoinCapabilities`](#0x1_transaction_fee_AptosCoinCapabilities)
-  [Resource `CollectedFeesPerBlockAndBatches`](#0x1_transaction_fee_CollectedFeesPerBlockAndBatches)
-  [Resource `CollectedFeesPerBlock`](#0x1_transaction_fee_CollectedFeesPerBlock)
-  [Constants](#@Constants_0)
-  [Function `initialize_fee_collection_and_distributions`](#0x1_transaction_fee_initialize_fee_collection_and_distributions)
-  [Function `is_fees_collection_enabled`](#0x1_transaction_fee_is_fees_collection_enabled)
-  [Function `upgrade_distribution_percentages`](#0x1_transaction_fee_upgrade_distribution_percentages)
-  [Function `register_proposers_for_fee_collection`](#0x1_transaction_fee_register_proposers_for_fee_collection)
-  [Function `process_collected_fees`](#0x1_transaction_fee_process_collected_fees)
-  [Function `burn_fee`](#0x1_transaction_fee_burn_fee)
-  [Function `collect_fee_for_batch`](#0x1_transaction_fee_collect_fee_for_batch)
-  [Function `store_aptos_coin_burn_cap`](#0x1_transaction_fee_store_aptos_coin_burn_cap)
-  [Function `initialize_fee_collection_and_distribution`](#0x1_transaction_fee_initialize_fee_collection_and_distribution)
-  [Function `upgrade_burn_percentage`](#0x1_transaction_fee_upgrade_burn_percentage)
-  [Function `register_proposer_for_fee_collection`](#0x1_transaction_fee_register_proposer_for_fee_collection)
-  [Function `collect_fee`](#0x1_transaction_fee_collect_fee)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a name="0x1_transaction_fee_AptosCoinCapabilities"></a>

## Resource `AptosCoinCapabilities`

Stores burn capability to burn the gas fees.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_transaction_fee_CollectedFeesPerBlockAndBatches"></a>

## Resource `CollectedFeesPerBlockAndBatches`

Stores information about the block proposer and the amount of fees
collected when executing the block.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>block_proposer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>batch_proposers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amounts: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>block_distribution_percentage: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>batch_distribution_percentage: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_transaction_fee_CollectedFeesPerBlock"></a>

## Resource `CollectedFeesPerBlock`



<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_percentage: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_transaction_fee_EALREADY_COLLECTING_FEES"></a>

Transaction fees are already being collected and the struct holding
information about collected amounts is already published.


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_EALREADY_COLLECTING_FEES">EALREADY_COLLECTING_FEES</a>: u64 = 1;
</code></pre>



<a name="0x1_transaction_fee_EINVALID_PERCENTAGE"></a>

Percentage is out of range [0, 100].


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_EINVALID_PERCENTAGE">EINVALID_PERCENTAGE</a>: u64 = 3;
</code></pre>



<a name="0x1_transaction_fee_ETOO_MANY_BATCH_PROPOSERS"></a>

Trying to register more batch proposers than the number of aggregatable
coins in the system.


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_ETOO_MANY_BATCH_PROPOSERS">ETOO_MANY_BATCH_PROPOSERS</a>: u64 = 2;
</code></pre>



<a name="0x1_transaction_fee_NUM_BATCH_PROPOSERS"></a>

Length of <code>amounts</code> vector.


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_NUM_BATCH_PROPOSERS">NUM_BATCH_PROPOSERS</a>: u64 = 300;
</code></pre>



<a name="0x1_transaction_fee_initialize_fee_collection_and_distributions"></a>

## Function `initialize_fee_collection_and_distributions`

Initializes the resource storing information about gas fees collection and
distribution. Should be called by on-chain governance.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distributions">initialize_fee_collection_and_distributions</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, block_distribution_percentage: u8, batch_distribution_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distributions">initialize_fee_collection_and_distributions</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, block_distribution_percentage: u8, batch_distribution_percentage: u8) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="transaction_fee.md#0x1_transaction_fee_EALREADY_COLLECTING_FEES">EALREADY_COLLECTING_FEES</a>)
    );
    <b>assert</b>!(block_distribution_percentage + batch_distribution_percentage &lt;= 100, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_fee.md#0x1_transaction_fee_EINVALID_PERCENTAGE">EINVALID_PERCENTAGE</a>));

    // Make sure stakng <b>module</b> is aware of transaction fees collection.
    <a href="stake.md#0x1_stake_initialize_validator_fees">stake::initialize_validator_fees</a>(aptos_framework);

    // All aggregators are pre-initialized in order <b>to</b> avoid creating/deleting more <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a> items.
    <b>let</b> i = 0;
    <b>let</b> amounts = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>while</b> (i &lt; <a href="transaction_fee.md#0x1_transaction_fee_NUM_BATCH_PROPOSERS">NUM_BATCH_PROPOSERS</a>) {
        <b>let</b> amount = <a href="coin.md#0x1_coin_initialize_aggregatable_coin">coin::initialize_aggregatable_coin</a>(aptos_framework);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> amounts, amount);
        i = i + 1;
    };

    // Initially, no fees are collected, so the <a href="block.md#0x1_block">block</a> proposer is not set.
    <b>let</b> collected_fees = <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a> {
        block_proposer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        batch_proposers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        amounts,
        block_distribution_percentage,
        batch_distribution_percentage,
    };
    <b>move_to</b>(aptos_framework, collected_fees);
}
</code></pre>



</details>

<a name="0x1_transaction_fee_is_fees_collection_enabled"></a>

## Function `is_fees_collection_enabled`



<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>(): bool {
    <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework)
}
</code></pre>



</details>

<a name="0x1_transaction_fee_upgrade_distribution_percentages"></a>

## Function `upgrade_distribution_percentages`

Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_distribution_percentages">upgrade_distribution_percentages</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_block_distribution_percentage: u8, new_batch_distribution_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_distribution_percentages">upgrade_distribution_percentages</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_block_distribution_percentage: u8,
    new_batch_distribution_percentage: u8,
) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>, <a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(new_block_distribution_percentage + new_batch_distribution_percentage &lt;= 100, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_fee.md#0x1_transaction_fee_EINVALID_PERCENTAGE">EINVALID_PERCENTAGE</a>));

    // Upgrade <b>has</b> no effect unless fees are being collected.
    <b>if</b> (<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        // We must process all the fees before upgrading the distribution
        // percentages. Otherwise new percentages will be used <b>to</b> distribute
        // fees for this <a href="block.md#0x1_block">block</a>.
        <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>();

        <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework);
        config.block_distribution_percentage = new_block_distribution_percentage;
        config.batch_distribution_percentage = new_batch_distribution_percentage;
    }
}
</code></pre>



</details>

<a name="0x1_transaction_fee_register_proposers_for_fee_collection"></a>

## Function `register_proposers_for_fee_collection`

Registers new block and batch proposers to collect transaction fees.
This function should only be called at the beginning of the block.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposers_for_fee_collection">register_proposers_for_fee_collection</a>(block_proposer_addr: <b>address</b>, batch_proposers_addr: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposers_for_fee_collection">register_proposers_for_fee_collection</a>(
    block_proposer_addr: <b>address</b>,
    batch_proposers_addr: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a> {
    <b>if</b> (<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&batch_proposers_addr) &lt;= <a href="transaction_fee.md#0x1_transaction_fee_NUM_BATCH_PROPOSERS">NUM_BATCH_PROPOSERS</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_fee.md#0x1_transaction_fee_ETOO_MANY_BATCH_PROPOSERS">ETOO_MANY_BATCH_PROPOSERS</a>));

        <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&<b>mut</b> config.block_proposer, block_proposer_addr);
        <b>let</b> batch_proposers = &<b>mut</b> config.batch_proposers;
        *batch_proposers = batch_proposers_addr;
    }
}
</code></pre>



</details>

<a name="0x1_transaction_fee_process_collected_fees"></a>

## Function `process_collected_fees`

Calculates the fee which should be distributed to block/batch proposers at the
end of an epoch, and records it in the system. This function should only be called
at the beginning of the block or during reconfiguration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>() <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>, <a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a> {
    <b>if</b> (!<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        <b>return</b>
    };
    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework);

    // All collected fees are burnt <b>if</b> the <a href="block.md#0x1_block">block</a> proposer is not set or when
    // the <a href="block.md#0x1_block">block</a> is proposed by the VM.
    <b>let</b> burn_all = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&config.block_proposer) || (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&config.block_proposer) && *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&config.block_proposer) == @vm_reserved);

    <b>let</b> i = 0;
    <b>let</b> amount_for_block_proposer = 0;
    <b>let</b> undistributed_coin = <a href="coin.md#0x1_coin_zero">coin::zero</a>&lt;AptosCoin&gt;();
    <b>let</b> num_batch_proposers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&config.batch_proposers);

    // If the <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> of batch proposers is empty, we still have <b>to</b> process fees for
    <b>if</b> (num_batch_proposers == 0) {
        // TODO: refactor!
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(undistributed_coin);
        <b>let</b> aggregatable_coin = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> config.amounts, 0);
        <b>if</b> (<a href="coin.md#0x1_coin_is_aggregatable_coin_zero">coin::is_aggregatable_coin_zero</a>(aggregatable_coin)) {
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&config.block_proposer)) {
                <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> config.block_proposer);
            };
            <b>return</b>
        };
        <b>let</b> <a href="coin.md#0x1_coin">coin</a> = <a href="coin.md#0x1_coin_drain_aggregatable_coin">coin::drain_aggregatable_coin</a>(aggregatable_coin);

        <b>if</b> (burn_all) {
            <a href="coin.md#0x1_coin_burn">coin::burn</a>(
                <a href="coin.md#0x1_coin">coin</a>,
                &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).burn_cap,
            );
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&config.block_proposer)) {
                <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> config.block_proposer);
            };
            <b>return</b>
        };

        <b>let</b> block_proposer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> config.block_proposer);
        amount_for_block_proposer = (config.block_distribution_percentage <b>as</b> u64) * <a href="coin.md#0x1_coin_value">coin::value</a>(&<a href="coin.md#0x1_coin">coin</a>) / 100;
        <b>if</b> (amount_for_block_proposer &gt; 0) {
            <a href="stake.md#0x1_stake_add_transaction_fee">stake::add_transaction_fee</a>(block_proposer_addr, <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, amount_for_block_proposer));
        };

        <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&<a href="coin.md#0x1_coin">coin</a>) == 0) {
            <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(<a href="coin.md#0x1_coin">coin</a>);
        } <b>else</b> {
            <a href="coin.md#0x1_coin_burn">coin::burn</a>(
                <a href="coin.md#0x1_coin">coin</a>,
                &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).burn_cap,
            );
        };
        <b>return</b>
    };

    <b>while</b> (i &lt; num_batch_proposers) {
        // First, get the collected amount and check <b>if</b> we can avoid calculations.
        <b>let</b> aggregatable_coin = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> config.amounts, i);
        <b>if</b> (<a href="coin.md#0x1_coin_is_aggregatable_coin_zero">coin::is_aggregatable_coin_zero</a>(aggregatable_coin)) {
            i = i + 1;
            <b>continue</b>
        };
        <b>let</b> <a href="coin.md#0x1_coin">coin</a> = <a href="coin.md#0x1_coin_drain_aggregatable_coin">coin::drain_aggregatable_coin</a>(aggregatable_coin);

        <b>if</b> (burn_all) {
            <a href="coin.md#0x1_coin_burn">coin::burn</a>(
                <a href="coin.md#0x1_coin">coin</a>,
                &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).burn_cap,
            );
            i = i + 1;
            <b>continue</b>
        };

        // Otherwise, some portion of fees <b>has</b> <b>to</b> go <b>to</b> the batch proposer
        // and the remaining amount is accumulated for later <b>use</b>.
        <b>let</b> batch_proposer_addr = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&config.batch_proposers, i);
        <b>let</b> amount_for_batch_proposer = (config.batch_distribution_percentage <b>as</b> u64) * <a href="coin.md#0x1_coin_value">coin::value</a>(&<a href="coin.md#0x1_coin">coin</a>) / 100;
        amount_for_block_proposer = amount_for_block_proposer + (config.block_distribution_percentage <b>as</b> u64) * <a href="coin.md#0x1_coin_value">coin::value</a>(&<a href="coin.md#0x1_coin">coin</a>) / 100;
        <b>if</b> (amount_for_batch_proposer &gt; 0) {
            <a href="stake.md#0x1_stake_add_transaction_fee">stake::add_transaction_fee</a>(batch_proposer_addr, <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, amount_for_batch_proposer));
        };
        <a href="coin.md#0x1_coin_merge">coin::merge</a>(&<b>mut</b> undistributed_coin, <a href="coin.md#0x1_coin">coin</a>);
        i = i + 1;
    };

    <b>if</b> (burn_all || <a href="coin.md#0x1_coin_value">coin::value</a>(&undistributed_coin) == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(undistributed_coin);
        // Also unset the proposer. See the rationale for setting proposer
        // <b>to</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() below.
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&config.block_proposer)) {
            <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> config.block_proposer);
        };
        <b>return</b>
    };

    // Extract the <b>address</b> of proposer here and reset it <b>to</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(). This
    // is particularly useful <b>to</b> avoid <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> undesired side-effects <b>where</b> coins are
    // collected but never distributed or distributed <b>to</b> the wrong <a href="account.md#0x1_account">account</a>.
    // With this design, processing collected fees enforces that all fees will be burnt
    // unless the <a href="block.md#0x1_block">block</a> proposer is specified in the <a href="block.md#0x1_block">block</a> prologue. When we have a governance
    // proposal that triggers <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a>, we distribute pending fees and burn the
    // fee for the proposal. Otherwise, that fee would be leaked <b>to</b> the next <a href="block.md#0x1_block">block</a>.
    <b>let</b> block_proposer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> config.block_proposer);
    <b>if</b> (amount_for_block_proposer &gt; 0) {
        <a href="stake.md#0x1_stake_add_transaction_fee">stake::add_transaction_fee</a>(block_proposer_addr, <a href="coin.md#0x1_coin_extract">coin::extract</a>(&<b>mut</b> undistributed_coin, amount_for_block_proposer));
    };

    <b>if</b> (<a href="coin.md#0x1_coin_value">coin::value</a>(&undistributed_coin) == 0) {
        <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(undistributed_coin);
    } <b>else</b> {
        <a href="coin.md#0x1_coin_burn">coin::burn</a>(
            undistributed_coin,
            &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).burn_cap,
        );
    };
}
</code></pre>



</details>

<a name="0x1_transaction_fee_burn_fee"></a>

## Function `burn_fee`

Burn transaction fees in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">burn_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">burn_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a> {
    <a href="coin.md#0x1_coin_burn_from">coin::burn_from</a>&lt;AptosCoin&gt;(
        <a href="account.md#0x1_account">account</a>,
        fee,
        &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a>&gt;(@aptos_framework).burn_cap,
    );
}
</code></pre>



</details>

<a name="0x1_transaction_fee_collect_fee_for_batch"></a>

## Function `collect_fee_for_batch`

Collect transaction fees in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee_for_batch">collect_fee_for_batch</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64, batch_index: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee_for_batch">collect_fee_for_batch</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64, batch_index: u16) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a> {
    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlockAndBatches">CollectedFeesPerBlockAndBatches</a>&gt;(@aptos_framework);

    // Here, we are always optimistic and always collect fees. If the proposer is not set,
    // or we cannot redistribute fees later for some reason (e.g. <a href="account.md#0x1_account">account</a> cannot receive AptoCoin)
    // we burn them all at once. This way we avoid having a check for every transaction epilogue.
    <b>let</b> aggregatable_coin = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> config.amounts, (batch_index <b>as</b> u64));
    <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">coin::collect_into_aggregatable_coin</a>&lt;AptosCoin&gt;(<a href="account.md#0x1_account">account</a>, fee, aggregatable_coin);
}
</code></pre>



</details>

<a name="0x1_transaction_fee_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_aptos_coin_burn_cap">store_aptos_coin_burn_cap</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: BurnCapability&lt;AptosCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="transaction_fee.md#0x1_transaction_fee_AptosCoinCapabilities">AptosCoinCapabilities</a> { burn_cap })
}
</code></pre>



</details>

<a name="0x1_transaction_fee_initialize_fee_collection_and_distribution"></a>

## Function `initialize_fee_collection_and_distribution`



<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distribution">initialize_fee_collection_and_distribution</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distribution">initialize_fee_collection_and_distribution</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _burn_percentage: u8) {
}
</code></pre>



</details>

<a name="0x1_transaction_fee_upgrade_burn_percentage"></a>

## Function `upgrade_burn_percentage`



<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_burn_percentage">upgrade_burn_percentage</a>(_aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _new_burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_burn_percentage">upgrade_burn_percentage</a>(
    _aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _new_burn_percentage: u8
) {
}
</code></pre>



</details>

<a name="0x1_transaction_fee_register_proposer_for_fee_collection"></a>

## Function `register_proposer_for_fee_collection`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">register_proposer_for_fee_collection</a>(_proposer_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">register_proposer_for_fee_collection</a>(_proposer_addr: <b>address</b>) {
}
</code></pre>



</details>

<a name="0x1_transaction_fee_collect_fee"></a>

## Function `collect_fee`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">collect_fee</a>(_account: <b>address</b>, _fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">collect_fee</a>(_account: <b>address</b>, _fee: u64) {
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
