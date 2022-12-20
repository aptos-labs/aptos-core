
<a name="0x1_fee_destribution"></a>

# Module `0x1::fee_destribution`



-  [Resource `DistributionInfo`](#0x1_fee_destribution_DistributionInfo)
-  [Constants](#@Constants_0)
-  [Function `initialize_distribution_info`](#0x1_fee_destribution_initialize_distribution_info)
-  [Function `collect_fee`](#0x1_fee_destribution_collect_fee)
-  [Function `set_receiver`](#0x1_fee_destribution_set_receiver)
-  [Function `maybe_distribute_fees`](#0x1_fee_destribution_maybe_distribute_fees)


<pre><code><b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;
</code></pre>



<a name="0x1_fee_destribution_DistributionInfo"></a>

## Resource `DistributionInfo`

Resource which holds the collected transaction fees and their receiver.


<pre><code><b>struct</b> <a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>balance: <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>receiver: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fee_destribution_EDISTRIBUTION_INFO_EXISTS"></a>

When struct holding distribution ifnormation already exists.


<pre><code><b>const</b> <a href="fee_distribution.md#0x1_fee_destribution_EDISTRIBUTION_INFO_EXISTS">EDISTRIBUTION_INFO_EXISTS</a>: u64 = 1;
</code></pre>



<a name="0x1_fee_destribution_initialize_distribution_info"></a>

## Function `initialize_distribution_info`

Initializes the resource holding information for gas fees distribution.
Should be called by on-chain governance.


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_initialize_distribution_info">initialize_distribution_info</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_initialize_distribution_info">initialize_distribution_info</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="fee_distribution.md#0x1_fee_destribution_EDISTRIBUTION_INFO_EXISTS">EDISTRIBUTION_INFO_EXISTS</a>)
    );

    <b>let</b> zero = <a href="coin.md#0x1_coin_initialize_aggregator_coin">coin::initialize_aggregator_coin</a>(aptos_framework);
    <b>let</b> info = <a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a> {
        balance: zero,
        receiver: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    };
    <b>move_to</b>(aptos_framework, info);
}
</code></pre>



</details>

<a name="0x1_fee_destribution_collect_fee"></a>

## Function `collect_fee`

Called by transaction epilogue to collect the gas fees from the specified account.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_collect_fee">collect_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_collect_fee">collect_fee</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, fee: u64) <b>acquires</b> <a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a> {
    <b>let</b> distribution_info = <b>borrow_global_mut</b>&lt;<a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a>&gt;(@aptos_framework);
    <b>let</b> dst_coin = &<b>mut</b> distribution_info.balance;
    <a href="coin.md#0x1_coin_collect_from">coin::collect_from</a>(<a href="account.md#0x1_account">account</a>, fee, dst_coin);
}
</code></pre>



</details>

<a name="0x1_fee_destribution_set_receiver"></a>

## Function `set_receiver`

Sets the receiver of the collected fees for the next block.


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_set_receiver">set_receiver</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_set_receiver">set_receiver</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver_addr: <b>address</b>) <b>acquires</b> <a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a> {
    // Can only be called by the VM.
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>let</b> distribution_info = <b>borrow_global_mut</b>&lt;<a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a>&gt;(@aptos_framework);
    <b>let</b> _ = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&<b>mut</b> distribution_info.receiver, receiver_addr);
}
</code></pre>



</details>

<a name="0x1_fee_destribution_maybe_distribute_fees"></a>

## Function `maybe_distribute_fees`

Distributes collected transaction fees to the receiver. Should be called
at the beginning of each block.


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_maybe_distribute_fees">maybe_distribute_fees</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fee_distribution.md#0x1_fee_destribution_maybe_distribute_fees">maybe_distribute_fees</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a> {
    // Can only be called by the VM.
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>let</b> distribution_info = <b>borrow_global_mut</b>&lt;<a href="fee_distribution.md#0x1_fee_destribution_DistributionInfo">DistributionInfo</a>&gt;(@aptos_framework);

    // First, do nothing <b>if</b> there are no collected fees.
    <b>if</b> (<a href="coin.md#0x1_coin_is_zero">coin::is_zero</a>(&distribution_info.balance)) {
        <b>return</b>
    };

    <b>let</b> <a href="coin.md#0x1_coin">coin</a> = <a href="coin.md#0x1_coin_drain">coin::drain</a>(&<b>mut</b> distribution_info.balance);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&distribution_info.receiver)) {
        <b>let</b> receiver_addr = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&distribution_info.receiver);

        // There is a receiver, but it might not have <a href="account.md#0x1_account">account</a> registered for storing
        // coins, so check for that.
        <b>let</b> receiver_has_account = <a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(receiver_addr);
        <b>if</b> (receiver_has_account) {
            // If all checks passed, deposit coins <b>to</b> the receiver's <a href="account.md#0x1_account">account</a>.
            <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(receiver_addr, <a href="coin.md#0x1_coin">coin</a>);
            <b>return</b>
        };
    };

    // Otherwise, burn the collected coins.
    <a href="transaction_fee.md#0x1_transaction_fee_burn_collected_fee">transaction_fee::burn_collected_fee</a>(<a href="coin.md#0x1_coin">coin</a>);
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
