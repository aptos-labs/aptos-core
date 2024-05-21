
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
-  [Function `balance`](#0x1_primary_fungible_store_balance)
-  [Function `is_balance_at_least`](#0x1_primary_fungible_store_is_balance_at_least)
-  [Function `is_frozen`](#0x1_primary_fungible_store_is_frozen)
-  [Function `withdraw`](#0x1_primary_fungible_store_withdraw)
-  [Function `deposit`](#0x1_primary_fungible_store_deposit)
-  [Function `force_deposit`](#0x1_primary_fungible_store_force_deposit)
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


<pre><code>use 0x1::dispatchable_fungible_asset;<br/>use 0x1::features;<br/>use 0x1::fungible_asset;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/></code></pre>



<a id="0x1_primary_fungible_store_DeriveRefPod"></a>

## Resource `DeriveRefPod`

A resource that holds the derive ref for the fungible asset metadata object. This is used to create primary
stores for users with deterministic addresses so that users can easily deposit/withdraw/transfer fungible
assets.


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::object::ObjectGroup])]<br/>struct DeriveRefPod has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata_derive_ref: object::DeriveRef</code>
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


<pre><code>public fun create_primary_store_enabled_fungible_asset(constructor_ref: &amp;object::ConstructorRef, maximum_supply: option::Option&lt;u128&gt;, name: string::String, symbol: string::String, decimals: u8, icon_uri: string::String, project_uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_primary_store_enabled_fungible_asset(<br/>    constructor_ref: &amp;ConstructorRef,<br/>    maximum_supply: Option&lt;u128&gt;,<br/>    name: String,<br/>    symbol: String,<br/>    decimals: u8,<br/>    icon_uri: String,<br/>    project_uri: String,<br/>) &#123;<br/>    fungible_asset::add_fungibility(<br/>        constructor_ref,<br/>        maximum_supply,<br/>        name,<br/>        symbol,<br/>        decimals,<br/>        icon_uri,<br/>        project_uri,<br/>    );<br/>    let metadata_obj &#61; &amp;object::generate_signer(constructor_ref);<br/>    move_to(metadata_obj, DeriveRefPod &#123;<br/>        metadata_derive_ref: object::generate_derive_ref(constructor_ref),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_ensure_primary_store_exists"></a>

## Function `ensure_primary_store_exists`

Ensure that the primary store object for the given address exists. If it doesn't, create it.


<pre><code>public fun ensure_primary_store_exists&lt;T: key&gt;(owner: address, metadata: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::FungibleStore&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun ensure_primary_store_exists&lt;T: key&gt;(<br/>    owner: address,<br/>    metadata: Object&lt;T&gt;,<br/>): Object&lt;FungibleStore&gt; acquires DeriveRefPod &#123;<br/>    if (!primary_store_exists(owner, metadata)) &#123;<br/>        create_primary_store(owner, metadata)<br/>    &#125; else &#123;<br/>        primary_store(owner, metadata)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_create_primary_store"></a>

## Function `create_primary_store`

Create a primary store object to hold fungible asset for the given address.


<pre><code>public fun create_primary_store&lt;T: key&gt;(owner_addr: address, metadata: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::FungibleStore&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_primary_store&lt;T: key&gt;(<br/>    owner_addr: address,<br/>    metadata: Object&lt;T&gt;,<br/>): Object&lt;FungibleStore&gt; acquires DeriveRefPod &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;metadata);<br/>    object::address_to_object&lt;Metadata&gt;(metadata_addr);<br/>    let derive_ref &#61; &amp;borrow_global&lt;DeriveRefPod&gt;(metadata_addr).metadata_derive_ref;<br/>    let constructor_ref &#61; if (metadata_addr &#61;&#61; @aptos_fungible_asset &amp;&amp; features::primary_apt_fungible_store_at_user_address_enabled(<br/>    )) &#123;<br/>        &amp;object::create_sticky_object_at_address(owner_addr, owner_addr)<br/>    &#125; else &#123;<br/>        &amp;object::create_user_derived_object(owner_addr, derive_ref)<br/>    &#125;;<br/>    // Disable ungated transfer as deterministic stores shouldn&apos;t be transferrable.<br/>    let transfer_ref &#61; &amp;object::generate_transfer_ref(constructor_ref);<br/>    object::disable_ungated_transfer(transfer_ref);<br/><br/>    fungible_asset::create_store(constructor_ref, metadata)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_address"></a>

## Function `primary_store_address`

Get the address of the primary store for the given account.


<pre><code>&#35;[view]<br/>public fun primary_store_address&lt;T: key&gt;(owner: address, metadata: object::Object&lt;T&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun primary_store_address&lt;T: key&gt;(owner: address, metadata: Object&lt;T&gt;): address &#123;<br/>    let metadata_addr &#61; object::object_address(&amp;metadata);<br/>    if (metadata_addr &#61;&#61; @aptos_fungible_asset &amp;&amp; features::primary_apt_fungible_store_at_user_address_enabled()) &#123;<br/>        owner<br/>    &#125; else &#123;<br/>        object::create_user_derived_object_address(owner, metadata_addr)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store"></a>

## Function `primary_store`

Get the primary store object for the given account.


<pre><code>&#35;[view]<br/>public fun primary_store&lt;T: key&gt;(owner: address, metadata: object::Object&lt;T&gt;): object::Object&lt;fungible_asset::FungibleStore&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun primary_store&lt;T: key&gt;(owner: address, metadata: Object&lt;T&gt;): Object&lt;FungibleStore&gt; &#123;<br/>    let store &#61; primary_store_address(owner, metadata);<br/>    object::address_to_object&lt;FungibleStore&gt;(store)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_primary_store_exists"></a>

## Function `primary_store_exists`

Return whether the given account's primary store exists.


<pre><code>&#35;[view]<br/>public fun primary_store_exists&lt;T: key&gt;(account: address, metadata: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun primary_store_exists&lt;T: key&gt;(account: address, metadata: Object&lt;T&gt;): bool &#123;<br/>    fungible_asset::store_exists(primary_store_address(account, metadata))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_balance"></a>

## Function `balance`

Get the balance of <code>account</code>'s primary store.


<pre><code>&#35;[view]<br/>public fun balance&lt;T: key&gt;(account: address, metadata: object::Object&lt;T&gt;): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance&lt;T: key&gt;(account: address, metadata: Object&lt;T&gt;): u64 &#123;<br/>    if (primary_store_exists(account, metadata)) &#123;<br/>        fungible_asset::balance(primary_store(account, metadata))<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_is_balance_at_least"></a>

## Function `is_balance_at_least`



<pre><code>&#35;[view]<br/>public fun is_balance_at_least&lt;T: key&gt;(account: address, metadata: object::Object&lt;T&gt;, amount: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_balance_at_least&lt;T: key&gt;(account: address, metadata: Object&lt;T&gt;, amount: u64): bool &#123;<br/>    if (primary_store_exists(account, metadata)) &#123;<br/>        fungible_asset::is_balance_at_least(primary_store(account, metadata), amount)<br/>    &#125; else &#123;<br/>        amount &#61;&#61; 0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_is_frozen"></a>

## Function `is_frozen`

Return whether the given account's primary store is frozen.


<pre><code>&#35;[view]<br/>public fun is_frozen&lt;T: key&gt;(account: address, metadata: object::Object&lt;T&gt;): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_frozen&lt;T: key&gt;(account: address, metadata: Object&lt;T&gt;): bool &#123;<br/>    if (primary_store_exists(account, metadata)) &#123;<br/>        fungible_asset::is_frozen(primary_store(account, metadata))<br/>    &#125; else &#123;<br/>        false<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from the given account's primary store.


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, metadata: object::Object&lt;T&gt;, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw&lt;T: key&gt;(owner: &amp;signer, metadata: Object&lt;T&gt;, amount: u64): FungibleAsset acquires DeriveRefPod &#123;<br/>    let store &#61; ensure_primary_store_exists(signer::address_of(owner), metadata);<br/>    // Check if the store object has been burnt or not. If so, unburn it first.<br/>    may_be_unburn(owner, store);<br/>    dispatchable_fungible_asset::withdraw(owner, store, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_deposit"></a>

## Function `deposit`

Deposit fungible asset <code>fa</code> to the given account's primary store.


<pre><code>public fun deposit(owner: address, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit(owner: address, fa: FungibleAsset) acquires DeriveRefPod &#123;<br/>    let metadata &#61; fungible_asset::asset_metadata(&amp;fa);<br/>    let store &#61; ensure_primary_store_exists(owner, metadata);<br/>    dispatchable_fungible_asset::deposit(store, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_force_deposit"></a>

## Function `force_deposit`

Deposit fungible asset <code>fa</code> to the given account's primary store.


<pre><code>public(friend) fun force_deposit(owner: address, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun force_deposit(owner: address, fa: FungibleAsset) acquires DeriveRefPod &#123;<br/>    let metadata &#61; fungible_asset::asset_metadata(&amp;fa);<br/>    let store &#61; ensure_primary_store_exists(owner, metadata);<br/>    fungible_asset::deposit_internal(store, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset from sender's primary store to receiver's primary store.


<pre><code>public entry fun transfer&lt;T: key&gt;(sender: &amp;signer, metadata: object::Object&lt;T&gt;, recipient: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer&lt;T: key&gt;(<br/>    sender: &amp;signer,<br/>    metadata: Object&lt;T&gt;,<br/>    recipient: address,<br/>    amount: u64,<br/>) acquires DeriveRefPod &#123;<br/>    let sender_store &#61; ensure_primary_store_exists(signer::address_of(sender), metadata);<br/>    // Check if the sender store object has been burnt or not. If so, unburn it first.<br/>    may_be_unburn(sender, sender_store);<br/>    let recipient_store &#61; ensure_primary_store_exists(recipient, metadata);<br/>    dispatchable_fungible_asset::transfer(sender, sender_store, recipient_store, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer_assert_minimum_deposit"></a>

## Function `transfer_assert_minimum_deposit`

Transfer <code>amount</code> of fungible asset from sender's primary store to receiver's primary store.
Use the minimum deposit assertion api to make sure receipient will receive a minimum amount of fund.


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(sender: &amp;signer, metadata: object::Object&lt;T&gt;, recipient: address, amount: u64, expected: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_assert_minimum_deposit&lt;T: key&gt;(<br/>    sender: &amp;signer,<br/>    metadata: Object&lt;T&gt;,<br/>    recipient: address,<br/>    amount: u64,<br/>    expected: u64,<br/>) acquires DeriveRefPod &#123;<br/>    let sender_store &#61; ensure_primary_store_exists(signer::address_of(sender), metadata);<br/>    // Check if the sender store object has been burnt or not. If so, unburn it first.<br/>    may_be_unburn(sender, sender_store);<br/>    let recipient_store &#61; ensure_primary_store_exists(recipient, metadata);<br/>    dispatchable_fungible_asset::transfer_assert_minimum_deposit(<br/>        sender,<br/>        sender_store,<br/>        recipient_store,<br/>        amount,<br/>        expected<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_mint"></a>

## Function `mint`

Mint to the primary store of <code>owner</code>.


<pre><code>public fun mint(mint_ref: &amp;fungible_asset::MintRef, owner: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint(mint_ref: &amp;MintRef, owner: address, amount: u64) acquires DeriveRefPod &#123;<br/>    let primary_store &#61; ensure_primary_store_exists(owner, fungible_asset::mint_ref_metadata(mint_ref));<br/>    fungible_asset::mint_to(mint_ref, primary_store, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_burn"></a>

## Function `burn`

Burn from the primary store of <code>owner</code>.


<pre><code>public fun burn(burn_ref: &amp;fungible_asset::BurnRef, owner: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun burn(burn_ref: &amp;BurnRef, owner: address, amount: u64) &#123;<br/>    let primary_store &#61; primary_store(owner, fungible_asset::burn_ref_metadata(burn_ref));<br/>    fungible_asset::burn_from(burn_ref, primary_store, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_set_frozen_flag"></a>

## Function `set_frozen_flag`

Freeze/Unfreeze the primary store of <code>owner</code>.


<pre><code>public fun set_frozen_flag(transfer_ref: &amp;fungible_asset::TransferRef, owner: address, frozen: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_frozen_flag(transfer_ref: &amp;TransferRef, owner: address, frozen: bool) acquires DeriveRefPod &#123;<br/>    let primary_store &#61; ensure_primary_store_exists(owner, fungible_asset::transfer_ref_metadata(transfer_ref));<br/>    fungible_asset::set_frozen_flag(transfer_ref, primary_store, frozen);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw from the primary store of <code>owner</code> ignoring frozen flag.


<pre><code>public fun withdraw_with_ref(transfer_ref: &amp;fungible_asset::TransferRef, owner: address, amount: u64): fungible_asset::FungibleAsset<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_ref(transfer_ref: &amp;TransferRef, owner: address, amount: u64): FungibleAsset &#123;<br/>    let from_primary_store &#61; primary_store(owner, fungible_asset::transfer_ref_metadata(transfer_ref));<br/>    fungible_asset::withdraw_with_ref(transfer_ref, from_primary_store, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit from the primary store of <code>owner</code> ignoring frozen flag.


<pre><code>public fun deposit_with_ref(transfer_ref: &amp;fungible_asset::TransferRef, owner: address, fa: fungible_asset::FungibleAsset)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_with_ref(transfer_ref: &amp;TransferRef, owner: address, fa: FungibleAsset) acquires DeriveRefPod &#123;<br/>    let from_primary_store &#61; ensure_primary_store_exists(<br/>        owner,<br/>        fungible_asset::transfer_ref_metadata(transfer_ref)<br/>    );<br/>    fungible_asset::deposit_with_ref(transfer_ref, from_primary_store, fa);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>amount</code> of FA from the primary store of <code>from</code> to that of <code>to</code> ignoring frozen flag.


<pre><code>public fun transfer_with_ref(transfer_ref: &amp;fungible_asset::TransferRef, from: address, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_with_ref(<br/>    transfer_ref: &amp;TransferRef,<br/>    from: address,<br/>    to: address,<br/>    amount: u64<br/>) acquires DeriveRefPod &#123;<br/>    let from_primary_store &#61; primary_store(from, fungible_asset::transfer_ref_metadata(transfer_ref));<br/>    let to_primary_store &#61; ensure_primary_store_exists(to, fungible_asset::transfer_ref_metadata(transfer_ref));<br/>    fungible_asset::transfer_with_ref(transfer_ref, from_primary_store, to_primary_store, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_primary_fungible_store_may_be_unburn"></a>

## Function `may_be_unburn`



<pre><code>fun may_be_unburn(owner: &amp;signer, store: object::Object&lt;fungible_asset::FungibleStore&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun may_be_unburn(owner: &amp;signer, store: Object&lt;FungibleStore&gt;) &#123;<br/>    if (object::is_burnt(store)) &#123;<br/>        object::unburn(owner, store);<br/>    &#125;;<br/>&#125;<br/></code></pre>



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


<pre><code>pragma verify &#61; false;<br/></code></pre>




<a id="0x1_primary_fungible_store_spec_primary_store_exists"></a>


<pre><code>fun spec_primary_store_exists&lt;T: key&gt;(account: address, metadata: Object&lt;T&gt;): bool &#123;<br/>   fungible_asset::store_exists(spec_primary_store_address(account, metadata))<br/>&#125;<br/></code></pre>




<a id="0x1_primary_fungible_store_spec_primary_store_address"></a>


<pre><code>fun spec_primary_store_address&lt;T: key&gt;(owner: address, metadata: Object&lt;T&gt;): address &#123;<br/>   let metadata_addr &#61; object::object_address(metadata);<br/>   if (metadata_addr &#61;&#61; @aptos_fungible_asset &amp;&amp; features::spec_is_enabled(<br/>       features::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS<br/>   )) &#123;<br/>       owner<br/>   &#125; else &#123;<br/>       object::spec_create_user_derived_object_address(owner, metadata_addr)<br/>   &#125;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
