
<a id="0x7_native_position"></a>

# Module `0x7::native_position`

Native position store — framework-level storage for perp / spot position
data keyed by <code>(exchange_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, market)</code>, with an in-memory
residency model, dedicated <code>position_db</code> + <code>position_merkle_db</code>, and an
<code>AggregatorV2</code>-bounded per-exchange ceiling.

Access is gated by <code><a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a></code>. Any exchange can register,
obtain a cap, and read/write positions in its own <code>exchange_id</code>
namespace. Governance can <code><a href="native_position.md#0x7_native_position_deny">deny</a>(exchange_id)</code> to lock a compromised
exchange out without wiping its state.

See <code>PLAN_native_position.md</code> in the repo root for the design rationale.


-  [Struct `PositionCreated`](#0x7_native_position_PositionCreated)
-  [Struct `PositionUpdated`](#0x7_native_position_PositionUpdated)
-  [Struct `PositionRemoved`](#0x7_native_position_PositionRemoved)
-  [Struct `ExchangeRegistered`](#0x7_native_position_ExchangeRegistered)
-  [Struct `ExchangeDenied`](#0x7_native_position_ExchangeDenied)
-  [Struct `ExchangeReenabled`](#0x7_native_position_ExchangeReenabled)
-  [Struct `ExchangeCapability`](#0x7_native_position_ExchangeCapability)
-  [Enum `Position`](#0x7_native_position_Position)
-  [Constants](#@Constants_0)
-  [Function `register`](#0x7_native_position_register)
-  [Function `unregister`](#0x7_native_position_unregister)
-  [Function `deny`](#0x7_native_position_deny)
-  [Function `reenable`](#0x7_native_position_reenable)
-  [Function `update_ceiling`](#0x7_native_position_update_ceiling)
-  [Function `exchange_id`](#0x7_native_position_exchange_id)
-  [Function `create_position`](#0x7_native_position_create_position)
-  [Function `update_position`](#0x7_native_position_update_position)
-  [Function `remove_position`](#0x7_native_position_remove_position)
-  [Function `new_perp_v1`](#0x7_native_position_new_perp_v1)
-  [Function `new_spot_v1`](#0x7_native_position_new_spot_v1)
-  [Function `is_perp_v1`](#0x7_native_position_is_perp_v1)
-  [Function `unpack_perp_v1`](#0x7_native_position_unpack_perp_v1)
-  [Function `native_register`](#0x7_native_position_native_register)
-  [Function `native_deny`](#0x7_native_position_native_deny)
-  [Function `native_reenable`](#0x7_native_position_native_reenable)
-  [Function `native_create_position`](#0x7_native_position_native_create_position)
-  [Function `native_update_position`](#0x7_native_position_native_update_position)
-  [Function `native_remove_position`](#0x7_native_position_native_remove_position)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="position_counts.md#0x7_position_counts">0x7::position_counts</a>;
</code></pre>



<a id="0x7_native_position_PositionCreated"></a>

## Struct `PositionCreated`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_PositionCreated">PositionCreated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_PositionUpdated"></a>

## Struct `PositionUpdated`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_PositionUpdated">PositionUpdated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_PositionRemoved"></a>

## Struct `PositionRemoved`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_PositionRemoved">PositionRemoved</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_ExchangeRegistered"></a>

## Struct `ExchangeRegistered`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_ExchangeRegistered">ExchangeRegistered</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_ExchangeDenied"></a>

## Struct `ExchangeDenied`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_ExchangeDenied">ExchangeDenied</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_ExchangeReenabled"></a>

## Struct `ExchangeReenabled`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="native_position.md#0x7_native_position_ExchangeReenabled">ExchangeReenabled</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_ExchangeCapability"></a>

## Struct `ExchangeCapability`

Opaque capability for calling native-position functions in an
exchange's own namespace.

- <code><b>has</b> store</code>, no <code><b>copy</b></code>, no <code>drop</code>.
- Idempotent: calling <code><a href="native_position.md#0x7_native_position_register">register</a>()</code> twice with the same signer returns
a cap with the same <code>exchange_id</code>.
- A cap is a permission, not a unique authority.  An exchange can
hold multiple caps for the same <code>exchange_id</code>; all are equally
valid.  The exchange is responsible for custody.


<pre><code><b>struct</b> <a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>exchange_addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>exchange_id: u32</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_native_position_Position"></a>

## Enum `Position`

Deserialized form of a persisted position. Mirrors the
native-position <code>NativePosition</code> Rust enum: one byte variant tag
plus fixed-width fields.


<pre><code>enum <a href="native_position.md#0x7_native_position_Position">Position</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>PerpV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_long: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>entry_px_times_size_sum: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>avg_entry_px: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>user_leverage: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>is_isolated: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>funding_index: i128</code>
</dt>
<dd>
 Signed funding index at last position update. Matches
 etna's <code>AccumulativeIndex { index: i128 }</code>.
</dd>
<dt>
<code>unrealized_funding_before: i64</code>
</dt>
<dd>
 Signed accrued funding before the last update. Matches
 etna's <code>unrealized_funding_amount_before_last_update: i64</code>.
</dd>
<dt>
<code><a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>SpotV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>size: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>is_long: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>entry_px_times_size_sum: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>avg_entry_px: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_native_position_EPOSITION_LIMIT"></a>

Attempting to add a position that would cross the per-exchange limit.
Propagated from the position-count aggregator.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EPOSITION_LIMIT">EPOSITION_LIMIT</a>: u64 = 5;
</code></pre>



<a id="0x7_native_position_EBAD_CAPABILITY"></a>

The supplied capability references an unknown exchange_id.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EBAD_CAPABILITY">EBAD_CAPABILITY</a>: u64 = 6;
</code></pre>



<a id="0x7_native_position_EEXCHANGE_DENIED"></a>

This <code>exchange_id</code> has been disabled by governance.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EEXCHANGE_DENIED">EEXCHANGE_DENIED</a>: u64 = 3;
</code></pre>



<a id="0x7_native_position_EEXCHANGE_NOT_REGISTERED"></a>

Exchange has not been registered yet.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EEXCHANGE_NOT_REGISTERED">EEXCHANGE_NOT_REGISTERED</a>: u64 = 2;
</code></pre>



<a id="0x7_native_position_EFEATURE_DISABLED"></a>

Feature <code>NATIVE_POSITION</code> is not enabled on this chain.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EFEATURE_DISABLED">EFEATURE_DISABLED</a>: u64 = 1;
</code></pre>



<a id="0x7_native_position_EPOSITION_NOT_FOUND"></a>

Position requested does not exist.


<pre><code><b>const</b> <a href="native_position.md#0x7_native_position_EPOSITION_NOT_FOUND">EPOSITION_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x7_native_position_register"></a>

## Function `register`

Register an exchange, allocating an <code>exchange_id</code> and an
<code>AggregatorV2</code>-backed position counter bounded at <code>initial_max</code>.

Idempotent per signer: subsequent calls from the same signer return a
cap with the same <code>exchange_id</code>. The stored <code>initial_max</code> from the
first call sticks; use <code>update_ceiling</code> via governance to tune later.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_register">register</a>(exchange: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_max: u64): <a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_register">register</a>(exchange: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_max: u64): <a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a> {
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_native_position_enabled">features::is_native_position_enabled</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="native_position.md#0x7_native_position_EFEATURE_DISABLED">EFEATURE_DISABLED</a>),
    );
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(exchange);
    <b>let</b> exchange_id = <a href="native_position.md#0x7_native_position_native_register">native_register</a>(addr, initial_max);
    // If this is the first time we've seen `exchange_id`, allocate the
    // counter.  Subsequent calls are no-ops via the existence check.
    <b>let</b> first_register = !<a href="position_counts.md#0x7_position_counts_counter_exists">position_counts::counter_exists</a>(exchange_id);
    <b>if</b> (first_register) {
        <a href="position_counts.md#0x7_position_counts_allocate_counter">position_counts::allocate_counter</a>(exchange_id, initial_max);
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_ExchangeRegistered">ExchangeRegistered</a> { exchange_addr: addr, exchange_id });
    };
    <a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a> { exchange_addr: addr, exchange_id }
}
</code></pre>



</details>

<a id="0x7_native_position_unregister"></a>

## Function `unregister`

Destroy a capability. The underlying <code>exchange_id</code> stays allocated
and any other caps for the same id remain valid.  Re-registering
from the same signer returns the existing id.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_unregister">unregister</a>(cap: <a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_unregister">unregister</a>(cap: <a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a>) {
    <b>let</b> <a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a> { exchange_addr: _, exchange_id: _ } = cap;
}
</code></pre>



</details>

<a id="0x7_native_position_deny"></a>

## Function `deny`

Governance-only: lock an <code>exchange_id</code> out. All future native calls
that carry a cap with this id abort <code><a href="native_position.md#0x7_native_position_EEXCHANGE_DENIED">EEXCHANGE_DENIED</a></code>. Persisted
positions are untouched — this is a lockout, not a wipe.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_deny">deny</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_deny">deny</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32) {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="native_position.md#0x7_native_position_native_deny">native_deny</a>(exchange_id);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_ExchangeDenied">ExchangeDenied</a> { exchange_id });
}
</code></pre>



</details>

<a id="0x7_native_position_reenable"></a>

## Function `reenable`

Governance-only: re-enable an <code>exchange_id</code> previously locked via
<code>deny</code>. Use only if the compromise has been resolved.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_reenable">reenable</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_reenable">reenable</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32) {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <a href="native_position.md#0x7_native_position_native_reenable">native_reenable</a>(exchange_id);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_ExchangeReenabled">ExchangeReenabled</a> { exchange_id });
}
</code></pre>



</details>

<a id="0x7_native_position_update_ceiling"></a>

## Function `update_ceiling`

Governance-only: bump or shrink the per-exchange position-count
ceiling. Delegates to <code><a href="position_counts.md#0x7_position_counts">position_counts</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_update_ceiling">update_ceiling</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32, new_max: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_update_ceiling">update_ceiling</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32, new_max: u64) {
    <a href="position_counts.md#0x7_position_counts_update_ceiling">position_counts::update_ceiling</a>(framework, exchange_id, new_max);
}
</code></pre>



</details>

<a id="0x7_native_position_exchange_id"></a>

## Function `exchange_id`



<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_exchange_id">exchange_id</a>(cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_exchange_id">exchange_id</a>(cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a>): u32 {
    cap.exchange_id
}
</code></pre>



</details>

<a id="0x7_native_position_create_position"></a>

## Function `create_position`

Create a brand-new position. Bumps the per-exchange aggregator.
Emits a paired <code>UserMarkets</code> write to add <code>market</code> to the user's set.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_create_position">create_position</a>(cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>, position: <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_create_position">create_position</a>(
    cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    market: <b>address</b>,
    position: <a href="native_position.md#0x7_native_position_Position">Position</a>,
) {
    <a href="native_position.md#0x7_native_position_native_create_position">native_create_position</a>(cap.exchange_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, market, position);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_PositionCreated">PositionCreated</a> {
        exchange_id: cap.exchange_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        market,
    });
}
</code></pre>



</details>

<a id="0x7_native_position_update_position"></a>

## Function `update_position`

Mutate an existing position's data in place. Does NOT touch
<code>UserMarkets</code> — modification doesn't change set membership.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_update_position">update_position</a>(cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>, position: <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_update_position">update_position</a>(
    cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    market: <b>address</b>,
    position: <a href="native_position.md#0x7_native_position_Position">Position</a>,
) {
    <a href="native_position.md#0x7_native_position_native_update_position">native_update_position</a>(cap.exchange_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, market, position);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_PositionUpdated">PositionUpdated</a> {
        exchange_id: cap.exchange_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        market,
    });
}
</code></pre>



</details>

<a id="0x7_native_position_remove_position"></a>

## Function `remove_position`

Remove a position. Decrements the per-exchange aggregator. Emits a
paired <code>UserMarkets</code> write to remove <code>market</code> from the user's set
(or delete the entire set entry if this was the user's last market).


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_remove_position">remove_position</a>(cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">native_position::ExchangeCapability</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_remove_position">remove_position</a>(
    cap: &<a href="native_position.md#0x7_native_position_ExchangeCapability">ExchangeCapability</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    market: <b>address</b>,
) {
    <a href="native_position.md#0x7_native_position_native_remove_position">native_remove_position</a>(cap.exchange_id, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, market);
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="native_position.md#0x7_native_position_PositionRemoved">PositionRemoved</a> {
        exchange_id: cap.exchange_id,
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        market,
    });
}
</code></pre>



</details>

<a id="0x7_native_position_new_perp_v1"></a>

## Function `new_perp_v1`

Construct a <code>Position::PerpV1</code> with the given field values.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_new_perp_v1">new_perp_v1</a>(size: u64, is_long: bool, entry_px_times_size_sum: u128, avg_entry_px: u64, user_leverage: u8, is_isolated: bool, funding_index: i128, unrealized_funding_before: i64, <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64): <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_new_perp_v1">new_perp_v1</a>(
    size: u64,
    is_long: bool,
    entry_px_times_size_sum: u128,
    avg_entry_px: u64,
    user_leverage: u8,
    is_isolated: bool,
    funding_index: i128,
    unrealized_funding_before: i64,
    <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64,
): <a href="native_position.md#0x7_native_position_Position">Position</a> {
    Position::PerpV1 {
        size,
        is_long,
        entry_px_times_size_sum,
        avg_entry_px,
        user_leverage,
        is_isolated,
        funding_index,
        unrealized_funding_before,
        <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>,
    }
}
</code></pre>



</details>

<a id="0x7_native_position_new_spot_v1"></a>

## Function `new_spot_v1`

Construct a <code>Position::SpotV1</code> with the given field values.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_new_spot_v1">new_spot_v1</a>(size: u64, is_long: bool, entry_px_times_size_sum: u128, avg_entry_px: u64, <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64): <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_new_spot_v1">new_spot_v1</a>(
    size: u64,
    is_long: bool,
    entry_px_times_size_sum: u128,
    avg_entry_px: u64,
    <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>: u64,
): <a href="native_position.md#0x7_native_position_Position">Position</a> {
    Position::SpotV1 {
        size,
        is_long,
        entry_px_times_size_sum,
        avg_entry_px,
        <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>,
    }
}
</code></pre>



</details>

<a id="0x7_native_position_is_perp_v1"></a>

## Function `is_perp_v1`

True iff <code>pos</code> is the PerpV1 variant.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_is_perp_v1">is_perp_v1</a>(pos: &<a href="native_position.md#0x7_native_position_Position">native_position::Position</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_is_perp_v1">is_perp_v1</a>(pos: &<a href="native_position.md#0x7_native_position_Position">Position</a>): bool {
    match (pos) {
        Position::PerpV1 { .. } =&gt; <b>true</b>,
        Position::SpotV1 { .. } =&gt; <b>false</b>,
    }
}
</code></pre>



</details>

<a id="0x7_native_position_unpack_perp_v1"></a>

## Function `unpack_perp_v1`

Destructure a <code>Position::PerpV1</code> into its field tuple. Aborts
if <code>pos</code> is not the PerpV1 variant.


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_unpack_perp_v1">unpack_perp_v1</a>(pos: <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>): (u64, bool, u128, u64, u8, bool, i128, i64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="native_position.md#0x7_native_position_unpack_perp_v1">unpack_perp_v1</a>(
    pos: <a href="native_position.md#0x7_native_position_Position">Position</a>,
): (u64, bool, u128, u64, u8, bool, i128, i64, u64) {
    match (pos) {
        Position::PerpV1 {
            size,
            is_long,
            entry_px_times_size_sum,
            avg_entry_px,
            user_leverage,
            is_isolated,
            funding_index,
            unrealized_funding_before,
            <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>,
        } =&gt; (
            size,
            is_long,
            entry_px_times_size_sum,
            avg_entry_px,
            user_leverage,
            is_isolated,
            funding_index,
            unrealized_funding_before,
            <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">timestamp</a>,
        ),
        Position::SpotV1 { .. } =&gt; <b>abort</b> 0xDEAD_BEEF,
    }
}
</code></pre>



</details>

<a id="0x7_native_position_native_register"></a>

## Function `native_register`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_register">native_register</a>(exchange_addr: <b>address</b>, initial_max: u64): u32
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_register">native_register</a>(exchange_addr: <b>address</b>, initial_max: u64): u32;
</code></pre>



</details>

<a id="0x7_native_position_native_deny"></a>

## Function `native_deny`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_deny">native_deny</a>(exchange_id: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_deny">native_deny</a>(exchange_id: u32);
</code></pre>



</details>

<a id="0x7_native_position_native_reenable"></a>

## Function `native_reenable`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_reenable">native_reenable</a>(exchange_id: u32)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_reenable">native_reenable</a>(exchange_id: u32);
</code></pre>



</details>

<a id="0x7_native_position_native_create_position"></a>

## Function `native_create_position`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_create_position">native_create_position</a>(exchange_id: u32, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>, position: <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_create_position">native_create_position</a>(
    exchange_id: u32,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    market: <b>address</b>,
    position: <a href="native_position.md#0x7_native_position_Position">Position</a>,
);
</code></pre>



</details>

<a id="0x7_native_position_native_update_position"></a>

## Function `native_update_position`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_update_position">native_update_position</a>(exchange_id: u32, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>, position: <a href="native_position.md#0x7_native_position_Position">native_position::Position</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_update_position">native_update_position</a>(
    exchange_id: u32,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    market: <b>address</b>,
    position: <a href="native_position.md#0x7_native_position_Position">Position</a>,
);
</code></pre>



</details>

<a id="0x7_native_position_native_remove_position"></a>

## Function `native_remove_position`



<pre><code><b>fun</b> <a href="native_position.md#0x7_native_position_native_remove_position">native_remove_position</a>(exchange_id: u32, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="native_position.md#0x7_native_position_native_remove_position">native_remove_position</a>(exchange_id: u32, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, market: <b>address</b>);
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
