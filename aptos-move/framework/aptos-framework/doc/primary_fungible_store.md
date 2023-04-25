
<a name="0x1_primary_fungible_store"></a>

# Module `0x1::primary_fungible_store`

This defines the module for interacting with primary stores of accounts/objects, which have deterministic addresses


-  [Resource `DeriveRefPod`](#0x1_primary_fungible_store_DeriveRefPod)
-  [Function `create_primary_store_enabled_fungible_asset`](#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset)
-  [Function `ensure_primary_store_exists`](#0x1_primary_fungible_store_ensure_primary_store_exists)
-  [Function `create_primary_store`](#0x1_primary_fungible_store_create_primary_store)
-  [Function `primary_store_address`](#0x1_primary_fungible_store_primary_store_address)
-  [Function `primary_store`](#0x1_primary_fungible_store_primary_store)
-  [Function `primary_store_exists`](#0x1_primary_fungible_store_primary_store_exists)
-  [Function `balance`](#0x1_primary_fungible_store_balance)
-  [Function `is_frozen`](#0x1_primary_fungible_store_is_frozen)
-  [Function `withdraw`](#0x1_primary_fungible_store_withdraw)
-  [Function `deposit`](#0x1_primary_fungible_store_deposit)
-  [Function `transfer`](#0x1_primary_fungible_store_transfer)
-  [Specification](#@Specification_0)


<pre><code><b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_primary_fungible_store_DeriveRefPod"></a>

## Resource `DeriveRefPod`

Resource stored on the fungible asset metadata object to allow creating primary stores for it.


<pre><code><b>struct</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata_derive_ref: <a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset"></a>

## Function `create_primary_store_enabled_fungible_asset`

Creators of fungible assets can call this to enable support for creating primary (deterministic) stores for
their users.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">create_primary_store_enabled_fungible_asset</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, monitoring_supply_with_maximum: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;&gt;, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">create_primary_store_enabled_fungible_asset</a>(
    constructor_ref: &ConstructorRef,
    monitoring_supply_with_maximum: Option&lt;Option&lt;u128&gt;&gt;,
    name: String,
    symbol: String,
    decimals: u8,
) {
    <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">fungible_asset::add_fungibility</a>(constructor_ref, monitoring_supply_with_maximum, name, symbol, decimals);
    <b>let</b> metadata_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(metadata_obj, <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
        metadata_derive_ref: <a href="object.md#0x1_object_generate_derive_ref">object::generate_derive_ref</a>(constructor_ref),
    });
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_ensure_primary_store_exists"></a>

## Function `ensure_primary_store_exists`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>&lt;T: key&gt;(
    owner: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleStore&gt; <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>if</b> (!<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>(owner, metadata)) {
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">create_primary_store</a>(owner, metadata)
    } <b>else</b> {
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(owner, metadata)
    }
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_create_primary_store"></a>

## Function `create_primary_store`

Create a primary store object to hold fungible asset for the given address.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">create_primary_store</a>&lt;T: key&gt;(owner_addr: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">create_primary_store</a>&lt;T: key&gt;(
    owner_addr: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleStore&gt; <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <b>let</b> derive_ref = &<b>borrow_global</b>&lt;<a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a>&gt;(metadata_addr).metadata_derive_ref;
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_user_derived_object">object::create_user_derived_object</a>(owner_addr, derive_ref);

    // Disable ungated transfer <b>as</b> deterministic stores shouldn't be transferrable.
    <b>let</b> transfer_ref = &<a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(constructor_ref);
    <a href="object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(transfer_ref);

    <a href="fungible_asset.md#0x1_fungible_asset_create_store">fungible_asset::create_store</a>(constructor_ref, metadata)
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_primary_store_address"></a>

## Function `primary_store_address`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_primary_store"></a>

## Function `primary_store`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleStore&gt; {
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>(owner, metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;FungibleStore&gt;(store)
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_primary_store_exists"></a>

## Function `primary_store_exists`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_balance"></a>

## Function `balance`

Get the balance of <code><a href="account.md#0x1_account">account</a></code>'s primary store.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): u64 {
    <b>if</b> (<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>(<a href="account.md#0x1_account">account</a>, metadata)) {
        <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(<a href="account.md#0x1_account">account</a>, metadata))
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_is_frozen"></a>

## Function `is_frozen`

Return whether the given account's primary store is frozen.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_frozen">is_frozen</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_frozen">is_frozen</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <b>if</b> (<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>(<a href="account.md#0x1_account">account</a>, metadata)) {
        <a href="fungible_asset.md#0x1_fungible_asset_is_frozen">fungible_asset::is_frozen</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(<a href="account.md#0x1_account">account</a>, metadata))
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from <code>store</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;T&gt;, amount: u64): FungibleAsset {
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw">fungible_asset::withdraw</a>(owner, store, amount)
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of fungible asset to the given account's primary store.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">deposit</a>(owner: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">deposit</a>(owner: <b>address</b>, fa: FungibleAsset) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&fa);
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">fungible_asset::deposit</a>(store, fa);
}
</code></pre>



</details>

<a name="0x1_primary_fungible_store_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset from sender's primary store to receiver's primary store.


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, recipient: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer">transfer</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: Object&lt;T&gt;,
    recipient: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> sender_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), metadata);
    <b>let</b> recipient_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(recipient, metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_transfer">fungible_asset::transfer</a>(sender, sender_store, recipient_store, amount);
}
</code></pre>



</details>

<a name="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
