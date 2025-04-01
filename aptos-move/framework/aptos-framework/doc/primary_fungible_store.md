
<a id="0x1_primary_fungible_store"></a>

# Module `0x1::primary_fungible_store`

This module provides a way for creators of fungible assets to enable support for creating primary (deterministic)
stores for their users. This is useful for assets that are meant to be used as a currency, as it allows users to
easily create a store for their account and deposit/withdraw/transfer fungible assets to/from it.

The transfer flow works as below:
1. The sender calls <code>transfer</code> on the fungible asset metadata object to transfer <code>amount</code> of fungible asset to
<code>recipient</code>.
2. The fungible asset metadata object calls <code>ensure_primary_store_exists</code> to ensure that both the sender's and the
recipient's primary stores exist. If either doesn't, it will be created.
3. The fungible asset metadata object calls <code>withdraw</code> on the sender's primary store to withdraw <code>amount</code> of
fungible asset from it. This emits a withdraw event.
4. The fungible asset metadata object calls <code>deposit</code> on the recipient's primary store to deposit <code>amount</code> of
fungible asset to it. This emits an deposit event.


-  [Resource `DeriveRefPod`](#0x1_primary_fungible_store_DeriveRefPod)
-  [Function `create_primary_store_enabled_fungible_asset`](#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset)
-  [Function `ensure_primary_store_exists`](#0x1_primary_fungible_store_ensure_primary_store_exists)
-  [Function `create_primary_store`](#0x1_primary_fungible_store_create_primary_store)
-  [Function `primary_store_address`](#0x1_primary_fungible_store_primary_store_address)
-  [Function `primary_store`](#0x1_primary_fungible_store_primary_store)
-  [Function `primary_store_exists`](#0x1_primary_fungible_store_primary_store_exists)
-  [Function `primary_store_address_inlined`](#0x1_primary_fungible_store_primary_store_address_inlined)
-  [Function `primary_store_inlined`](#0x1_primary_fungible_store_primary_store_inlined)
-  [Function `primary_store_exists_inlined`](#0x1_primary_fungible_store_primary_store_exists_inlined)
-  [Function `grant_permission`](#0x1_primary_fungible_store_grant_permission)
-  [Function `grant_apt_permission`](#0x1_primary_fungible_store_grant_apt_permission)
-  [Function `balance`](#0x1_primary_fungible_store_balance)
-  [Function `is_balance_at_least`](#0x1_primary_fungible_store_is_balance_at_least)
-  [Function `is_frozen`](#0x1_primary_fungible_store_is_frozen)
-  [Function `withdraw`](#0x1_primary_fungible_store_withdraw)
-  [Function `deposit`](#0x1_primary_fungible_store_deposit)
-  [Function `deposit_with_signer`](#0x1_primary_fungible_store_deposit_with_signer)
-  [Function `transfer`](#0x1_primary_fungible_store_transfer)
-  [Function `transfer_assert_minimum_deposit`](#0x1_primary_fungible_store_transfer_assert_minimum_deposit)
-  [Function `mint`](#0x1_primary_fungible_store_mint)
-  [Function `burn`](#0x1_primary_fungible_store_burn)
-  [Function `set_frozen_flag`](#0x1_primary_fungible_store_set_frozen_flag)
-  [Function `withdraw_with_ref`](#0x1_primary_fungible_store_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_primary_fungible_store_deposit_with_ref)
-  [Function `transfer_with_ref`](#0x1_primary_fungible_store_transfer_with_ref)
-  [Function `may_be_unburn`](#0x1_primary_fungible_store_may_be_unburn)
-  [Specification](#@Specification_0)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)


<pre><code><b>use</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">0x1::dispatchable_fungible_asset</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_primary_fungible_store_DeriveRefPod"></a>

## Resource `DeriveRefPod`

A resource that holds the derive ref for the fungible asset metadata object. This is used to create primary
stores for users with deterministic addresses so that users can easily deposit/withdraw/transfer fungible
assets.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> <b>has</b> key
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

<a id="0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset"></a>

## Function `create_primary_store_enabled_fungible_asset`

Create a fungible asset with primary store support. When users transfer fungible assets to each other, their
primary stores will be created automatically if they don't exist. Primary stores have deterministic addresses
so that users can easily deposit/withdraw/transfer fungible assets.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">create_primary_store_enabled_fungible_asset</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u128&gt;, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8, icon_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, project_uri: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store_enabled_fungible_asset">create_primary_store_enabled_fungible_asset</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: Option&lt;u128&gt;,
    name: String,
    symbol: String,
    decimals: u8,
    icon_uri: String,
    project_uri: String,
) {
    <a href="fungible_asset.md#0x1_fungible_asset_add_fungibility">fungible_asset::add_fungibility</a>(
        constructor_ref,
        maximum_supply,
        name,
        symbol,
        decimals,
        icon_uri,
        project_uri,
    );
    <b>let</b> metadata_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>move_to</b>(metadata_obj, <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
        metadata_derive_ref: <a href="object.md#0x1_object_generate_derive_ref">object::generate_derive_ref</a>(constructor_ref),
    });
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_ensure_primary_store_exists"></a>

## Function `ensure_primary_store_exists`

Ensure that the primary store object for the given address exists. If it doesn't, create it.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>&lt;T: key&gt;(
    owner: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;FungibleStore&gt; <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> store_addr = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>(owner, metadata);
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(store_addr)) {
        <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>(store_addr)
    } <b>else</b> {
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_create_primary_store">create_primary_store</a>(owner, metadata)
    }
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_create_primary_store"></a>

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
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;Metadata&gt;(metadata_addr);
    <b>let</b> derive_ref = &<b>borrow_global</b>&lt;<a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a>&gt;(metadata_addr).metadata_derive_ref;
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_user_derived_object">object::create_user_derived_object</a>(owner_addr, derive_ref);
    // Disable ungated transfer <b>as</b> deterministic stores shouldn't be transferrable.
    <b>let</b> transfer_ref = &<a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(constructor_ref);
    <a href="object.md#0x1_object_disable_ungated_transfer">object::disable_ungated_transfer</a>(transfer_ref);

    <a href="fungible_asset.md#0x1_fungible_asset_create_store">fungible_asset::create_store</a>(constructor_ref, metadata)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_address"></a>

## Function `primary_store_address`

Get the address of the primary store for the given account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store"></a>

## Function `primary_store`

Get the primary store object for the given account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleStore&gt; {
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>(owner, metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;FungibleStore&gt;(store)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_exists"></a>

## Function `primary_store_exists`

Return whether the given account's primary store exists.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address">primary_store_address</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_address_inlined"></a>

## Function `primary_store_address_inlined`

Get the address of the primary store for the given account.
Use instead of the corresponding view functions for dispatchable hooks to avoid circular dependencies of modules.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_inlined"></a>

## Function `primary_store_inlined`

Get the primary store object for the given account.
Use instead of the corresponding view functions for dispatchable hooks to avoid circular dependencies of modules.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_inlined">primary_store_inlined</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_inlined">primary_store_inlined</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): Object&lt;FungibleStore&gt; {
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>(owner, metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>(store)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_exists_inlined"></a>

## Function `primary_store_exists_inlined`

Return whether the given account's primary store exists.
Use instead of the corresponding view functions for dispatchable hooks to avoid circular dependencies of modules.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists_inlined">primary_store_exists_inlined</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists_inlined">primary_store_exists_inlined</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
    <a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_grant_permission"></a>

## Function `grant_permission`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_grant_permission">grant_permission</a>&lt;T: key&gt;(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_grant_permission">grant_permission</a>&lt;T: key&gt;(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: Object&lt;T&gt;,
    amount: u64
) {
    <a href="fungible_asset.md#0x1_fungible_asset_grant_permission_by_address">fungible_asset::grant_permission_by_address</a>(
        master,
        permissioned,
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned), metadata),
        amount
    );
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_grant_apt_permission"></a>

## Function `grant_apt_permission`



<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_grant_apt_permission">grant_apt_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_grant_apt_permission">grant_apt_permission</a>(
    master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    permissioned: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    amount: u64
) {
    <a href="fungible_asset.md#0x1_fungible_asset_grant_permission_by_address">fungible_asset::grant_permission_by_address</a>(
        master,
        permissioned,
        <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(permissioned), @aptos_fungible_asset),
        amount
    );
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_balance"></a>

## Function `balance`

Get the balance of <code><a href="account.md#0x1_account">account</a></code>'s primary store.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_balance">balance</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): u64 {
    <b>if</b> (<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>(<a href="account.md#0x1_account">account</a>, metadata)) {
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_derived_balance">dispatchable_fungible_asset::derived_balance</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(<a href="account.md#0x1_account">account</a>, metadata))
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_is_balance_at_least"></a>

## Function `is_balance_at_least`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_balance_at_least">is_balance_at_least</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;, amount: u64): bool {
    <b>if</b> (<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_store_exists</a>(<a href="account.md#0x1_account">account</a>, metadata)) {
        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_is_derived_balance_at_least">dispatchable_fungible_asset::is_derived_balance_at_least</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(<a href="account.md#0x1_account">account</a>, metadata), amount)
    } <b>else</b> {
        amount == 0
    }
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_is_frozen"></a>

## Function `is_frozen`

Return whether the given account's primary store is frozen.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_is_frozen">is_frozen</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
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

<a id="0x1_primary_fungible_store_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from the given account's primary store.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: Object&lt;T&gt;, amount: u64): FungibleAsset <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), metadata);
    // Check <b>if</b> the store <a href="object.md#0x1_object">object</a> <b>has</b> been burnt or not. If so, unburn it first.
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_may_be_unburn">may_be_unburn</a>(owner, store);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_withdraw">dispatchable_fungible_asset::withdraw</a>(owner, store, amount)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_deposit"></a>

## Function `deposit`

Deposit fungible asset <code>fa</code> to the given account's primary store.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">deposit</a>(owner: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit">deposit</a>(owner: <b>address</b>, fa: FungibleAsset) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&fa);
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner, metadata);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">dispatchable_fungible_asset::deposit</a>(store, fa);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_deposit_with_signer"></a>

## Function `deposit_with_signer`

Deposit fungible asset <code>fa</code> to the given account's primary store using signer.

If <code>owner</code> is a permissioned signer, the signer will be granted with permission to withdraw
the same amount of fund in the future.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit_with_signer">deposit_with_signer</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit_with_signer">deposit_with_signer</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, fa: FungibleAsset) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_refill_permission">fungible_asset::refill_permission</a>(
        owner,
        <a href="fungible_asset.md#0x1_fungible_asset_amount">fungible_asset::amount</a>(&fa),
        <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_address_inlined">primary_store_address_inlined</a>(
            <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner),
            <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">fungible_asset::metadata_from_asset</a>(&fa),
        )
    );
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">fungible_asset::asset_metadata</a>(&fa);
    <b>let</b> store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), metadata);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_deposit">dispatchable_fungible_asset::deposit</a>(store, fa);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer"></a>

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
    // Check <b>if</b> the sender store <a href="object.md#0x1_object">object</a> <b>has</b> been burnt or not. If so, unburn it first.
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_may_be_unburn">may_be_unburn</a>(sender, sender_store);
    <b>let</b> recipient_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(recipient, metadata);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">dispatchable_fungible_asset::transfer</a>(sender, sender_store, recipient_store, amount);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer <code>amount</code> of fungible asset from sender's primary store to receiver's primary store.
Use the minimum deposit assertion api to make sure receipient will receive a minimum amount of fund.


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, recipient: <b>address</b>, amount: u64, expected: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer_assert_minimum_deposit">transfer_assert_minimum_deposit</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    metadata: Object&lt;T&gt;,
    recipient: <b>address</b>,
    amount: u64,
    expected: u64,
) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> sender_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), metadata);
    // Check <b>if</b> the sender store <a href="object.md#0x1_object">object</a> <b>has</b> been burnt or not. If so, unburn it first.
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_may_be_unburn">may_be_unburn</a>(sender, sender_store);
    <b>let</b> recipient_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(recipient, metadata);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer_assert_minimum_deposit">dispatchable_fungible_asset::transfer_assert_minimum_deposit</a>(
        sender,
        sender_store,
        recipient_store,
        amount,
        expected
    );
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_mint"></a>

## Function `mint`

Mint to the primary store of <code>owner</code>.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_mint">mint</a>(mint_ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, owner: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_mint">mint</a>(mint_ref: &MintRef, owner: <b>address</b>, amount: u64) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner, <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">fungible_asset::mint_ref_metadata</a>(mint_ref));
    <a href="fungible_asset.md#0x1_fungible_asset_mint_to">fungible_asset::mint_to</a>(mint_ref, primary_store, amount);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_burn"></a>

## Function `burn`

Burn from the primary store of <code>owner</code>.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_burn">burn</a>(burn_ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, owner: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_burn">burn</a>(burn_ref: &BurnRef, owner: <b>address</b>, amount: u64) {
    <b>let</b> primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(owner, <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">fungible_asset::burn_ref_metadata</a>(burn_ref));
    <a href="fungible_asset.md#0x1_fungible_asset_burn_from">fungible_asset::burn_from</a>(burn_ref, primary_store, amount);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_set_frozen_flag"></a>

## Function `set_frozen_flag`

Freeze/Unfreeze the primary store of <code>owner</code>.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_set_frozen_flag">set_frozen_flag</a>(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, owner: <b>address</b>, frozen: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_set_frozen_flag">set_frozen_flag</a>(transfer_ref: &TransferRef, owner: <b>address</b>, frozen: bool) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(owner, <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(transfer_ref));
    <a href="fungible_asset.md#0x1_fungible_asset_set_frozen_flag">fungible_asset::set_frozen_flag</a>(transfer_ref, primary_store, frozen);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw from the primary store of <code>owner</code> ignoring frozen flag.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, owner: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref: &TransferRef, owner: <b>address</b>, amount: u64): FungibleAsset {
    <b>let</b> from_primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(owner, <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(transfer_ref));
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">fungible_asset::withdraw_with_ref</a>(transfer_ref, from_primary_store, amount)
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit to the primary store of <code>owner</code> ignoring frozen flag.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit_with_ref">deposit_with_ref</a>(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, owner: <b>address</b>, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_deposit_with_ref">deposit_with_ref</a>(transfer_ref: &TransferRef, owner: <b>address</b>, fa: FungibleAsset) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> to_primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(
        owner,
        <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(transfer_ref)
    );
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">fungible_asset::deposit_with_ref</a>(transfer_ref, to_primary_store, fa);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of FA from the primary store of <code>from</code> to that of <code><b>to</b></code> ignoring frozen flag.


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer_with_ref">transfer_with_ref</a>(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: <b>address</b>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_transfer_with_ref">transfer_with_ref</a>(
    transfer_ref: &TransferRef,
    from: <b>address</b>,
    <b>to</b>: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_DeriveRefPod">DeriveRefPod</a> {
    <b>let</b> from_primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_store</a>(from, <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(transfer_ref));
    <b>let</b> to_primary_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">ensure_primary_store_exists</a>(<b>to</b>, <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">fungible_asset::transfer_ref_metadata</a>(transfer_ref));
    <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">fungible_asset::transfer_with_ref</a>(transfer_ref, from_primary_store, to_primary_store, amount);
}
</code></pre>



</details>

<a id="0x1_primary_fungible_store_may_be_unburn"></a>

## Function `may_be_unburn`



<pre><code><b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_may_be_unburn">may_be_unburn</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_may_be_unburn">may_be_unburn</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, store: Object&lt;FungibleStore&gt;) {
    <b>if</b> (<a href="object.md#0x1_object_is_burnt">object::is_burnt</a>(store)) {
        <a href="object.md#0x1_object_unburn">object::unburn</a>(owner, store);
    };
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Creating a fungible asset with primary store support should initiate a derived reference and store it under the metadata object.</td>
<td>Medium</td>
<td>The function create_primary_store_enabled_fungible_asset makes an existing object, fungible, via the fungible_asset::add_fungibility function and initializes the DeriveRefPod resource by generating a DeriveRef for the object and then stores it under the object address.</td>
<td>Audited that the DeriveRefPod has been properly initialized and stored under the metadata object.</td>
</tr>

<tr>
<td>2</td>
<td>Fetching and creating a primary fungible store of an asset should only succeed if the object supports primary store.</td>
<td>Low</td>
<td>The function create_primary_store is used to create a primary store by borrowing the DeriveRef resource from the object. In case the resource does not exist, creation will fail. The function ensure_primary_store_exists is used to fetch the primary store if it exists, otherwise it will create one via the create_primary function.</td>
<td>Audited that it aborts if the DeriveRefPod doesn't exist. Audited that it aborts if the FungibleStore resource exists already under the object address.</td>
</tr>

<tr>
<td>3</td>
<td>It should be possible to create a primary store to hold a fungible asset.</td>
<td>Medium</td>
<td>The function create_primary_store borrows the DeriveRef resource from DeriveRefPod and then creates the store which is returned.</td>
<td>Audited that it returns the newly created FungibleStore.</td>
</tr>

<tr>
<td>4</td>
<td>Fetching the balance or the frozen status of a primary store should never abort.</td>
<td>Low</td>
<td>The function balance returns the balance of the store, if the store exists, otherwise it returns 0. The function is_frozen returns the frozen flag of the fungible store, if the store exists, otherwise it returns false.</td>
<td>Audited that the balance function returns the balance of the FungibleStore. Audited that the is_frozen function returns the frozen status of the FungibleStore resource. Audited that it never aborts.</td>
</tr>

<tr>
<td>5</td>
<td>The ability to withdraw, deposit, transfer, mint and burn should only be available for assets with primary store support.</td>
<td>Medium</td>
<td>The primary store is fetched before performing either of withdraw, deposit, transfer, mint, burn operation. If the FungibleStore resource doesn't exist the operation will fail.</td>
<td>Audited that it aborts if the primary store FungibleStore doesn't exist.</td>
</tr>

<tr>
<td>6</td>
<td>The action of depositing a fungible asset of the same type as the store should never fail if the store is not frozen.</td>
<td>Medium</td>
<td>The function deposit fetches the owner's store, if it doesn't exist it will be created, and then deposits the fungible asset to it. The function deposit_with_ref fetches the owner's store, if it doesn't exist it will be created, and then deposit the fungible asset via the fungible_asset::deposit_with_ref function. Depositing fails if the metadata of the FungibleStore and FungibleAsset differs.</td>
<td>Audited that it aborts if the store is frozen (deposit). Audited that the balance of the store is increased by the deposit amount (deposit, deposit_with_ref). Audited that it aborts if the metadata of the store and the asset differs (deposit, deposit_with_ref).</td>
</tr>

<tr>
<td>7</td>
<td>Withdrawing should only be allowed to the owner of an existing store with sufficient balance.</td>
<td>Critical</td>
<td>The withdraw function fetches the owner's store via the primary_store function and then calls fungible_asset::withdraw which validates the owner of the store, checks the frozen status and the balance of the store. The withdraw_with_ref function fetches the store of the owner via primary_store function and calls the fungible_asset::withdraw_with_ref which validates transfer_ref's metadata with the withdrawing stores metadata, and the balance of the store.</td>
<td>Audited that it aborts if the owner doesn't own the store (withdraw). Audited that it aborts if the store is frozen (withdraw). Audited that it aborts if the transfer ref's metadata doesn't match the withdrawing store's metadata (withdraw_with_ref). Audited that it aborts if the store doesn't have sufficient balance. Audited that the store is not burned. Audited that the balance of the store is decreased by the amount withdrawn.</td>
</tr>

<tr>
<td>8</td>
<td>Only the fungible store owner is allowed to unburn a burned store.</td>
<td>High</td>
<td>The function may_be_unburn checks if the store is burned and then proceeds to call object::unburn which ensures that the owner of the object matches the address of the signer.</td>
<td>Audited that the store is unburned successfully.</td>
</tr>

<tr>
<td>9</td>
<td>Only the owner of a primary store can transfer its balance to any recipient's primary store.</td>
<td>High</td>
<td>The function transfer fetches sender and recipient's primary stores, if the sender's store is burned it unburns the store and calls the fungile_asset::transfer to proceed with the transfer, which first withdraws the assets from the sender's store and then deposits to the recipient's store. The function transfer_with_ref fetches the sender's and recipient's stores and calls the fungible_asset::transfer_with_ref function which withdraws the asset with the ref from the sender and deposits the asset to the recipient with the ref.</td>
<td>Audited the deposit and withdraw (transfer). Audited the deposit_with_ref and withdraw_with_ref (transfer_with_ref). Audited that the store balance of the sender is decreased by the specified amount and its added to the recipients store. (transfer, transfer_with_ref) Audited that the sender's store is not burned (transfer).</td>
</tr>

<tr>
<td>10</td>
<td>Minting an amount of assets to an unfrozen store is only allowed with a valid mint reference.</td>
<td>High</td>
<td>The mint function fetches the primary store and calls the fungible_asset::mint_to, which mints with MintRef's metadata which internally validates the amount and the increases the total supply of the asset. And the minted asset is deposited to the provided store by validating that the store is unfrozen and the store's metadata is the same as the depositing asset's metadata.</td>
<td>Audited that it aborts if the amount is equal to 0. Audited that it aborts if the store is frozen. Audited that it aborts if the mint_ref's metadata is not the same as the store's metadata. Audited that the asset's total supply is increased by the amount minted. Audited that the balance of the store is increased by the minted amount.</td>
</tr>

<tr>
<td>11</td>
<td>Burning an amount of assets from an existing unfrozen store is only allowed with a valid burn reference.</td>
<td>High</td>
<td>The burn function fetches the primary store and calls the fungible_asset::burn_from function which withdraws the amount from the store while enforcing that the store has enough balance and burns the withdrawn asset after validating the asset's metadata and the BurnRef's metadata followed by decreasing the supply of the asset.</td>
<td>Audited that it aborts if the metadata of the store is not same as the BurnRef's metadata. Audited that it aborts if the burning amount is 0. Audited that it aborts if the store doesn't have enough balance. Audited that it aborts if the asset's metadata is not same as the BurnRef's metadata. Audited that the total supply of the asset is decreased. Audited that the store's balance is reduced by the amount burned.</td>
</tr>

<tr>
<td>12</td>
<td>Setting the frozen flag of a store is only allowed with a valid reference.</td>
<td>High</td>
<td>The function set_frozen_flag fetches the primary store and calls fungible_asset::set_frozen_flag which validates the TransferRef's metadata with the store's metadata and then updates the frozen flag.</td>
<td>Audited that it aborts if the store's metadata is not same as the TransferRef's metadata. Audited that the status of the frozen flag is updated correctly.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_primary_fungible_store_spec_primary_store_exists"></a>


<pre><code><b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_spec_primary_store_exists">spec_primary_store_exists</a>&lt;T: key&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, metadata: Object&lt;T&gt;): bool {
   <a href="fungible_asset.md#0x1_fungible_asset_store_exists">fungible_asset::store_exists</a>(<a href="primary_fungible_store.md#0x1_primary_fungible_store_spec_primary_store_address">spec_primary_store_address</a>(<a href="account.md#0x1_account">account</a>, metadata))
}
</code></pre>




<a id="0x1_primary_fungible_store_spec_primary_store_address"></a>


<pre><code><b>fun</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store_spec_primary_store_address">spec_primary_store_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
   <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);
   <a href="object.md#0x1_object_spec_create_user_derived_object_address">object::spec_create_user_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
