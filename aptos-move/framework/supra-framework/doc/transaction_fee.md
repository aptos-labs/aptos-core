
<a id="0x1_transaction_fee"></a>

# Module `0x1::transaction_fee`

This module provides an interface to burn or collect and redistribute transaction fees.


-  [Resource `SupraCoinCapabilities`](#0x1_transaction_fee_SupraCoinCapabilities)
-  [Resource `SupraFABurnCapabilities`](#0x1_transaction_fee_SupraFABurnCapabilities)
-  [Resource `SupraCoinMintCapability`](#0x1_transaction_fee_SupraCoinMintCapability)
-  [Resource `CollectedFeesPerBlock`](#0x1_transaction_fee_CollectedFeesPerBlock)
-  [Struct `FeeStatement`](#0x1_transaction_fee_FeeStatement)
-  [Constants](#@Constants_0)
-  [Function `initialize_fee_collection_and_distribution`](#0x1_transaction_fee_initialize_fee_collection_and_distribution)
-  [Function `is_fees_collection_enabled`](#0x1_transaction_fee_is_fees_collection_enabled)
-  [Function `upgrade_burn_percentage`](#0x1_transaction_fee_upgrade_burn_percentage)
-  [Function `register_proposer_for_fee_collection`](#0x1_transaction_fee_register_proposer_for_fee_collection)
-  [Function `burn_coin_fraction`](#0x1_transaction_fee_burn_coin_fraction)
-  [Function `process_collected_fees`](#0x1_transaction_fee_process_collected_fees)
-  [Function `burn_fee`](#0x1_transaction_fee_burn_fee)
-  [Function `mint_and_refund`](#0x1_transaction_fee_mint_and_refund)
-  [Function `collect_fee`](#0x1_transaction_fee_collect_fee)
-  [Function `store_supra_coin_burn_cap`](#0x1_transaction_fee_store_supra_coin_burn_cap)
-  [Function `convert_to_aptos_fa_burn_ref`](#0x1_transaction_fee_convert_to_aptos_fa_burn_ref)
-  [Function `store_supra_coin_mint_cap`](#0x1_transaction_fee_store_supra_coin_mint_cap)
-  [Function `initialize_storage_refund`](#0x1_transaction_fee_initialize_storage_refund)
-  [Function `emit_fee_statement`](#0x1_transaction_fee_emit_fee_statement)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Resource `CollectedFeesPerBlock`](#@Specification_1_CollectedFeesPerBlock)
    -  [Function `initialize_fee_collection_and_distribution`](#@Specification_1_initialize_fee_collection_and_distribution)
    -  [Function `upgrade_burn_percentage`](#@Specification_1_upgrade_burn_percentage)
    -  [Function `register_proposer_for_fee_collection`](#@Specification_1_register_proposer_for_fee_collection)
    -  [Function `burn_coin_fraction`](#@Specification_1_burn_coin_fraction)
    -  [Function `process_collected_fees`](#@Specification_1_process_collected_fees)
    -  [Function `burn_fee`](#@Specification_1_burn_fee)
    -  [Function `mint_and_refund`](#@Specification_1_mint_and_refund)
    -  [Function `collect_fee`](#@Specification_1_collect_fee)
    -  [Function `store_supra_coin_burn_cap`](#@Specification_1_store_supra_coin_burn_cap)
    -  [Function `store_supra_coin_mint_cap`](#@Specification_1_store_supra_coin_mint_cap)
    -  [Function `initialize_storage_refund`](#@Specification_1_initialize_storage_refund)
    -  [Function `emit_fee_statement`](#@Specification_1_emit_fee_statement)


<pre><code><b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="supra_account.md#0x1_supra_account">0x1::supra_account</a>;
<b>use</b> <a href="supra_coin.md#0x1_supra_coin">0x1::supra_coin</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_transaction_fee_SupraCoinCapabilities"></a>

## Resource `SupraCoinCapabilities`

Stores burn capability to burn the gas fees.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_SupraFABurnCapabilities"></a>

## Resource `SupraFABurnCapabilities`

Stores burn capability to burn the gas fees.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_ref: <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_SupraCoinMintCapability"></a>

## Resource `SupraCoinMintCapability`

Stores mint capability to mint the refunds.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_CollectedFeesPerBlock"></a>

## Resource `CollectedFeesPerBlock`

Stores information about the block proposer and the amount of fees
collected when executing the block.


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
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

<a id="0x1_transaction_fee_FeeStatement"></a>

## Struct `FeeStatement`

Breakdown of fee charge and refund for a transaction.
The structure is:

- Net charge or refund (not in the statement)
- total charge: total_charge_gas_units, matches <code>gas_used</code> in the on-chain <code>TransactionInfo</code>.
This is the sum of the sub-items below. Notice that there's potential precision loss when
the conversion between internal and external gas units and between native token and gas
units, so it's possible that the numbers don't add up exactly. -- This number is the final
charge, while the break down is merely informational.
- gas charge for execution (CPU time): <code>execution_gas_units</code>
- gas charge for IO (storage random access): <code>io_gas_units</code>
- storage fee charge (storage space): <code>storage_fee_quants</code>, to be included in
<code>total_charge_gas_unit</code>, this number is converted to gas units according to the user
specified <code>gas_unit_price</code> on the transaction.
- storage deletion refund: <code>storage_fee_refund_quants</code>, this is not included in <code>gas_used</code> or
<code>total_charge_gas_units</code>, the net charge / refund is calculated by
<code>total_charge_gas_units</code> * <code>gas_unit_price</code> - <code>storage_fee_refund_quants</code>.

This is meant to emitted as a module event.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_FeeStatement">FeeStatement</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total_charge_gas_units: u64</code>
</dt>
<dd>
 Total gas charge.
</dd>
<dt>
<code>execution_gas_units: u64</code>
</dt>
<dd>
 Execution gas charge.
</dd>
<dt>
<code>io_gas_units: u64</code>
</dt>
<dd>
 IO gas charge.
</dd>
<dt>
<code>storage_fee_quants: u64</code>
</dt>
<dd>
 Storage fee charge.
</dd>
<dt>
<code>storage_fee_refund_quants: u64</code>
</dt>
<dd>
 Storage fee refund.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_fee_EALREADY_COLLECTING_FEES"></a>

Gas fees are already being collected and the struct holding
information about collected amounts is already published.


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_EALREADY_COLLECTING_FEES">EALREADY_COLLECTING_FEES</a>: u64 = 1;
</code></pre>



<a id="0x1_transaction_fee_EFA_GAS_CHARGING_NOT_ENABLED"></a>



<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_EFA_GAS_CHARGING_NOT_ENABLED">EFA_GAS_CHARGING_NOT_ENABLED</a>: u64 = 5;
</code></pre>



<a id="0x1_transaction_fee_EINVALID_BURN_PERCENTAGE"></a>

The burn percentage is out of range [0, 100].


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_EINVALID_BURN_PERCENTAGE">EINVALID_BURN_PERCENTAGE</a>: u64 = 3;
</code></pre>



<a id="0x1_transaction_fee_ENO_LONGER_SUPPORTED"></a>

No longer supported.


<pre><code><b>const</b> <a href="transaction_fee.md#0x1_transaction_fee_ENO_LONGER_SUPPORTED">ENO_LONGER_SUPPORTED</a>: u64 = 4;
</code></pre>



<a id="0x1_transaction_fee_initialize_fee_collection_and_distribution"></a>

## Function `initialize_fee_collection_and_distribution`

Initializes the resource storing information about gas fees collection and
distribution. Should be called by on-chain governance.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distribution">initialize_fee_collection_and_distribution</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distribution">initialize_fee_collection_and_distribution</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_percentage: u8) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="transaction_fee.md#0x1_transaction_fee_EALREADY_COLLECTING_FEES">EALREADY_COLLECTING_FEES</a>)
    );
    <b>assert</b>!(burn_percentage &lt;= 100, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_fee.md#0x1_transaction_fee_EINVALID_BURN_PERCENTAGE">EINVALID_BURN_PERCENTAGE</a>));

    // Make sure stakng <b>module</b> is aware of transaction fees collection.
    <a href="stake.md#0x1_stake_initialize_validator_fees">stake::initialize_validator_fees</a>(supra_framework);

    // Initially, no fees are collected and the <a href="block.md#0x1_block">block</a> proposer is not set.
    <b>let</b> collected_fees = <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> {
        amount: <a href="coin.md#0x1_coin_initialize_aggregatable_coin">coin::initialize_aggregatable_coin</a>(supra_framework),
        proposer: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        burn_percentage,
    };
    <b>move_to</b>(supra_framework, collected_fees);
}
</code></pre>



</details>

<a id="0x1_transaction_fee_is_fees_collection_enabled"></a>

## Function `is_fees_collection_enabled`



<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>(): bool {
    <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework)
}
</code></pre>



</details>

<a id="0x1_transaction_fee_upgrade_burn_percentage"></a>

## Function `upgrade_burn_percentage`

Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_burn_percentage">upgrade_burn_percentage</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_burn_percentage">upgrade_burn_percentage</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_burn_percentage: u8
) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>, <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>assert</b>!(new_burn_percentage &lt;= 100, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_fee.md#0x1_transaction_fee_EINVALID_BURN_PERCENTAGE">EINVALID_BURN_PERCENTAGE</a>));

    // Prior <b>to</b> upgrading the burn percentage, make sure <b>to</b> process collected
    // fees. Otherwise we would <b>use</b> the new (incorrect) burn_percentage when
    // processing fees later!
    <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>();

    <b>if</b> (<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        // Upgrade <b>has</b> no effect unless fees are being collected.
        <b>let</b> burn_percentage = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).burn_percentage;
        *burn_percentage = new_burn_percentage
    }
}
</code></pre>



</details>

<a id="0x1_transaction_fee_register_proposer_for_fee_collection"></a>

## Function `register_proposer_for_fee_collection`

Registers the proposer of the block for gas fees collection. This function
can only be called at the beginning of the block.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">register_proposer_for_fee_collection</a>(proposer_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">register_proposer_for_fee_collection</a>(proposer_addr: <b>address</b>) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> {
    <b>if</b> (<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        <b>let</b> collected_fees = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);
        <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&<b>mut</b> collected_fees.proposer, proposer_addr);
    }
}
</code></pre>



</details>

<a id="0x1_transaction_fee_burn_coin_fraction"></a>

## Function `burn_coin_fraction`

Burns a specified fraction of the coin.


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(<a href="coin.md#0x1_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;, burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(<a href="coin.md#0x1_coin">coin</a>: &<b>mut</b> Coin&lt;SupraCoin&gt;, burn_percentage: u8) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> {
    <b>assert</b>!(burn_percentage &lt;= 100, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="transaction_fee.md#0x1_transaction_fee_EINVALID_BURN_PERCENTAGE">EINVALID_BURN_PERCENTAGE</a>));

    <b>let</b> collected_amount = <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="coin.md#0x1_coin">coin</a>);
    <b>spec</b> {
        // We <b>assume</b> that `burn_percentage * collected_amount` does not overflow.
        <b>assume</b> burn_percentage * collected_amount &lt;= MAX_U64;
    };
    <b>let</b> amount_to_burn = (burn_percentage <b>as</b> u64) * collected_amount / 100;
    <b>if</b> (amount_to_burn != 0) {
        <b>let</b> coin_to_burn = <a href="coin.md#0x1_coin_extract">coin::extract</a>(<a href="coin.md#0x1_coin">coin</a>, amount_to_burn);
        <a href="coin.md#0x1_coin_burn">coin::burn</a>(
            coin_to_burn,
            &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework).burn_cap,
        );
    }
}
</code></pre>



</details>

<a id="0x1_transaction_fee_process_collected_fees"></a>

## Function `process_collected_fees`

Calculates the fee which should be distributed to the block proposer at the
end of an epoch, and records it in the system. This function can only be called
at the beginning of the block or during reconfiguration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>() <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>, <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> {
    <b>if</b> (!<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>()) {
        <b>return</b>
    };
    <b>let</b> collected_fees = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);

    // If there are no collected fees, only unset the proposer. See the rationale for
    // setting proposer <b>to</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() below.
    <b>if</b> (<a href="coin.md#0x1_coin_is_aggregatable_coin_zero">coin::is_aggregatable_coin_zero</a>(&collected_fees.amount)) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&collected_fees.proposer)) {
            <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> collected_fees.proposer);
        };
        <b>return</b>
    };

    // Otherwise get the collected fee, and check <b>if</b> it can distributed later.
    <b>let</b> <a href="coin.md#0x1_coin">coin</a> = <a href="coin.md#0x1_coin_drain_aggregatable_coin">coin::drain_aggregatable_coin</a>(&<b>mut</b> collected_fees.amount);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&collected_fees.proposer)) {
        // Extract the <b>address</b> of proposer here and reset it <b>to</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(). This
        // is particularly useful <b>to</b> avoid <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> undesired side-effects <b>where</b> coins are
        // collected but never distributed or distributed <b>to</b> the wrong <a href="account.md#0x1_account">account</a>.
        // With this design, processing collected fees enforces that all fees will be burnt
        // unless the proposer is specified in the <a href="block.md#0x1_block">block</a> prologue. When we have a governance
        // proposal that triggers <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a>, we distribute pending fees and burn the
        // fee for the proposal. Otherwise, that fee would be leaked <b>to</b> the next <a href="block.md#0x1_block">block</a>.
        <b>let</b> proposer = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> collected_fees.proposer);

        // Since the <a href="block.md#0x1_block">block</a> can be produced by the VM itself, we have <b>to</b> make sure we catch
        // this case.
        <b>if</b> (proposer == @vm_reserved) {
            <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(&<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, 100);
            <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(<a href="coin.md#0x1_coin">coin</a>);
            <b>return</b>
        };

        <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(&<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, collected_fees.burn_percentage);
        <a href="stake.md#0x1_stake_add_transaction_fee">stake::add_transaction_fee</a>(proposer, <a href="coin.md#0x1_coin">coin</a>);
        <b>return</b>
    };

    // If checks did not pass, simply burn all collected coins and <b>return</b> none.
    <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(&<b>mut</b> <a href="coin.md#0x1_coin">coin</a>, 100);
    <a href="coin.md#0x1_coin_destroy_zero">coin::destroy_zero</a>(<a href="coin.md#0x1_coin">coin</a>)
}
</code></pre>



</details>

<a id="0x1_transaction_fee_burn_fee"></a>

## Function `burn_fee`

Burn transaction fees in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">burn_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">burn_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>, <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>&gt;(@supra_framework)) {
        <b>let</b> burn_ref = &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>&gt;(@supra_framework).burn_ref;
        <a href="supra_account.md#0x1_supra_account_burn_from_fungible_store">supra_account::burn_from_fungible_store</a>(burn_ref, <a href="account.md#0x1_account">account</a>, fee);
    } <b>else</b> {
        <b>let</b> burn_cap = &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework).burn_cap;
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>()) {
            <b>let</b> (burn_ref, burn_receipt) = <a href="coin.md#0x1_coin_get_paired_burn_ref">coin::get_paired_burn_ref</a>(burn_cap);
            <a href="supra_account.md#0x1_supra_account_burn_from_fungible_store">supra_account::burn_from_fungible_store</a>(&burn_ref, <a href="account.md#0x1_account">account</a>, fee);
            <a href="coin.md#0x1_coin_return_paired_burn_ref">coin::return_paired_burn_ref</a>(burn_ref, burn_receipt);
        } <b>else</b> {
            <a href="coin.md#0x1_coin_burn_from">coin::burn_from</a>&lt;SupraCoin&gt;(
                <a href="account.md#0x1_account">account</a>,
                fee,
                burn_cap,
            );
        };
    };
}
</code></pre>



</details>

<a id="0x1_transaction_fee_mint_and_refund"></a>

## Function `mint_and_refund`

Mint refund in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">mint_and_refund</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, refund: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">mint_and_refund</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, refund: u64) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a> {
    <b>let</b> mint_cap = &<b>borrow_global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a>&gt;(@supra_framework).mint_cap;
    <b>let</b> refund_coin = <a href="coin.md#0x1_coin_mint">coin::mint</a>(refund, mint_cap);
    <a href="coin.md#0x1_coin_force_deposit">coin::force_deposit</a>(<a href="account.md#0x1_account">account</a>, refund_coin);
}
</code></pre>



</details>

<a id="0x1_transaction_fee_collect_fee"></a>

## Function `collect_fee`

Collect transaction fees in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">collect_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">collect_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> {
    <b>let</b> collected_fees = <b>borrow_global_mut</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);

    // Here, we are always optimistic and always collect fees. If the proposer is not set,
    // or we cannot redistribute fees later for some reason (e.g. <a href="account.md#0x1_account">account</a> cannot receive AptoCoin)
    // we burn them all at once. This way we avoid having a check for every transaction epilogue.
    <b>let</b> collected_amount = &<b>mut</b> collected_fees.amount;
    <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">coin::collect_into_aggregatable_coin</a>&lt;SupraCoin&gt;(<a href="account.md#0x1_account">account</a>, fee, collected_amount);
}
</code></pre>



</details>

<a id="0x1_transaction_fee_store_supra_coin_burn_cap"></a>

## Function `store_supra_coin_burn_cap`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_burn_cap">store_supra_coin_burn_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_burn_cap">store_supra_coin_burn_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: BurnCapability&lt;SupraCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>()) {
        <b>let</b> burn_ref = <a href="coin.md#0x1_coin_convert_and_take_paired_burn_ref">coin::convert_and_take_paired_burn_ref</a>(burn_cap);
        <b>move_to</b>(supra_framework, <a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a> { burn_ref });
    } <b>else</b> {
        <b>move_to</b>(supra_framework, <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> { burn_cap })
    }
}
</code></pre>



</details>

<a id="0x1_transaction_fee_convert_to_aptos_fa_burn_ref"></a>

## Function `convert_to_aptos_fa_burn_ref`



<pre><code><b>public</b> entry <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_convert_to_aptos_fa_burn_ref">convert_to_aptos_fa_burn_ref</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_convert_to_aptos_fa_burn_ref">convert_to_aptos_fa_burn_ref</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_supra_store_enabled">features::operations_default_to_fa_supra_store_enabled</a>(), <a href="transaction_fee.md#0x1_transaction_fee_EFA_GAS_CHARGING_NOT_ENABLED">EFA_GAS_CHARGING_NOT_ENABLED</a>);
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>let</b> <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a> {
        burn_cap,
    } = <b>move_from</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework));
    <b>let</b> burn_ref = <a href="coin.md#0x1_coin_convert_and_take_paired_burn_ref">coin::convert_and_take_paired_burn_ref</a>(burn_cap);
    <b>move_to</b>(supra_framework, <a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a> { burn_ref });
}
</code></pre>



</details>

<a id="0x1_transaction_fee_store_supra_coin_mint_cap"></a>

## Function `store_supra_coin_mint_cap`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_mint_cap">store_supra_coin_mint_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_mint_cap">store_supra_coin_mint_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: MintCapability&lt;SupraCoin&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>move_to</b>(supra_framework, <a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a> { mint_cap })
}
</code></pre>



</details>

<a id="0x1_transaction_fee_initialize_storage_refund"></a>

## Function `initialize_storage_refund`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_storage_refund">initialize_storage_refund</a>(_: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_storage_refund">initialize_storage_refund</a>(_: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_implemented">error::not_implemented</a>(<a href="transaction_fee.md#0x1_transaction_fee_ENO_LONGER_SUPPORTED">ENO_LONGER_SUPPORTED</a>)
}
</code></pre>



