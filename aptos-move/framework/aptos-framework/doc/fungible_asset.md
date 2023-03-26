
<a name="0x1_fungible_asset"></a>

# Module `0x1::fungible_asset`

This defines the fungible asset module that can issue fungible asset of any <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code> object. The
metadata object can be any object that equipped with <code><a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a></code> resource.


-  [Resource `FungibleAssetMetadata`](#0x1_fungible_asset_FungibleAssetMetadata)
-  [Resource `FungibleAssetWallet`](#0x1_fungible_asset_FungibleAssetWallet)
-  [Resource `FungibleAssetWalletEvents`](#0x1_fungible_asset_FungibleAssetWalletEvents)
-  [Struct `FungibleAsset`](#0x1_fungible_asset_FungibleAsset)
-  [Struct `MintRef`](#0x1_fungible_asset_MintRef)
-  [Struct `TransferRef`](#0x1_fungible_asset_TransferRef)
-  [Struct `BurnRef`](#0x1_fungible_asset_BurnRef)
-  [Struct `MintEvent`](#0x1_fungible_asset_MintEvent)
-  [Struct `BurnEvent`](#0x1_fungible_asset_BurnEvent)
-  [Struct `DepositEvent`](#0x1_fungible_asset_DepositEvent)
-  [Struct `WithdrawEvent`](#0x1_fungible_asset_WithdrawEvent)
-  [Struct `SetUngatedTransferEvent`](#0x1_fungible_asset_SetUngatedTransferEvent)
-  [Constants](#@Constants_0)
-  [Function `make_object_fungible`](#0x1_fungible_asset_make_object_fungible)
-  [Function `supply`](#0x1_fungible_asset_supply)
-  [Function `maximum`](#0x1_fungible_asset_maximum)
-  [Function `name`](#0x1_fungible_asset_name)
-  [Function `symbol`](#0x1_fungible_asset_symbol)
-  [Function `decimals`](#0x1_fungible_asset_decimals)
-  [Function `deterministic_wallet_address`](#0x1_fungible_asset_deterministic_wallet_address)
-  [Function `wallet_exists`](#0x1_fungible_asset_wallet_exists)
-  [Function `metadata_from_asset`](#0x1_fungible_asset_metadata_from_asset)
-  [Function `wallet_metadata`](#0x1_fungible_asset_wallet_metadata)
-  [Function `amount`](#0x1_fungible_asset_amount)
-  [Function `balance`](#0x1_fungible_asset_balance)
-  [Function `ungated_transfer_allowed`](#0x1_fungible_asset_ungated_transfer_allowed)
-  [Function `asset_metadata`](#0x1_fungible_asset_asset_metadata)
-  [Function `mint_ref_metadata`](#0x1_fungible_asset_mint_ref_metadata)
-  [Function `transfer_ref_metadata`](#0x1_fungible_asset_transfer_ref_metadata)
-  [Function `burn_ref_metadata`](#0x1_fungible_asset_burn_ref_metadata)
-  [Function `transfer`](#0x1_fungible_asset_transfer)
-  [Function `create_deterministic_wallet`](#0x1_fungible_asset_create_deterministic_wallet)
-  [Function `initialize_arbitrary_wallet`](#0x1_fungible_asset_initialize_arbitrary_wallet)
-  [Function `withdraw`](#0x1_fungible_asset_withdraw)
-  [Function `deposit`](#0x1_fungible_asset_deposit)
-  [Function `mint`](#0x1_fungible_asset_mint)
-  [Function `mint_to`](#0x1_fungible_asset_mint_to)
-  [Function `set_ungated_transfer`](#0x1_fungible_asset_set_ungated_transfer)
-  [Function `burn`](#0x1_fungible_asset_burn)
-  [Function `withdraw_with_ref`](#0x1_fungible_asset_withdraw_with_ref)
-  [Function `deposit_with_ref`](#0x1_fungible_asset_deposit_with_ref)
-  [Function `transfer_with_ref`](#0x1_fungible_asset_transfer_with_ref)
-  [Function `extract`](#0x1_fungible_asset_extract)
-  [Function `merge`](#0x1_fungible_asset_merge)
-  [Function `destroy_zero`](#0x1_fungible_asset_destroy_zero)
-  [Function `deposit_internal`](#0x1_fungible_asset_deposit_internal)
-  [Function `withdraw_internal`](#0x1_fungible_asset_withdraw_internal)
-  [Function `increase_supply`](#0x1_fungible_asset_increase_supply)
-  [Function `decrease_supply`](#0x1_fungible_asset_decrease_supply)


<pre><code><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
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
 The current supply of the fungible asset.
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
 Number of decimals used for display purposes.
 For example, if <code>decimals</code> equals <code>2</code>, a balance of <code>505</code> coins should
 be displayed to a user as <code>5.05</code> (<code>505 / 10 ** 2</code>).
</dd>
<dt>
<code>derive_ref: <a href="object.md#0x1_object_DeriveRef">object::DeriveRef</a></code>
</dt>
<dd>
 The ref used to create wallet objects for users later.
</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAssetWallet"></a>

## Resource `FungibleAssetWallet`

The wallet object that holds fungible assets of a specific type associated with an account.


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
 Fungible Assets transferring is a common operation, this allows for freezing/unfreezing accounts.
</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAssetWalletEvents"></a>

## Resource `FungibleAssetWalletEvents`



<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>deposit_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">fungible_asset::DepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">fungible_asset::WithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>set_ungated_transfer_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_SetUngatedTransferEvent">fungible_asset::SetUngatedTransferEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_FungibleAsset"></a>

## Struct `FungibleAsset`

FungibleAsset can be passed into function for type safety and to guarantee a specific amount.
FungibleAsset cannot be stored directly and will have to be deposited back into a wallet.


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

MintRef can be used to mint the fungible asset into an account's wallet.


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

TransferRef can be used to allow or disallow the owner of fungible assets from transferring the asset
and allow the holder of TransferRef to transfer fungible assets from any account.


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

BurnRef can be used to burn fungible assets from a given holder account.


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

<a name="0x1_fungible_asset_MintEvent"></a>

## Struct `MintEvent`

Emitted when fungible assets are minted.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_MintEvent">MintEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_BurnEvent"></a>

## Struct `BurnEvent`

Emitted when fungible assets are burnt.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_BurnEvent">BurnEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_DepositEvent"></a>

## Struct `DepositEvent`

Emitted when fungible assets are deposited into a wallet.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">DepositEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Emitted when fungible assets are withdrawn from a wallet.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_fungible_asset_SetUngatedTransferEvent"></a>

## Struct `SetUngatedTransferEvent`

Emitted when a wallet's ungated (owner) transfer permission is updated.


<pre><code><b>struct</b> <a href="fungible_asset.md#0x1_fungible_asset_SetUngatedTransferEvent">SetUngatedTransferEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>transfer_allowed: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_asset_EINSUFFICIENT_BALANCE"></a>

Insufficient balance to withdraw or transfer.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 4;
</code></pre>



<a name="0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO"></a>

Cannot destroy non-empty fungible assets.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>: u64 = 12;
</code></pre>



<a name="0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH"></a>

Burn ref and wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH">EBURN_REF_AND_WALLET_MISMATCH</a>: u64 = 10;
</code></pre>



<a name="0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH"></a>

Fungible asset and wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH">EFUNGIBLE_ASSET_AND_WALLET_MISMATCH</a>: u64 = 11;
</code></pre>



<a name="0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED"></a>

The fungible asset's supply has exceeded maximum.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>: u64 = 5;
</code></pre>



<a name="0x1_fungible_asset_EMINT_REF_AND_WALLET_MISMATCH"></a>

The mint ref and the the wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EMINT_REF_AND_WALLET_MISMATCH">EMINT_REF_AND_WALLET_MISMATCH</a>: u64 = 7;
</code></pre>



<a name="0x1_fungible_asset_ENOT_WALLET_OWNER"></a>

Account is not the wallet's owner.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ENOT_WALLET_OWNER">ENOT_WALLET_OWNER</a>: u64 = 8;
</code></pre>



<a name="0x1_fungible_asset_ESUPPLY_UNDERFLOW"></a>

More tokens than remaining supply are being burnt.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>: u64 = 6;
</code></pre>



<a name="0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH"></a>

The transfer ref and the fungible asset do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>: u64 = 2;
</code></pre>



<a name="0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH"></a>

Transfer ref and wallet do not match.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>: u64 = 9;
</code></pre>



<a name="0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED"></a>

Account cannot transfer or receive fungible assets.


<pre><code><b>const</b> <a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>: u64 = 3;
</code></pre>



<a name="0x1_fungible_asset_make_object_fungible"></a>

## Function `make_object_fungible`

Make an existing object fungible by adding the FungibleAssetMetadata resource.
This returns the capabilities to mint, burn, and transfer.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_make_object_fungible">make_object_fungible</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8): (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_make_object_fungible">make_object_fungible</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8,
): (<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>, <a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>) {
    <b>let</b> metadata_object_signer = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>let</b> converted_maximum = <b>if</b> (maximum_supply == 0) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(maximum_supply)
    };
    <b>move_to</b>(metadata_object_signer,
        <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
            supply: 0,
            maximum: converted_maximum,
            name,
            symbol,
            decimals,
            derive_ref: <a href="object.md#0x1_object_generate_derive_ref">object::generate_derive_ref</a>(constructor_ref),
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


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_supply">supply</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(&metadata).supply
}
</code></pre>



</details>

<a name="0x1_fungible_asset_maximum"></a>

## Function `maximum`

Get the maximum supply from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_maximum">maximum</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): Option&lt;u64&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(&metadata).maximum
}
</code></pre>



</details>

<a name="0x1_fungible_asset_name"></a>

## Function `name`

Get the name of the fungible asset from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_name">name</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(&metadata).name
}
</code></pre>



</details>

<a name="0x1_fungible_asset_symbol"></a>

## Function `symbol`

Get the symbol of the fungible asset from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_symbol">symbol</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(&metadata).symbol
}
</code></pre>



</details>

<a name="0x1_fungible_asset_decimals"></a>

## Function `decimals`

Get the decimals from <code>metadata</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_decimals">decimals</a>&lt;T: key&gt;(metadata: Object&lt;T&gt;): u8 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    borrow_fungible_metadata(&metadata).decimals
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deterministic_wallet_address"></a>

## Function `deterministic_wallet_address`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deterministic_wallet_address">deterministic_wallet_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deterministic_wallet_address">deterministic_wallet_address</a>&lt;T: key&gt;(owner: <b>address</b>, metadata: Object&lt;T&gt;): <b>address</b> {
    <b>let</b> metadata_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&metadata);
    <a href="object.md#0x1_object_create_derived_object_address">object::create_derived_object_address</a>(owner, metadata_addr)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_wallet_exists"></a>

## Function `wallet_exists`

Return whether the provided address has a wallet initialized.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_wallet_exists">wallet_exists</a>(wallet: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_wallet_exists">wallet_exists</a>(wallet: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(wallet)
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

<a name="0x1_fungible_asset_wallet_metadata"></a>

## Function `wallet_metadata`

Return the underlying metadata object.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>&lt;T: key&gt;(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>&lt;T: key&gt;(wallet: Object&lt;T&gt;): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    borrow_wallet_resource(&wallet).metadata
}
</code></pre>



</details>

<a name="0x1_fungible_asset_amount"></a>

## Function `amount`

Return <code>amount</code> of a given fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_amount">amount</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): u64 {
    fa.amount
}
</code></pre>



</details>

<a name="0x1_fungible_asset_balance"></a>

## Function `balance`

Get the balance of a given wallet.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_balance">balance</a>&lt;T: key&gt;(wallet: Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    <b>if</b> (<a href="fungible_asset.md#0x1_fungible_asset_wallet_exists">wallet_exists</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet))) {
        borrow_wallet_resource(&wallet).balance
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_ungated_transfer_allowed"></a>

## Function `ungated_transfer_allowed`

Return whether a wallet can freely send or receive fungible assets.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>&lt;T: key&gt;(wallet: Object&lt;T&gt;): bool <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
    borrow_wallet_resource(&wallet).allow_ungated_transfer
}
</code></pre>



</details>

<a name="0x1_fungible_asset_asset_metadata"></a>

## Function `asset_metadata`



<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">fungible_asset::FungibleAssetMetadata</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_asset_metadata">asset_metadata</a>(fa: &<a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt; {
    fa.metadata
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

<a name="0x1_fungible_asset_transfer"></a>

## Function `transfer`

Transfer <code>amount</code> of fungible asset from <code>from_wallet</code>, which should be owned by <code>sender</code>, to <code>receiver</code>.
Note: it does not move the underlying object.


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer">transfer</a>&lt;T: key&gt;(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>(sender, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(<b>to</b>, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_create_deterministic_wallet"></a>

## Function `create_deterministic_wallet`

Create a new wallet object to hold fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_deterministic_wallet">create_deterministic_wallet</a>&lt;T: key&gt;(owner_addr: <b>address</b>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_create_deterministic_wallet">create_deterministic_wallet</a>&lt;T: key&gt;(
    owner_addr: <b>address</b>,
    metadata: Object&lt;T&gt;,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt; <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>let</b> owner = &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(owner_addr);
    <b>let</b> derive_ref = &borrow_fungible_metadata(&metadata).derive_ref;
    <b>let</b> constructor_ref = &<a href="object.md#0x1_object_create_derived_object">object::create_derived_object</a>(owner, derive_ref);
    <a href="fungible_asset.md#0x1_fungible_asset_initialize_arbitrary_wallet">initialize_arbitrary_wallet</a>(constructor_ref, metadata)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_initialize_arbitrary_wallet"></a>

## Function `initialize_arbitrary_wallet`

Allow an object to hold a wallet for fungible assets.
Applications can use this to create multiple wallets for isolating fungible assets for different purposes.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_initialize_arbitrary_wallet">initialize_arbitrary_wallet</a>&lt;T: key&gt;(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, metadata: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">fungible_asset::FungibleAssetWallet</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_initialize_arbitrary_wallet">initialize_arbitrary_wallet</a>&lt;T: key&gt;(
    constructor_ref: &ConstructorRef,
    metadata: Object&lt;T&gt;,
): Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt; {
    <b>let</b> wallet_obj = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>let</b> metadata = <a href="object.md#0x1_object_convert">object::convert</a>&lt;T, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>&gt;(metadata);
    <b>move_to</b>(wallet_obj, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a> {
        metadata,
        balance: 0,
        allow_ungated_transfer: <b>true</b>,
    });
    <b>move_to</b>(wallet_obj,
        <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
            deposit_events: <a href="object.md#0x1_object_new_event_handle">object::new_event_handle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">DepositEvent</a>&gt;(wallet_obj),
            withdraw_events: <a href="object.md#0x1_object_new_event_handle">object::new_event_handle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">WithdrawEvent</a>&gt;(wallet_obj),
            set_ungated_transfer_events: <a href="object.md#0x1_object_new_event_handle">object::new_event_handle</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_SetUngatedTransferEvent">SetUngatedTransferEvent</a>&gt;(wallet_obj),
        }
    );

    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(constructor_ref)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_withdraw"></a>

## Function `withdraw`

Withdraw <code>amount</code> of fungible asset from <code>wallet</code> by the owner.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw">withdraw</a>&lt;T: key&gt;(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    wallet: Object&lt;T&gt;,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(<a href="object.md#0x1_object_owns">object::owns</a>(wallet, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="fungible_asset.md#0x1_fungible_asset_ENOT_WALLET_OWNER">ENOT_WALLET_OWNER</a>));
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet), amount)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deposit"></a>

## Function `deposit`

Deposit <code>amount</code> of fungible asset to <code>wallet</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>&lt;T: key&gt;(
    wallet: Object&lt;T&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset_ungated_transfer_allowed">ungated_transfer_allowed</a>(wallet), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EUNGATED_TRANSFER_IS_NOT_ALLOWED">EUNGATED_TRANSFER_IS_NOT_ALLOWED</a>));
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(wallet, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_mint"></a>

## Function `mint`

Mint the specified <code>amount</code> of fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a> {
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> metadata = ref.metadata;
    <a href="fungible_asset.md#0x1_fungible_asset_increase_supply">increase_supply</a>(&metadata, amount);

    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_mint_to"></a>

## Function `mint_to`

Mint the specified <code>amount</code> of fungible asset to a destination wallet.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">fungible_asset::MintRef</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_mint_to">mint_to</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_MintRef">MintRef</a>,
    wallet: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_deposit">deposit</a>(wallet, <a href="fungible_asset.md#0x1_fungible_asset_mint">mint</a>(ref, amount));
}
</code></pre>



</details>

<a name="0x1_fungible_asset_set_ungated_transfer"></a>

## Function `set_ungated_transfer`

Enable/disable a wallet's ability to do direct transfers of fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, allow: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_set_ungated_transfer">set_ungated_transfer</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: Object&lt;T&gt;,
    allow: bool,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(
        ref.metadata == <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>),
    );
    <b>let</b> wallet_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet);
    <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(wallet_addr).allow_ungated_transfer = allow;

    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a>&gt;(wallet_addr);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> events.set_ungated_transfer_events, <a href="fungible_asset.md#0x1_fungible_asset_SetUngatedTransferEvent">SetUngatedTransferEvent</a> { transfer_allowed: allow });
}
</code></pre>



</details>

<a name="0x1_fungible_asset_burn"></a>

## Function `burn`

Burn the <code>amount</code> of fungible metadata from the given wallet.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">fungible_asset::BurnRef</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_burn">burn</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_BurnRef">BurnRef</a>,
    wallet: Object&lt;T&gt;,
    amount: u64
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetMetadata">FungibleAssetMetadata</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>let</b> metadata = ref.metadata;
    <b>assert</b>!(metadata == <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>(wallet), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EBURN_REF_AND_WALLET_MISMATCH">EBURN_REF_AND_WALLET_MISMATCH</a>));
    <b>let</b> wallet_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet);
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata,
        amount,
    } = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(wallet_addr, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_decrease_supply">decrease_supply</a>(&metadata, amount);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_withdraw_with_ref"></a>

## Function `withdraw_with_ref`

Withdraw <code>amount</code> of fungible metadata from <code>wallet</code> ignoring <code>allow_ungated_transfer</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: Object&lt;T&gt;,
    amount: u64
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(
        ref.metadata == <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>(wallet),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_WALLET_MISMATCH">ETRANSFER_REF_AND_WALLET_MISMATCH</a>),
    );
    <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(<a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet), amount)
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deposit_with_ref"></a>

## Function `deposit_with_ref`

Deposit fungible asset into <code>wallet</code> ignoring <code>allow_ungated_transfer</code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>&lt;T: key&gt;(
    ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    wallet: Object&lt;T&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(
        ref.metadata == fa.metadata,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH">ETRANSFER_REF_AND_FUNGIBLE_ASSET_MISMATCH</a>)
    );
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>(wallet, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_transfer_with_ref"></a>

## Function `transfer_with_ref`

Transfer <code>ammount</code> of  fungible metadata with <code><a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a></code> even ungated transfer is disabled.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">fungible_asset::TransferRef</a>, from: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, <b>to</b>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_transfer_with_ref">transfer_with_ref</a>&lt;T: key&gt;(
    transfer_ref: &<a href="fungible_asset.md#0x1_fungible_asset_TransferRef">TransferRef</a>,
    from: Object&lt;T&gt;,
    <b>to</b>: Object&lt;T&gt;,
    amount: u64,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>let</b> fa = <a href="fungible_asset.md#0x1_fungible_asset_withdraw_with_ref">withdraw_with_ref</a>(transfer_ref, from, amount);
    <a href="fungible_asset.md#0x1_fungible_asset_deposit_with_ref">deposit_with_ref</a>(transfer_ref, <b>to</b>, fa);
}
</code></pre>



</details>

<a name="0x1_fungible_asset_extract"></a>

## Function `extract`

Extract a given amount from the given fungible asset and return a new one.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_extract">extract</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
    <b>assert</b>!(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount = <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.amount - amount;
    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> {
        metadata: <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>.metadata,
        amount,
    }
}
</code></pre>



</details>

<a name="0x1_fungible_asset_merge"></a>

## Function `merge`

"Merges" the two given fungible assets. The coin passed in as <code>dst_fungible_asset</code> will have a value equal
to the sum of the two (<code>dst_fungible_asset</code> and <code>src_fungible_asset</code>).


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_merge">merge</a>(dst_fungible_asset: &<b>mut</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>, src_fungible_asset: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata: _, amount } = src_fungible_asset;
    dst_fungible_asset.amount = dst_fungible_asset.amount + amount;
}
</code></pre>



</details>

<a name="0x1_fungible_asset_destroy_zero"></a>

## Function `destroy_zero`

Destroy an empty fungible asset.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_destroy_zero">destroy_zero</a>(<a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>) {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { amount, metadata: _ } = <a href="fungible_asset.md#0x1_fungible_asset">fungible_asset</a>;
    <b>assert</b>!(amount == 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_IS_NOT_ZERO">EAMOUNT_IS_NOT_ZERO</a>));
}
</code></pre>



</details>

<a name="0x1_fungible_asset_deposit_internal"></a>

## Function `deposit_internal`



<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>&lt;T: key&gt;(wallet: <a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">deposit_internal</a>&lt;T: key&gt;(
    wallet: Object&lt;T&gt;,
    fa: <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a>,
) <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>let</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount } = fa;
    <b>let</b> wallet_metadata = <a href="fungible_asset.md#0x1_fungible_asset_wallet_metadata">wallet_metadata</a>(wallet);
    <b>assert</b>!(metadata == wallet_metadata, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EFUNGIBLE_ASSET_AND_WALLET_MISMATCH">EFUNGIBLE_ASSET_AND_WALLET_MISMATCH</a>));
    <b>let</b> wallet_addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(&wallet);
    <b>let</b> wallet = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(wallet_addr);
    wallet.balance = wallet.balance + amount;

    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a>&gt;(wallet_addr);
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> events.deposit_events, <a href="fungible_asset.md#0x1_fungible_asset_DepositEvent">DepositEvent</a> { amount });
}
</code></pre>



</details>

<a name="0x1_fungible_asset_withdraw_internal"></a>

## Function `withdraw_internal`

Extract <code>amount</code> of fungible asset from <code>wallet</code>.


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(wallet_addr: <b>address</b>, amount: u64): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">fungible_asset::FungibleAsset</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">withdraw_internal</a>(
    wallet_addr: <b>address</b>,
    amount: u64,
): <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> <b>acquires</b> <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>, <a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> wallet = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWallet">FungibleAssetWallet</a>&gt;(wallet_addr);
    <b>assert</b>!(wallet.balance &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    wallet.balance = wallet.balance - amount;

    <b>let</b> events = <b>borrow_global_mut</b>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleAssetWalletEvents">FungibleAssetWalletEvents</a>&gt;(wallet_addr);
    <b>let</b> metadata = wallet.metadata;
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> events.withdraw_events, <a href="fungible_asset.md#0x1_fungible_asset_WithdrawEvent">WithdrawEvent</a> { amount });

    <a href="fungible_asset.md#0x1_fungible_asset_FungibleAsset">FungibleAsset</a> { metadata, amount }
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
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> fungible_metadata = borrow_fungible_metadata_mut(metadata);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&fungible_metadata.maximum)) {
        <b>let</b> max = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&fungible_metadata.maximum);
        <b>assert</b>!(max - fungible_metadata.supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EMAX_SUPPLY_EXCEEDED">EMAX_SUPPLY_EXCEEDED</a>))
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
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_EAMOUNT_CANNOT_BE_ZERO">EAMOUNT_CANNOT_BE_ZERO</a>));
    <b>let</b> fungible_metadata = borrow_fungible_metadata_mut(metadata);
    <b>assert</b>!(fungible_metadata.supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_asset.md#0x1_fungible_asset_ESUPPLY_UNDERFLOW">ESUPPLY_UNDERFLOW</a>));
    fungible_metadata.supply = fungible_metadata.supply - amount;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/guides/move-guides/book/SUMMARY
