
<a id="0x1_transaction_limits"></a>

# Module `0x1::transaction_limits`

Manages configuration and validation for higher transaction limits based on
staking.

Users can request multipliers to transaction limits (e..g, execution limit
or IO limit) if they prove they control a significant stake:
- as a stake pool owner,
- as a delegated voter,
- as a delegation pool delegator.
For example, one can request 2.5x on execution limits and 5x on IO limits.
Multipliers are in basis points where 1 maps to 100, to support fractions.

The on-chain config stores a vector of tiers. Each tier maps multiplier to
the required minimum stake threshold. A smallest multiplier that is greater
than or equal to the requested multiplier is chosen.


-  [Struct `TxnLimitTier`](#0x1_transaction_limits_TxnLimitTier)
-  [Enum Resource `TxnLimitsConfig`](#0x1_transaction_limits_TxnLimitsConfig)
-  [Enum `RequestedMultipliers`](#0x1_transaction_limits_RequestedMultipliers)
-  [Enum `UserTxnLimitsRequest`](#0x1_transaction_limits_UserTxnLimitsRequest)
-  [Constants](#@Constants_0)
-  [Function `new_tier`](#0x1_transaction_limits_new_tier)
-  [Function `validate_tiers`](#0x1_transaction_limits_validate_tiers)
-  [Function `new_tiers`](#0x1_transaction_limits_new_tiers)
-  [Function `find_min_stake_required`](#0x1_transaction_limits_find_min_stake_required)
-  [Function `initialize`](#0x1_transaction_limits_initialize)
-  [Function `update_config`](#0x1_transaction_limits_update_config)
-  [Function `validate_enough_stake`](#0x1_transaction_limits_validate_enough_stake)
-  [Function `validate_high_txn_limits`](#0x1_transaction_limits_validate_high_txn_limits)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="delegation_pool.md#0x1_delegation_pool">0x1::delegation_pool</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="staking_config.md#0x1_staking_config">0x1::staking_config</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_transaction_limits_TxnLimitTier"></a>

## Struct `TxnLimitTier`

A single tier: the minimum committed stake required and the multiplier
it unlocks.


<pre><code><b>struct</b> <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_stake: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>multiplier_bps: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_limits_TxnLimitsConfig"></a>

## Enum Resource `TxnLimitsConfig`

On-chain configuration for higher transaction limits. Stores a vector
of tiers for each dimension (e.g., execution, IO). Tiers are ordered
monotonically by both minimum stakes and multipliers.


<pre><code>enum <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>execution_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>io_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_transaction_limits_RequestedMultipliers"></a>

## Enum `RequestedMultipliers`

Multipliers requested by the user, expressed in basis points (That is,
1x is 100, 2.5x is 250).

INVARIANT: must match Rust enum for BCS serialization.


<pre><code>enum <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">RequestedMultipliers</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>execution_bps: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>io_bps: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_transaction_limits_UserTxnLimitsRequest"></a>

## Enum `UserTxnLimitsRequest`

Request for higher transaction limits, passed to the prologue. Carries
the proof that the sender has enough stake.

INVARIANT: must match Rust enum for BCS serialization.


<pre><code>enum <a href="transaction_limits.md#0x1_transaction_limits_UserTxnLimitsRequest">UserTxnLimitsRequest</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>StakePoolOwner</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>multipliers: <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">transaction_limits::RequestedMultipliers</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>DelegatedVoter</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>multipliers: <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">transaction_limits::RequestedMultipliers</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>DelegationPoolDelegator</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pool_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>multipliers: <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">transaction_limits::RequestedMultipliers</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_limits_ENOT_DELEGATED_VOTER"></a>

Fee payer is not the delegated voter of the specified stake pool.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>: u64 = 3;
</code></pre>



<a id="0x1_transaction_limits_EDELEGATION_POOL_NOT_FOUND"></a>

No delegation pool exists at the specified address.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_EDELEGATION_POOL_NOT_FOUND">EDELEGATION_POOL_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x1_transaction_limits_EINSUFFICIENT_STAKE"></a>

Committed stake is insufficient for the requested multiplier tier.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_EINSUFFICIENT_STAKE">EINSUFFICIENT_STAKE</a>: u64 = 5;
</code></pre>



<a id="0x1_transaction_limits_EINVALID_MULTIPLIER"></a>

Multiplier must be > 100 bps (> 1x).


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_EINVALID_MULTIPLIER">EINVALID_MULTIPLIER</a>: u64 = 7;
</code></pre>



<a id="0x1_transaction_limits_EMULTIPLIER_NOT_AVAILABLE"></a>

Requested multiplier is not available in any configured tier.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_EMULTIPLIER_NOT_AVAILABLE">EMULTIPLIER_NOT_AVAILABLE</a>: u64 = 8;
</code></pre>



<a id="0x1_transaction_limits_ENOT_STAKE_POOL_OWNER"></a>

Fee payer is not the owner of the specified stake pool.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_ENOT_STAKE_POOL_OWNER">ENOT_STAKE_POOL_OWNER</a>: u64 = 2;
</code></pre>



<a id="0x1_transaction_limits_ESTAKE_POOL_NOT_FOUND"></a>

No stake pool exists at the specified address.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_ESTAKE_POOL_NOT_FOUND">ESTAKE_POOL_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_transaction_limits_ETHRESHOLDS_NOT_MONOTONIC"></a>

Config tiers are not monotonically ordered.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_ETHRESHOLDS_NOT_MONOTONIC">ETHRESHOLDS_NOT_MONOTONIC</a>: u64 = 6;
</code></pre>



<a id="0x1_transaction_limits_EVECTOR_LENGTH_MISMATCH"></a>

Min-stakes and multipliers vectors have different lengths.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_EVECTOR_LENGTH_MISMATCH">EVECTOR_LENGTH_MISMATCH</a>: u64 = 9;
</code></pre>



<a id="0x1_transaction_limits_MAX_MULTIPLIER_BPS"></a>

Every multiplier must be less than or equal to this maximum (100x).

INVARIANT: must match Rust version checked by VM.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_MAX_MULTIPLIER_BPS">MAX_MULTIPLIER_BPS</a>: u64 = 10000;
</code></pre>



<a id="0x1_transaction_limits_MIN_MULTIPLIER_BPS"></a>

Every multiplier must be greater than this minimum (1x).

INVARIANT: must match Rust version checked by VM.


<pre><code><b>const</b> <a href="transaction_limits.md#0x1_transaction_limits_MIN_MULTIPLIER_BPS">MIN_MULTIPLIER_BPS</a>: u64 = 100;
</code></pre>



<a id="0x1_transaction_limits_new_tier"></a>

## Function `new_tier`

Creates a new tier. Aborts if multiplier is not in (100, 10000] bps.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_new_tier">new_tier</a>(min_stake: u64, multiplier_bps: u64): <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_new_tier">new_tier</a>(min_stake: u64, multiplier_bps: u64): <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a> {
    <b>assert</b>!(
        multiplier_bps &gt; <a href="transaction_limits.md#0x1_transaction_limits_MIN_MULTIPLIER_BPS">MIN_MULTIPLIER_BPS</a> && multiplier_bps &lt;= <a href="transaction_limits.md#0x1_transaction_limits_MAX_MULTIPLIER_BPS">MAX_MULTIPLIER_BPS</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_EINVALID_MULTIPLIER">EINVALID_MULTIPLIER</a>)
    );
    <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a> { min_stake, multiplier_bps }
}
</code></pre>



</details>

<a id="0x1_transaction_limits_validate_tiers"></a>

## Function `validate_tiers`

Aborts if:
- Minimum stake tiers are not monotonically increasing.
- Multiplier tiers are not strictly monotonically increasing.


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_tiers">validate_tiers</a>(tiers: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_tiers">validate_tiers</a>(tiers: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a>&gt;) {
    <b>let</b> i = 1;
    <b>let</b> len = tiers.length();

    <b>while</b> (i &lt; len) {
        <b>let</b> prev = &tiers[i - 1];
        <b>let</b> curr = &tiers[i];
        <b>assert</b>!(
            curr.min_stake &gt;= prev.min_stake
                && curr.multiplier_bps &gt; prev.multiplier_bps,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_ETHRESHOLDS_NOT_MONOTONIC">ETHRESHOLDS_NOT_MONOTONIC</a>)
        );
        i += 1;
    };
}
</code></pre>



</details>

<a id="0x1_transaction_limits_new_tiers"></a>

## Function `new_tiers`

Builds a vector of tiers from inputs.

Aborts if:
- Minimum stakes and multipliers vectors have different lengths.
- Minimum stakes and multipliers vectors are not monotonically
increasing.
- Multiplier is not valid (1x or below).


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_new_tiers">new_tiers</a>(min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_new_tiers">new_tiers</a>(min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
    : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a>&gt; {
    <b>let</b> len = min_stakes.length();
    <b>assert</b>!(
        len == multipliers_bps.length(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_EVECTOR_LENGTH_MISMATCH">EVECTOR_LENGTH_MISMATCH</a>)
    );

    <b>let</b> tiers = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        tiers.push_back(<a href="transaction_limits.md#0x1_transaction_limits_new_tier">new_tier</a>(min_stakes[i], multipliers_bps[i]));
        i += 1;
    };
    <a href="transaction_limits.md#0x1_transaction_limits_validate_tiers">validate_tiers</a>(&tiers);

    tiers
}
</code></pre>



</details>

<a id="0x1_transaction_limits_find_min_stake_required"></a>

## Function `find_min_stake_required`

Finds the smallest tier whose multiplier is greater than or equal to
the requested multiplier. Returns minimum stake correspondng to this
tier.

Aborts if no tier can cover the request.


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_find_min_stake_required">find_min_stake_required</a>(tiers: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;, multiplier_bps: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_find_min_stake_required">find_min_stake_required</a>(
    tiers: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a>&gt;, multiplier_bps: u64
): u64 {
    <b>let</b> (found, i) = tiers.find(|t| t.multiplier_bps &gt;= multiplier_bps);
    <b>assert</b>!(found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_EMULTIPLIER_NOT_AVAILABLE">EMULTIPLIER_NOT_AVAILABLE</a>));
    tiers[i].min_stake
}
</code></pre>



</details>

<a id="0x1_transaction_limits_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, execution_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;, io_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">transaction_limits::TxnLimitTier</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_initialize">initialize</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    execution_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a>&gt;,
    io_tiers: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitTier">TxnLimitTier</a>&gt;
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <a href="transaction_limits.md#0x1_transaction_limits_validate_tiers">validate_tiers</a>(&execution_tiers);
    <a href="transaction_limits.md#0x1_transaction_limits_validate_tiers">validate_tiers</a>(&io_tiers);

    <b>move_to</b>(
        aptos_framework,
        TxnLimitsConfig::V1 { execution_tiers, io_tiers }
    );
}
</code></pre>



</details>

<a id="0x1_transaction_limits_update_config"></a>

## Function `update_config`

Governance-only: update stake thresholds and multipliers.


<pre><code><b>public</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_update_config">update_config</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, execution_min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, execution_multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, io_min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, io_multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_update_config">update_config</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    execution_min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    execution_multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    io_min_stakes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    io_multipliers_bps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> execution_tiers = <a href="transaction_limits.md#0x1_transaction_limits_new_tiers">new_tiers</a>(
        execution_min_stakes, execution_multipliers_bps
    );
    <b>let</b> io_tiers = <a href="transaction_limits.md#0x1_transaction_limits_new_tiers">new_tiers</a>(io_min_stakes, io_multipliers_bps);

    <b>if</b> (!<b>exists</b>&lt;<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(
            aptos_framework,
            TxnLimitsConfig::V1 { execution_tiers, io_tiers }
        );
    } <b>else</b> {
        <b>let</b> config = &<b>mut</b> <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a>[@aptos_framework];
        config.execution_tiers = execution_tiers;
        config.io_tiers = io_tiers;
    }
}
</code></pre>



</details>

<a id="0x1_transaction_limits_validate_enough_stake"></a>

## Function `validate_enough_stake`

Aborts if:
- Requested multipliers are not well-formed.
- Transaction limits config does not exist or there is no tier
matching the requested multipliers.
- There is not enough stake to cover the minimum required amount.


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_enough_stake">validate_enough_stake</a>(stake_amount: u64, multipliers: <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">transaction_limits::RequestedMultipliers</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_enough_stake">validate_enough_stake</a>(
    stake_amount: u64, multipliers: <a href="transaction_limits.md#0x1_transaction_limits_RequestedMultipliers">RequestedMultipliers</a>
) <b>acquires</b> <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a> {
    <b>let</b> (execution_bps, io_bps) =
        match(multipliers) {
            RequestedMultipliers::V1 { execution_bps, io_bps } =&gt; (execution_bps, io_bps)
        };
    <b>assert</b>!(
        execution_bps &gt; <a href="transaction_limits.md#0x1_transaction_limits_MIN_MULTIPLIER_BPS">MIN_MULTIPLIER_BPS</a> && execution_bps &lt;= <a href="transaction_limits.md#0x1_transaction_limits_MAX_MULTIPLIER_BPS">MAX_MULTIPLIER_BPS</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_EINVALID_MULTIPLIER">EINVALID_MULTIPLIER</a>)
    );
    <b>assert</b>!(
        io_bps &gt; <a href="transaction_limits.md#0x1_transaction_limits_MIN_MULTIPLIER_BPS">MIN_MULTIPLIER_BPS</a> && io_bps &lt;= <a href="transaction_limits.md#0x1_transaction_limits_MAX_MULTIPLIER_BPS">MAX_MULTIPLIER_BPS</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="transaction_limits.md#0x1_transaction_limits_EINVALID_MULTIPLIER">EINVALID_MULTIPLIER</a>)
    );

    <b>let</b> config = &<a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a>[@aptos_framework];
    <b>let</b> execution_threshold =
        <a href="transaction_limits.md#0x1_transaction_limits_find_min_stake_required">find_min_stake_required</a>(&config.execution_tiers, execution_bps);
    <b>let</b> io_threshold = <a href="transaction_limits.md#0x1_transaction_limits_find_min_stake_required">find_min_stake_required</a>(&config.io_tiers, io_bps);

    <b>assert</b>!(
        stake_amount &gt;= execution_threshold,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="transaction_limits.md#0x1_transaction_limits_EINSUFFICIENT_STAKE">EINSUFFICIENT_STAKE</a>)
    );
    <b>assert</b>!(
        stake_amount &gt;= io_threshold, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="transaction_limits.md#0x1_transaction_limits_EINSUFFICIENT_STAKE">EINSUFFICIENT_STAKE</a>)
    );
}
</code></pre>



</details>

<a id="0x1_transaction_limits_validate_high_txn_limits"></a>

## Function `validate_high_txn_limits`

Only called during prologue to validate that the fee payer qualifies
for the requested limit multipliers.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_high_txn_limits">validate_high_txn_limits</a>(fee_payer: <b>address</b>, request: <a href="transaction_limits.md#0x1_transaction_limits_UserTxnLimitsRequest">transaction_limits::UserTxnLimitsRequest</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>friend</b> <b>fun</b> <a href="transaction_limits.md#0x1_transaction_limits_validate_high_txn_limits">validate_high_txn_limits</a>(
    fee_payer: <b>address</b>, request: <a href="transaction_limits.md#0x1_transaction_limits_UserTxnLimitsRequest">UserTxnLimitsRequest</a>
) <b>acquires</b> <a href="transaction_limits.md#0x1_transaction_limits_TxnLimitsConfig">TxnLimitsConfig</a> {
    match(request) {
        StakePoolOwner { multipliers } =&gt; {
            <b>assert</b>!(
                <a href="stake.md#0x1_stake_owner_cap_exists">stake::owner_cap_exists</a>(fee_payer),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="transaction_limits.md#0x1_transaction_limits_ENOT_STAKE_POOL_OWNER">ENOT_STAKE_POOL_OWNER</a>)
            );
            <b>let</b> pool_address = <a href="stake.md#0x1_stake_get_pool_address_for_owner">stake::get_pool_address_for_owner</a>(fee_payer);
            <b>let</b> stake_amount = <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">aptos_governance::get_voting_power</a>(pool_address);
            <a href="transaction_limits.md#0x1_transaction_limits_validate_enough_stake">validate_enough_stake</a>(stake_amount, multipliers);
        },
        DelegatedVoter { pool_address, multipliers } =&gt; {
            <b>assert</b>!(
                <a href="stake.md#0x1_stake_stake_pool_exists">stake::stake_pool_exists</a>(pool_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="transaction_limits.md#0x1_transaction_limits_ESTAKE_POOL_NOT_FOUND">ESTAKE_POOL_NOT_FOUND</a>)
            );
            <b>assert</b>!(
                fee_payer == <a href="stake.md#0x1_stake_get_delegated_voter">stake::get_delegated_voter</a>(pool_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="transaction_limits.md#0x1_transaction_limits_ENOT_DELEGATED_VOTER">ENOT_DELEGATED_VOTER</a>)
            );
            <b>let</b> stake_amount = <a href="aptos_governance.md#0x1_aptos_governance_get_voting_power">aptos_governance::get_voting_power</a>(pool_address);
            <a href="transaction_limits.md#0x1_transaction_limits_validate_enough_stake">validate_enough_stake</a>(stake_amount, multipliers);
        },
        DelegationPoolDelegator { pool_address, multipliers } =&gt; {
            <b>assert</b>!(
                <a href="delegation_pool.md#0x1_delegation_pool_delegation_pool_exists">delegation_pool::delegation_pool_exists</a>(pool_address),
                <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="transaction_limits.md#0x1_transaction_limits_EDELEGATION_POOL_NOT_FOUND">EDELEGATION_POOL_NOT_FOUND</a>)
            );
            <b>let</b> (active, _, pending_inactive) = <a href="delegation_pool.md#0x1_delegation_pool_get_stake">delegation_pool::get_stake</a>(
                pool_address, fee_payer
            );
            <a href="transaction_limits.md#0x1_transaction_limits_validate_enough_stake">validate_enough_stake</a>(active + pending_inactive, multipliers);
        }
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