</details>

<a id="0x1_transaction_fee_emit_fee_statement"></a>

## Function `emit_fee_statement`



<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_emit_fee_statement">emit_fee_statement</a>(fee_statement: <a href="transaction_fee.md#0x1_transaction_fee_FeeStatement">transaction_fee::FeeStatement</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_emit_fee_statement">emit_fee_statement</a>(fee_statement: <a href="transaction_fee.md#0x1_transaction_fee_FeeStatement">FeeStatement</a>) {
    <a href="event.md#0x1_event_emit">event::emit</a>(fee_statement)
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
<td>Given the blockchain is in an operating state, it guarantees that the Supra framework signer may burn Supra coins.</td>
<td>Critical</td>
<td>The SupraCoinCapabilities structure is defined in this module and it stores burn capability to burn the gas fees.</td>
<td>Formally Verified via <a href="#high-level-req-1">module</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The initialization function may only be called once.</td>
<td>Medium</td>
<td>The initialize_fee_collection_and_distribution function ensures CollectedFeesPerBlock does not already exist.</td>
<td>Formally verified via <a href="#high-level-req-2">initialize_fee_collection_and_distribution</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only the admin address is authorized to call the initialization function.</td>
<td>Critical</td>
<td>The initialize_fee_collection_and_distribution function ensures only the Supra framework address calls it.</td>
<td>Formally verified via <a href="#high-level-req-3">initialize_fee_collection_and_distribution</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The percentage of the burnt collected fee is always a value from 0 to 100.</td>
<td>Medium</td>
<td>During the initialization of CollectedFeesPerBlock in Initialize_fee_collection_and_distribution, and while upgrading burn percentage, it asserts that burn_percentage is within the specified limits.</td>
<td>Formally verified via <a href="#high-level-req-4">CollectedFeesPerBlock</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Prior to upgrading the burn percentage, it must process all the fees collected up to that point.</td>
<td>Critical</td>
<td>The upgrade_burn_percentage function ensures process_collected_fees function is called before updating the burn percentage.</td>
<td>Formally verified in <a href="#high-level-req-5">ProcessCollectedFeesRequiresAndEnsures</a>.</td>
</tr>

