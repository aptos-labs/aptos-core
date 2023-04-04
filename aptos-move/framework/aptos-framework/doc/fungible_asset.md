
<a name="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code> object. The
metadata object can be any object that equipped with <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code> resource.


-  [Resource `FungibleAssetMetadata`](#0x1_fungible_asset_FungibleAssetMetadata)
-  [Resource `FungibleAssetWallet`](#0x1_fungible_asset_FungibleAssetWallet)
-  [Struct `FungibleAsset`](#0x1_fungible_asset_FungibleAsset)
-  [Struct `MintRef`](#0x1_fungible_asset_MintRef)
-  [Struct `TransferRef`](#0x1_fungible_asset_TransferRef)
-  [Struct `BurnRef`](#0x1_fungible_asset_BurnRef)
-  [Constants](#@Constants_0)
-  [Function `init_metadata`](#0x1_fungible_asset_init_metadata)
-  [Function `supply`](#0x1_fungible_asset_supply)
-  [Function `maximum`](#0x1_fungible_asset_maximum)
-  [Function `name`](#0x1_fungible_asset_name)
-  [Function `symbol`](#0x1_fungible_asset_symbol)
-  [Function `decimals`](#0x1_fungible_asset_decimals)
-  [Function `verify`](#0x1_fungible_asset_verify)
-  [Function `new_fungible_asset_wallet_object`](#0x1_fungible_asset_new_fungible_asset_wallet_object)
-  [Function `metadata_from_asset`](#0x1_fungible_asset_metadata_from_asset)
-  [Function `metadata_from_wallet`](#0x1_fungible_asset_metadata_from_wallet)
-  [Function `amount`](#0x1_fungible_asset_amount)
-  [Function `destory_fungible_asset_wallet`](#0x1_fungible_asset_destory_fungible_asset_wallet)
-  [Function `balance`](#0x1_fungible_asset_balance)
-  [Function `ungated_transfer_allowed`](#0x1_fungible_asset_ungated_transfer_allowed)
-  [Function `mint`](#0x1_fungible_asset_mint)
-  [Function `set_ungated_transfer`](#0x1_fungible_asset_set_ungated_transfer)
-  [Function `burn`](#0x1_fungible_asset_burn)
-  [Function `withdraw`](#0x1_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_fungible_asset_deposit)
-  [Function `transfer`](#0x1_fungible_asset_transfer)
-  [Function `withdraw_with_ref`](#0x1_fungible_asset_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_fungible_asset_deposit_with_ref)
-  [Function `transfer_with_ref`](#0x1_fungible_asset_transfer_with_ref)
-  [Function `mint_ref_metadata`](#0x1_fungible_asset_mint_ref_metadata)
-  [Function `transfer_ref_metadata`](#0x1_fungible_asset_transfer_ref_metadata)
-  [Function `burn_ref_metadata`](#0x1_fungible_asset_burn_ref_metadata)
-  [Function `extract`](#0x1_fungible_asset_extract)
-  [Function `increase_supply`](#0x1_fungible_asset_increase_supply)
-  [Function `decrease_supply`](#0x1_fungible_asset_decrease_supply)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_fungible_asset_FungibleAssetMetadata"></a>

## Resource `FungibleAssetMetadata`

Define the metadata required of an metadata to be fungible.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>supply: u64</code>
</dt>
<dd>
 The current supply.
</dd>
<dt>
<code>maximum: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>
 The maximum supply limit where <code><a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()</code> means no limit.
</dd>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the fungible metadata, i.e., "USDT".
</dd>
<dt>
<code>symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Symbol of the fungible metadata, usually a shorter version of the name.
 For example, Singapore Dollar is SGD.
</dd>
<dt>
<code>decimals: u8</code>
</dt>
<dd>
 Number of decimals used to get its user representation.
 For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
 be displayed to a user as <code>5.05</code> (<code>505 / 10 ** 2</code>).
</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAssetWallet"></a>

## Resource `FungibleAssetWallet`

The resource of an object holding the properties of fungible assets.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;</code>
</dt>
<dd>
 The address of the base metadata object.
</dd>
<dt>
<code>balance: u64</code>
</dt>
<dd>
 The balance of the fungible metadata.
</dd>
<dt>
<code>allow_ungated_transfer: bool</code>
</dt>
<dd>
 Fungible Assets transferring is a common operation, this allows for disabling and enabling
 transfers bypassing the use of a TransferRef.
</dd>
<dt>
<code>delete_ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a></code>
</dt>
<dd>
 The delete_ref of this object, used for cleanup.
</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

The transferable version of fungible metadata.
Note: it does not have <code>store</code> ability, only used in hot potato pattern.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;</code>
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

<a name="0x1_fungible_asset_MintRef"></a>

## Struct `MintRef`

Ref to mint.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_TransferRef"></a>

## Struct `TransferRef`

Ref to control the transfer.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_BurnRef"></a>

## Struct `BurnRef`

Ref to burn..


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient amount.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 5;
</code></pre>



<a name="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH"></a>

The burn ref and the the wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH">EBURN_REF_AND_WALLET_MISMATCH</a>: u64 = 3;
</code></pre>



<a name="0x1_fungible_asset_ECURRENT_SUPPLY_OVERFLOW"></a>

Current supply overflow


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ECURRENT_SUPPLY_OVERFLOW">ECURRENT_SUPPLY_OVERFLOW</a>: u64 = 8;
</code></pre>



<a name="0x1_fungible_asset_ECURRENT_SUPPLY_UNDERFLOW"></a>

Current supply underflow


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ECURRENT_SUPPLY_UNDERFLOW">ECURRENT_SUPPLY_UNDERFLOW</a>: u64 = 9;
</code></pre>



<a name="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH"></a>

FungibleAsset type and the wallet type mismatch.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH">EFUNGIBLE_ASSET_AND_WALLET_MISMATCH</a>: u64 = 6;
</code></pre>



<a name="0x1_fungible_asset_ENOT_OWNER"></a>

Not the owner,


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_OWNER">ENOT_OWNER</a>: u64 = 10;
</code></pre>



<a name="0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH"></a>

The transfer ref and the wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>: u64 = 2;
</code></pre>



<a name="0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED"></a>

The token account is still allow_ungated_transfer so cannot be deleted.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>: u64 = 4;
</code></pre>



<a name="0x1_fungible_asset_EZERO_AMOUNT"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 7;
</code></pre>



<a name="0x1_fungible_asset_init_metadata"></a>

## Function `init_metadata`

The initialization of an object with <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_init_metadata">init_metadata</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8): (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_init_metadata">init_metadata</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8,
): (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>) {
    <b>let</b> metadata_object_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>let</b> converted_maximum = <b>if</b> (maximum_supply == 0) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(maximum_supply)
    };
    <b>move_to</b>(&metadata_object_signer,
        <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
            supply: 0,
            maximum: converted_maximum,
            name,
            symbol,
            decimals,
        }
    );
    <b>let</b> metadata = <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt;(constructor_ref);
    (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a> { metadata }, <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a> { metadata }, <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a> { metadata })
}
</code></pre>



</details>

<a name="0x1_fungible_asset_supply"></a>

## Function `supply`

Get the current supply from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(metadata).supply
}
</code></pre>



</details>

<a name="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): Option&lt;u64&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(metadata).maximum
}
</code></pre>



</details>

<a name="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(metadata).name
}
</code></pre>



</details>

<a name="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(metadata).symbol
}
</code></pre>



</details>

<a name="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): u8 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(metadata).decimals
}
</code></pre>



</details>

<a name="0x1_fungible_asset_verify"></a>

## Function `verify`

Verify any object is equipped with <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code> and return the object.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_verify">verify</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_verify">verify</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    <b>let</b> addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(metadata);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt;(addr)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_new_fungible_asset_wallet_object"></a>

## Function `new_fungible_asset_wallet_object`

Create a new wallet object to hold fungible asset.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_new_fungible_asset_wallet_object">new_fungible_asset_wallet_object</a>&lt;T: key&gt;(creator_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_new_fungible_asset_wallet_object">new_fungible_asset_wallet_object</a>&lt;T: key&gt;(
    creator_ref: &ConstructorRef,
    metadata: &Object&lt;T&gt;,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt; {
    <b>let</b> wallet_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(creator_ref);
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">verify</a>(metadata);

    <b>move_to</b>(&wallet_signer, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
        metadata,
        balance: 0,
        allow_ungated_transfer: <b>true</b>,
        delete_ref: <a href="object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(creator_ref)
    });
    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(creator_ref)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_metadata_from_asset"></a>

## Function `metadata_from_asset`

Return the underlying metadata object


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_asset">metadata_from_asset</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    fa.metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_metadata_from_wallet"></a>

## Function `metadata_from_wallet`

Return the underlying metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    borrow_fungible_asset(wallet).metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_amount"></a>

## Function `amount`

Return <code>amount</code> inside.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): u64 {
    fa.amount
}
</code></pre>



</details>

<a name="0x1_fungible_asset_destory_fungible_asset_wallet"></a>

## Function `destory_fungible_asset_wallet`

Destroy <code>wallet</code> object.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destory_fungible_asset_wallet">destory_fungible_asset_wallet</a>(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destory_fungible_asset_wallet">destory_fungible_asset_wallet</a>(
    wallet: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
        metadata: _,
        balance: _,
        allow_ungated_transfer: _,
        delete_ref
    } = <b>move_from</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet));
    <a href="object.md#0x1_object_delete">object::delete</a>(delete_ref);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of <code>wallet</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>(wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>(wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    borrow_fungible_asset(wallet).balance
}
</code></pre>



</details>

<a name="0x1_fungible_asset_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Return <code>allowed_ungated_transfer</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    borrow_fungible_asset(wallet).allow_ungated_transfer
}
</code></pre>



</details>

<a name="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the <code>amount</code> of fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> metadata = <a href="fungible_asset.md#0x1_fungible_asset_verify">verify</a>(&ref.metadata);
    <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>(&ref.metadata, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Enable/disable the direct transfer of fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    allow: bool,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>assert</b>!(
        &ref.metadata == &<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>)
    );
    borrow_fungible_asset_mut(wallet).allow_ungated_transfer = allow;
}
</code></pre>



</details>

<a name="0x1_fungible_asset_burn"></a>

## Function `burn`

Burn the <code>amount</code> of fungible metadata from <code><a href="account.md#0x1_account">account</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>,
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    amount: u64
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>assert</b>!(
        &ref.metadata == &<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH">EBURN_REF_AND_WALLET_MISMATCH</a>)
    );
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount,
    } = <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(wallet, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>(&metadata, amount);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdarw <code>amount</code> of fungible asset from <code>wallet</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    amount: u64
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    assert_owner(<a href="account.md#0x1_account">account</a>, wallet);
    <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(wallet, amount)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of fungible asset to <code>wallet</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount } = fa;
    // ensure merging the same <a href="coin.md#0x1_coin">coin</a>
    <b>let</b> wallet = borrow_fungible_asset_mut(wallet);
    <b>assert</b>!(wallet.allow_ungated_transfer, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <b>assert</b>!(wallet.metadata == metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH">EFUNGIBLE_ASSET_AND_WALLET_MISMATCH</a>));
    wallet.balance = wallet.balance + amount;
}
</code></pre>



</details>

<a name="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible metadata of <code>metadata</code> to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, amount: u64, from: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, <b>to</b>: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    amount: u64,
    from: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    <b>to</b>: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(<a href="account.md#0x1_account">account</a>, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdarw <code>amount</code> of fungible metadata from <code><a href="account.md#0x1_account">account</a></code> ignoring <code>allow_ungated_transfer</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    amount: u64
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>assert</b>!(
        &ref.metadata == &<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>)
    );
    <b>let</b> ungated_transfer_allowed = <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(ref, wallet, <b>true</b>);
    };
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(wallet, amount);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(ref, wallet, <b>false</b>);
    };
    fa
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit fungible asset into <code><a href="account.md#0x1_account">account</a></code> ignoring <code>allow_ungated_transfer</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>assert</b>!(
        &ref.metadata == &<a href="fungible_asset.md#0x1_fungible_asset_metadata_from_wallet">metadata_from_wallet</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>)
    );
    <b>let</b> ungated_transfer_allowed = <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(ref, wallet, <b>true</b>);
    };
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(wallet, fa);
    <b>if</b> (!ungated_transfer_allowed) {
        <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>(ref, wallet, <b>false</b>);
    };
}
</code></pre>



</details>

<a name="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>ammount</code> of  fungible metadata with <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code> even ungated transfer is disabled.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, <b>to</b>: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>(
    transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    from: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    <b>to</b>: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(transfer_ref, <b>to</b>, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_mint_ref_metadata"></a>

## Function `mint_ref_metadata`

Get the underlying metadata object from <code><a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_ref_metadata">mint_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_transfer_ref_metadata"></a>

## Function `transfer_ref_metadata`

Get the underlying metadata object from <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_ref_metadata">transfer_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_burn_ref_metadata"></a>

## Function `burn_ref_metadata`

Get the underlying metadata object from <code><a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn_ref_metadata">burn_ref_metadata</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    ref.metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract <code>amount</code> of fungible asset from <code>wallet</code>.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(wallet: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(
    wallet: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> wallet = borrow_fungible_asset_mut(wallet);
    <b>assert</b>!(wallet.allow_ungated_transfer, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <b>assert</b>!(wallet.balance &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    wallet.balance = wallet.balance - amount;
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata: wallet.metadata,
        amount
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible metadata by minting.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EZERO_AMOUNT">EZERO_AMOUNT</a>));
    <b>let</b> fungible_metadata = borrow_fungible_metadata_mut(metadata);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&fungible_metadata.maximum)) {
        <b>let</b> max = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&fungible_metadata.maximum);
        <b>assert</b>!(max - fungible_metadata.supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ECURRENT_SUPPLY_OVERFLOW">ECURRENT_SUPPLY_OVERFLOW</a>))
    };
    fungible_metadata.supply = fungible_metadata.supply + amount;
}
</code></pre>



</details>

<a name="0x1_fungible_asset_decrease_supply"></a>

## Function `decrease_supply`

Decrease the supply of a fungible metadata by burning.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>&lt;T: key&gt;(metadata: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EZERO_AMOUNT">EZERO_AMOUNT</a>));
    <b>let</b> fungible_metadata = borrow_fungible_metadata_mut(metadata);
    <b>assert</b>!(fungible_metadata.supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ECURRENT_SUPPLY_UNDERFLOW">ECURRENT_SUPPLY_UNDERFLOW</a>));
    fungible_metadata.supply = fungible_metadata.supply - amount;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
