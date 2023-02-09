
<a name="0x1_storage_deposit"></a>

# Module `0x1::storage_deposit`



-  [Resource `GlobalStorageDeposit`](#0x1_storage_deposit_GlobalStorageDeposit)
-  [Struct `SlotDepositEvent`](#0x1_storage_deposit_SlotDepositEvent)
-  [Struct `ExcessBytesPenaltyEvent`](#0x1_storage_deposit_ExcessBytesPenaltyEvent)
-  [Struct `SlotRefundEvent`](#0x1_storage_deposit_SlotRefundEvent)
-  [Struct `DepositEntry`](#0x1_storage_deposit_DepositEntry)
-  [Struct `ChargeSchedule`](#0x1_storage_deposit_ChargeSchedule)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_storage_deposit_initialize)
-  [Function `charge_and_refund`](#0x1_storage_deposit_charge_and_refund)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a name="0x1_storage_deposit_GlobalStorageDeposit"></a>

## Resource `GlobalStorageDeposit`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>deposit: <a href="coin.md#0x1_coin_AggregatableCoin">coin::AggregatableCoin</a>&lt;<a href="aptos_coin.md#0x1_aptos_coin_AptosCoin">aptos_coin::AptosCoin</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>slot_deposit_event: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_SlotDepositEvent">storage_deposit::SlotDepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>excess_bytes_penalty_event: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_ExcessBytesPenaltyEvent">storage_deposit::ExcessBytesPenaltyEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>slot_refund_event: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_SlotRefundEvent">storage_deposit::SlotRefundEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_deposit_SlotDepositEvent"></a>

## Struct `SlotDepositEvent`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_SlotDepositEvent">SlotDepositEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_deposit_ExcessBytesPenaltyEvent"></a>

## Struct `ExcessBytesPenaltyEvent`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_ExcessBytesPenaltyEvent">ExcessBytesPenaltyEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payer: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_deposit_SlotRefundEvent"></a>

## Struct `SlotRefundEvent`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_SlotRefundEvent">SlotRefundEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payee: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_deposit_DepositEntry"></a>

## Struct `DepositEntry`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_DepositEntry">DepositEntry</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_storage_deposit_ChargeSchedule"></a>

## Struct `ChargeSchedule`



<pre><code><b>struct</b> <a href="storage_deposit.md#0x1_storage_deposit_ChargeSchedule">ChargeSchedule</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>slot_charges: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_DepositEntry">storage_deposit::DepositEntry</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>slot_refunds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_DepositEntry">storage_deposit::DepositEntry</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>excess_bytes_penalties: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="storage_deposit.md#0x1_storage_deposit_DepositEntry">storage_deposit::DepositEntry</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_storage_deposit_MAX_U128"></a>

Maximum possible coin supply.


<pre><code><b>const</b> <a href="storage_deposit.md#0x1_storage_deposit_MAX_U128">MAX_U128</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a name="0x1_storage_deposit_EGLOBAL_STORAGE_DEPOSIT"></a>



<pre><code><b>const</b> <a href="storage_deposit.md#0x1_storage_deposit_EGLOBAL_STORAGE_DEPOSIT">EGLOBAL_STORAGE_DEPOSIT</a>: u64 = 0;
</code></pre>



<a name="0x1_storage_deposit_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> entry <b>fun</b> <a href="storage_deposit.md#0x1_storage_deposit_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="storage_deposit.md#0x1_storage_deposit_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="storage_deposit.md#0x1_storage_deposit_EGLOBAL_STORAGE_DEPOSIT">EGLOBAL_STORAGE_DEPOSIT</a>)
    );

    <b>let</b> global_storage_deposit = <a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a> {
        // FIXME(aldenhu): needs the limit <b>to</b> be u128 (u64 implied)
        deposit: <a href="coin.md#0x1_coin_initialize_aggregatable_coin">coin::initialize_aggregatable_coin</a>&lt;AptosCoin&gt;(aptos_framework),
        slot_deposit_event: new_event_handle&lt;<a href="storage_deposit.md#0x1_storage_deposit_SlotDepositEvent">SlotDepositEvent</a>&gt;(aptos_framework),
        excess_bytes_penalty_event: new_event_handle&lt;<a href="storage_deposit.md#0x1_storage_deposit_ExcessBytesPenaltyEvent">ExcessBytesPenaltyEvent</a>&gt;(aptos_framework),
        slot_refund_event: new_event_handle&lt;<a href="storage_deposit.md#0x1_storage_deposit_SlotRefundEvent">SlotRefundEvent</a>&gt;(aptos_framework),
    };

    <b>move_to</b>&lt;<a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a>&gt;(aptos_framework, global_storage_deposit);
}
</code></pre>



</details>

<a name="0x1_storage_deposit_charge_and_refund"></a>

## Function `charge_and_refund`



<pre><code><b>public</b> <b>fun</b> <a href="storage_deposit.md#0x1_storage_deposit_charge_and_refund">charge_and_refund</a>(schedule: <a href="storage_deposit.md#0x1_storage_deposit_ChargeSchedule">storage_deposit::ChargeSchedule</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="storage_deposit.md#0x1_storage_deposit_charge_and_refund">charge_and_refund</a>(schedule: <a href="storage_deposit.md#0x1_storage_deposit_ChargeSchedule">ChargeSchedule</a>) <b>acquires</b> <a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a>&gt;(@aptos_framework),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="storage_deposit.md#0x1_storage_deposit_EGLOBAL_STORAGE_DEPOSIT">EGLOBAL_STORAGE_DEPOSIT</a>)
    );
    <b>let</b> global_storage_deposit = <b>borrow_global_mut</b>&lt;<a href="storage_deposit.md#0x1_storage_deposit_GlobalStorageDeposit">GlobalStorageDeposit</a>&gt;(@aptos_framework);

    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&schedule.slot_charges);
    <b>while</b> (i &lt;= len) {
        <b>let</b> entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&schedule.slot_charges, i);
        <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">coin::collect_into_aggregatable_coin</a>&lt;AptosCoin&gt;(entry.<a href="account.md#0x1_account">account</a>, entry.amount, &<b>mut</b> global_storage_deposit.deposit);
        // FIXME(aldenhu): central events kills concurrency, probably need <b>to</b> augment Account <b>with</b> these events
        // TODO: emit <a href="event.md#0x1_event">event</a>
        i = i + 1;
    };

    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&schedule.slot_refunds);
    <b>while</b> (i &lt;= len) {
        <b>let</b> entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&schedule.slot_charges, i);
        <b>let</b> <a href="coin.md#0x1_coin">coin</a> = <a href="coin.md#0x1_coin_extract_from_aggregatable_coin">coin::extract_from_aggregatable_coin</a>(&<b>mut</b> global_storage_deposit.deposit, entry.amount);
        <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(entry.<a href="account.md#0x1_account">account</a>, <a href="coin.md#0x1_coin">coin</a>);
        // FIXME(aldenhu): central events kills concurrency, probably need <b>to</b> augment Account <b>with</b> these events
        // TODO: emit <a href="event.md#0x1_event">event</a>
        i = i + 1;
    };

    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&schedule.excess_bytes_penalties);
    <b>while</b> (i &lt;= len) {
        <b>let</b> entry = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&schedule.slot_charges, i);
        <a href="coin.md#0x1_coin_collect_into_aggregatable_coin">coin::collect_into_aggregatable_coin</a>&lt;AptosCoin&gt;(entry.<a href="account.md#0x1_account">account</a>, entry.amount, &<b>mut</b> global_storage_deposit.deposit);
        // FIXME(aldenhu): central events kills concurrency, probably need <b>to</b> augment Account <b>with</b> these events
        // TODO: emit <a href="event.md#0x1_event">event</a>
        i = i + 1;
    };
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