<tr>
<td>6</td>
<td>The presence of the resource, indicating collected fees per block under the Supra framework account, is a prerequisite for the successful execution of the following functionalities: Upgrading burn percentage. Registering a block proposer. Processing collected fees.</td>
<td>Low</td>
<td>The functions: upgrade_burn_percentage, register_proposer_for_fee_collection, and process_collected_fees all ensure that the CollectedFeesPerBlock resource exists under supra_framework by calling the is_fees_collection_enabled method, which returns a boolean value confirming if the resource exists or not.</td>
<td>Formally verified via <a href="#high-level-req-6.1">register_proposer_for_fee_collection</a>, <a href="#high-level-req-6.2">process_collected_fees</a>, and <a href="#high-level-req-6.3">upgrade_burn_percentage</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_strict;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework) || <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>&gt;(@supra_framework);
</code></pre>



<a id="@Specification_1_CollectedFeesPerBlock"></a>

### Resource `CollectedFeesPerBlock`


<pre><code><b>struct</b> <a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a> <b>has</b> key
</code></pre>



<dl>
<dt>
<code>amount: <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;</code>
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



<pre><code>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>invariant</b> burn_percentage &lt;= 100;
</code></pre>



<a id="@Specification_1_initialize_fee_collection_and_distribution"></a>

