
<a id="0x3_token"></a>

# Module `0x3::token`

This module provides the foundation for Tokens.<br/> Checkout our developer doc on our token standard https://aptos.dev/standards


-  [Struct `Token`](#0x3_token_Token)
-  [Struct `TokenId`](#0x3_token_TokenId)
-  [Struct `TokenDataId`](#0x3_token_TokenDataId)
-  [Struct `TokenData`](#0x3_token_TokenData)
-  [Struct `Royalty`](#0x3_token_Royalty)
-  [Struct `TokenMutabilityConfig`](#0x3_token_TokenMutabilityConfig)
-  [Resource `TokenStore`](#0x3_token_TokenStore)
-  [Struct `CollectionMutabilityConfig`](#0x3_token_CollectionMutabilityConfig)
-  [Resource `Collections`](#0x3_token_Collections)
-  [Struct `CollectionData`](#0x3_token_CollectionData)
-  [Struct `WithdrawCapability`](#0x3_token_WithdrawCapability)
-  [Struct `DepositEvent`](#0x3_token_DepositEvent)
-  [Struct `Deposit`](#0x3_token_Deposit)
-  [Struct `WithdrawEvent`](#0x3_token_WithdrawEvent)
-  [Struct `Withdraw`](#0x3_token_Withdraw)
-  [Struct `CreateTokenDataEvent`](#0x3_token_CreateTokenDataEvent)
-  [Struct `CreateTokenData`](#0x3_token_CreateTokenData)
-  [Struct `MintTokenEvent`](#0x3_token_MintTokenEvent)
-  [Struct `MintToken`](#0x3_token_MintToken)
-  [Struct `BurnTokenEvent`](#0x3_token_BurnTokenEvent)
-  [Struct `BurnToken`](#0x3_token_BurnToken)
-  [Struct `MutateTokenPropertyMapEvent`](#0x3_token_MutateTokenPropertyMapEvent)
-  [Struct `MutateTokenPropertyMap`](#0x3_token_MutateTokenPropertyMap)
-  [Struct `CreateCollectionEvent`](#0x3_token_CreateCollectionEvent)
-  [Struct `CreateCollection`](#0x3_token_CreateCollection)
-  [Constants](#@Constants_0)
-  [Function `create_collection_script`](#0x3_token_create_collection_script)
-  [Function `create_token_script`](#0x3_token_create_token_script)
-  [Function `mint_script`](#0x3_token_mint_script)
-  [Function `mutate_token_properties`](#0x3_token_mutate_token_properties)
-  [Function `direct_transfer_script`](#0x3_token_direct_transfer_script)
-  [Function `opt_in_direct_transfer`](#0x3_token_opt_in_direct_transfer)
-  [Function `transfer_with_opt_in`](#0x3_token_transfer_with_opt_in)
-  [Function `burn_by_creator`](#0x3_token_burn_by_creator)
-  [Function `burn`](#0x3_token_burn)
-  [Function `mutate_collection_description`](#0x3_token_mutate_collection_description)
-  [Function `mutate_collection_uri`](#0x3_token_mutate_collection_uri)
-  [Function `mutate_collection_maximum`](#0x3_token_mutate_collection_maximum)
-  [Function `mutate_tokendata_maximum`](#0x3_token_mutate_tokendata_maximum)
-  [Function `mutate_tokendata_uri`](#0x3_token_mutate_tokendata_uri)
-  [Function `mutate_tokendata_royalty`](#0x3_token_mutate_tokendata_royalty)
-  [Function `mutate_tokendata_description`](#0x3_token_mutate_tokendata_description)
-  [Function `mutate_tokendata_property`](#0x3_token_mutate_tokendata_property)
-  [Function `mutate_one_token`](#0x3_token_mutate_one_token)
-  [Function `create_royalty`](#0x3_token_create_royalty)
-  [Function `deposit_token`](#0x3_token_deposit_token)
-  [Function `direct_deposit_with_opt_in`](#0x3_token_direct_deposit_with_opt_in)
-  [Function `direct_transfer`](#0x3_token_direct_transfer)
-  [Function `initialize_token_store`](#0x3_token_initialize_token_store)
-  [Function `merge`](#0x3_token_merge)
-  [Function `split`](#0x3_token_split)
-  [Function `token_id`](#0x3_token_token_id)
-  [Function `transfer`](#0x3_token_transfer)
-  [Function `create_withdraw_capability`](#0x3_token_create_withdraw_capability)
-  [Function `withdraw_with_capability`](#0x3_token_withdraw_with_capability)
-  [Function `partial_withdraw_with_capability`](#0x3_token_partial_withdraw_with_capability)
-  [Function `withdraw_token`](#0x3_token_withdraw_token)
-  [Function `create_collection`](#0x3_token_create_collection)
-  [Function `check_collection_exists`](#0x3_token_check_collection_exists)
-  [Function `check_tokendata_exists`](#0x3_token_check_tokendata_exists)
-  [Function `create_tokendata`](#0x3_token_create_tokendata)
-  [Function `get_collection_supply`](#0x3_token_get_collection_supply)
-  [Function `get_collection_description`](#0x3_token_get_collection_description)
-  [Function `get_collection_uri`](#0x3_token_get_collection_uri)
-  [Function `get_collection_maximum`](#0x3_token_get_collection_maximum)
-  [Function `get_token_supply`](#0x3_token_get_token_supply)
-  [Function `get_tokendata_largest_property_version`](#0x3_token_get_tokendata_largest_property_version)
-  [Function `get_token_id`](#0x3_token_get_token_id)
-  [Function `get_direct_transfer`](#0x3_token_get_direct_transfer)
-  [Function `create_token_mutability_config`](#0x3_token_create_token_mutability_config)
-  [Function `create_collection_mutability_config`](#0x3_token_create_collection_mutability_config)
-  [Function `mint_token`](#0x3_token_mint_token)
-  [Function `mint_token_to`](#0x3_token_mint_token_to)
-  [Function `create_token_id`](#0x3_token_create_token_id)
-  [Function `create_token_data_id`](#0x3_token_create_token_data_id)
-  [Function `create_token_id_raw`](#0x3_token_create_token_id_raw)
-  [Function `balance_of`](#0x3_token_balance_of)
-  [Function `has_token_store`](#0x3_token_has_token_store)
-  [Function `get_royalty`](#0x3_token_get_royalty)
-  [Function `get_royalty_numerator`](#0x3_token_get_royalty_numerator)
-  [Function `get_royalty_denominator`](#0x3_token_get_royalty_denominator)
-  [Function `get_royalty_payee`](#0x3_token_get_royalty_payee)
-  [Function `get_token_amount`](#0x3_token_get_token_amount)
-  [Function `get_token_id_fields`](#0x3_token_get_token_id_fields)
-  [Function `get_token_data_id_fields`](#0x3_token_get_token_data_id_fields)
-  [Function `get_property_map`](#0x3_token_get_property_map)
-  [Function `get_tokendata_maximum`](#0x3_token_get_tokendata_maximum)
-  [Function `get_tokendata_uri`](#0x3_token_get_tokendata_uri)
-  [Function `get_tokendata_description`](#0x3_token_get_tokendata_description)
-  [Function `get_tokendata_royalty`](#0x3_token_get_tokendata_royalty)
-  [Function `get_tokendata_id`](#0x3_token_get_tokendata_id)
-  [Function `get_tokendata_mutability_config`](#0x3_token_get_tokendata_mutability_config)
-  [Function `get_token_mutability_maximum`](#0x3_token_get_token_mutability_maximum)
-  [Function `get_token_mutability_royalty`](#0x3_token_get_token_mutability_royalty)
-  [Function `get_token_mutability_uri`](#0x3_token_get_token_mutability_uri)
-  [Function `get_token_mutability_description`](#0x3_token_get_token_mutability_description)
-  [Function `get_token_mutability_default_properties`](#0x3_token_get_token_mutability_default_properties)
-  [Function `get_collection_mutability_config`](#0x3_token_get_collection_mutability_config)
-  [Function `get_collection_mutability_description`](#0x3_token_get_collection_mutability_description)
-  [Function `get_collection_mutability_uri`](#0x3_token_get_collection_mutability_uri)
-  [Function `get_collection_mutability_maximum`](#0x3_token_get_collection_mutability_maximum)
-  [Function `destroy_token_data`](#0x3_token_destroy_token_data)
-  [Function `destroy_collection_data`](#0x3_token_destroy_collection_data)
-  [Function `withdraw_with_event_internal`](#0x3_token_withdraw_with_event_internal)
-  [Function `update_token_property_internal`](#0x3_token_update_token_property_internal)
-  [Function `direct_deposit`](#0x3_token_direct_deposit)
-  [Function `assert_collection_exists`](#0x3_token_assert_collection_exists)
-  [Function `assert_tokendata_exists`](#0x3_token_assert_tokendata_exists)
-  [Function `assert_non_standard_reserved_property`](#0x3_token_assert_non_standard_reserved_property)
-  [Function `initialize_token_script`](#0x3_token_initialize_token_script)
-  [Function `initialize_token`](#0x3_token_initialize_token)
-  [Specification](#@Specification_1)
    -  [Function `create_collection_script`](#@Specification_1_create_collection_script)
    -  [Function `create_token_script`](#@Specification_1_create_token_script)
    -  [Function `mint_script`](#@Specification_1_mint_script)
    -  [Function `mutate_token_properties`](#@Specification_1_mutate_token_properties)
    -  [Function `direct_transfer_script`](#@Specification_1_direct_transfer_script)
    -  [Function `opt_in_direct_transfer`](#@Specification_1_opt_in_direct_transfer)
    -  [Function `transfer_with_opt_in`](#@Specification_1_transfer_with_opt_in)
    -  [Function `burn_by_creator`](#@Specification_1_burn_by_creator)
    -  [Function `burn`](#@Specification_1_burn)
    -  [Function `mutate_collection_description`](#@Specification_1_mutate_collection_description)
    -  [Function `mutate_collection_uri`](#@Specification_1_mutate_collection_uri)
    -  [Function `mutate_collection_maximum`](#@Specification_1_mutate_collection_maximum)
    -  [Function `mutate_tokendata_maximum`](#@Specification_1_mutate_tokendata_maximum)
    -  [Function `mutate_tokendata_uri`](#@Specification_1_mutate_tokendata_uri)
    -  [Function `mutate_tokendata_royalty`](#@Specification_1_mutate_tokendata_royalty)
    -  [Function `mutate_tokendata_description`](#@Specification_1_mutate_tokendata_description)
    -  [Function `mutate_tokendata_property`](#@Specification_1_mutate_tokendata_property)
    -  [Function `mutate_one_token`](#@Specification_1_mutate_one_token)
    -  [Function `create_royalty`](#@Specification_1_create_royalty)
    -  [Function `deposit_token`](#@Specification_1_deposit_token)
    -  [Function `direct_deposit_with_opt_in`](#@Specification_1_direct_deposit_with_opt_in)
    -  [Function `direct_transfer`](#@Specification_1_direct_transfer)
    -  [Function `initialize_token_store`](#@Specification_1_initialize_token_store)
    -  [Function `merge`](#@Specification_1_merge)
    -  [Function `split`](#@Specification_1_split)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `withdraw_with_capability`](#@Specification_1_withdraw_with_capability)
    -  [Function `partial_withdraw_with_capability`](#@Specification_1_partial_withdraw_with_capability)
    -  [Function `withdraw_token`](#@Specification_1_withdraw_token)
    -  [Function `create_collection`](#@Specification_1_create_collection)
    -  [Function `check_collection_exists`](#@Specification_1_check_collection_exists)
    -  [Function `check_tokendata_exists`](#@Specification_1_check_tokendata_exists)
    -  [Function `create_tokendata`](#@Specification_1_create_tokendata)
    -  [Function `get_collection_supply`](#@Specification_1_get_collection_supply)
    -  [Function `get_collection_description`](#@Specification_1_get_collection_description)
    -  [Function `get_collection_uri`](#@Specification_1_get_collection_uri)
    -  [Function `get_collection_maximum`](#@Specification_1_get_collection_maximum)
    -  [Function `get_token_supply`](#@Specification_1_get_token_supply)
    -  [Function `get_tokendata_largest_property_version`](#@Specification_1_get_tokendata_largest_property_version)
    -  [Function `create_token_mutability_config`](#@Specification_1_create_token_mutability_config)
    -  [Function `create_collection_mutability_config`](#@Specification_1_create_collection_mutability_config)
    -  [Function `mint_token`](#@Specification_1_mint_token)
    -  [Function `mint_token_to`](#@Specification_1_mint_token_to)
    -  [Function `create_token_data_id`](#@Specification_1_create_token_data_id)
    -  [Function `create_token_id_raw`](#@Specification_1_create_token_id_raw)
    -  [Function `get_royalty`](#@Specification_1_get_royalty)
    -  [Function `get_property_map`](#@Specification_1_get_property_map)
    -  [Function `get_tokendata_maximum`](#@Specification_1_get_tokendata_maximum)
    -  [Function `get_tokendata_uri`](#@Specification_1_get_tokendata_uri)
    -  [Function `get_tokendata_description`](#@Specification_1_get_tokendata_description)
    -  [Function `get_tokendata_royalty`](#@Specification_1_get_tokendata_royalty)
    -  [Function `get_tokendata_mutability_config`](#@Specification_1_get_tokendata_mutability_config)
    -  [Function `get_collection_mutability_config`](#@Specification_1_get_collection_mutability_config)
    -  [Function `withdraw_with_event_internal`](#@Specification_1_withdraw_with_event_internal)
    -  [Function `update_token_property_internal`](#@Specification_1_update_token_property_internal)
    -  [Function `direct_deposit`](#@Specification_1_direct_deposit)
    -  [Function `assert_collection_exists`](#@Specification_1_assert_collection_exists)
    -  [Function `assert_tokendata_exists`](#@Specification_1_assert_tokendata_exists)
    -  [Function `assert_non_standard_reserved_property`](#@Specification_1_assert_non_standard_reserved_property)
    -  [Function `initialize_token_script`](#@Specification_1_initialize_token_script)
    -  [Function `initialize_token`](#@Specification_1_initialize_token)


<pre><code>use 0x1::account;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::table;<br/>use 0x1::timestamp;<br/>use 0x3::property_map;<br/>use 0x3::token_event_store;<br/></code></pre>



<a id="0x3_token_Token"></a>

## Struct `Token`



<pre><code>struct Token has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>
 the amount of tokens. Only property_version &#61; 0 can have a value bigger than 1.
</dd>
<dt>
<code>token_properties: property_map::PropertyMap</code>
</dt>
<dd>
 The properties with this token.<br/> when property_version &#61; 0, the token_properties are the same as default_properties in TokenData, we don&apos;t store it.<br/> when the property_map mutates, a new property_version is assigned to the token.
</dd>
</dl>


</details>

<a id="0x3_token_TokenId"></a>

## Struct `TokenId`

global unique identifier of a token


<pre><code>struct TokenId has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_data_id: token::TokenDataId</code>
</dt>
<dd>
 the id to the common token data shared by token with different property_version
</dd>
<dt>
<code>property_version: u64</code>
</dt>
<dd>
 The version of the property map; when a fungible token is mutated, a new property version is created and assigned to the token to make it an NFT
</dd>
</dl>


</details>

<a id="0x3_token_TokenDataId"></a>

## Struct `TokenDataId`

globally unique identifier of tokendata


<pre><code>struct TokenDataId has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>
 The address of the creator, eg: 0xcafe
</dd>
<dt>
<code>collection: string::String</code>
</dt>
<dd>
 The name of collection; this is unique under the same account, eg: &quot;Aptos Animal Collection&quot;
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 The name of the token; this is the same as the name field of TokenData
</dd>
</dl>


</details>

<a id="0x3_token_TokenData"></a>

## Struct `TokenData`

The shared TokenData by tokens with different property_version


<pre><code>struct TokenData has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>maximum: u64</code>
</dt>
<dd>
 The maximal number of tokens that can be minted under this TokenData; if the maximum is 0, there is no limit
</dd>
<dt>
<code>largest_property_version: u64</code>
</dt>
<dd>
 The current largest property version of all tokens with this TokenData
</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>
 The number of tokens with this TokenData. Supply is only tracked for the limited token whose maximum is not 0
</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off&#45;chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul&#45;7Mv6vzm7JwE69O&#45;I&#45;vd6Bz2QriJO1niwCh4
</dd>
<dt>
<code>royalty: token::Royalty</code>
</dt>
<dd>
 The denominator and numerator for calculating the royalty fee; it also contains payee account address for depositing the Royalty
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: &quot;Aptos Animal &#35;1234&quot;
</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>
 Describes this Token
</dd>
<dt>
<code>default_properties: property_map::PropertyMap</code>
</dt>
<dd>
 The properties are stored in the TokenData that are shared by all tokens
</dd>
<dt>
<code>mutability_config: token::TokenMutabilityConfig</code>
</dt>
<dd>
 Control the TokenData field mutability
</dd>
</dl>


</details>

<a id="0x3_token_Royalty"></a>

## Struct `Royalty`

The royalty of a token


<pre><code>struct Royalty has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>payee_address: address</code>
</dt>
<dd>
 if the token is jointly owned by multiple creators, the group of creators should create a shared account.<br/> the payee_address will be the shared account address.
</dd>
</dl>


</details>

<a id="0x3_token_TokenMutabilityConfig"></a>

## Struct `TokenMutabilityConfig`

This config specifies which fields in the TokenData are mutable


<pre><code>struct TokenMutabilityConfig has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>maximum: bool</code>
</dt>
<dd>
 control if the token maximum is mutable
</dd>
<dt>
<code>uri: bool</code>
</dt>
<dd>
 control if the token uri is mutable
</dd>
<dt>
<code>royalty: bool</code>
</dt>
<dd>
 control if the token royalty is mutable
</dd>
<dt>
<code>description: bool</code>
</dt>
<dd>
 control if the token description is mutable
</dd>
<dt>
<code>properties: bool</code>
</dt>
<dd>
 control if the property map is mutable
</dd>
</dl>


</details>

<a id="0x3_token_TokenStore"></a>

## Resource `TokenStore`

Represents token resources owned by token owner


<pre><code>struct TokenStore has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: table::Table&lt;token::TokenId, token::Token&gt;</code>
</dt>
<dd>
 the tokens owned by a token owner
</dd>
<dt>
<code>direct_transfer: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>deposit_events: event::EventHandle&lt;token::DepositEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: event::EventHandle&lt;token::WithdrawEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_events: event::EventHandle&lt;token::BurnTokenEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mutate_token_property_events: event::EventHandle&lt;token::MutateTokenPropertyMapEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionMutabilityConfig"></a>

## Struct `CollectionMutabilityConfig`

This config specifies which fields in the Collection are mutable


<pre><code>struct CollectionMutabilityConfig has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: bool</code>
</dt>
<dd>
 control if description is mutable
</dd>
<dt>
<code>uri: bool</code>
</dt>
<dd>
 control if uri is mutable
</dd>
<dt>
<code>maximum: bool</code>
</dt>
<dd>
 control if collection maxium is mutable
</dd>
</dl>


</details>

<a id="0x3_token_Collections"></a>

## Resource `Collections`

Represent collection and token metadata for a creator


<pre><code>struct Collections has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_data: table::Table&lt;string::String, token::CollectionData&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>token_data: table::Table&lt;token::TokenDataId, token::TokenData&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_collection_events: event::EventHandle&lt;token::CreateCollectionEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_token_data_events: event::EventHandle&lt;token::CreateTokenDataEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mint_token_events: event::EventHandle&lt;token::MintTokenEvent&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionData"></a>

## Struct `CollectionData`

Represent the collection metadata


<pre><code>struct CollectionData has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: string::String</code>
</dt>
<dd>
 A description for the token collection Eg: &quot;Aptos Toad Overload&quot;
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: &quot;Animal Collection&quot;
</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>
 The URI for the collection; its length should be smaller than 512 characters
</dd>
<dt>
<code>supply: u64</code>
</dt>
<dd>
 The number of different TokenData entries in this collection
</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>
 If maximal is a non&#45;zero value, the number of created TokenData entries should be smaller or equal to this maximum<br/> If maximal is 0, Aptos doesn&apos;t track the supply of this collection, and there is no limit
</dd>
<dt>
<code>mutability_config: token::CollectionMutabilityConfig</code>
</dt>
<dd>
 control which collectionData field is mutable
</dd>
</dl>


</details>

<a id="0x3_token_WithdrawCapability"></a>

## Struct `WithdrawCapability`

capability to withdraw without signer, this struct should be non&#45;copyable


<pre><code>struct WithdrawCapability has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_owner: address</code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_sec: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_DepositEvent"></a>

## Struct `DepositEvent`

Set of data sent to the event stream during a receive


<pre><code>struct DepositEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_Deposit"></a>

## Struct `Deposit`

Set of data sent to the event stream during a receive


<pre><code>&#35;[event]<br/>struct Deposit has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_WithdrawEvent"></a>

## Struct `WithdrawEvent`

Set of data sent to the event stream during a withdrawal


<pre><code>struct WithdrawEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_Withdraw"></a>

## Struct `Withdraw`

Set of data sent to the event stream during a withdrawal


<pre><code>&#35;[event]<br/>struct Withdraw has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_CreateTokenDataEvent"></a>

## Struct `CreateTokenDataEvent`

token creation event id of token created


<pre><code>struct CreateTokenDataEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenDataId</code>
</dt>
<dd>

</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: token::TokenMutabilityConfig</code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateTokenData"></a>

## Struct `CreateTokenData`



<pre><code>&#35;[event]<br/>struct CreateTokenData has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenDataId</code>
</dt>
<dd>

</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_denominator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_points_numerator: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: token::TokenMutabilityConfig</code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MintTokenEvent"></a>

## Struct `MintTokenEvent`

mint token event. This event triggered when creator adds more supply to existing token


<pre><code>struct MintTokenEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenDataId</code>
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

<a id="0x3_token_MintToken"></a>

## Struct `MintToken`



<pre><code>&#35;[event]<br/>struct MintToken has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenDataId</code>
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

<a id="0x3_token_BurnTokenEvent"></a>

## Struct `BurnTokenEvent`



<pre><code>struct BurnTokenEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_BurnToken"></a>

## Struct `BurnToken`



<pre><code>&#35;[event]<br/>struct BurnToken has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: token::TokenId</code>
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

<a id="0x3_token_MutateTokenPropertyMapEvent"></a>

## Struct `MutateTokenPropertyMapEvent`



<pre><code>struct MutateTokenPropertyMapEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MutateTokenPropertyMap"></a>

## Struct `MutateTokenPropertyMap`



<pre><code>&#35;[event]<br/>struct MutateTokenPropertyMap has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: token::TokenId</code>
</dt>
<dd>

</dd>
<dt>
<code>keys: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: vector&lt;string::String&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateCollectionEvent"></a>

## Struct `CreateCollectionEvent`

create collection event with creator address and collection name


<pre><code>struct CreateCollectionEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateCollection"></a>

## Struct `CreateCollection`



<pre><code>&#35;[event]<br/>struct CreateCollection has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>description: string::String</code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x3_token_EINSUFFICIENT_BALANCE"></a>

Insufficient token balance


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 5;<br/></code></pre>



<a id="0x3_token_EURI_TOO_LONG"></a>

The URI is too long


<pre><code>const EURI_TOO_LONG: u64 &#61; 27;<br/></code></pre>



<a id="0x3_token_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;<br/></code></pre>



<a id="0x3_token_BURNABLE_BY_CREATOR"></a>



<pre><code>const BURNABLE_BY_CREATOR: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 67, 82, 69, 65, 84, 79, 82];<br/></code></pre>



<a id="0x3_token_BURNABLE_BY_OWNER"></a>



<pre><code>const BURNABLE_BY_OWNER: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 79, 87, 78, 69, 82];<br/></code></pre>



<a id="0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND"></a>



<pre><code>const COLLECTION_DESCRIPTION_MUTABLE_IND: u64 &#61; 0;<br/></code></pre>



<a id="0x3_token_COLLECTION_MAX_MUTABLE_IND"></a>



<pre><code>const COLLECTION_MAX_MUTABLE_IND: u64 &#61; 2;<br/></code></pre>



<a id="0x3_token_COLLECTION_URI_MUTABLE_IND"></a>



<pre><code>const COLLECTION_URI_MUTABLE_IND: u64 &#61; 1;<br/></code></pre>



<a id="0x3_token_EALREADY_HAS_BALANCE"></a>

The token has balance and cannot be initialized


<pre><code>const EALREADY_HAS_BALANCE: u64 &#61; 0;<br/></code></pre>



<a id="0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY"></a>

Reserved fields for token contract<br/> Cannot be updated by user


<pre><code>const ECANNOT_UPDATE_RESERVED_PROPERTY: u64 &#61; 32;<br/></code></pre>



<a id="0x3_token_ECOLLECTIONS_NOT_PUBLISHED"></a>

There isn&apos;t any collection under this account


<pre><code>const ECOLLECTIONS_NOT_PUBLISHED: u64 &#61; 1;<br/></code></pre>



<a id="0x3_token_ECOLLECTION_ALREADY_EXISTS"></a>

The collection already exists


<pre><code>const ECOLLECTION_ALREADY_EXISTS: u64 &#61; 3;<br/></code></pre>



<a id="0x3_token_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is too long


<pre><code>const ECOLLECTION_NAME_TOO_LONG: u64 &#61; 25;<br/></code></pre>



<a id="0x3_token_ECOLLECTION_NOT_PUBLISHED"></a>

Cannot find collection in creator&apos;s account


<pre><code>const ECOLLECTION_NOT_PUBLISHED: u64 &#61; 2;<br/></code></pre>



<a id="0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM"></a>

Exceeds the collection&apos;s maximal number of token_data


<pre><code>const ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM: u64 &#61; 4;<br/></code></pre>



<a id="0x3_token_ECREATOR_CANNOT_BURN_TOKEN"></a>

Token is not burnable by creator


<pre><code>const ECREATOR_CANNOT_BURN_TOKEN: u64 &#61; 31;<br/></code></pre>



<a id="0x3_token_EFIELD_NOT_MUTABLE"></a>

The field is not mutable


<pre><code>const EFIELD_NOT_MUTABLE: u64 &#61; 13;<br/></code></pre>



<a id="0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT"></a>

Withdraw capability doesn&apos;t have sufficient amount


<pre><code>const EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT: u64 &#61; 38;<br/></code></pre>



<a id="0x3_token_EINVALID_MAXIMUM"></a>

Collection or tokendata maximum must be larger than supply


<pre><code>const EINVALID_MAXIMUM: u64 &#61; 36;<br/></code></pre>



<a id="0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR"></a>

Royalty invalid if the numerator is larger than the denominator


<pre><code>const EINVALID_ROYALTY_NUMERATOR_DENOMINATOR: u64 &#61; 34;<br/></code></pre>



<a id="0x3_token_EINVALID_TOKEN_MERGE"></a>

Cannot merge the two tokens with different token id


<pre><code>const EINVALID_TOKEN_MERGE: u64 &#61; 6;<br/></code></pre>



<a id="0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM"></a>

Exceed the token data maximal allowed


<pre><code>const EMINT_WOULD_EXCEED_TOKEN_MAXIMUM: u64 &#61; 7;<br/></code></pre>



<a id="0x3_token_ENFT_NAME_TOO_LONG"></a>

The NFT name is too long


<pre><code>const ENFT_NAME_TOO_LONG: u64 &#61; 26;<br/></code></pre>



<a id="0x3_token_ENFT_NOT_SPLITABLE"></a>

Cannot split a token that only has 1 amount


<pre><code>const ENFT_NOT_SPLITABLE: u64 &#61; 18;<br/></code></pre>



<a id="0x3_token_ENO_BURN_CAPABILITY"></a>

No burn capability


<pre><code>const ENO_BURN_CAPABILITY: u64 &#61; 8;<br/></code></pre>



<a id="0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot burn 0 Token


<pre><code>const ENO_BURN_TOKEN_WITH_ZERO_AMOUNT: u64 &#61; 29;<br/></code></pre>



<a id="0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot deposit a Token with 0 amount


<pre><code>const ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT: u64 &#61; 28;<br/></code></pre>



<a id="0x3_token_ENO_MINT_CAPABILITY"></a>

No mint capability


<pre><code>const ENO_MINT_CAPABILITY: u64 &#61; 19;<br/></code></pre>



<a id="0x3_token_ENO_MUTATE_CAPABILITY"></a>

Not authorized to mutate


<pre><code>const ENO_MUTATE_CAPABILITY: u64 &#61; 14;<br/></code></pre>



<a id="0x3_token_ENO_TOKEN_IN_TOKEN_STORE"></a>

Token not in the token store


<pre><code>const ENO_TOKEN_IN_TOKEN_STORE: u64 &#61; 15;<br/></code></pre>



<a id="0x3_token_EOWNER_CANNOT_BURN_TOKEN"></a>

Token is not burnable by owner


<pre><code>const EOWNER_CANNOT_BURN_TOKEN: u64 &#61; 30;<br/></code></pre>



<a id="0x3_token_EPROPERTY_RESERVED_BY_STANDARD"></a>

The property is reserved by token standard


<pre><code>const EPROPERTY_RESERVED_BY_STANDARD: u64 &#61; 40;<br/></code></pre>



<a id="0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST"></a>

Royalty payee account does not exist


<pre><code>const EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST: u64 &#61; 35;<br/></code></pre>



<a id="0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT"></a>

TOKEN with 0 amount is not allowed


<pre><code>const ETOKEN_CANNOT_HAVE_ZERO_AMOUNT: u64 &#61; 33;<br/></code></pre>



<a id="0x3_token_ETOKEN_DATA_ALREADY_EXISTS"></a>

TokenData already exists


<pre><code>const ETOKEN_DATA_ALREADY_EXISTS: u64 &#61; 9;<br/></code></pre>



<a id="0x3_token_ETOKEN_DATA_NOT_PUBLISHED"></a>

TokenData not published


<pre><code>const ETOKEN_DATA_NOT_PUBLISHED: u64 &#61; 10;<br/></code></pre>



<a id="0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH"></a>

Token Properties count doesn&apos;t match


<pre><code>const ETOKEN_PROPERTIES_COUNT_NOT_MATCH: u64 &#61; 37;<br/></code></pre>



<a id="0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT"></a>

Cannot split token to an amount larger than its amount


<pre><code>const ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT: u64 &#61; 12;<br/></code></pre>



<a id="0x3_token_ETOKEN_STORE_NOT_PUBLISHED"></a>

TokenStore doesn&apos;t exist


<pre><code>const ETOKEN_STORE_NOT_PUBLISHED: u64 &#61; 11;<br/></code></pre>



<a id="0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER"></a>

User didn&apos;t opt&#45;in direct transfer


<pre><code>const EUSER_NOT_OPT_IN_DIRECT_TRANSFER: u64 &#61; 16;<br/></code></pre>



<a id="0x3_token_EWITHDRAW_PROOF_EXPIRES"></a>

Withdraw proof expires


<pre><code>const EWITHDRAW_PROOF_EXPIRES: u64 &#61; 39;<br/></code></pre>



<a id="0x3_token_EWITHDRAW_ZERO"></a>

Cannot withdraw 0 token


<pre><code>const EWITHDRAW_ZERO: u64 &#61; 17;<br/></code></pre>



<a id="0x3_token_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code>const MAX_COLLECTION_NAME_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x3_token_MAX_NFT_NAME_LENGTH"></a>



<pre><code>const MAX_NFT_NAME_LENGTH: u64 &#61; 128;<br/></code></pre>



<a id="0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND"></a>



<pre><code>const TOKEN_DESCRIPTION_MUTABLE_IND: u64 &#61; 3;<br/></code></pre>



<a id="0x3_token_TOKEN_MAX_MUTABLE_IND"></a>



<pre><code>const TOKEN_MAX_MUTABLE_IND: u64 &#61; 0;<br/></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE"></a>



<pre><code>const TOKEN_PROPERTY_MUTABLE: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 80, 82, 79, 80, 69, 82, 84, 89, 95, 77, 85, 84, 65, 84, 66, 76, 69];<br/></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE_IND"></a>



<pre><code>const TOKEN_PROPERTY_MUTABLE_IND: u64 &#61; 4;<br/></code></pre>



<a id="0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND"></a>



<pre><code>const TOKEN_PROPERTY_VALUE_MUTABLE_IND: u64 &#61; 5;<br/></code></pre>



<a id="0x3_token_TOKEN_ROYALTY_MUTABLE_IND"></a>



<pre><code>const TOKEN_ROYALTY_MUTABLE_IND: u64 &#61; 2;<br/></code></pre>



<a id="0x3_token_TOKEN_URI_MUTABLE_IND"></a>



<pre><code>const TOKEN_URI_MUTABLE_IND: u64 &#61; 1;<br/></code></pre>



<a id="0x3_token_create_collection_script"></a>

## Function `create_collection_script`

create a empty token collection with parameters


<pre><code>public entry fun create_collection_script(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_collection_script(<br/>    creator: &amp;signer,<br/>    name: String,<br/>    description: String,<br/>    uri: String,<br/>    maximum: u64,<br/>    mutate_setting: vector&lt;bool&gt;,<br/>) acquires Collections &#123;<br/>    create_collection(<br/>        creator,<br/>        name,<br/>        description,<br/>        uri,<br/>        maximum,<br/>        mutate_setting<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_token_script"></a>

## Function `create_token_script`

create token with raw inputs


<pre><code>public entry fun create_token_script(account: &amp;signer, collection: string::String, name: string::String, description: string::String, balance: u64, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: vector&lt;bool&gt;, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_token_script(<br/>    account: &amp;signer,<br/>    collection: String,<br/>    name: String,<br/>    description: String,<br/>    balance: u64,<br/>    maximum: u64,<br/>    uri: String,<br/>    royalty_payee_address: address,<br/>    royalty_points_denominator: u64,<br/>    royalty_points_numerator: u64,<br/>    mutate_setting: vector&lt;bool&gt;,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    property_types: vector&lt;String&gt;<br/>) acquires Collections, TokenStore &#123;<br/>    let token_mut_config &#61; create_token_mutability_config(&amp;mutate_setting);<br/>    let tokendata_id &#61; create_tokendata(<br/>        account,<br/>        collection,<br/>        name,<br/>        description,<br/>        maximum,<br/>        uri,<br/>        royalty_payee_address,<br/>        royalty_points_denominator,<br/>        royalty_points_numerator,<br/>        token_mut_config,<br/>        property_keys,<br/>        property_values,<br/>        property_types<br/>    );<br/><br/>    mint_token(<br/>        account,<br/>        tokendata_id,<br/>        balance,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mint_script"></a>

## Function `mint_script`

Mint more token from an existing token_data. Mint only adds more token to property_version 0


<pre><code>public entry fun mint_script(account: &amp;signer, token_data_address: address, collection: string::String, name: string::String, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint_script(<br/>    account: &amp;signer,<br/>    token_data_address: address,<br/>    collection: String,<br/>    name: String,<br/>    amount: u64,<br/>) acquires Collections, TokenStore &#123;<br/>    let token_data_id &#61; create_token_data_id(<br/>        token_data_address,<br/>        collection,<br/>        name,<br/>    );<br/>    // only creator of the tokendata can mint more tokens for now<br/>    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));<br/>    mint_token(<br/>        account,<br/>        token_data_id,<br/>        amount,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_token_properties"></a>

## Function `mutate_token_properties`

mutate the token property and save the new property in TokenStore<br/> if the token property_version is 0, we will create a new property_version per token to generate a new token_id per token<br/> if the token property_version is not 0, we will just update the propertyMap and use the existing token_id (property_version)


<pre><code>public entry fun mutate_token_properties(account: &amp;signer, token_owner: address, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, amount: u64, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mutate_token_properties(<br/>    account: &amp;signer,<br/>    token_owner: address,<br/>    creator: address,<br/>    collection_name: String,<br/>    token_name: String,<br/>    token_property_version: u64,<br/>    amount: u64,<br/>    keys: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    types: vector&lt;String&gt;,<br/>) acquires Collections, TokenStore &#123;<br/>    assert!(signer::address_of(account) &#61;&#61; creator, error::not_found(ENO_MUTATE_CAPABILITY));<br/>    let i &#61; 0;<br/>    let token_id &#61; create_token_id_raw(<br/>        creator,<br/>        collection_name,<br/>        token_name,<br/>        token_property_version,<br/>    );<br/>    // give a new property_version for each token<br/>    while (i &lt; amount) &#123;<br/>        mutate_one_token(account, token_owner, token_id, keys, values, types);<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_direct_transfer_script"></a>

## Function `direct_transfer_script`



<pre><code>public entry fun direct_transfer_script(sender: &amp;signer, receiver: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun direct_transfer_script(<br/>    sender: &amp;signer,<br/>    receiver: &amp;signer,<br/>    creators_address: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>    amount: u64,<br/>) acquires TokenStore &#123;<br/>    let token_id &#61; create_token_id_raw(creators_address, collection, name, property_version);<br/>    direct_transfer(sender, receiver, token_id, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_opt_in_direct_transfer"></a>

## Function `opt_in_direct_transfer`



<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool) acquires TokenStore &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    initialize_token_store(account);<br/>    let opt_in_flag &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(addr).direct_transfer;<br/>    &#42;opt_in_flag &#61; opt_in;<br/>    token_event_store::emit_token_opt_in_event(account, opt_in);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfer_with_opt_in"></a>

## Function `transfer_with_opt_in`

Transfers <code>amount</code> of tokens from <code>from</code> to <code>to</code>.<br/> The receiver <code>to</code> has to opt&#45;in direct transfer first


<pre><code>public entry fun transfer_with_opt_in(from: &amp;signer, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_with_opt_in(<br/>    from: &amp;signer,<br/>    creator: address,<br/>    collection_name: String,<br/>    token_name: String,<br/>    token_property_version: u64,<br/>    to: address,<br/>    amount: u64,<br/>) acquires TokenStore &#123;<br/>    let token_id &#61; create_token_id_raw(creator, collection_name, token_name, token_property_version);<br/>    transfer(from, token_id, to, amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_burn_by_creator"></a>

## Function `burn_by_creator`

Burn a token by creator when the token&apos;s BURNABLE_BY_CREATOR is true<br/> The token is owned at address owner


<pre><code>public entry fun burn_by_creator(creator: &amp;signer, owner: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn_by_creator(<br/>    creator: &amp;signer,<br/>    owner: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>    amount: u64,<br/>) acquires Collections, TokenStore &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    assert!(amount &gt; 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));<br/>    let token_id &#61; create_token_id_raw(creator_address, collection, name, property_version);<br/>    let creator_addr &#61; token_id.token_data_id.creator;<br/>    assert!(<br/>        exists&lt;Collections&gt;(creator_addr),<br/>        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),<br/>    );<br/><br/>    let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_address);<br/>    assert!(<br/>        table::contains(&amp;collections.token_data, token_id.token_data_id),<br/>        error::not_found(ETOKEN_DATA_NOT_PUBLISHED),<br/>    );<br/><br/>    let token_data &#61; table::borrow_mut(<br/>        &amp;mut collections.token_data,<br/>        token_id.token_data_id,<br/>    );<br/><br/>    // The property should be explicitly set in the property_map for creator to burn the token<br/>    assert!(<br/>        property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_CREATOR)),<br/>        error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN)<br/>    );<br/><br/>    let burn_by_creator_flag &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_CREATOR));<br/>    assert!(burn_by_creator_flag, error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN));<br/><br/>    // Burn the tokens.<br/>    let Token &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; withdraw_with_event_internal(owner, token_id, amount);<br/>    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(owner);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(BurnToken &#123; id: token_id, amount: burned_amount &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;BurnTokenEvent&gt;(<br/>        &amp;mut token_store.burn_events,<br/>        BurnTokenEvent &#123; id: token_id, amount: burned_amount &#125;<br/>    );<br/><br/>    if (token_data.maximum &gt; 0) &#123;<br/>        token_data.supply &#61; token_data.supply &#45; burned_amount;<br/><br/>        // Delete the token_data if supply drops to 0.<br/>        if (token_data.supply &#61;&#61; 0) &#123;<br/>            destroy_token_data(table::remove(&amp;mut collections.token_data, token_id.token_data_id));<br/><br/>            // update the collection supply<br/>            let collection_data &#61; table::borrow_mut(<br/>                &amp;mut collections.collection_data,<br/>                token_id.token_data_id.collection<br/>            );<br/>            if (collection_data.maximum &gt; 0) &#123;<br/>                collection_data.supply &#61; collection_data.supply &#45; 1;<br/>                // delete the collection data if the collection supply equals 0<br/>                if (collection_data.supply &#61;&#61; 0) &#123;<br/>                    destroy_collection_data(table::remove(&amp;mut collections.collection_data, collection_data.name));<br/>                &#125;;<br/>            &#125;;<br/>        &#125;;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_burn"></a>

## Function `burn`

Burn a token by the token owner


<pre><code>public entry fun burn(owner: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn(<br/>    owner: &amp;signer,<br/>    creators_address: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>    amount: u64<br/>) acquires Collections, TokenStore &#123;<br/>    assert!(amount &gt; 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));<br/>    let token_id &#61; create_token_id_raw(creators_address, collection, name, property_version);<br/>    let creator_addr &#61; token_id.token_data_id.creator;<br/>    assert!(<br/>        exists&lt;Collections&gt;(creator_addr),<br/>        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),<br/>    );<br/><br/>    let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_addr);<br/>    assert!(<br/>        table::contains(&amp;collections.token_data, token_id.token_data_id),<br/>        error::not_found(ETOKEN_DATA_NOT_PUBLISHED),<br/>    );<br/><br/>    let token_data &#61; table::borrow_mut(<br/>        &amp;mut collections.token_data,<br/>        token_id.token_data_id,<br/>    );<br/><br/>    assert!(<br/>        property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_OWNER)),<br/>        error::permission_denied(EOWNER_CANNOT_BURN_TOKEN)<br/>    );<br/>    let burn_by_owner_flag &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_OWNER));<br/>    assert!(burn_by_owner_flag, error::permission_denied(EOWNER_CANNOT_BURN_TOKEN));<br/><br/>    // Burn the tokens.<br/>    let Token &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; withdraw_token(owner, token_id, amount);<br/>    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(signer::address_of(owner));<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(BurnToken &#123; id: token_id, amount: burned_amount &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;BurnTokenEvent&gt;(<br/>        &amp;mut token_store.burn_events,<br/>        BurnTokenEvent &#123; id: token_id, amount: burned_amount &#125;<br/>    );<br/><br/>    // Decrease the supply correspondingly by the amount of tokens burned.<br/>    let token_data &#61; table::borrow_mut(<br/>        &amp;mut collections.token_data,<br/>        token_id.token_data_id,<br/>    );<br/><br/>    // only update the supply if we tracking the supply and maximal<br/>    // maximal &#61;&#61; 0 is reserved for unlimited token and collection with no tracking info.<br/>    if (token_data.maximum &gt; 0) &#123;<br/>        token_data.supply &#61; token_data.supply &#45; burned_amount;<br/><br/>        // Delete the token_data if supply drops to 0.<br/>        if (token_data.supply &#61;&#61; 0) &#123;<br/>            destroy_token_data(table::remove(&amp;mut collections.token_data, token_id.token_data_id));<br/><br/>            // update the collection supply<br/>            let collection_data &#61; table::borrow_mut(<br/>                &amp;mut collections.collection_data,<br/>                token_id.token_data_id.collection<br/>            );<br/><br/>            // only update and check the supply for unlimited collection<br/>            if (collection_data.maximum &gt; 0)&#123;<br/>                collection_data.supply &#61; collection_data.supply &#45; 1;<br/>                // delete the collection data if the collection supply equals 0<br/>                if (collection_data.supply &#61;&#61; 0) &#123;<br/>                    destroy_collection_data(table::remove(&amp;mut collections.collection_data, collection_data.name));<br/>                &#125;;<br/>            &#125;;<br/>        &#125;;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_collection_description"></a>

## Function `mutate_collection_description`



<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: string::String, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: String, description: String) acquires Collections &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    assert!(collection_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_collection_description_mutate_event(creator, collection_name, collection_data.description, description);<br/>    collection_data.description &#61; description;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_collection_uri"></a>

## Function `mutate_collection_uri`



<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: string::String, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: String, uri: String) acquires Collections &#123;<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));<br/>    let creator_address &#61; signer::address_of(creator);<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    assert!(collection_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_collection_uri_mutate_event(creator, collection_name, collection_data.uri , uri);<br/>    collection_data.uri &#61; uri;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_collection_maximum"></a>

## Function `mutate_collection_maximum`



<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: string::String, maximum: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: String, maximum: u64) acquires Collections &#123;<br/>    let creator_address &#61; signer::address_of(creator);<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    // cannot change maximum from 0 and cannot change maximum to 0<br/>    assert!(collection_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, error::invalid_argument(EINVALID_MAXIMUM));<br/>    assert!(maximum &gt;&#61; collection_data.supply, error::invalid_argument(EINVALID_MAXIMUM));<br/>    assert!(collection_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_collection_maximum_mutate_event(creator, collection_name, collection_data.maximum, maximum);<br/>    collection_data.maximum &#61; maximum;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_maximum"></a>

## Function `mutate_tokendata_maximum`



<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: token::TokenDataId, maximum: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: TokenDataId, maximum: u64) acquires Collections &#123;<br/>    assert_tokendata_exists(creator, token_data_id);<br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/>    // cannot change maximum from 0 and cannot change maximum to 0<br/>    assert!(token_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, error::invalid_argument(EINVALID_MAXIMUM));<br/>    assert!(maximum &gt;&#61; token_data.supply, error::invalid_argument(EINVALID_MAXIMUM));<br/>    assert!(token_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_token_maximum_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.maximum, maximum);<br/>    token_data.maximum &#61; maximum;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_uri"></a>

## Function `mutate_tokendata_uri`



<pre><code>public fun mutate_tokendata_uri(creator: &amp;signer, token_data_id: token::TokenDataId, uri: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_uri(<br/>    creator: &amp;signer,<br/>    token_data_id: TokenDataId,<br/>    uri: String<br/>) acquires Collections &#123;<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));<br/>    assert_tokendata_exists(creator, token_data_id);<br/><br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/>    assert!(token_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_token_uri_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.uri ,uri);<br/>    token_data.uri &#61; uri;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_royalty"></a>

## Function `mutate_tokendata_royalty`



<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: token::TokenDataId, royalty: token::Royalty)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: TokenDataId, royalty: Royalty) acquires Collections &#123;<br/>    assert_tokendata_exists(creator, token_data_id);<br/><br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/>    assert!(token_data.mutability_config.royalty, error::permission_denied(EFIELD_NOT_MUTABLE));<br/><br/>    token_event_store::emit_token_royalty_mutate_event(<br/>        creator,<br/>        token_data_id.collection,<br/>        token_data_id.name,<br/>        token_data.royalty.royalty_points_numerator,<br/>        token_data.royalty.royalty_points_denominator,<br/>        token_data.royalty.payee_address,<br/>        royalty.royalty_points_numerator,<br/>        royalty.royalty_points_denominator,<br/>        royalty.payee_address<br/>    );<br/>    token_data.royalty &#61; royalty;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_description"></a>

## Function `mutate_tokendata_description`



<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: token::TokenDataId, description: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: TokenDataId, description: String) acquires Collections &#123;<br/>    assert_tokendata_exists(creator, token_data_id);<br/><br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/>    assert!(token_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    token_event_store::emit_token_descrition_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.description, description);<br/>    token_data.description &#61; description;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_tokendata_property"></a>

## Function `mutate_tokendata_property`

Allow creator to mutate the default properties in TokenData


<pre><code>public fun mutate_tokendata_property(creator: &amp;signer, token_data_id: token::TokenDataId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_property(<br/>    creator: &amp;signer,<br/>    token_data_id: TokenDataId,<br/>    keys: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    types: vector&lt;String&gt;,<br/>) acquires Collections &#123;<br/>    assert_tokendata_exists(creator, token_data_id);<br/>    let key_len &#61; vector::length(&amp;keys);<br/>    let val_len &#61; vector::length(&amp;values);<br/>    let typ_len &#61; vector::length(&amp;types);<br/>    assert!(key_len &#61;&#61; val_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));<br/>    assert!(key_len &#61;&#61; typ_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));<br/><br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/>    assert!(token_data.mutability_config.properties, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    let i: u64 &#61; 0;<br/>    let old_values: vector&lt;Option&lt;PropertyValue&gt;&gt; &#61; vector::empty();<br/>    let new_values: vector&lt;PropertyValue&gt; &#61; vector::empty();<br/>    assert_non_standard_reserved_property(&amp;keys);<br/>    while (i &lt; vector::length(&amp;keys))&#123;<br/>        let key &#61; vector::borrow(&amp;keys, i);<br/>        let old_pv &#61; if (property_map::contains_key(&amp;token_data.default_properties, key)) &#123;<br/>            option::some(&#42;property_map::borrow(&amp;token_data.default_properties, key))<br/>        &#125; else &#123;<br/>            option::none&lt;PropertyValue&gt;()<br/>        &#125;;<br/>        vector::push_back(&amp;mut old_values, old_pv);<br/>        let new_pv &#61; property_map::create_property_value_raw(&#42;vector::borrow(&amp;values, i), &#42;vector::borrow(&amp;types, i));<br/>        vector::push_back(&amp;mut new_values, new_pv);<br/>        if (option::is_some(&amp;old_pv)) &#123;<br/>            property_map::update_property_value(&amp;mut token_data.default_properties, key, new_pv);<br/>        &#125; else &#123;<br/>            property_map::add(&amp;mut token_data.default_properties, &#42;key, new_pv);<br/>        &#125;;<br/>        i &#61; i &#43; 1;<br/>    &#125;;<br/>    token_event_store::emit_default_property_mutate_event(creator, token_data_id.collection, token_data_id.name, keys, old_values, new_values);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mutate_one_token"></a>

## Function `mutate_one_token`

Mutate the token_properties of one token.


<pre><code>public fun mutate_one_token(account: &amp;signer, token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_one_token(<br/>    account: &amp;signer,<br/>    token_owner: address,<br/>    token_id: TokenId,<br/>    keys: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    types: vector&lt;String&gt;,<br/>): TokenId acquires Collections, TokenStore &#123;<br/>    let creator &#61; token_id.token_data_id.creator;<br/>    assert!(signer::address_of(account) &#61;&#61; creator, error::permission_denied(ENO_MUTATE_CAPABILITY));<br/>    // validate if the properties is mutable<br/>    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(<br/>        creator<br/>    ).token_data;<br/><br/>    assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_id.token_data_id);<br/><br/>    // if default property is mutatable, token property is alwasy mutable<br/>    // we only need to check TOKEN_PROPERTY_MUTABLE when default property is immutable<br/>    if (!token_data.mutability_config.properties) &#123;<br/>        assert!(<br/>            property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(TOKEN_PROPERTY_MUTABLE)),<br/>            error::permission_denied(EFIELD_NOT_MUTABLE)<br/>        );<br/><br/>        let token_prop_mutable &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(TOKEN_PROPERTY_MUTABLE));<br/>        assert!(token_prop_mutable, error::permission_denied(EFIELD_NOT_MUTABLE));<br/>    &#125;;<br/><br/>    // check if the property_version is 0 to determine if we need to update the property_version<br/>    if (token_id.property_version &#61;&#61; 0) &#123;<br/>        let token &#61; withdraw_with_event_internal(token_owner, token_id, 1);<br/>        // give a new property_version for each token<br/>        let cur_property_version &#61; token_data.largest_property_version &#43; 1;<br/>        let new_token_id &#61; create_token_id(token_id.token_data_id, cur_property_version);<br/>        let new_token &#61; Token &#123;<br/>            id: new_token_id,<br/>            amount: 1,<br/>            token_properties: token_data.default_properties,<br/>        &#125;;<br/>        direct_deposit(token_owner, new_token);<br/>        update_token_property_internal(token_owner, new_token_id, keys, values, types);<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(MutateTokenPropertyMap &#123;<br/>                old_id: token_id,<br/>                new_id: new_token_id,<br/>                keys,<br/>                values,<br/>                types<br/>            &#125;);<br/>        &#125;;<br/>        event::emit_event&lt;MutateTokenPropertyMapEvent&gt;(<br/>            &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).mutate_token_property_events,<br/>            MutateTokenPropertyMapEvent &#123;<br/>                old_id: token_id,<br/>                new_id: new_token_id,<br/>                keys,<br/>                values,<br/>                types<br/>            &#125;,<br/>        );<br/><br/>        token_data.largest_property_version &#61; cur_property_version;<br/>        // burn the orignial property_version 0 token after mutation<br/>        let Token &#123; id: _, amount: _, token_properties: _ &#125; &#61; token;<br/>        new_token_id<br/>    &#125; else &#123;<br/>        // only 1 copy for the token with property verion bigger than 0<br/>        update_token_property_internal(token_owner, token_id, keys, values, types);<br/>        if (std::features::module_event_migration_enabled()) &#123;<br/>            event::emit(MutateTokenPropertyMap &#123;<br/>                old_id: token_id,<br/>                new_id: token_id,<br/>                keys,<br/>                values,<br/>                types<br/>            &#125;);<br/>        &#125;;<br/>        event::emit_event&lt;MutateTokenPropertyMapEvent&gt;(<br/>            &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).mutate_token_property_events,<br/>            MutateTokenPropertyMapEvent &#123;<br/>                old_id: token_id,<br/>                new_id: token_id,<br/>                keys,<br/>                values,<br/>                types<br/>            &#125;,<br/>        );<br/>        token_id<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_royalty"></a>

## Function `create_royalty`



<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): token::Royalty<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): Royalty &#123;<br/>    assert!(royalty_points_numerator &lt;&#61; royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));<br/>    assert!(account::exists_at(payee_address), error::invalid_argument(EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST));<br/>    Royalty &#123;<br/>        royalty_points_numerator,<br/>        royalty_points_denominator,<br/>        payee_address<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_deposit_token"></a>

## Function `deposit_token`

Deposit the token balance into the owner&apos;s account and emit an event.


<pre><code>public fun deposit_token(account: &amp;signer, token: token::Token)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_token(account: &amp;signer, token: Token) acquires TokenStore &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/>    initialize_token_store(account);<br/>    direct_deposit(account_addr, token)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_direct_deposit_with_opt_in"></a>

## Function `direct_deposit_with_opt_in`

direct deposit if user opt in direct transfer


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: token::Token)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: Token) acquires TokenStore &#123;<br/>    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(account_addr).direct_transfer;<br/>    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));<br/>    direct_deposit(account_addr, token);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_direct_transfer"></a>

## Function `direct_transfer`



<pre><code>public fun direct_transfer(sender: &amp;signer, receiver: &amp;signer, token_id: token::TokenId, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun direct_transfer(<br/>    sender: &amp;signer,<br/>    receiver: &amp;signer,<br/>    token_id: TokenId,<br/>    amount: u64,<br/>) acquires TokenStore &#123;<br/>    let token &#61; withdraw_token(sender, token_id, amount);<br/>    deposit_token(receiver, token);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_initialize_token_store"></a>

## Function `initialize_token_store`



<pre><code>public fun initialize_token_store(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_token_store(account: &amp;signer) &#123;<br/>    if (!exists&lt;TokenStore&gt;(signer::address_of(account))) &#123;<br/>        move_to(<br/>            account,<br/>            TokenStore &#123;<br/>                tokens: table::new(),<br/>                direct_transfer: false,<br/>                deposit_events: account::new_event_handle&lt;DepositEvent&gt;(account),<br/>                withdraw_events: account::new_event_handle&lt;WithdrawEvent&gt;(account),<br/>                burn_events: account::new_event_handle&lt;BurnTokenEvent&gt;(account),<br/>                mutate_token_property_events: account::new_event_handle&lt;MutateTokenPropertyMapEvent&gt;(account),<br/>            &#125;,<br/>        );<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_merge"></a>

## Function `merge`



<pre><code>public fun merge(dst_token: &amp;mut token::Token, source_token: token::Token)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge(dst_token: &amp;mut Token, source_token: Token) &#123;<br/>    assert!(&amp;dst_token.id &#61;&#61; &amp;source_token.id, error::invalid_argument(EINVALID_TOKEN_MERGE));<br/>    dst_token.amount &#61; dst_token.amount &#43; source_token.amount;<br/>    let Token &#123; id: _, amount: _, token_properties: _ &#125; &#61; source_token;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_split"></a>

## Function `split`



<pre><code>public fun split(dst_token: &amp;mut token::Token, amount: u64): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun split(dst_token: &amp;mut Token, amount: u64): Token &#123;<br/>    assert!(dst_token.id.property_version &#61;&#61; 0, error::invalid_state(ENFT_NOT_SPLITABLE));<br/>    assert!(dst_token.amount &gt; amount, error::invalid_argument(ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT));<br/>    assert!(amount &gt; 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));<br/>    dst_token.amount &#61; dst_token.amount &#45; amount;<br/>    Token &#123;<br/>        id: dst_token.id,<br/>        amount,<br/>        token_properties: property_map::empty(),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_token_id"></a>

## Function `token_id`



<pre><code>public fun token_id(token: &amp;token::Token): &amp;token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun token_id(token: &amp;Token): &amp;TokenId &#123;<br/>    &amp;token.id<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of tokens from <code>from</code> to <code>to</code>.


<pre><code>public fun transfer(from: &amp;signer, id: token::TokenId, to: address, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer(<br/>    from: &amp;signer,<br/>    id: TokenId,<br/>    to: address,<br/>    amount: u64,<br/>) acquires TokenStore &#123;<br/>    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(to).direct_transfer;<br/>    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));<br/>    let token &#61; withdraw_token(from, id, amount);<br/>    direct_deposit(to, token);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_withdraw_capability"></a>

## Function `create_withdraw_capability`

Token owner can create this one&#45;time withdraw capability with an expiration time


<pre><code>public fun create_withdraw_capability(owner: &amp;signer, token_id: token::TokenId, amount: u64, expiration_sec: u64): token::WithdrawCapability<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_withdraw_capability(<br/>    owner: &amp;signer,<br/>    token_id: TokenId,<br/>    amount: u64,<br/>    expiration_sec: u64,<br/>): WithdrawCapability &#123;<br/>    WithdrawCapability &#123;<br/>        token_owner: signer::address_of(owner),<br/>        token_id,<br/>        amount,<br/>        expiration_sec,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_withdraw_with_capability"></a>

## Function `withdraw_with_capability`

Withdraw the token with a capability


<pre><code>public fun withdraw_with_capability(withdraw_proof: token::WithdrawCapability): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_capability(<br/>    withdraw_proof: WithdrawCapability,<br/>): Token acquires TokenStore &#123;<br/>    // verify the delegation hasn&apos;t expired yet<br/>    assert!(timestamp::now_seconds() &lt;&#61; withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));<br/><br/>    withdraw_with_event_internal(<br/>        withdraw_proof.token_owner,<br/>        withdraw_proof.token_id,<br/>        withdraw_proof.amount,<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_partial_withdraw_with_capability"></a>

## Function `partial_withdraw_with_capability`

Withdraw the token with a capability.


<pre><code>public fun partial_withdraw_with_capability(withdraw_proof: token::WithdrawCapability, withdraw_amount: u64): (token::Token, option::Option&lt;token::WithdrawCapability&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun partial_withdraw_with_capability(<br/>    withdraw_proof: WithdrawCapability,<br/>    withdraw_amount: u64,<br/>): (Token, Option&lt;WithdrawCapability&gt;) acquires TokenStore &#123;<br/>    // verify the delegation hasn&apos;t expired yet<br/>    assert!(timestamp::now_seconds() &lt;&#61; withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));<br/><br/>    assert!(withdraw_amount &lt;&#61; withdraw_proof.amount, error::invalid_argument(EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT));<br/><br/>    let res: Option&lt;WithdrawCapability&gt; &#61; if (withdraw_amount &#61;&#61; withdraw_proof.amount) &#123;<br/>        option::none&lt;WithdrawCapability&gt;()<br/>    &#125; else &#123;<br/>        option::some(<br/>            WithdrawCapability &#123;<br/>                token_owner: withdraw_proof.token_owner,<br/>                token_id: withdraw_proof.token_id,<br/>                amount: withdraw_proof.amount &#45; withdraw_amount,<br/>                expiration_sec: withdraw_proof.expiration_sec,<br/>            &#125;<br/>        )<br/>    &#125;;<br/><br/>    (<br/>        withdraw_with_event_internal(<br/>            withdraw_proof.token_owner,<br/>            withdraw_proof.token_id,<br/>            withdraw_amount,<br/>        ),<br/>        res<br/>    )<br/><br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code>public fun withdraw_token(account: &amp;signer, id: token::TokenId, amount: u64): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_token(<br/>    account: &amp;signer,<br/>    id: TokenId,<br/>    amount: u64,<br/>): Token acquires TokenStore &#123;<br/>    let account_addr &#61; signer::address_of(account);<br/>    withdraw_with_event_internal(account_addr, id, amount)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_collection"></a>

## Function `create_collection`

Create a new collection to hold tokens


<pre><code>public fun create_collection(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection(<br/>    creator: &amp;signer,<br/>    name: String,<br/>    description: String,<br/>    uri: String,<br/>    maximum: u64,<br/>    mutate_setting: vector&lt;bool&gt;<br/>) acquires Collections &#123;<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));<br/>    let account_addr &#61; signer::address_of(creator);<br/>    if (!exists&lt;Collections&gt;(account_addr)) &#123;<br/>        move_to(<br/>            creator,<br/>            Collections &#123;<br/>                collection_data: table::new(),<br/>                token_data: table::new(),<br/>                create_collection_events: account::new_event_handle&lt;CreateCollectionEvent&gt;(creator),<br/>                create_token_data_events: account::new_event_handle&lt;CreateTokenDataEvent&gt;(creator),<br/>                mint_token_events: account::new_event_handle&lt;MintTokenEvent&gt;(creator),<br/>            &#125;,<br/>        )<br/>    &#125;;<br/><br/>    let collection_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(account_addr).collection_data;<br/><br/>    assert!(<br/>        !table::contains(collection_data, name),<br/>        error::already_exists(ECOLLECTION_ALREADY_EXISTS),<br/>    );<br/><br/>    let mutability_config &#61; create_collection_mutability_config(&amp;mutate_setting);<br/>    let collection &#61; CollectionData &#123;<br/>        description,<br/>        name: name,<br/>        uri,<br/>        supply: 0,<br/>        maximum,<br/>        mutability_config<br/>    &#125;;<br/><br/>    table::add(collection_data, name, collection);<br/>    let collection_handle &#61; borrow_global_mut&lt;Collections&gt;(account_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CreateCollection &#123;<br/>                creator: account_addr,<br/>                collection_name: name,<br/>                uri,<br/>                description,<br/>                maximum,<br/>            &#125;<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;CreateCollectionEvent&gt;(<br/>        &amp;mut collection_handle.create_collection_events,<br/>        CreateCollectionEvent &#123;<br/>            creator: account_addr,<br/>            collection_name: name,<br/>            uri,<br/>            description,<br/>            maximum,<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code>public fun check_collection_exists(creator: address, name: string::String): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun check_collection_exists(creator: address, name: String): bool acquires Collections &#123;<br/>    assert!(<br/>        exists&lt;Collections&gt;(creator),<br/>        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),<br/>    );<br/><br/>    let collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).collection_data;<br/>    table::contains(collection_data, name)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_check_tokendata_exists"></a>

## Function `check_tokendata_exists`



<pre><code>public fun check_tokendata_exists(creator: address, collection_name: string::String, token_name: string::String): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun check_tokendata_exists(creator: address, collection_name: String, token_name: String): bool acquires Collections &#123;<br/>    assert!(<br/>        exists&lt;Collections&gt;(creator),<br/>        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),<br/>    );<br/><br/>    let token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).token_data;<br/>    let token_data_id &#61; create_token_data_id(creator, collection_name, token_name);<br/>    table::contains(token_data, token_data_id)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_tokendata"></a>

## Function `create_tokendata`



<pre><code>public fun create_tokendata(account: &amp;signer, collection: string::String, name: string::String, description: string::String, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: token::TokenMutabilityConfig, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;): token::TokenDataId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_tokendata(<br/>    account: &amp;signer,<br/>    collection: String,<br/>    name: String,<br/>    description: String,<br/>    maximum: u64,<br/>    uri: String,<br/>    royalty_payee_address: address,<br/>    royalty_points_denominator: u64,<br/>    royalty_points_numerator: u64,<br/>    token_mutate_config: TokenMutabilityConfig,<br/>    property_keys: vector&lt;String&gt;,<br/>    property_values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    property_types: vector&lt;String&gt;<br/>): TokenDataId acquires Collections &#123;<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;collection) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));<br/>    assert!(royalty_points_numerator &lt;&#61; royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));<br/><br/>    let account_addr &#61; signer::address_of(account);<br/>    assert!(<br/>        exists&lt;Collections&gt;(account_addr),<br/>        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),<br/>    );<br/>    let collections &#61; borrow_global_mut&lt;Collections&gt;(account_addr);<br/><br/>    let token_data_id &#61; create_token_data_id(account_addr, collection, name);<br/><br/>    assert!(<br/>        table::contains(&amp;collections.collection_data, token_data_id.collection),<br/>        error::not_found(ECOLLECTION_NOT_PUBLISHED),<br/>    );<br/>    assert!(<br/>        !table::contains(&amp;collections.token_data, token_data_id),<br/>        error::already_exists(ETOKEN_DATA_ALREADY_EXISTS),<br/>    );<br/><br/>    let collection &#61; table::borrow_mut(&amp;mut collections.collection_data, token_data_id.collection);<br/><br/>    // if collection maximum &#61;&#61; 0, user don&apos;t want to enforce supply constraint.<br/>    // we don&apos;t track supply to make token creation parallelizable<br/>    if (collection.maximum &gt; 0) &#123;<br/>        collection.supply &#61; collection.supply &#43; 1;<br/>        assert!(<br/>            collection.maximum &gt;&#61; collection.supply,<br/>            error::invalid_argument(ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM),<br/>        );<br/>    &#125;;<br/><br/>    let token_data &#61; TokenData &#123;<br/>        maximum,<br/>        largest_property_version: 0,<br/>        supply: 0,<br/>        uri,<br/>        royalty: create_royalty(royalty_points_numerator, royalty_points_denominator, royalty_payee_address),<br/>        name,<br/>        description,<br/>        default_properties: property_map::new(property_keys, property_values, property_types),<br/>        mutability_config: token_mutate_config,<br/>    &#125;;<br/><br/>    table::add(&amp;mut collections.token_data, token_data_id, token_data);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            CreateTokenData &#123;<br/>                id: token_data_id,<br/>                description,<br/>                maximum,<br/>                uri,<br/>                royalty_payee_address,<br/>                royalty_points_denominator,<br/>                royalty_points_numerator,<br/>                name,<br/>                mutability_config: token_mutate_config,<br/>                property_keys,<br/>                property_values,<br/>                property_types,<br/>            &#125;<br/>        );<br/>    &#125;;<br/><br/>    event::emit_event&lt;CreateTokenDataEvent&gt;(<br/>        &amp;mut collections.create_token_data_events,<br/>        CreateTokenDataEvent &#123;<br/>            id: token_data_id,<br/>            description,<br/>            maximum,<br/>            uri,<br/>            royalty_payee_address,<br/>            royalty_points_denominator,<br/>            royalty_points_numerator,<br/>            name,<br/>            mutability_config: token_mutate_config,<br/>            property_keys,<br/>            property_values,<br/>            property_types,<br/>        &#125;,<br/>    );<br/>    token_data_id<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_supply"></a>

## Function `get_collection_supply`

return the number of distinct token_data_id created under this collection


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: string::String): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: String): Option&lt;u64&gt; acquires Collections &#123;<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/><br/>    if (collection_data.maximum &gt; 0) &#123;<br/>        option::some(collection_data.supply)<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_description"></a>

## Function `get_collection_description`



<pre><code>public fun get_collection_description(creator_address: address, collection_name: string::String): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_description(creator_address: address, collection_name: String): String acquires Collections &#123;<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    collection_data.description<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_uri"></a>

## Function `get_collection_uri`



<pre><code>public fun get_collection_uri(creator_address: address, collection_name: string::String): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_uri(creator_address: address, collection_name: String): String acquires Collections &#123;<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    collection_data.uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_maximum"></a>

## Function `get_collection_maximum`



<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: string::String): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: String): u64 acquires Collections &#123;<br/>    assert_collection_exists(creator_address, collection_name);<br/>    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);<br/>    collection_data.maximum<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_supply"></a>

## Function `get_token_supply`

return the number of distinct token_id created under this TokenData


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: token::TokenDataId): option::Option&lt;u64&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: TokenDataId): Option&lt;u64&gt; acquires Collections &#123;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    let token_data &#61; table::borrow(all_token_data, token_data_id);<br/><br/>    if (token_data.maximum &gt; 0) &#123;<br/>        option::some(token_data.supply)<br/>    &#125; else &#123;<br/>        option::none&lt;u64&gt;()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_largest_property_version"></a>

## Function `get_tokendata_largest_property_version`

return the largest_property_version of this TokenData


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: token::TokenDataId): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: TokenDataId): u64 acquires Collections &#123;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    table::borrow(all_token_data, token_data_id).largest_property_version<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_id"></a>

## Function `get_token_id`

return the TokenId for a given Token


<pre><code>public fun get_token_id(token: &amp;token::Token): token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_id(token: &amp;Token): TokenId &#123;<br/>    token.id<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_direct_transfer"></a>

## Function `get_direct_transfer`



<pre><code>public fun get_direct_transfer(receiver: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_direct_transfer(receiver: address): bool acquires TokenStore &#123;<br/>    if (!exists&lt;TokenStore&gt;(receiver)) &#123;<br/>        return false<br/>    &#125;;<br/><br/>    borrow_global&lt;TokenStore&gt;(receiver).direct_transfer<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_token_mutability_config"></a>

## Function `create_token_mutability_config`



<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::TokenMutabilityConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): TokenMutabilityConfig &#123;<br/>    TokenMutabilityConfig &#123;<br/>        maximum: &#42;vector::borrow(mutate_setting, TOKEN_MAX_MUTABLE_IND),<br/>        uri: &#42;vector::borrow(mutate_setting, TOKEN_URI_MUTABLE_IND),<br/>        royalty: &#42;vector::borrow(mutate_setting, TOKEN_ROYALTY_MUTABLE_IND),<br/>        description: &#42;vector::borrow(mutate_setting, TOKEN_DESCRIPTION_MUTABLE_IND),<br/>        properties: &#42;vector::borrow(mutate_setting, TOKEN_PROPERTY_MUTABLE_IND),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_collection_mutability_config"></a>

## Function `create_collection_mutability_config`



<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::CollectionMutabilityConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): CollectionMutabilityConfig &#123;<br/>    CollectionMutabilityConfig &#123;<br/>        description: &#42;vector::borrow(mutate_setting, COLLECTION_DESCRIPTION_MUTABLE_IND),<br/>        uri: &#42;vector::borrow(mutate_setting, COLLECTION_URI_MUTABLE_IND),<br/>        maximum: &#42;vector::borrow(mutate_setting, COLLECTION_MAX_MUTABLE_IND),<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mint_token"></a>

## Function `mint_token`



<pre><code>public fun mint_token(account: &amp;signer, token_data_id: token::TokenDataId, amount: u64): token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token(<br/>    account: &amp;signer,<br/>    token_data_id: TokenDataId,<br/>    amount: u64,<br/>): TokenId acquires Collections, TokenStore &#123;<br/>    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));<br/>    let creator_addr &#61; token_data_id.creator;<br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/><br/>    if (token_data.maximum &gt; 0) &#123;<br/>        assert!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));<br/>        token_data.supply &#61; token_data.supply &#43; amount;<br/>    &#125;;<br/><br/>    // we add more tokens with property_version 0<br/>    let token_id &#61; create_token_id(token_data_id, 0);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(MintToken &#123; id: token_data_id, amount &#125;)<br/>    &#125;;<br/>    event::emit_event&lt;MintTokenEvent&gt;(<br/>        &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).mint_token_events,<br/>        MintTokenEvent &#123;<br/>            id: token_data_id,<br/>            amount,<br/>        &#125;<br/>    );<br/><br/>    deposit_token(account,<br/>        Token &#123;<br/>            id: token_id,<br/>            amount,<br/>            token_properties: property_map::empty(), // same as default properties no need to store<br/>        &#125;<br/>    );<br/><br/>    token_id<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_mint_token_to"></a>

## Function `mint_token_to`

create tokens and directly deposite to receiver&apos;s address. The receiver should opt&#45;in direct transfer


<pre><code>public fun mint_token_to(account: &amp;signer, receiver: address, token_data_id: token::TokenDataId, amount: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token_to(<br/>    account: &amp;signer,<br/>    receiver: address,<br/>    token_data_id: TokenDataId,<br/>    amount: u64,<br/>) acquires Collections, TokenStore &#123;<br/>    assert!(exists&lt;TokenStore&gt;(receiver), error::not_found(ETOKEN_STORE_NOT_PUBLISHED));<br/>    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(receiver).direct_transfer;<br/>    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));<br/><br/>    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));<br/>    let creator_addr &#61; token_data_id.creator;<br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);<br/><br/>    if (token_data.maximum &gt; 0) &#123;<br/>        assert!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));<br/>        token_data.supply &#61; token_data.supply &#43; amount;<br/>    &#125;;<br/><br/>    // we add more tokens with property_version 0<br/>    let token_id &#61; create_token_id(token_data_id, 0);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(MintToken &#123; id: token_data_id, amount &#125;)<br/>    &#125;;<br/>    event::emit_event&lt;MintTokenEvent&gt;(<br/>        &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).mint_token_events,<br/>        MintTokenEvent &#123;<br/>            id: token_data_id,<br/>            amount,<br/>        &#125;<br/>    );<br/><br/>    direct_deposit(receiver,<br/>        Token &#123;<br/>            id: token_id,<br/>            amount,<br/>            token_properties: property_map::empty(), // same as default properties no need to store<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_token_id"></a>

## Function `create_token_id`



<pre><code>public fun create_token_id(token_data_id: token::TokenDataId, property_version: u64): token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_id(token_data_id: TokenDataId, property_version: u64): TokenId &#123;<br/>    TokenId &#123;<br/>        token_data_id,<br/>        property_version,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_token_data_id"></a>

## Function `create_token_data_id`



<pre><code>public fun create_token_data_id(creator: address, collection: string::String, name: string::String): token::TokenDataId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_data_id(<br/>    creator: address,<br/>    collection: String,<br/>    name: String,<br/>): TokenDataId &#123;<br/>    assert!(string::length(&amp;collection) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));<br/>    assert!(string::length(&amp;name) &lt;&#61; MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));<br/>    TokenDataId &#123; creator, collection, name &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_create_token_id_raw"></a>

## Function `create_token_id_raw`



<pre><code>public fun create_token_id_raw(creator: address, collection: string::String, name: string::String, property_version: u64): token::TokenId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_id_raw(<br/>    creator: address,<br/>    collection: String,<br/>    name: String,<br/>    property_version: u64,<br/>): TokenId &#123;<br/>    TokenId &#123;<br/>        token_data_id: create_token_data_id(creator, collection, name),<br/>        property_version,<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_balance_of"></a>

## Function `balance_of`



<pre><code>public fun balance_of(owner: address, id: token::TokenId): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance_of(owner: address, id: TokenId): u64 acquires TokenStore &#123;<br/>    if (!exists&lt;TokenStore&gt;(owner)) &#123;<br/>        return 0<br/>    &#125;;<br/>    let token_store &#61; borrow_global&lt;TokenStore&gt;(owner);<br/>    if (table::contains(&amp;token_store.tokens, id)) &#123;<br/>        table::borrow(&amp;token_store.tokens, id).amount<br/>    &#125; else &#123;<br/>        0<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_has_token_store"></a>

## Function `has_token_store`



<pre><code>public fun has_token_store(owner: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun has_token_store(owner: address): bool &#123;<br/>    exists&lt;TokenStore&gt;(owner)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_royalty"></a>

## Function `get_royalty`



<pre><code>public fun get_royalty(token_id: token::TokenId): token::Royalty<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty(token_id: TokenId): Royalty acquires Collections &#123;<br/>    let token_data_id &#61; token_id.token_data_id;<br/>    get_tokendata_royalty(token_data_id)<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_royalty_numerator"></a>

## Function `get_royalty_numerator`



<pre><code>public fun get_royalty_numerator(royalty: &amp;token::Royalty): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_numerator(royalty: &amp;Royalty): u64 &#123;<br/>    royalty.royalty_points_numerator<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_royalty_denominator"></a>

## Function `get_royalty_denominator`



<pre><code>public fun get_royalty_denominator(royalty: &amp;token::Royalty): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_denominator(royalty: &amp;Royalty): u64 &#123;<br/>    royalty.royalty_points_denominator<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_royalty_payee"></a>

## Function `get_royalty_payee`



<pre><code>public fun get_royalty_payee(royalty: &amp;token::Royalty): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_payee(royalty: &amp;Royalty): address &#123;<br/>    royalty.payee_address<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_amount"></a>

## Function `get_token_amount`



<pre><code>public fun get_token_amount(token: &amp;token::Token): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_amount(token: &amp;Token): u64 &#123;<br/>    token.amount<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_id_fields"></a>

## Function `get_token_id_fields`

return the creator address, collection name, token name and property_version


<pre><code>public fun get_token_id_fields(token_id: &amp;token::TokenId): (address, string::String, string::String, u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_id_fields(token_id: &amp;TokenId): (address, String, String, u64) &#123;<br/>    (<br/>        token_id.token_data_id.creator,<br/>        token_id.token_data_id.collection,<br/>        token_id.token_data_id.name,<br/>        token_id.property_version,<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_data_id_fields"></a>

## Function `get_token_data_id_fields`



<pre><code>public fun get_token_data_id_fields(token_data_id: &amp;token::TokenDataId): (address, string::String, string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_data_id_fields(token_data_id: &amp;TokenDataId): (address, String, String) &#123;<br/>    (<br/>        token_data_id.creator,<br/>        token_data_id.collection,<br/>        token_data_id.name,<br/>    )<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_property_map"></a>

## Function `get_property_map`

return a copy of the token property map.<br/> if property_version &#61; 0, return the default property map<br/> if property_version &gt; 0, return the property value stored at owner&apos;s token store


<pre><code>public fun get_property_map(owner: address, token_id: token::TokenId): property_map::PropertyMap<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_property_map(owner: address, token_id: TokenId): PropertyMap acquires Collections, TokenStore &#123;<br/>    assert!(balance_of(owner, token_id) &gt; 0, error::not_found(EINSUFFICIENT_BALANCE));<br/>    // if property_version &#61; 0, return default property map<br/>    if (token_id.property_version &#61;&#61; 0) &#123;<br/>        let creator_addr &#61; token_id.token_data_id.creator;<br/>        let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_addr).token_data;<br/>        assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>        let token_data &#61; table::borrow(all_token_data, token_id.token_data_id);<br/>        token_data.default_properties<br/>    &#125; else &#123;<br/>        let tokens &#61; &amp;borrow_global&lt;TokenStore&gt;(owner).tokens;<br/>        table::borrow(tokens, token_id).token_properties<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_maximum"></a>

## Function `get_tokendata_maximum`



<pre><code>public fun get_tokendata_maximum(token_data_id: token::TokenDataId): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_maximum(token_data_id: TokenDataId): u64 acquires Collections &#123;<br/>    let creator_address &#61; token_data_id.creator;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/><br/>    let token_data &#61; table::borrow(all_token_data, token_data_id);<br/>    token_data.maximum<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_uri"></a>

## Function `get_tokendata_uri`



<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: token::TokenDataId): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: TokenDataId): String acquires Collections &#123;<br/>    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/><br/>    let token_data &#61; table::borrow(all_token_data, token_data_id);<br/>    token_data.uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_description"></a>

## Function `get_tokendata_description`



<pre><code>public fun get_tokendata_description(token_data_id: token::TokenDataId): string::String<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_description(token_data_id: TokenDataId): String acquires Collections &#123;<br/>    let creator_address &#61; token_data_id.creator;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/><br/>    let token_data &#61; table::borrow(all_token_data, token_data_id);<br/>    token_data.description<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_royalty"></a>

## Function `get_tokendata_royalty`



<pre><code>public fun get_tokendata_royalty(token_data_id: token::TokenDataId): token::Royalty<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_royalty(token_data_id: TokenDataId): Royalty acquires Collections &#123;<br/>    let creator_address &#61; token_data_id.creator;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/><br/>    let token_data &#61; table::borrow(all_token_data, token_data_id);<br/>    token_data.royalty<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_id"></a>

## Function `get_tokendata_id`

return the token_data_id from the token_id


<pre><code>public fun get_tokendata_id(token_id: token::TokenId): token::TokenDataId<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_id(token_id: TokenId): TokenDataId &#123;<br/>    token_id.token_data_id<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_tokendata_mutability_config"></a>

## Function `get_tokendata_mutability_config`

return the mutation setting of the token


<pre><code>public fun get_tokendata_mutability_config(token_data_id: token::TokenDataId): token::TokenMutabilityConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_mutability_config(token_data_id: TokenDataId): TokenMutabilityConfig acquires Collections &#123;<br/>    let creator_addr &#61; token_data_id.creator;<br/>    assert!(exists&lt;Collections&gt;(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_addr).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>    table::borrow(all_token_data, token_data_id).mutability_config<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_mutability_maximum"></a>

## Function `get_token_mutability_maximum`

return if the token&apos;s maximum is mutable


<pre><code>public fun get_token_mutability_maximum(config: &amp;token::TokenMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_maximum(config: &amp;TokenMutabilityConfig): bool &#123;<br/>    config.maximum<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_mutability_royalty"></a>

## Function `get_token_mutability_royalty`

return if the token royalty is mutable with a token mutability config


<pre><code>public fun get_token_mutability_royalty(config: &amp;token::TokenMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_royalty(config: &amp;TokenMutabilityConfig): bool &#123;<br/>    config.royalty<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_mutability_uri"></a>

## Function `get_token_mutability_uri`

return if the token uri is mutable with a token mutability config


<pre><code>public fun get_token_mutability_uri(config: &amp;token::TokenMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_uri(config: &amp;TokenMutabilityConfig): bool &#123;<br/>    config.uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_mutability_description"></a>

## Function `get_token_mutability_description`

return if the token description is mutable with a token mutability config


<pre><code>public fun get_token_mutability_description(config: &amp;token::TokenMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_description(config: &amp;TokenMutabilityConfig): bool &#123;<br/>    config.description<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_token_mutability_default_properties"></a>

## Function `get_token_mutability_default_properties`

return if the tokendata&apos;s default properties is mutable with a token mutability config


<pre><code>public fun get_token_mutability_default_properties(config: &amp;token::TokenMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_default_properties(config: &amp;TokenMutabilityConfig): bool &#123;<br/>    config.properties<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_config"></a>

## Function `get_collection_mutability_config`

return the collection mutation setting


<pre><code>&#35;[view]<br/>public fun get_collection_mutability_config(creator: address, collection_name: string::String): token::CollectionMutabilityConfig<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_config(<br/>    creator: address,<br/>    collection_name: String<br/>): CollectionMutabilityConfig acquires Collections &#123;<br/>    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).collection_data;<br/>    assert!(table::contains(all_collection_data, collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));<br/>    table::borrow(all_collection_data, collection_name).mutability_config<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_description"></a>

## Function `get_collection_mutability_description`

return if the collection description is mutable with a collection mutability config


<pre><code>public fun get_collection_mutability_description(config: &amp;token::CollectionMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_description(config: &amp;CollectionMutabilityConfig): bool &#123;<br/>    config.description<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_uri"></a>

## Function `get_collection_mutability_uri`

return if the collection uri is mutable with a collection mutability config


<pre><code>public fun get_collection_mutability_uri(config: &amp;token::CollectionMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_uri(config: &amp;CollectionMutabilityConfig): bool &#123;<br/>    config.uri<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_get_collection_mutability_maximum"></a>

## Function `get_collection_mutability_maximum`

return if the collection maximum is mutable with collection mutability config


<pre><code>public fun get_collection_mutability_maximum(config: &amp;token::CollectionMutabilityConfig): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_maximum(config: &amp;CollectionMutabilityConfig): bool &#123;<br/>    config.maximum<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_destroy_token_data"></a>

## Function `destroy_token_data`



<pre><code>fun destroy_token_data(token_data: token::TokenData)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_token_data(token_data: TokenData) &#123;<br/>    let TokenData &#123;<br/>        maximum: _,<br/>        largest_property_version: _,<br/>        supply: _,<br/>        uri: _,<br/>        royalty: _,<br/>        name: _,<br/>        description: _,<br/>        default_properties: _,<br/>        mutability_config: _,<br/>    &#125; &#61; token_data;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_destroy_collection_data"></a>

## Function `destroy_collection_data`



<pre><code>fun destroy_collection_data(collection_data: token::CollectionData)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_collection_data(collection_data: CollectionData) &#123;<br/>    let CollectionData &#123;<br/>        description: _,<br/>        name: _,<br/>        uri: _,<br/>        supply: _,<br/>        maximum: _,<br/>        mutability_config: _,<br/>    &#125; &#61; collection_data;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_withdraw_with_event_internal"></a>

## Function `withdraw_with_event_internal`



<pre><code>fun withdraw_with_event_internal(account_addr: address, id: token::TokenId, amount: u64): token::Token<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_with_event_internal(<br/>    account_addr: address,<br/>    id: TokenId,<br/>    amount: u64,<br/>): Token acquires TokenStore &#123;<br/>    // It does not make sense to withdraw 0 tokens.<br/>    assert!(amount &gt; 0, error::invalid_argument(EWITHDRAW_ZERO));<br/>    // Make sure the account has sufficient tokens to withdraw.<br/>    assert!(balance_of(account_addr, id) &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));<br/><br/>    assert!(<br/>        exists&lt;TokenStore&gt;(account_addr),<br/>        error::not_found(ETOKEN_STORE_NOT_PUBLISHED),<br/>    );<br/><br/>    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(account_addr);<br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Withdraw &#123; id, amount &#125;)<br/>    &#125;;<br/>    event::emit_event&lt;WithdrawEvent&gt;(<br/>        &amp;mut token_store.withdraw_events,<br/>        WithdrawEvent &#123; id, amount &#125;<br/>    );<br/>    let tokens &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(account_addr).tokens;<br/>    assert!(<br/>        table::contains(tokens, id),<br/>        error::not_found(ENO_TOKEN_IN_TOKEN_STORE),<br/>    );<br/>    // balance &gt; amount and amount &gt; 0 indirectly asserted that balance &gt; 0.<br/>    let balance &#61; &amp;mut table::borrow_mut(tokens, id).amount;<br/>    if (&#42;balance &gt; amount) &#123;<br/>        &#42;balance &#61; &#42;balance &#45; amount;<br/>        Token &#123; id, amount, token_properties: property_map::empty() &#125;<br/>    &#125; else &#123;<br/>        table::remove(tokens, id)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_update_token_property_internal"></a>

## Function `update_token_property_internal`



<pre><code>fun update_token_property_internal(token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_token_property_internal(<br/>    token_owner: address,<br/>    token_id: TokenId,<br/>    keys: vector&lt;String&gt;,<br/>    values: vector&lt;vector&lt;u8&gt;&gt;,<br/>    types: vector&lt;String&gt;,<br/>) acquires TokenStore &#123;<br/>    let tokens &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).tokens;<br/>    assert!(table::contains(tokens, token_id), error::not_found(ENO_TOKEN_IN_TOKEN_STORE));<br/><br/>    let value &#61; &amp;mut table::borrow_mut(tokens, token_id).token_properties;<br/>    assert_non_standard_reserved_property(&amp;keys);<br/>    property_map::update_property_map(value, keys, values, types);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_direct_deposit"></a>

## Function `direct_deposit`

Deposit the token balance into the recipients account and emit an event.


<pre><code>fun direct_deposit(account_addr: address, token: token::Token)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun direct_deposit(account_addr: address, token: Token) acquires TokenStore &#123;<br/>    assert!(token.amount &gt; 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));<br/>    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(account_addr);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(Deposit &#123; id: token.id, amount: token.amount &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;DepositEvent&gt;(<br/>        &amp;mut token_store.deposit_events,<br/>        DepositEvent &#123; id: token.id, amount: token.amount &#125;,<br/>    );<br/><br/>    assert!(<br/>        exists&lt;TokenStore&gt;(account_addr),<br/>        error::not_found(ETOKEN_STORE_NOT_PUBLISHED),<br/>    );<br/><br/>    if (!table::contains(&amp;token_store.tokens, token.id)) &#123;<br/>        table::add(&amp;mut token_store.tokens, token.id, token);<br/>    &#125; else &#123;<br/>        let recipient_token &#61; table::borrow_mut(&amp;mut token_store.tokens, token.id);<br/>        merge(recipient_token, token);<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_assert_collection_exists"></a>

## Function `assert_collection_exists`



<pre><code>fun assert_collection_exists(creator_address: address, collection_name: string::String)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_collection_exists(creator_address: address, collection_name: String) acquires Collections &#123;<br/>    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).collection_data;<br/>    assert!(table::contains(all_collection_data, collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_assert_tokendata_exists"></a>

## Function `assert_tokendata_exists`



<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: token::TokenDataId)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: TokenDataId) acquires Collections &#123;<br/>    let creator_addr &#61; token_data_id.creator;<br/>    assert!(signer::address_of(creator) &#61;&#61; creator_addr, error::permission_denied(ENO_MUTATE_CAPABILITY));<br/>    assert!(exists&lt;Collections&gt;(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));<br/>    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;<br/>    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_assert_non_standard_reserved_property"></a>

## Function `assert_non_standard_reserved_property`



<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;String&gt;) &#123;<br/>    vector::for_each_ref(keys, &#124;key&#124; &#123;<br/>        let key: &amp;String &#61; key;<br/>        let length &#61; string::length(key);<br/>        if (length &gt;&#61; 6) &#123;<br/>            let prefix &#61; string::sub_string(&amp;&#42;key, 0, 6);<br/>            assert!(prefix !&#61; string::utf8(b&quot;TOKEN_&quot;), error::permission_denied(EPROPERTY_RESERVED_BY_STANDARD));<br/>        &#125;;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_initialize_token_script"></a>

## Function `initialize_token_script`



<pre><code>public entry fun initialize_token_script(_account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_token_script(_account: &amp;signer) &#123;<br/>    abort 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x3_token_initialize_token"></a>

## Function `initialize_token`



<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: token::TokenId)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: TokenId) &#123;<br/>    abort 0<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_create_collection_script"></a>

### Function `create_collection_script`


<pre><code>public entry fun create_collection_script(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)<br/></code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;<br/> The length of the uri is up to MAX_URI_LENGTH;


<pre><code>pragma aborts_if_is_partial;<br/>include CreateCollectionAbortsIf;<br/></code></pre>



<a id="@Specification_1_create_token_script"></a>

### Function `create_token_script`


<pre><code>public entry fun create_token_script(account: &amp;signer, collection: string::String, name: string::String, description: string::String, balance: u64, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: vector&lt;bool&gt;, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;)<br/></code></pre>


the length of &apos;mutate_setting&apos; should maore than five.<br/> The creator of the TokenDataId is signer.<br/> The token_data_id should exist in the creator&apos;s collections..<br/> The sum of supply and mint Token is less than maximum.


<pre><code>pragma aborts_if_is_partial;<br/>let addr &#61; signer::address_of(account);<br/>let token_data_id &#61; spec_create_tokendata(addr, collection, name);<br/>let creator_addr &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if token_data_id.creator !&#61; addr;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>aborts_if balance &lt;&#61; 0;<br/>include CreateTokenMutabilityConfigAbortsIf;<br/>include CreateTokenMutabilityConfigAbortsIf;<br/></code></pre>




<a id="0x3_token_spec_create_tokendata"></a>


<pre><code>fun spec_create_tokendata(<br/>   creator: address,<br/>   collection: String,<br/>   name: String): TokenDataId &#123;<br/>   TokenDataId &#123; creator, collection, name &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_mint_script"></a>

### Function `mint_script`


<pre><code>public entry fun mint_script(account: &amp;signer, token_data_address: address, collection: string::String, name: string::String, amount: u64)<br/></code></pre>


only creator of the tokendata can mint tokens


<pre><code>pragma aborts_if_is_partial;<br/>let token_data_id &#61; spec_create_token_data_id(<br/>    token_data_address,<br/>    collection,<br/>    name,<br/>);<br/>let addr &#61; signer::address_of(account);<br/>let creator_addr &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if token_data_id.creator !&#61; signer::address_of(account);<br/>include CreateTokenDataIdAbortsIf&#123;<br/>creator: token_data_address,<br/>collection: collection,<br/>name: name<br/>&#125;;<br/>include MintTokenAbortsIf &#123;<br/>token_data_id: token_data_id<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_mutate_token_properties"></a>

### Function `mutate_token_properties`


<pre><code>public entry fun mutate_token_properties(account: &amp;signer, token_owner: address, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, amount: u64, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>


The signer is creator.


<pre><code>pragma aborts_if_is_partial;<br/>let addr &#61; signer::address_of(account);<br/>aborts_if addr !&#61; creator;<br/>include CreateTokenDataIdAbortsIf &#123;<br/>    creator: creator,<br/>    collection: collection_name,<br/>    name: token_name<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_direct_transfer_script"></a>

### Function `direct_transfer_script`


<pre><code>public entry fun direct_transfer_script(sender: &amp;signer, receiver: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>include CreateTokenDataIdAbortsIf&#123;<br/>    creator: creators_address,<br/>    collection: collection,<br/>    name: name<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_opt_in_direct_transfer"></a>

### Function `opt_in_direct_transfer`


<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let addr &#61; signer::address_of(account);<br/>let account_addr &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; MAX_U64;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/></code></pre>



<a id="@Specification_1_transfer_with_opt_in"></a>

### Function `transfer_with_opt_in`


<pre><code>public entry fun transfer_with_opt_in(from: &amp;signer, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, to: address, amount: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>include CreateTokenDataIdAbortsIf&#123;<br/>    creator: creator,<br/>    collection: collection_name,<br/>    name: token_name<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_burn_by_creator"></a>

### Function `burn_by_creator`


<pre><code>public entry fun burn_by_creator(creator: &amp;signer, owner: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let creator_address &#61; signer::address_of(creator);<br/>let token_id &#61; spec_create_token_id_raw(creator_address, collection, name, property_version);<br/>let creator_addr &#61; token_id.token_data_id.creator;<br/>let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_address);<br/>let token_data &#61; table::spec_get(<br/>    collections.token_data,<br/>    token_id.token_data_id,<br/>);<br/>aborts_if amount &lt;&#61; 0;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);<br/>aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_CREATOR));<br/></code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public entry fun burn(owner: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)<br/></code></pre>


The token_data_id should exist in token_data.


<pre><code>pragma aborts_if_is_partial;<br/>let token_id &#61; spec_create_token_id_raw(creators_address, collection, name, property_version);<br/>let creator_addr &#61; token_id.token_data_id.creator;<br/>let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_addr);<br/>let token_data &#61; table::spec_get(<br/>    collections.token_data,<br/>    token_id.token_data_id,<br/>);<br/>include CreateTokenDataIdAbortsIf &#123;<br/>creator: creators_address<br/>&#125;;<br/>aborts_if amount &lt;&#61; 0;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);<br/>aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_OWNER));<br/>aborts_if !string::spec_internal_check_utf8(BURNABLE_BY_OWNER);<br/></code></pre>




<a id="0x3_token_spec_create_token_id_raw"></a>


<pre><code>fun spec_create_token_id_raw(<br/>   creator: address,<br/>   collection: String,<br/>   name: String,<br/>   property_version: u64,<br/>): TokenId &#123;<br/>   let token_data_id &#61; TokenDataId &#123; creator, collection, name &#125;;<br/>   TokenId &#123;<br/>       token_data_id,<br/>       property_version<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_mutate_collection_description"></a>

### Function `mutate_collection_description`


<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: string::String, description: string::String)<br/></code></pre>


The description of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);<br/>include AssertCollectionExistsAbortsIf &#123;<br/>    creator_address: addr,<br/>    collection_name: collection_name<br/>&#125;;<br/>aborts_if !collection_data.mutability_config.description;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_collection_uri"></a>

### Function `mutate_collection_uri`


<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: string::String, uri: string::String)<br/></code></pre>


The uri of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);<br/>aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;<br/>include AssertCollectionExistsAbortsIf &#123;<br/>    creator_address: addr,<br/>    collection_name: collection_name<br/>&#125;;<br/>aborts_if !collection_data.mutability_config.uri;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_collection_maximum"></a>

### Function `mutate_collection_maximum`


<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: string::String, maximum: u64)<br/></code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.<br/> The maximum should more than suply.<br/> The maxium of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);<br/>include AssertCollectionExistsAbortsIf &#123;<br/>    creator_address: addr,<br/>    collection_name: collection_name<br/>&#125;;<br/>aborts_if collection_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;<br/>aborts_if maximum &lt; collection_data.supply;<br/>aborts_if !collection_data.mutability_config.maximum;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_tokendata_maximum"></a>

### Function `mutate_tokendata_maximum`


<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: token::TokenDataId, maximum: u64)<br/></code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.<br/> The maximum should more than suply.<br/> The token maximum is mutable


<pre><code>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>include AssertTokendataExistsAbortsIf;<br/>aborts_if token_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;<br/>aborts_if maximum &lt; token_data.supply;<br/>aborts_if !token_data.mutability_config.maximum;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_tokendata_uri"></a>

### Function `mutate_tokendata_uri`


<pre><code>public fun mutate_tokendata_uri(creator: &amp;signer, token_data_id: token::TokenDataId, uri: string::String)<br/></code></pre>


The length of uri should less than MAX_URI_LENGTH.<br/> The  creator of token_data_id should exist in Collections.<br/> The token uri is mutable


<pre><code>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>include AssertTokendataExistsAbortsIf;<br/>aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;<br/>aborts_if !token_data.mutability_config.uri;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_tokendata_royalty"></a>

### Function `mutate_tokendata_royalty`


<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: token::TokenDataId, royalty: token::Royalty)<br/></code></pre>


The token royalty is mutable


<pre><code>include AssertTokendataExistsAbortsIf;<br/>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if !token_data.mutability_config.royalty;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_tokendata_description"></a>

### Function `mutate_tokendata_description`


<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: token::TokenDataId, description: string::String)<br/></code></pre>


The token description is mutable


<pre><code>include AssertTokendataExistsAbortsIf;<br/>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if !token_data.mutability_config.description;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_mutate_tokendata_property"></a>

### Function `mutate_tokendata_property`


<pre><code>public fun mutate_tokendata_property(creator: &amp;signer, token_data_id: token::TokenDataId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>


The property map is mutable


<pre><code>pragma aborts_if_is_partial;<br/>let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>include AssertTokendataExistsAbortsIf;<br/>aborts_if len(keys) !&#61; len(values);<br/>aborts_if len(keys) !&#61; len(types);<br/>aborts_if !token_data.mutability_config.properties;<br/></code></pre>



<a id="@Specification_1_mutate_one_token"></a>

### Function `mutate_one_token`


<pre><code>public fun mutate_one_token(account: &amp;signer, token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): token::TokenId<br/></code></pre>


The signer is creator.<br/> The token_data_id should exist in token_data.<br/> The property map is mutable.


<pre><code>pragma aborts_if_is_partial;<br/>let creator &#61; token_id.token_data_id.creator;<br/>let addr &#61; signer::address_of(account);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_id.token_data_id);<br/>aborts_if addr !&#61; creator;<br/>aborts_if !exists&lt;Collections&gt;(creator);<br/>aborts_if !table::spec_contains(all_token_data, token_id.token_data_id);<br/>aborts_if !token_data.mutability_config.properties &amp;&amp; !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(TOKEN_PROPERTY_MUTABLE));<br/></code></pre>



<a id="@Specification_1_create_royalty"></a>

### Function `create_royalty`


<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): token::Royalty<br/></code></pre>




<pre><code>include CreateRoyaltyAbortsIf;<br/></code></pre>


The royalty_points_numerator should less than royalty_points_denominator.


<a id="0x3_token_CreateRoyaltyAbortsIf"></a>


<pre><code>schema CreateRoyaltyAbortsIf &#123;<br/>royalty_points_numerator: u64;<br/>royalty_points_denominator: u64;<br/>payee_address: address;<br/>aborts_if royalty_points_numerator &gt; royalty_points_denominator;<br/>aborts_if !exists&lt;account::Account&gt;(payee_address);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_deposit_token"></a>

### Function `deposit_token`


<pre><code>public fun deposit_token(account: &amp;signer, token: token::Token)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>pragma aborts_if_is_partial;<br/>let account_addr &#61; signer::address_of(account);<br/>include !exists&lt;TokenStore&gt;(account_addr) &#61;&#61;&gt; InitializeTokenStore;<br/>let token_id &#61; token.id;<br/>let token_amount &#61; token.amount;<br/>include DirectDepositAbortsIf;<br/></code></pre>



<a id="@Specification_1_direct_deposit_with_opt_in"></a>

### Function `direct_deposit_with_opt_in`


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: token::Token)<br/></code></pre>


The token can direct_transfer.


<pre><code>let opt_in_transfer &#61; global&lt;TokenStore&gt;(account_addr).direct_transfer;<br/>aborts_if !exists&lt;TokenStore&gt;(account_addr);<br/>aborts_if !opt_in_transfer;<br/>let token_id &#61; token.id;<br/>let token_amount &#61; token.amount;<br/>include DirectDepositAbortsIf;<br/></code></pre>



<a id="@Specification_1_direct_transfer"></a>

### Function `direct_transfer`


<pre><code>public fun direct_transfer(sender: &amp;signer, receiver: &amp;signer, token_id: token::TokenId, amount: u64)<br/></code></pre>


Cannot withdraw 0 tokens.<br/> Make sure the account has sufficient tokens to withdraw.


<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_initialize_token_store"></a>

### Function `initialize_token_store`


<pre><code>public fun initialize_token_store(account: &amp;signer)<br/></code></pre>




<pre><code>include InitializeTokenStore;<br/></code></pre>




<a id="0x3_token_InitializeTokenStore"></a>


<pre><code>schema InitializeTokenStore &#123;<br/>account: signer;<br/>let addr &#61; signer::address_of(account);<br/>let account_addr &#61; global&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code>public fun merge(dst_token: &amp;mut token::Token, source_token: token::Token)<br/></code></pre>




<pre><code>aborts_if dst_token.id !&#61; source_token.id;<br/>aborts_if dst_token.amount &#43; source_token.amount &gt; MAX_U64;<br/></code></pre>



<a id="@Specification_1_split"></a>

### Function `split`


<pre><code>public fun split(dst_token: &amp;mut token::Token, amount: u64): token::Token<br/></code></pre>




<pre><code>aborts_if dst_token.id.property_version !&#61; 0;<br/>aborts_if dst_token.amount &lt;&#61; amount;<br/>aborts_if amount &lt;&#61; 0;<br/></code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public fun transfer(from: &amp;signer, id: token::TokenId, to: address, amount: u64)<br/></code></pre>




<pre><code>let opt_in_transfer &#61; global&lt;TokenStore&gt;(to).direct_transfer;<br/>let account_addr &#61; signer::address_of(from);<br/>aborts_if !opt_in_transfer;<br/>pragma aborts_if_is_partial;<br/>include WithdrawWithEventInternalAbortsIf;<br/></code></pre>



<a id="@Specification_1_withdraw_with_capability"></a>

### Function `withdraw_with_capability`


<pre><code>public fun withdraw_with_capability(withdraw_proof: token::WithdrawCapability): token::Token<br/></code></pre>




<pre><code>let now_seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR &gt; withdraw_proof.expiration_sec;<br/>include WithdrawWithEventInternalAbortsIf&#123;<br/>account_addr: withdraw_proof.token_owner,<br/>id: withdraw_proof.token_id,<br/>amount: withdraw_proof.amount&#125;;<br/></code></pre>



<a id="@Specification_1_partial_withdraw_with_capability"></a>

### Function `partial_withdraw_with_capability`


<pre><code>public fun partial_withdraw_with_capability(withdraw_proof: token::WithdrawCapability, withdraw_amount: u64): (token::Token, option::Option&lt;token::WithdrawCapability&gt;)<br/></code></pre>




<pre><code>let now_seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds;<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR &gt; withdraw_proof.expiration_sec;<br/>aborts_if withdraw_amount &gt; withdraw_proof.amount;<br/>include WithdrawWithEventInternalAbortsIf&#123;<br/>    account_addr: withdraw_proof.token_owner,<br/>    id: withdraw_proof.token_id,<br/>    amount: withdraw_amount<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_withdraw_token"></a>

### Function `withdraw_token`


<pre><code>public fun withdraw_token(account: &amp;signer, id: token::TokenId, amount: u64): token::Token<br/></code></pre>


Cannot withdraw 0 tokens.<br/> Make sure the account has sufficient tokens to withdraw.


<pre><code>let account_addr &#61; signer::address_of(account);<br/>include WithdrawWithEventInternalAbortsIf;<br/></code></pre>



<a id="@Specification_1_create_collection"></a>

### Function `create_collection`


<pre><code>public fun create_collection(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)<br/></code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;<br/> The length of the uri is up to MAX_URI_LENGTH;<br/> The collection_data should not exist before you create it.


<pre><code>pragma aborts_if_is_partial;<br/>let account_addr &#61; signer::address_of(creator);<br/>aborts_if len(name.bytes) &gt; 128;<br/>aborts_if len(uri.bytes) &gt; 512;<br/>include CreateCollectionAbortsIf;<br/></code></pre>




<a id="0x3_token_CreateCollectionAbortsIf"></a>


<pre><code>schema CreateCollectionAbortsIf &#123;<br/>creator: signer;<br/>name: String;<br/>description: String;<br/>uri: String;<br/>maximum: u64;<br/>mutate_setting: vector&lt;bool&gt;;<br/>let addr &#61; signer::address_of(creator);<br/>let account &#61; global&lt;account::Account&gt;(addr);<br/>let collection &#61; global&lt;Collections&gt;(addr);<br/>let b &#61; !exists&lt;Collections&gt;(addr);<br/>let collection_data &#61; global&lt;Collections&gt;(addr).collection_data;<br/>aborts_if b &amp;&amp; !exists&lt;account::Account&gt;(addr);<br/>aborts_if len(name.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;<br/>aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;<br/>aborts_if b &amp;&amp; account.guid_creation_num &#43; 3 &gt;&#61; account::MAX_GUID_CREATION_NUM;<br/>aborts_if b &amp;&amp; account.guid_creation_num &#43; 3 &gt; MAX_U64;<br/>include CreateCollectionMutabilityConfigAbortsIf;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_check_collection_exists"></a>

### Function `check_collection_exists`


<pre><code>public fun check_collection_exists(creator: address, name: string::String): bool<br/></code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator);<br/></code></pre>



<a id="@Specification_1_check_tokendata_exists"></a>

### Function `check_tokendata_exists`


<pre><code>public fun check_tokendata_exists(creator: address, collection_name: string::String, token_name: string::String): bool<br/></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH<br/> The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>aborts_if !exists&lt;Collections&gt;(creator);<br/>include CreateTokenDataIdAbortsIf &#123;<br/>    creator: creator,<br/>    collection: collection_name,<br/>    name: token_name<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_create_tokendata"></a>

### Function `create_tokendata`


<pre><code>public fun create_tokendata(account: &amp;signer, collection: string::String, name: string::String, description: string::String, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: token::TokenMutabilityConfig, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;): token::TokenDataId<br/></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH<br/> The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>pragma verify &#61; false;<br/>pragma aborts_if_is_partial;<br/>let account_addr &#61; signer::address_of(account);<br/>let collections &#61; global&lt;Collections&gt;(account_addr);<br/>let token_data_id &#61; spec_create_token_data_id(account_addr, collection, name);<br/>let Collection &#61; table::spec_get(collections.collection_data, token_data_id.collection);<br/>let length &#61; len(property_keys);<br/>aborts_if len(name.bytes) &gt; MAX_NFT_NAME_LENGTH;<br/>aborts_if len(collection.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;<br/>aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;<br/>aborts_if royalty_points_numerator &gt; royalty_points_denominator;<br/>aborts_if !exists&lt;Collections&gt;(account_addr);<br/>include CreateTokenDataIdAbortsIf &#123;<br/>    creator: account_addr,<br/>    collection: collection,<br/>    name: name<br/>&#125;;<br/>aborts_if !table::spec_contains(collections.collection_data, collection);<br/>aborts_if table::spec_contains(collections.token_data, token_data_id);<br/>aborts_if Collection.maximum &gt; 0 &amp;&amp; Collection.supply &#43; 1 &gt; MAX_U64;<br/>aborts_if Collection.maximum &gt; 0 &amp;&amp; Collection.maximum &lt; Collection.supply &#43; 1;<br/>include CreateRoyaltyAbortsIf &#123;<br/>    payee_address: royalty_payee_address<br/>&#125;;<br/>aborts_if length &gt; property_map::MAX_PROPERTY_MAP_SIZE;<br/>aborts_if length !&#61; len(property_values);<br/>aborts_if length !&#61; len(property_types);<br/></code></pre>




<a id="0x3_token_spec_create_token_data_id"></a>


<pre><code>fun spec_create_token_data_id(<br/>   creator: address,<br/>   collection: String,<br/>   name: String,<br/>): TokenDataId &#123;<br/>   TokenDataId &#123; creator, collection, name &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_collection_supply"></a>

### Function `get_collection_supply`


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: string::String): option::Option&lt;u64&gt;<br/></code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_get_collection_description"></a>

### Function `get_collection_description`


<pre><code>public fun get_collection_description(creator_address: address, collection_name: string::String): string::String<br/></code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_get_collection_uri"></a>

### Function `get_collection_uri`


<pre><code>public fun get_collection_uri(creator_address: address, collection_name: string::String): string::String<br/></code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_get_collection_maximum"></a>

### Function `get_collection_maximum`


<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: string::String): u64<br/></code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;<br/></code></pre>



<a id="@Specification_1_get_token_supply"></a>

### Function `get_token_supply`


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: token::TokenDataId): option::Option&lt;u64&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_get_tokendata_largest_property_version"></a>

### Function `get_tokendata_largest_property_version`


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: token::TokenDataId): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_create_token_mutability_config"></a>

### Function `create_token_mutability_config`


<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::TokenMutabilityConfig<br/></code></pre>


The length of &apos;mutate_setting&apos; should more than five.<br/> The mutate_setting shuold have a value.


<pre><code>include CreateTokenMutabilityConfigAbortsIf;<br/></code></pre>




<a id="0x3_token_CreateTokenMutabilityConfigAbortsIf"></a>


<pre><code>schema CreateTokenMutabilityConfigAbortsIf &#123;<br/>mutate_setting: vector&lt;bool&gt;;<br/>aborts_if len(mutate_setting) &lt; 5;<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_MAX_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_URI_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_ROYALTY_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_DESCRIPTION_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_PROPERTY_MUTABLE_IND]);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_collection_mutability_config"></a>

### Function `create_collection_mutability_config`


<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::CollectionMutabilityConfig<br/></code></pre>




<pre><code>include CreateCollectionMutabilityConfigAbortsIf;<br/></code></pre>




<a id="0x3_token_CreateCollectionMutabilityConfigAbortsIf"></a>


<pre><code>schema CreateCollectionMutabilityConfigAbortsIf &#123;<br/>mutate_setting: vector&lt;bool&gt;;<br/>aborts_if len(mutate_setting) &lt; 3;<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_DESCRIPTION_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_URI_MUTABLE_IND]);<br/>aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_MAX_MUTABLE_IND]);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_mint_token"></a>

### Function `mint_token`


<pre><code>public fun mint_token(account: &amp;signer, token_data_id: token::TokenDataId, amount: u64): token::TokenId<br/></code></pre>


The creator of the TokenDataId is signer.<br/> The token_data_id should exist in the creator&apos;s collections..<br/> The sum of supply and the amount of mint Token is less than maximum.


<pre><code>pragma verify &#61; false;<br/></code></pre>




<a id="0x3_token_MintTokenAbortsIf"></a>


<pre><code>schema MintTokenAbortsIf &#123;<br/>account: signer;<br/>token_data_id: TokenDataId;<br/>amount: u64;<br/>let addr &#61; signer::address_of(account);<br/>let creator_addr &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if token_data_id.creator !&#61; addr;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/>aborts_if token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>aborts_if amount &lt;&#61; 0;<br/>include InitializeTokenStore;<br/>let token_id &#61; create_token_id(token_data_id, 0);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_mint_token_to"></a>

### Function `mint_token_to`


<pre><code>public fun mint_token_to(account: &amp;signer, receiver: address, token_data_id: token::TokenDataId, amount: u64)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(account);<br/>let opt_in_transfer &#61; global&lt;TokenStore&gt;(receiver).direct_transfer;<br/>let creator_addr &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>let token_data &#61; table::spec_get(all_token_data, token_data_id);<br/>aborts_if !exists&lt;TokenStore&gt;(receiver);<br/>aborts_if !opt_in_transfer;<br/>aborts_if token_data_id.creator !&#61; addr;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/>aborts_if token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;<br/>aborts_if amount &lt;&#61; 0;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>let token_id &#61; create_token_id(token_data_id, 0);<br/>include DirectDepositAbortsIf &#123;<br/>    account_addr: receiver,<br/>    token_id: token_id,<br/>    token_amount: amount,<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_create_token_data_id"></a>

### Function `create_token_data_id`


<pre><code>public fun create_token_data_id(creator: address, collection: string::String, name: string::String): token::TokenDataId<br/></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH<br/> The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>include CreateTokenDataIdAbortsIf;<br/></code></pre>




<a id="0x3_token_CreateTokenDataIdAbortsIf"></a>


<pre><code>schema CreateTokenDataIdAbortsIf &#123;<br/>creator: address;<br/>collection: String;<br/>name: String;<br/>aborts_if len(collection.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;<br/>aborts_if len(name.bytes) &gt; MAX_NFT_NAME_LENGTH;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_token_id_raw"></a>

### Function `create_token_id_raw`


<pre><code>public fun create_token_id_raw(creator: address, collection: string::String, name: string::String, property_version: u64): token::TokenId<br/></code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH<br/> The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>include CreateTokenDataIdAbortsIf;<br/></code></pre>




<a id="0x3_token_spec_balance_of"></a>


<pre><code>fun spec_balance_of(owner: address, id: TokenId): u64 &#123;<br/>   let token_store &#61; borrow_global&lt;TokenStore&gt;(owner);<br/>   if (!exists&lt;TokenStore&gt;(owner)) &#123;<br/>       0<br/>   &#125;<br/>   else if (table::spec_contains(token_store.tokens, id)) &#123;<br/>       table::spec_get(token_store.tokens, id).amount<br/>   &#125; else &#123;<br/>       0<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_royalty"></a>

### Function `get_royalty`


<pre><code>public fun get_royalty(token_id: token::TokenId): token::Royalty<br/></code></pre>




<pre><code>include GetTokendataRoyaltyAbortsIf &#123;<br/>    token_data_id: token_id.token_data_id<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_get_property_map"></a>

### Function `get_property_map`


<pre><code>public fun get_property_map(owner: address, token_id: token::TokenId): property_map::PropertyMap<br/></code></pre>




<pre><code>let creator_addr &#61; token_id.token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>aborts_if spec_balance_of(owner, token_id) &lt;&#61; 0;<br/>aborts_if token_id.property_version &#61;&#61; 0 &amp;&amp; !table::spec_contains(all_token_data, token_id.token_data_id);<br/>aborts_if token_id.property_version &#61;&#61; 0 &amp;&amp; !exists&lt;Collections&gt;(creator_addr);<br/></code></pre>



<a id="@Specification_1_get_tokendata_maximum"></a>

### Function `get_tokendata_maximum`


<pre><code>public fun get_tokendata_maximum(token_data_id: token::TokenDataId): u64<br/></code></pre>




<pre><code>let creator_address &#61; token_data_id.creator;<br/>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_get_tokendata_uri"></a>

### Function `get_tokendata_uri`


<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: token::TokenDataId): string::String<br/></code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_get_tokendata_description"></a>

### Function `get_tokendata_description`


<pre><code>public fun get_tokendata_description(token_data_id: token::TokenDataId): string::String<br/></code></pre>




<pre><code>let creator_address &#61; token_data_id.creator;<br/>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_get_tokendata_royalty"></a>

### Function `get_tokendata_royalty`


<pre><code>public fun get_tokendata_royalty(token_data_id: token::TokenDataId): token::Royalty<br/></code></pre>




<pre><code>include GetTokendataRoyaltyAbortsIf;<br/></code></pre>




<a id="0x3_token_GetTokendataRoyaltyAbortsIf"></a>


<pre><code>schema GetTokendataRoyaltyAbortsIf &#123;<br/>token_data_id: TokenDataId;<br/>let creator_address &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;<br/>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_tokendata_mutability_config"></a>

### Function `get_tokendata_mutability_config`


<pre><code>public fun get_tokendata_mutability_config(token_data_id: token::TokenDataId): token::TokenMutabilityConfig<br/></code></pre>




<pre><code>let creator_addr &#61; token_data_id.creator;<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/></code></pre>



<a id="@Specification_1_get_collection_mutability_config"></a>

### Function `get_collection_mutability_config`


<pre><code>&#35;[view]<br/>public fun get_collection_mutability_config(creator: address, collection_name: string::String): token::CollectionMutabilityConfig<br/></code></pre>




<pre><code>let all_collection_data &#61; global&lt;Collections&gt;(creator).collection_data;<br/>aborts_if !exists&lt;Collections&gt;(creator);<br/>aborts_if !table::spec_contains(all_collection_data, collection_name);<br/></code></pre>



<a id="@Specification_1_withdraw_with_event_internal"></a>

### Function `withdraw_with_event_internal`


<pre><code>fun withdraw_with_event_internal(account_addr: address, id: token::TokenId, amount: u64): token::Token<br/></code></pre>




<pre><code>include WithdrawWithEventInternalAbortsIf;<br/></code></pre>




<a id="0x3_token_WithdrawWithEventInternalAbortsIf"></a>


<pre><code>schema WithdrawWithEventInternalAbortsIf &#123;<br/>account_addr: address;<br/>id: TokenId;<br/>amount: u64;<br/>let tokens &#61; global&lt;TokenStore&gt;(account_addr).tokens;<br/>aborts_if amount &lt;&#61; 0;<br/>aborts_if spec_balance_of(account_addr, id) &lt; amount;<br/>aborts_if !exists&lt;TokenStore&gt;(account_addr);<br/>aborts_if !table::spec_contains(tokens, id);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_update_token_property_internal"></a>

### Function `update_token_property_internal`


<pre><code>fun update_token_property_internal(token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let tokens &#61; global&lt;TokenStore&gt;(token_owner).tokens;<br/>aborts_if !exists&lt;TokenStore&gt;(token_owner);<br/>aborts_if !table::spec_contains(tokens, token_id);<br/></code></pre>



<a id="@Specification_1_direct_deposit"></a>

### Function `direct_deposit`


<pre><code>fun direct_deposit(account_addr: address, token: token::Token)<br/></code></pre>




<pre><code>let token_id &#61; token.id;<br/>let token_amount &#61; token.amount;<br/>include DirectDepositAbortsIf;<br/></code></pre>




<a id="0x3_token_DirectDepositAbortsIf"></a>


<pre><code>schema DirectDepositAbortsIf &#123;<br/>account_addr: address;<br/>token_id: TokenId;<br/>token_amount: u64;<br/>let token_store &#61; global&lt;TokenStore&gt;(account_addr);<br/>let recipient_token &#61; table::spec_get(token_store.tokens, token_id);<br/>let b &#61; table::spec_contains(token_store.tokens, token_id);<br/>aborts_if token_amount &lt;&#61; 0;<br/>aborts_if !exists&lt;TokenStore&gt;(account_addr);<br/>aborts_if b &amp;&amp; recipient_token.id !&#61; token_id;<br/>aborts_if b &amp;&amp; recipient_token.amount &#43; token_amount &gt; MAX_U64;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_assert_collection_exists"></a>

### Function `assert_collection_exists`


<pre><code>fun assert_collection_exists(creator_address: address, collection_name: string::String)<br/></code></pre>


The collection_name should exist in collection_data of the creator_address&apos;s Collections.


<pre><code>include AssertCollectionExistsAbortsIf;<br/></code></pre>




<a id="0x3_token_AssertCollectionExistsAbortsIf"></a>


<pre><code>schema AssertCollectionExistsAbortsIf &#123;<br/>creator_address: address;<br/>collection_name: String;<br/>let all_collection_data &#61; global&lt;Collections&gt;(creator_address).collection_data;<br/>aborts_if !exists&lt;Collections&gt;(creator_address);<br/>aborts_if !table::spec_contains(all_collection_data, collection_name);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_assert_tokendata_exists"></a>

### Function `assert_tokendata_exists`


<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: token::TokenDataId)<br/></code></pre>


The creator of token_data_id should be signer.<br/> The  creator of token_data_id exists in Collections.<br/> The token_data_id is in the all_token_data.


<pre><code>include AssertTokendataExistsAbortsIf;<br/></code></pre>




<a id="0x3_token_AssertTokendataExistsAbortsIf"></a>


<pre><code>schema AssertTokendataExistsAbortsIf &#123;<br/>creator: signer;<br/>token_data_id: TokenDataId;<br/>let creator_addr &#61; token_data_id.creator;<br/>let addr &#61; signer::address_of(creator);<br/>aborts_if addr !&#61; creator_addr;<br/>aborts_if !exists&lt;Collections&gt;(creator_addr);<br/>let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;<br/>aborts_if !table::spec_contains(all_token_data, token_data_id);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_assert_non_standard_reserved_property"></a>

### Function `assert_non_standard_reserved_property`


<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;string::String&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_initialize_token_script"></a>

### Function `initialize_token_script`


<pre><code>public entry fun initialize_token_script(_account: &amp;signer)<br/></code></pre>


Deprecated function


<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_initialize_token"></a>

### Function `initialize_token`


<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: token::TokenId)<br/></code></pre>


Deprecated function


<pre><code>pragma verify &#61; false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
