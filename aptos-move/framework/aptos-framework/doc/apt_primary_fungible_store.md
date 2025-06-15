
<a id="0x1_apt_primary_fungible_store"></a>

# Module `0x1::apt_primary_fungible_store`



-  [Function `store_address`](#0x1_apt_primary_fungible_store_store_address)
-  [Function `is_balance_at_least`](#0x1_apt_primary_fungible_store_is_balance_at_least)
-  [Function `burn_from`](#0x1_apt_primary_fungible_store_burn_from)
-  [Function `ensure_primary_store_exists`](#0x1_apt_primary_fungible_store_ensure_primary_store_exists)
-  [Function `transfer`](#0x1_apt_primary_fungible_store_transfer)


<pre><code><b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_apt_primary_fungible_store_store_address"></a>

## Function `store_address`



<pre><code><b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_store_address">store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_store_address">store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b> {
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(<a href="account.md#0x1_account">account</a>, @aptos_fungible_asset)
}
</code></pre>



</details>

<a id="0x1_apt_primary_fungible_store_is_balance_at_least"></a>

## Function `is_balance_at_least`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_is_balance_at_least">is_balance_at_least</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_is_balance_at_least">is_balance_at_least</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64): bool {
    <b>let</b> store_addr = <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_store_address">store_address</a>(<a href="account.md#0x1_account">account</a>);
    <a href="fungible_asset.md#0x1_fungible_asset_is_address_balance_at_least">fungible_asset::is_address_balance_at_least</a>(store_addr, amount)
}
</code></pre>



</details>

<a id="0x1_apt_primary_fungible_store_burn_from"></a>

## Function `burn_from`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_burn_from">burn_from</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_burn_from">burn_from</a>(
    ref: &BurnRef,
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    amount: u64,
) {
    // Skip burning <b>if</b> amount is zero. This shouldn't <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> out <b>as</b> it's called <b>as</b> part of transaction fee burning.
    <b>if</b> (amount != 0) {
        <b>let</b> store_addr = <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_store_address">store_address</a>(<a href="account.md#0x1_account">account</a>);
        <a href="fungible_asset.md#0x1_fungible_asset_address_burn_from">fungible_asset::address_burn_from</a>(ref, store_addr, amount);
    };
}
</code></pre>



</details>

<a id="0x1_apt_primary_fungible_store_ensure_primary_store_exists"></a>

## Function `ensure_primary_store_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) inline <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner: <b>address</b>): <b>address</b> {
    <b>let</b> store_addr = <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_store_address">store_address</a>(owner);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(store_addr)) {
        store_addr
    } <b>else</b> {
        <a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">primary_fungible_store::create_primary_store</a>(owner, <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;Metadata&gt;(@aptos_fungible_asset)))
    }
}
</code></pre>



</details>

<a id="0x1_apt_primary_fungible_store_transfer"></a>

## Function `transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_transfer">transfer</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_transfer">transfer</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient: <b>address</b>,
    amount: u64,
) {
    <b>let</b> sender_store = <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender));
    <b>let</b> recipient_store = <a href="apt_primary_fungible_store.md#0x1_apt_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(recipient);

    // <b>use</b> <b>internal</b> APIs, <b>as</b> they skip:
    // - owner, frozen and dispatchable checks
    // <b>as</b> APT cannot be frozen or have dispatch, and PFS cannot be transferred
    // (PFS could potentially be burned. regular transfer would permanently unburn the store.
    // Ignoring the check here <b>has</b> the equivalent of unburning, transferring, and then burning again)
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">fungible_asset::deposit_internal</a>(recipient_store, <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">fungible_asset::withdraw_internal</a>(sender_store, amount));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