### Function `initialize_fee_collection_and_distribution`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_fee_collection_and_distribution">initialize_fee_collection_and_distribution</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_percentage: u8)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);
<b>aborts_if</b> burn_percentage &gt; 100;
<b>let</b> aptos_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_supra_framework_address">system_addresses::is_supra_framework_address</a>(aptos_addr);
<b>aborts_if</b> <b>exists</b>&lt;ValidatorFees&gt;(aptos_addr);
<b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotSupraFramework">system_addresses::AbortsIfNotSupraFramework</a> { <a href="account.md#0x1_account">account</a>: supra_framework };
<b>include</b> <a href="aggregator_factory.md#0x1_aggregator_factory_CreateAggregatorInternalAbortsIf">aggregator_factory::CreateAggregatorInternalAbortsIf</a>;
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(aptos_addr);
<b>ensures</b> <b>exists</b>&lt;ValidatorFees&gt;(aptos_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_upgrade_burn_percentage"></a>

### Function `upgrade_burn_percentage`


<pre><code><b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_upgrade_burn_percentage">upgrade_burn_percentage</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_burn_percentage: u8)
</code></pre>




<pre><code><b>aborts_if</b> new_burn_percentage &gt; 100;
<b>let</b> aptos_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_supra_framework_address">system_addresses::is_supra_framework_address</a>(aptos_addr);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a> and <a id="high-level-req-6.3" href="#high-level-req">high-level requirement 6</a>:
<b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures">ProcessCollectedFeesRequiresAndEnsures</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework) ==&gt;
    <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).burn_percentage == new_burn_percentage;
</code></pre>



<a id="@Specification_1_register_proposer_for_fee_collection"></a>

### Function `register_proposer_for_fee_collection`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_register_proposer_for_fee_collection">register_proposer_for_fee_collection</a>(proposer_addr: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
// This enforces <a id="high-level-req-6.1" href="#high-level-req">high-level requirement 6</a>:
<b>ensures</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>() ==&gt;
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).proposer) == proposer_addr;
</code></pre>



<a id="@Specification_1_burn_coin_fraction"></a>

### Function `burn_coin_fraction`


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_coin_fraction">burn_coin_fraction</a>(<a href="coin.md#0x1_coin">coin</a>: &<b>mut</b> <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;, burn_percentage: u8)
</code></pre>




<pre><code><b>requires</b> burn_percentage &lt;= 100;
<b>requires</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework);
<b>requires</b> <b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(@supra_framework);
<b>let</b> amount_to_burn = (burn_percentage * <a href="coin.md#0x1_coin_value">coin::value</a>(<a href="coin.md#0x1_coin">coin</a>)) / 100;
<b>include</b> amount_to_burn &gt; 0 ==&gt; <a href="coin.md#0x1_coin_CoinSubAbortsIf">coin::CoinSubAbortsIf</a>&lt;SupraCoin&gt; { amount: amount_to_burn };
<b>ensures</b> <a href="coin.md#0x1_coin">coin</a>.value == <b>old</b>(<a href="coin.md#0x1_coin">coin</a>).value - amount_to_burn;
</code></pre>




<a id="0x1_transaction_fee_collectedFeesAggregator"></a>


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collectedFeesAggregator">collectedFeesAggregator</a>(): AggregatableCoin&lt;SupraCoin&gt; {
   <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).amount
}
</code></pre>




<a id="0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply"></a>


<pre><code><b>schema</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">RequiresCollectedFeesPerValueLeqBlockAptosSupply</a> {
    <b>let</b> maybe_supply = <a href="coin.md#0x1_coin_get_coin_supply_opt">coin::get_coin_supply_opt</a>&lt;SupraCoin&gt;();
    <b>requires</b>
        (<a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>() && <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(maybe_supply)) ==&gt;
            (<a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(<b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).amount.value) &lt;=
                <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(
                    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(<a href="coin.md#0x1_coin_get_coin_supply_opt">coin::get_coin_supply_opt</a>&lt;SupraCoin&gt;())
                ));
}
</code></pre>




<a id="0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures"></a>


<pre><code><b>schema</b> <a href="transaction_fee.md#0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures">ProcessCollectedFeesRequiresAndEnsures</a> {
    <b>requires</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework);
    <b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@supra_framework);
    <b>requires</b> <b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(@supra_framework);
    <b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;
    <b>aborts_if</b> <b>false</b>;
    <b>let</b> collected_fees = <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);
    <b>let</b> <b>post</b> post_collected_fees = <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);
    <b>let</b> pre_amount = <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(collected_fees.amount.value);
    <b>let</b> <b>post</b> post_amount = <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(post_collected_fees.amount.value);
    <b>let</b> fees_table = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@supra_framework).fees_table;
    <b>let</b> <b>post</b> post_fees_table = <b>global</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@supra_framework).fees_table;
    <b>let</b> proposer = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(collected_fees.proposer);
    <b>let</b> fee_to_add = pre_amount - pre_amount * collected_fees.burn_percentage / 100;
    <b>ensures</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>() ==&gt; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_collected_fees.proposer) && post_amount == 0;
    <b>ensures</b> <a href="transaction_fee.md#0x1_transaction_fee_is_fees_collection_enabled">is_fees_collection_enabled</a>() && <a href="aggregator.md#0x1_aggregator_spec_read">aggregator::spec_read</a>(collected_fees.amount.value) &gt; 0 &&
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(collected_fees.proposer) ==&gt;
        <b>if</b> (proposer != @vm_reserved) {
            <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(fees_table, proposer)) {
                <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_fees_table, proposer).value == <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(
                    fees_table,
                    proposer
                ).value + fee_to_add
            } <b>else</b> {
                <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(post_fees_table, proposer).value == fee_to_add
            }
        } <b>else</b> {
            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_collected_fees.proposer) && post_amount == 0
        };
}
</code></pre>



<a id="@Specification_1_process_collected_fees"></a>

### Function `process_collected_fees`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">process_collected_fees</a>()
</code></pre>




<pre><code>// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
<b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures">ProcessCollectedFeesRequiresAndEnsures</a>;
</code></pre>



<a id="@Specification_1_burn_fee"></a>

### Function `burn_fee`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_burn_fee">burn_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>


<code><a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a></code> should be exists.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(@supra_framework);
<b>let</b> account_addr = <a href="account.md#0x1_account">account</a>;
<b>let</b> amount = fee;
<b>let</b> aptos_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;SupraCoin&gt;().account_address;
<b>let</b> coin_store = <b>global</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(account_addr);
<b>let</b> <b>post</b> post_coin_store = <b>global</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(account_addr);
<b>aborts_if</b> amount != 0 && !(<b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(aptos_addr)
    && <b>exists</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(account_addr));
<b>aborts_if</b> coin_store.<a href="coin.md#0x1_coin">coin</a>.value &lt; amount;
<b>let</b> maybe_supply = <b>global</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(aptos_addr).supply;
<b>let</b> supply_aggr = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(maybe_supply);
<b>let</b> value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(supply_aggr);
<b>let</b> <b>post</b> post_maybe_supply = <b>global</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(aptos_addr).supply;
<b>let</b> <b>post</b> post_supply = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(post_maybe_supply);
<b>let</b> <b>post</b> post_value = <a href="optional_aggregator.md#0x1_optional_aggregator_optional_aggregator_value">optional_aggregator::optional_aggregator_value</a>(post_supply);
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply) && value &lt; amount;
<b>ensures</b> post_coin_store.<a href="coin.md#0x1_coin">coin</a>.value == coin_store.<a href="coin.md#0x1_coin">coin</a>.value - amount;
<b>ensures</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(maybe_supply)) {
    post_value == value - amount
} <b>else</b> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_none">option::spec_is_none</a>(post_maybe_supply)
};
<b>ensures</b> <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt; == <b>old</b>(<a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;) - amount;
</code></pre>



<a id="@Specification_1_mint_and_refund"></a>

### Function `mint_and_refund`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_mint_and_refund">mint_and_refund</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, refund: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> aptos_addr = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;SupraCoin&gt;().account_address;
<b>aborts_if</b> (refund != 0) && !<b>exists</b>&lt;CoinInfo&lt;SupraCoin&gt;&gt;(aptos_addr);
<b>include</b> <a href="coin.md#0x1_coin_CoinAddAbortsIf">coin::CoinAddAbortsIf</a>&lt;SupraCoin&gt; { amount: refund };
<b>aborts_if</b> !<b>exists</b>&lt;CoinStore&lt;SupraCoin&gt;&gt;(<a href="account.md#0x1_account">account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a>&gt;(@supra_framework);
<b>let</b> supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;;
<b>let</b> <b>post</b> post_supply = <a href="coin.md#0x1_coin_supply">coin::supply</a>&lt;SupraCoin&gt;;
<b>aborts_if</b> [abstract] supply + refund &gt; MAX_U128;
<b>ensures</b> post_supply == supply + refund;
</code></pre>



<a id="@Specification_1_collect_fee"></a>

### Function `collect_fee`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_collect_fee">collect_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> collected_fees = <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).amount;
<b>let</b> aggr = collected_fees.value;
<b>let</b> coin_store = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;SupraCoin&gt;&gt;(<a href="account.md#0x1_account">account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework);
<b>aborts_if</b> fee &gt; 0 && !<b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;SupraCoin&gt;&gt;(<a href="account.md#0x1_account">account</a>);
<b>aborts_if</b> fee &gt; 0 && coin_store.<a href="coin.md#0x1_coin">coin</a>.value &lt; fee;
<b>aborts_if</b> fee &gt; 0 && <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)
    + fee &gt; <a href="aggregator.md#0x1_aggregator_spec_get_limit">aggregator::spec_get_limit</a>(aggr);
<b>aborts_if</b> fee &gt; 0 && <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(aggr)
    + fee &gt; MAX_U128;
<b>let</b> <b>post</b> post_coin_store = <b>global</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;SupraCoin&gt;&gt;(<a href="account.md#0x1_account">account</a>);
<b>let</b> <b>post</b> post_collected_fees = <b>global</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_CollectedFeesPerBlock">CollectedFeesPerBlock</a>&gt;(@supra_framework).amount;
<b>ensures</b> post_coin_store.<a href="coin.md#0x1_coin">coin</a>.value == coin_store.<a href="coin.md#0x1_coin">coin</a>.value - fee;
<b>ensures</b> <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(post_collected_fees.value) == <a href="aggregator.md#0x1_aggregator_spec_aggregator_get_val">aggregator::spec_aggregator_get_val</a>(
    aggr
) + fee;
</code></pre>



<a id="@Specification_1_store_supra_coin_burn_cap"></a>

### Function `store_supra_coin_burn_cap`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_burn_cap">store_supra_coin_burn_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, burn_cap: <a href="coin.md#0x1_coin_BurnCapability">coin::BurnCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;)
</code></pre>


Ensure caller is admin.
Aborts if <code><a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a></code> already exists.


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_supra_framework_address">system_addresses::is_supra_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>&gt;(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraFABurnCapabilities">SupraFABurnCapabilities</a>&gt;(addr) || <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinCapabilities">SupraCoinCapabilities</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_store_supra_coin_mint_cap"></a>

### Function `store_supra_coin_mint_cap`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_store_supra_coin_mint_cap">store_supra_coin_mint_cap</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, mint_cap: <a href="coin.md#0x1_coin_MintCapability">coin::MintCapability</a>&lt;<a href="supra_coin.md#0x1_supra_coin_SupraCoin">supra_coin::SupraCoin</a>&gt;)
</code></pre>


Ensure caller is admin.
Aborts if <code><a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a></code> already exists.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(supra_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_supra_framework_address">system_addresses::is_supra_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="transaction_fee.md#0x1_transaction_fee_SupraCoinMintCapability">SupraCoinMintCapability</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_initialize_storage_refund"></a>

### Function `initialize_storage_refund`


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_initialize_storage_refund">initialize_storage_refund</a>(_: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>


Historical. Aborts.


<pre><code><b>aborts_if</b> <b>true</b>;
</code></pre>



<a id="@Specification_1_emit_fee_statement"></a>

### Function `emit_fee_statement`


<pre><code><b>fun</b> <a href="transaction_fee.md#0x1_transaction_fee_emit_fee_statement">emit_fee_statement</a>(fee_statement: <a href="transaction_fee.md#0x1_transaction_fee_FeeStatement">transaction_fee::FeeStatement</a>)
</code></pre>


Aborts if module event feature is not enabled.


[move-book]: https://aptos.dev/move/book/SUMMARY
