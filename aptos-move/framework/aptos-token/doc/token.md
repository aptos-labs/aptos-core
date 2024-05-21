
<a id="0x3_token"></a>

# Module `0x3::token`

This module provides the foundation for Tokens.
Checkout our developer doc on our token standard https://aptos.dev/standards


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


<pre><code>use 0x1::account;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x1::table;
use 0x1::timestamp;
use 0x3::property_map;
use 0x3::token_event_store;
</code></pre>



<a id="0x3_token_Token"></a>

## Struct `Token`



<pre><code>struct Token has store
</code></pre>



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
 the amount of tokens. Only property_version = 0 can have a value bigger than 1.
</dd>
<dt>
<code>token_properties: property_map::PropertyMap</code>
</dt>
<dd>
 The properties with this token.
 when property_version = 0, the token_properties are the same as default_properties in TokenData, we don't store it.
 when the property_map mutates, a new property_version is assigned to the token.
</dd>
</dl>


</details>

<a id="0x3_token_TokenId"></a>

## Struct `TokenId`

global unique identifier of a token


<pre><code>struct TokenId has copy, drop, store
</code></pre>



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


<pre><code>struct TokenDataId has copy, drop, store
</code></pre>



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
 The name of collection; this is unique under the same account, eg: "Aptos Animal Collection"
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


<pre><code>struct TokenData has store
</code></pre>



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
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul-7Mv6vzm7JwE69O-I-vd6Bz2QriJO1niwCh4
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
 The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: "Aptos Animal #1234"
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


<pre><code>struct Royalty has copy, drop, store
</code></pre>



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
 if the token is jointly owned by multiple creators, the group of creators should create a shared account.
 the payee_address will be the shared account address.
</dd>
</dl>


</details>

<a id="0x3_token_TokenMutabilityConfig"></a>

## Struct `TokenMutabilityConfig`

This config specifies which fields in the TokenData are mutable


<pre><code>struct TokenMutabilityConfig has copy, drop, store
</code></pre>



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


<pre><code>struct TokenStore has key
</code></pre>



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


<pre><code>struct CollectionMutabilityConfig has copy, drop, store
</code></pre>



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


<pre><code>struct Collections has key
</code></pre>



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


<pre><code>struct CollectionData has store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: string::String</code>
</dt>
<dd>
 A description for the token collection Eg: "Aptos Toad Overload"
</dd>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: "Animal Collection"
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
 If maximal is a non-zero value, the number of created TokenData entries should be smaller or equal to this maximum
 If maximal is 0, Aptos doesn't track the supply of this collection, and there is no limit
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

capability to withdraw without signer, this struct should be non-copyable


<pre><code>struct WithdrawCapability has drop, store
</code></pre>



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


<pre><code>struct DepositEvent has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct Deposit has drop, store
</code></pre>



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


<pre><code>struct WithdrawEvent has drop, store
</code></pre>



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


<pre><code>&#35;[event]
struct Withdraw has drop, store
</code></pre>



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


<pre><code>struct CreateTokenDataEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct CreateTokenData has drop, store
</code></pre>



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


<pre><code>struct MintTokenEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct MintToken has drop, store
</code></pre>



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



<pre><code>struct BurnTokenEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct BurnToken has drop, store
</code></pre>



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



<pre><code>struct MutateTokenPropertyMapEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct MutateTokenPropertyMap has drop, store
</code></pre>



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


<pre><code>struct CreateCollectionEvent has drop, store
</code></pre>



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



<pre><code>&#35;[event]
struct CreateCollection has drop, store
</code></pre>



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


<pre><code>const EINSUFFICIENT_BALANCE: u64 &#61; 5;
</code></pre>



<a id="0x3_token_EURI_TOO_LONG"></a>

The URI is too long


<pre><code>const EURI_TOO_LONG: u64 &#61; 27;
</code></pre>



<a id="0x3_token_MAX_URI_LENGTH"></a>



<pre><code>const MAX_URI_LENGTH: u64 &#61; 512;
</code></pre>



<a id="0x3_token_BURNABLE_BY_CREATOR"></a>



<pre><code>const BURNABLE_BY_CREATOR: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 67, 82, 69, 65, 84, 79, 82];
</code></pre>



<a id="0x3_token_BURNABLE_BY_OWNER"></a>



<pre><code>const BURNABLE_BY_OWNER: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 79, 87, 78, 69, 82];
</code></pre>



<a id="0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND"></a>



<pre><code>const COLLECTION_DESCRIPTION_MUTABLE_IND: u64 &#61; 0;
</code></pre>



<a id="0x3_token_COLLECTION_MAX_MUTABLE_IND"></a>



<pre><code>const COLLECTION_MAX_MUTABLE_IND: u64 &#61; 2;
</code></pre>



<a id="0x3_token_COLLECTION_URI_MUTABLE_IND"></a>



<pre><code>const COLLECTION_URI_MUTABLE_IND: u64 &#61; 1;
</code></pre>



<a id="0x3_token_EALREADY_HAS_BALANCE"></a>

The token has balance and cannot be initialized


<pre><code>const EALREADY_HAS_BALANCE: u64 &#61; 0;
</code></pre>



<a id="0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY"></a>

Reserved fields for token contract
Cannot be updated by user


<pre><code>const ECANNOT_UPDATE_RESERVED_PROPERTY: u64 &#61; 32;
</code></pre>



<a id="0x3_token_ECOLLECTIONS_NOT_PUBLISHED"></a>

There isn't any collection under this account


<pre><code>const ECOLLECTIONS_NOT_PUBLISHED: u64 &#61; 1;
</code></pre>



<a id="0x3_token_ECOLLECTION_ALREADY_EXISTS"></a>

The collection already exists


<pre><code>const ECOLLECTION_ALREADY_EXISTS: u64 &#61; 3;
</code></pre>



<a id="0x3_token_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is too long


<pre><code>const ECOLLECTION_NAME_TOO_LONG: u64 &#61; 25;
</code></pre>



<a id="0x3_token_ECOLLECTION_NOT_PUBLISHED"></a>

Cannot find collection in creator's account


<pre><code>const ECOLLECTION_NOT_PUBLISHED: u64 &#61; 2;
</code></pre>



<a id="0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM"></a>

Exceeds the collection's maximal number of token_data


<pre><code>const ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM: u64 &#61; 4;
</code></pre>



<a id="0x3_token_ECREATOR_CANNOT_BURN_TOKEN"></a>

Token is not burnable by creator


<pre><code>const ECREATOR_CANNOT_BURN_TOKEN: u64 &#61; 31;
</code></pre>



<a id="0x3_token_EFIELD_NOT_MUTABLE"></a>

The field is not mutable


<pre><code>const EFIELD_NOT_MUTABLE: u64 &#61; 13;
</code></pre>



<a id="0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT"></a>

Withdraw capability doesn't have sufficient amount


<pre><code>const EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT: u64 &#61; 38;
</code></pre>



<a id="0x3_token_EINVALID_MAXIMUM"></a>

Collection or tokendata maximum must be larger than supply


<pre><code>const EINVALID_MAXIMUM: u64 &#61; 36;
</code></pre>



<a id="0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR"></a>

Royalty invalid if the numerator is larger than the denominator


<pre><code>const EINVALID_ROYALTY_NUMERATOR_DENOMINATOR: u64 &#61; 34;
</code></pre>



<a id="0x3_token_EINVALID_TOKEN_MERGE"></a>

Cannot merge the two tokens with different token id


<pre><code>const EINVALID_TOKEN_MERGE: u64 &#61; 6;
</code></pre>



<a id="0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM"></a>

Exceed the token data maximal allowed


<pre><code>const EMINT_WOULD_EXCEED_TOKEN_MAXIMUM: u64 &#61; 7;
</code></pre>



<a id="0x3_token_ENFT_NAME_TOO_LONG"></a>

The NFT name is too long


<pre><code>const ENFT_NAME_TOO_LONG: u64 &#61; 26;
</code></pre>



<a id="0x3_token_ENFT_NOT_SPLITABLE"></a>

Cannot split a token that only has 1 amount


<pre><code>const ENFT_NOT_SPLITABLE: u64 &#61; 18;
</code></pre>



<a id="0x3_token_ENO_BURN_CAPABILITY"></a>

No burn capability


<pre><code>const ENO_BURN_CAPABILITY: u64 &#61; 8;
</code></pre>



<a id="0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot burn 0 Token


<pre><code>const ENO_BURN_TOKEN_WITH_ZERO_AMOUNT: u64 &#61; 29;
</code></pre>



<a id="0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot deposit a Token with 0 amount


<pre><code>const ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT: u64 &#61; 28;
</code></pre>



<a id="0x3_token_ENO_MINT_CAPABILITY"></a>

No mint capability


<pre><code>const ENO_MINT_CAPABILITY: u64 &#61; 19;
</code></pre>



<a id="0x3_token_ENO_MUTATE_CAPABILITY"></a>

Not authorized to mutate


<pre><code>const ENO_MUTATE_CAPABILITY: u64 &#61; 14;
</code></pre>



<a id="0x3_token_ENO_TOKEN_IN_TOKEN_STORE"></a>

Token not in the token store


<pre><code>const ENO_TOKEN_IN_TOKEN_STORE: u64 &#61; 15;
</code></pre>



<a id="0x3_token_EOWNER_CANNOT_BURN_TOKEN"></a>

Token is not burnable by owner


<pre><code>const EOWNER_CANNOT_BURN_TOKEN: u64 &#61; 30;
</code></pre>



<a id="0x3_token_EPROPERTY_RESERVED_BY_STANDARD"></a>

The property is reserved by token standard


<pre><code>const EPROPERTY_RESERVED_BY_STANDARD: u64 &#61; 40;
</code></pre>



<a id="0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST"></a>

Royalty payee account does not exist


<pre><code>const EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST: u64 &#61; 35;
</code></pre>



<a id="0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT"></a>

TOKEN with 0 amount is not allowed


<pre><code>const ETOKEN_CANNOT_HAVE_ZERO_AMOUNT: u64 &#61; 33;
</code></pre>



<a id="0x3_token_ETOKEN_DATA_ALREADY_EXISTS"></a>

TokenData already exists


<pre><code>const ETOKEN_DATA_ALREADY_EXISTS: u64 &#61; 9;
</code></pre>



<a id="0x3_token_ETOKEN_DATA_NOT_PUBLISHED"></a>

TokenData not published


<pre><code>const ETOKEN_DATA_NOT_PUBLISHED: u64 &#61; 10;
</code></pre>



<a id="0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH"></a>

Token Properties count doesn't match


<pre><code>const ETOKEN_PROPERTIES_COUNT_NOT_MATCH: u64 &#61; 37;
</code></pre>



<a id="0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT"></a>

Cannot split token to an amount larger than its amount


<pre><code>const ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT: u64 &#61; 12;
</code></pre>



<a id="0x3_token_ETOKEN_STORE_NOT_PUBLISHED"></a>

TokenStore doesn't exist


<pre><code>const ETOKEN_STORE_NOT_PUBLISHED: u64 &#61; 11;
</code></pre>



<a id="0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER"></a>

User didn't opt-in direct transfer


<pre><code>const EUSER_NOT_OPT_IN_DIRECT_TRANSFER: u64 &#61; 16;
</code></pre>



<a id="0x3_token_EWITHDRAW_PROOF_EXPIRES"></a>

Withdraw proof expires


<pre><code>const EWITHDRAW_PROOF_EXPIRES: u64 &#61; 39;
</code></pre>



<a id="0x3_token_EWITHDRAW_ZERO"></a>

Cannot withdraw 0 token


<pre><code>const EWITHDRAW_ZERO: u64 &#61; 17;
</code></pre>



<a id="0x3_token_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code>const MAX_COLLECTION_NAME_LENGTH: u64 &#61; 128;
</code></pre>



<a id="0x3_token_MAX_NFT_NAME_LENGTH"></a>



<pre><code>const MAX_NFT_NAME_LENGTH: u64 &#61; 128;
</code></pre>



<a id="0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND"></a>



<pre><code>const TOKEN_DESCRIPTION_MUTABLE_IND: u64 &#61; 3;
</code></pre>



<a id="0x3_token_TOKEN_MAX_MUTABLE_IND"></a>



<pre><code>const TOKEN_MAX_MUTABLE_IND: u64 &#61; 0;
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE"></a>



<pre><code>const TOKEN_PROPERTY_MUTABLE: vector&lt;u8&gt; &#61; [84, 79, 75, 69, 78, 95, 80, 82, 79, 80, 69, 82, 84, 89, 95, 77, 85, 84, 65, 84, 66, 76, 69];
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE_IND"></a>



<pre><code>const TOKEN_PROPERTY_MUTABLE_IND: u64 &#61; 4;
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND"></a>



<pre><code>const TOKEN_PROPERTY_VALUE_MUTABLE_IND: u64 &#61; 5;
</code></pre>



<a id="0x3_token_TOKEN_ROYALTY_MUTABLE_IND"></a>



<pre><code>const TOKEN_ROYALTY_MUTABLE_IND: u64 &#61; 2;
</code></pre>



<a id="0x3_token_TOKEN_URI_MUTABLE_IND"></a>



<pre><code>const TOKEN_URI_MUTABLE_IND: u64 &#61; 1;
</code></pre>



<a id="0x3_token_create_collection_script"></a>

## Function `create_collection_script`

create a empty token collection with parameters


<pre><code>public entry fun create_collection_script(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_collection_script(
    creator: &amp;signer,
    name: String,
    description: String,
    uri: String,
    maximum: u64,
    mutate_setting: vector&lt;bool&gt;,
) acquires Collections &#123;
    create_collection(
        creator,
        name,
        description,
        uri,
        maximum,
        mutate_setting
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_create_token_script"></a>

## Function `create_token_script`

create token with raw inputs


<pre><code>public entry fun create_token_script(account: &amp;signer, collection: string::String, name: string::String, description: string::String, balance: u64, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: vector&lt;bool&gt;, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun create_token_script(
    account: &amp;signer,
    collection: String,
    name: String,
    description: String,
    balance: u64,
    maximum: u64,
    uri: String,
    royalty_payee_address: address,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    mutate_setting: vector&lt;bool&gt;,
    property_keys: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
    property_types: vector&lt;String&gt;
) acquires Collections, TokenStore &#123;
    let token_mut_config &#61; create_token_mutability_config(&amp;mutate_setting);
    let tokendata_id &#61; create_tokendata(
        account,
        collection,
        name,
        description,
        maximum,
        uri,
        royalty_payee_address,
        royalty_points_denominator,
        royalty_points_numerator,
        token_mut_config,
        property_keys,
        property_values,
        property_types
    );

    mint_token(
        account,
        tokendata_id,
        balance,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_mint_script"></a>

## Function `mint_script`

Mint more token from an existing token_data. Mint only adds more token to property_version 0


<pre><code>public entry fun mint_script(account: &amp;signer, token_data_address: address, collection: string::String, name: string::String, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mint_script(
    account: &amp;signer,
    token_data_address: address,
    collection: String,
    name: String,
    amount: u64,
) acquires Collections, TokenStore &#123;
    let token_data_id &#61; create_token_data_id(
        token_data_address,
        collection,
        name,
    );
    // only creator of the tokendata can mint more tokens for now
    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
    mint_token(
        account,
        token_data_id,
        amount,
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_token_properties"></a>

## Function `mutate_token_properties`

mutate the token property and save the new property in TokenStore
if the token property_version is 0, we will create a new property_version per token to generate a new token_id per token
if the token property_version is not 0, we will just update the propertyMap and use the existing token_id (property_version)


<pre><code>public entry fun mutate_token_properties(account: &amp;signer, token_owner: address, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, amount: u64, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun mutate_token_properties(
    account: &amp;signer,
    token_owner: address,
    creator: address,
    collection_name: String,
    token_name: String,
    token_property_version: u64,
    amount: u64,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;,
) acquires Collections, TokenStore &#123;
    assert!(signer::address_of(account) &#61;&#61; creator, error::not_found(ENO_MUTATE_CAPABILITY));
    let i &#61; 0;
    let token_id &#61; create_token_id_raw(
        creator,
        collection_name,
        token_name,
        token_property_version,
    );
    // give a new property_version for each token
    while (i &lt; amount) &#123;
        mutate_one_token(account, token_owner, token_id, keys, values, types);
        i &#61; i &#43; 1;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x3_token_direct_transfer_script"></a>

## Function `direct_transfer_script`



<pre><code>public entry fun direct_transfer_script(sender: &amp;signer, receiver: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun direct_transfer_script(
    sender: &amp;signer,
    receiver: &amp;signer,
    creators_address: address,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) acquires TokenStore &#123;
    let token_id &#61; create_token_id_raw(creators_address, collection, name, property_version);
    direct_transfer(sender, receiver, token_id, amount);
&#125;
</code></pre>



</details>

<a id="0x3_token_opt_in_direct_transfer"></a>

## Function `opt_in_direct_transfer`



<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool) acquires TokenStore &#123;
    let addr &#61; signer::address_of(account);
    initialize_token_store(account);
    let opt_in_flag &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(addr).direct_transfer;
    &#42;opt_in_flag &#61; opt_in;
    token_event_store::emit_token_opt_in_event(account, opt_in);
&#125;
</code></pre>



</details>

<a id="0x3_token_transfer_with_opt_in"></a>

## Function `transfer_with_opt_in`

Transfers <code>amount</code> of tokens from <code>from</code> to <code>to</code>.
The receiver <code>to</code> has to opt-in direct transfer first


<pre><code>public entry fun transfer_with_opt_in(from: &amp;signer, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, to: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun transfer_with_opt_in(
    from: &amp;signer,
    creator: address,
    collection_name: String,
    token_name: String,
    token_property_version: u64,
    to: address,
    amount: u64,
) acquires TokenStore &#123;
    let token_id &#61; create_token_id_raw(creator, collection_name, token_name, token_property_version);
    transfer(from, token_id, to, amount);
&#125;
</code></pre>



</details>

<a id="0x3_token_burn_by_creator"></a>

## Function `burn_by_creator`

Burn a token by creator when the token's BURNABLE_BY_CREATOR is true
The token is owned at address owner


<pre><code>public entry fun burn_by_creator(creator: &amp;signer, owner: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn_by_creator(
    creator: &amp;signer,
    owner: address,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) acquires Collections, TokenStore &#123;
    let creator_address &#61; signer::address_of(creator);
    assert!(amount &gt; 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));
    let token_id &#61; create_token_id_raw(creator_address, collection, name, property_version);
    let creator_addr &#61; token_id.token_data_id.creator;
    assert!(
        exists&lt;Collections&gt;(creator_addr),
        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
    );

    let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_address);
    assert!(
        table::contains(&amp;collections.token_data, token_id.token_data_id),
        error::not_found(ETOKEN_DATA_NOT_PUBLISHED),
    );

    let token_data &#61; table::borrow_mut(
        &amp;mut collections.token_data,
        token_id.token_data_id,
    );

    // The property should be explicitly set in the property_map for creator to burn the token
    assert!(
        property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_CREATOR)),
        error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN)
    );

    let burn_by_creator_flag &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_CREATOR));
    assert!(burn_by_creator_flag, error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN));

    // Burn the tokens.
    let Token &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; withdraw_with_event_internal(owner, token_id, amount);
    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(owner);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(BurnToken &#123; id: token_id, amount: burned_amount &#125;);
    &#125;;
    event::emit_event&lt;BurnTokenEvent&gt;(
        &amp;mut token_store.burn_events,
        BurnTokenEvent &#123; id: token_id, amount: burned_amount &#125;
    );

    if (token_data.maximum &gt; 0) &#123;
        token_data.supply &#61; token_data.supply &#45; burned_amount;

        // Delete the token_data if supply drops to 0.
        if (token_data.supply &#61;&#61; 0) &#123;
            destroy_token_data(table::remove(&amp;mut collections.token_data, token_id.token_data_id));

            // update the collection supply
            let collection_data &#61; table::borrow_mut(
                &amp;mut collections.collection_data,
                token_id.token_data_id.collection
            );
            if (collection_data.maximum &gt; 0) &#123;
                collection_data.supply &#61; collection_data.supply &#45; 1;
                // delete the collection data if the collection supply equals 0
                if (collection_data.supply &#61;&#61; 0) &#123;
                    destroy_collection_data(table::remove(&amp;mut collections.collection_data, collection_data.name));
                &#125;;
            &#125;;
        &#125;;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x3_token_burn"></a>

## Function `burn`

Burn a token by the token owner


<pre><code>public entry fun burn(owner: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun burn(
    owner: &amp;signer,
    creators_address: address,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64
) acquires Collections, TokenStore &#123;
    assert!(amount &gt; 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));
    let token_id &#61; create_token_id_raw(creators_address, collection, name, property_version);
    let creator_addr &#61; token_id.token_data_id.creator;
    assert!(
        exists&lt;Collections&gt;(creator_addr),
        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
    );

    let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_addr);
    assert!(
        table::contains(&amp;collections.token_data, token_id.token_data_id),
        error::not_found(ETOKEN_DATA_NOT_PUBLISHED),
    );

    let token_data &#61; table::borrow_mut(
        &amp;mut collections.token_data,
        token_id.token_data_id,
    );

    assert!(
        property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_OWNER)),
        error::permission_denied(EOWNER_CANNOT_BURN_TOKEN)
    );
    let burn_by_owner_flag &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(BURNABLE_BY_OWNER));
    assert!(burn_by_owner_flag, error::permission_denied(EOWNER_CANNOT_BURN_TOKEN));

    // Burn the tokens.
    let Token &#123; id: _, amount: burned_amount, token_properties: _ &#125; &#61; withdraw_token(owner, token_id, amount);
    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(signer::address_of(owner));
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(BurnToken &#123; id: token_id, amount: burned_amount &#125;);
    &#125;;
    event::emit_event&lt;BurnTokenEvent&gt;(
        &amp;mut token_store.burn_events,
        BurnTokenEvent &#123; id: token_id, amount: burned_amount &#125;
    );

    // Decrease the supply correspondingly by the amount of tokens burned.
    let token_data &#61; table::borrow_mut(
        &amp;mut collections.token_data,
        token_id.token_data_id,
    );

    // only update the supply if we tracking the supply and maximal
    // maximal &#61;&#61; 0 is reserved for unlimited token and collection with no tracking info.
    if (token_data.maximum &gt; 0) &#123;
        token_data.supply &#61; token_data.supply &#45; burned_amount;

        // Delete the token_data if supply drops to 0.
        if (token_data.supply &#61;&#61; 0) &#123;
            destroy_token_data(table::remove(&amp;mut collections.token_data, token_id.token_data_id));

            // update the collection supply
            let collection_data &#61; table::borrow_mut(
                &amp;mut collections.collection_data,
                token_id.token_data_id.collection
            );

            // only update and check the supply for unlimited collection
            if (collection_data.maximum &gt; 0)&#123;
                collection_data.supply &#61; collection_data.supply &#45; 1;
                // delete the collection data if the collection supply equals 0
                if (collection_data.supply &#61;&#61; 0) &#123;
                    destroy_collection_data(table::remove(&amp;mut collections.collection_data, collection_data.name));
                &#125;;
            &#125;;
        &#125;;
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_collection_description"></a>

## Function `mutate_collection_description`



<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: string::String, description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: String, description: String) acquires Collections &#123;
    let creator_address &#61; signer::address_of(creator);
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    assert!(collection_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_collection_description_mutate_event(creator, collection_name, collection_data.description, description);
    collection_data.description &#61; description;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_collection_uri"></a>

## Function `mutate_collection_uri`



<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: string::String, uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: String, uri: String) acquires Collections &#123;
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
    let creator_address &#61; signer::address_of(creator);
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    assert!(collection_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_collection_uri_mutate_event(creator, collection_name, collection_data.uri , uri);
    collection_data.uri &#61; uri;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_collection_maximum"></a>

## Function `mutate_collection_maximum`



<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: string::String, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: String, maximum: u64) acquires Collections &#123;
    let creator_address &#61; signer::address_of(creator);
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    // cannot change maximum from 0 and cannot change maximum to 0
    assert!(collection_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, error::invalid_argument(EINVALID_MAXIMUM));
    assert!(maximum &gt;&#61; collection_data.supply, error::invalid_argument(EINVALID_MAXIMUM));
    assert!(collection_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_collection_maximum_mutate_event(creator, collection_name, collection_data.maximum, maximum);
    collection_data.maximum &#61; maximum;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_maximum"></a>

## Function `mutate_tokendata_maximum`



<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: token::TokenDataId, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: TokenDataId, maximum: u64) acquires Collections &#123;
    assert_tokendata_exists(creator, token_data_id);
    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);
    // cannot change maximum from 0 and cannot change maximum to 0
    assert!(token_data.maximum !&#61; 0 &amp;&amp; maximum !&#61; 0, error::invalid_argument(EINVALID_MAXIMUM));
    assert!(maximum &gt;&#61; token_data.supply, error::invalid_argument(EINVALID_MAXIMUM));
    assert!(token_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_token_maximum_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.maximum, maximum);
    token_data.maximum &#61; maximum;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_uri"></a>

## Function `mutate_tokendata_uri`



<pre><code>public fun mutate_tokendata_uri(creator: &amp;signer, token_data_id: token::TokenDataId, uri: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_uri(
    creator: &amp;signer,
    token_data_id: TokenDataId,
    uri: String
) acquires Collections &#123;
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
    assert_tokendata_exists(creator, token_data_id);

    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);
    assert!(token_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_token_uri_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.uri ,uri);
    token_data.uri &#61; uri;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_royalty"></a>

## Function `mutate_tokendata_royalty`



<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: token::TokenDataId, royalty: token::Royalty)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: TokenDataId, royalty: Royalty) acquires Collections &#123;
    assert_tokendata_exists(creator, token_data_id);

    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);
    assert!(token_data.mutability_config.royalty, error::permission_denied(EFIELD_NOT_MUTABLE));

    token_event_store::emit_token_royalty_mutate_event(
        creator,
        token_data_id.collection,
        token_data_id.name,
        token_data.royalty.royalty_points_numerator,
        token_data.royalty.royalty_points_denominator,
        token_data.royalty.payee_address,
        royalty.royalty_points_numerator,
        royalty.royalty_points_denominator,
        royalty.payee_address
    );
    token_data.royalty &#61; royalty;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_description"></a>

## Function `mutate_tokendata_description`



<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: token::TokenDataId, description: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: TokenDataId, description: String) acquires Collections &#123;
    assert_tokendata_exists(creator, token_data_id);

    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);
    assert!(token_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));
    token_event_store::emit_token_descrition_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.description, description);
    token_data.description &#61; description;
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_property"></a>

## Function `mutate_tokendata_property`

Allow creator to mutate the default properties in TokenData


<pre><code>public fun mutate_tokendata_property(creator: &amp;signer, token_data_id: token::TokenDataId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_tokendata_property(
    creator: &amp;signer,
    token_data_id: TokenDataId,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;,
) acquires Collections &#123;
    assert_tokendata_exists(creator, token_data_id);
    let key_len &#61; vector::length(&amp;keys);
    let val_len &#61; vector::length(&amp;values);
    let typ_len &#61; vector::length(&amp;types);
    assert!(key_len &#61;&#61; val_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));
    assert!(key_len &#61;&#61; typ_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));

    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(token_data_id.creator).token_data;
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);
    assert!(token_data.mutability_config.properties, error::permission_denied(EFIELD_NOT_MUTABLE));
    let i: u64 &#61; 0;
    let old_values: vector&lt;Option&lt;PropertyValue&gt;&gt; &#61; vector::empty();
    let new_values: vector&lt;PropertyValue&gt; &#61; vector::empty();
    assert_non_standard_reserved_property(&amp;keys);
    while (i &lt; vector::length(&amp;keys))&#123;
        let key &#61; vector::borrow(&amp;keys, i);
        let old_pv &#61; if (property_map::contains_key(&amp;token_data.default_properties, key)) &#123;
            option::some(&#42;property_map::borrow(&amp;token_data.default_properties, key))
        &#125; else &#123;
            option::none&lt;PropertyValue&gt;()
        &#125;;
        vector::push_back(&amp;mut old_values, old_pv);
        let new_pv &#61; property_map::create_property_value_raw(&#42;vector::borrow(&amp;values, i), &#42;vector::borrow(&amp;types, i));
        vector::push_back(&amp;mut new_values, new_pv);
        if (option::is_some(&amp;old_pv)) &#123;
            property_map::update_property_value(&amp;mut token_data.default_properties, key, new_pv);
        &#125; else &#123;
            property_map::add(&amp;mut token_data.default_properties, &#42;key, new_pv);
        &#125;;
        i &#61; i &#43; 1;
    &#125;;
    token_event_store::emit_default_property_mutate_event(creator, token_data_id.collection, token_data_id.name, keys, old_values, new_values);
&#125;
</code></pre>



</details>

<a id="0x3_token_mutate_one_token"></a>

## Function `mutate_one_token`

Mutate the token_properties of one token.


<pre><code>public fun mutate_one_token(account: &amp;signer, token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mutate_one_token(
    account: &amp;signer,
    token_owner: address,
    token_id: TokenId,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;,
): TokenId acquires Collections, TokenStore &#123;
    let creator &#61; token_id.token_data_id.creator;
    assert!(signer::address_of(account) &#61;&#61; creator, error::permission_denied(ENO_MUTATE_CAPABILITY));
    // validate if the properties is mutable
    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(
        creator
    ).token_data;

    assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    let token_data &#61; table::borrow_mut(all_token_data, token_id.token_data_id);

    // if default property is mutatable, token property is alwasy mutable
    // we only need to check TOKEN_PROPERTY_MUTABLE when default property is immutable
    if (!token_data.mutability_config.properties) &#123;
        assert!(
            property_map::contains_key(&amp;token_data.default_properties, &amp;string::utf8(TOKEN_PROPERTY_MUTABLE)),
            error::permission_denied(EFIELD_NOT_MUTABLE)
        );

        let token_prop_mutable &#61; property_map::read_bool(&amp;token_data.default_properties, &amp;string::utf8(TOKEN_PROPERTY_MUTABLE));
        assert!(token_prop_mutable, error::permission_denied(EFIELD_NOT_MUTABLE));
    &#125;;

    // check if the property_version is 0 to determine if we need to update the property_version
    if (token_id.property_version &#61;&#61; 0) &#123;
        let token &#61; withdraw_with_event_internal(token_owner, token_id, 1);
        // give a new property_version for each token
        let cur_property_version &#61; token_data.largest_property_version &#43; 1;
        let new_token_id &#61; create_token_id(token_id.token_data_id, cur_property_version);
        let new_token &#61; Token &#123;
            id: new_token_id,
            amount: 1,
            token_properties: token_data.default_properties,
        &#125;;
        direct_deposit(token_owner, new_token);
        update_token_property_internal(token_owner, new_token_id, keys, values, types);
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(MutateTokenPropertyMap &#123;
                old_id: token_id,
                new_id: new_token_id,
                keys,
                values,
                types
            &#125;);
        &#125;;
        event::emit_event&lt;MutateTokenPropertyMapEvent&gt;(
            &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).mutate_token_property_events,
            MutateTokenPropertyMapEvent &#123;
                old_id: token_id,
                new_id: new_token_id,
                keys,
                values,
                types
            &#125;,
        );

        token_data.largest_property_version &#61; cur_property_version;
        // burn the orignial property_version 0 token after mutation
        let Token &#123; id: _, amount: _, token_properties: _ &#125; &#61; token;
        new_token_id
    &#125; else &#123;
        // only 1 copy for the token with property verion bigger than 0
        update_token_property_internal(token_owner, token_id, keys, values, types);
        if (std::features::module_event_migration_enabled()) &#123;
            event::emit(MutateTokenPropertyMap &#123;
                old_id: token_id,
                new_id: token_id,
                keys,
                values,
                types
            &#125;);
        &#125;;
        event::emit_event&lt;MutateTokenPropertyMapEvent&gt;(
            &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).mutate_token_property_events,
            MutateTokenPropertyMapEvent &#123;
                old_id: token_id,
                new_id: token_id,
                keys,
                values,
                types
            &#125;,
        );
        token_id
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_create_royalty"></a>

## Function `create_royalty`



<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): token::Royalty
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): Royalty &#123;
    assert!(royalty_points_numerator &lt;&#61; royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));
    assert!(account::exists_at(payee_address), error::invalid_argument(EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST));
    Royalty &#123;
        royalty_points_numerator,
        royalty_points_denominator,
        payee_address
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_deposit_token"></a>

## Function `deposit_token`

Deposit the token balance into the owner's account and emit an event.


<pre><code>public fun deposit_token(account: &amp;signer, token: token::Token)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun deposit_token(account: &amp;signer, token: Token) acquires TokenStore &#123;
    let account_addr &#61; signer::address_of(account);
    initialize_token_store(account);
    direct_deposit(account_addr, token)
&#125;
</code></pre>



</details>

<a id="0x3_token_direct_deposit_with_opt_in"></a>

## Function `direct_deposit_with_opt_in`

direct deposit if user opt in direct transfer


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: token::Token)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: Token) acquires TokenStore &#123;
    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(account_addr).direct_transfer;
    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));
    direct_deposit(account_addr, token);
&#125;
</code></pre>



</details>

<a id="0x3_token_direct_transfer"></a>

## Function `direct_transfer`



<pre><code>public fun direct_transfer(sender: &amp;signer, receiver: &amp;signer, token_id: token::TokenId, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun direct_transfer(
    sender: &amp;signer,
    receiver: &amp;signer,
    token_id: TokenId,
    amount: u64,
) acquires TokenStore &#123;
    let token &#61; withdraw_token(sender, token_id, amount);
    deposit_token(receiver, token);
&#125;
</code></pre>



</details>

<a id="0x3_token_initialize_token_store"></a>

## Function `initialize_token_store`



<pre><code>public fun initialize_token_store(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_token_store(account: &amp;signer) &#123;
    if (!exists&lt;TokenStore&gt;(signer::address_of(account))) &#123;
        move_to(
            account,
            TokenStore &#123;
                tokens: table::new(),
                direct_transfer: false,
                deposit_events: account::new_event_handle&lt;DepositEvent&gt;(account),
                withdraw_events: account::new_event_handle&lt;WithdrawEvent&gt;(account),
                burn_events: account::new_event_handle&lt;BurnTokenEvent&gt;(account),
                mutate_token_property_events: account::new_event_handle&lt;MutateTokenPropertyMapEvent&gt;(account),
            &#125;,
        );
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_merge"></a>

## Function `merge`



<pre><code>public fun merge(dst_token: &amp;mut token::Token, source_token: token::Token)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun merge(dst_token: &amp;mut Token, source_token: Token) &#123;
    assert!(&amp;dst_token.id &#61;&#61; &amp;source_token.id, error::invalid_argument(EINVALID_TOKEN_MERGE));
    dst_token.amount &#61; dst_token.amount &#43; source_token.amount;
    let Token &#123; id: _, amount: _, token_properties: _ &#125; &#61; source_token;
&#125;
</code></pre>



</details>

<a id="0x3_token_split"></a>

## Function `split`



<pre><code>public fun split(dst_token: &amp;mut token::Token, amount: u64): token::Token
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun split(dst_token: &amp;mut Token, amount: u64): Token &#123;
    assert!(dst_token.id.property_version &#61;&#61; 0, error::invalid_state(ENFT_NOT_SPLITABLE));
    assert!(dst_token.amount &gt; amount, error::invalid_argument(ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT));
    assert!(amount &gt; 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));
    dst_token.amount &#61; dst_token.amount &#45; amount;
    Token &#123;
        id: dst_token.id,
        amount,
        token_properties: property_map::empty(),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_token_id"></a>

## Function `token_id`



<pre><code>public fun token_id(token: &amp;token::Token): &amp;token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun token_id(token: &amp;Token): &amp;TokenId &#123;
    &amp;token.id
&#125;
</code></pre>



</details>

<a id="0x3_token_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of tokens from <code>from</code> to <code>to</code>.


<pre><code>public fun transfer(from: &amp;signer, id: token::TokenId, to: address, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer(
    from: &amp;signer,
    id: TokenId,
    to: address,
    amount: u64,
) acquires TokenStore &#123;
    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(to).direct_transfer;
    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));
    let token &#61; withdraw_token(from, id, amount);
    direct_deposit(to, token);
&#125;
</code></pre>



</details>

<a id="0x3_token_create_withdraw_capability"></a>

## Function `create_withdraw_capability`

Token owner can create this one-time withdraw capability with an expiration time


<pre><code>public fun create_withdraw_capability(owner: &amp;signer, token_id: token::TokenId, amount: u64, expiration_sec: u64): token::WithdrawCapability
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_withdraw_capability(
    owner: &amp;signer,
    token_id: TokenId,
    amount: u64,
    expiration_sec: u64,
): WithdrawCapability &#123;
    WithdrawCapability &#123;
        token_owner: signer::address_of(owner),
        token_id,
        amount,
        expiration_sec,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_withdraw_with_capability"></a>

## Function `withdraw_with_capability`

Withdraw the token with a capability


<pre><code>public fun withdraw_with_capability(withdraw_proof: token::WithdrawCapability): token::Token
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_with_capability(
    withdraw_proof: WithdrawCapability,
): Token acquires TokenStore &#123;
    // verify the delegation hasn&apos;t expired yet
    assert!(timestamp::now_seconds() &lt;&#61; withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));

    withdraw_with_event_internal(
        withdraw_proof.token_owner,
        withdraw_proof.token_id,
        withdraw_proof.amount,
    )
&#125;
</code></pre>



</details>

<a id="0x3_token_partial_withdraw_with_capability"></a>

## Function `partial_withdraw_with_capability`

Withdraw the token with a capability.


<pre><code>public fun partial_withdraw_with_capability(withdraw_proof: token::WithdrawCapability, withdraw_amount: u64): (token::Token, option::Option&lt;token::WithdrawCapability&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun partial_withdraw_with_capability(
    withdraw_proof: WithdrawCapability,
    withdraw_amount: u64,
): (Token, Option&lt;WithdrawCapability&gt;) acquires TokenStore &#123;
    // verify the delegation hasn&apos;t expired yet
    assert!(timestamp::now_seconds() &lt;&#61; withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));

    assert!(withdraw_amount &lt;&#61; withdraw_proof.amount, error::invalid_argument(EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT));

    let res: Option&lt;WithdrawCapability&gt; &#61; if (withdraw_amount &#61;&#61; withdraw_proof.amount) &#123;
        option::none&lt;WithdrawCapability&gt;()
    &#125; else &#123;
        option::some(
            WithdrawCapability &#123;
                token_owner: withdraw_proof.token_owner,
                token_id: withdraw_proof.token_id,
                amount: withdraw_proof.amount &#45; withdraw_amount,
                expiration_sec: withdraw_proof.expiration_sec,
            &#125;
        )
    &#125;;

    (
        withdraw_with_event_internal(
            withdraw_proof.token_owner,
            withdraw_proof.token_id,
            withdraw_amount,
        ),
        res
    )

&#125;
</code></pre>



</details>

<a id="0x3_token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code>public fun withdraw_token(account: &amp;signer, id: token::TokenId, amount: u64): token::Token
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun withdraw_token(
    account: &amp;signer,
    id: TokenId,
    amount: u64,
): Token acquires TokenStore &#123;
    let account_addr &#61; signer::address_of(account);
    withdraw_with_event_internal(account_addr, id, amount)
&#125;
</code></pre>



</details>

<a id="0x3_token_create_collection"></a>

## Function `create_collection`

Create a new collection to hold tokens


<pre><code>public fun create_collection(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection(
    creator: &amp;signer,
    name: String,
    description: String,
    uri: String,
    maximum: u64,
    mutate_setting: vector&lt;bool&gt;
) acquires Collections &#123;
    assert!(string::length(&amp;name) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
    let account_addr &#61; signer::address_of(creator);
    if (!exists&lt;Collections&gt;(account_addr)) &#123;
        move_to(
            creator,
            Collections &#123;
                collection_data: table::new(),
                token_data: table::new(),
                create_collection_events: account::new_event_handle&lt;CreateCollectionEvent&gt;(creator),
                create_token_data_events: account::new_event_handle&lt;CreateTokenDataEvent&gt;(creator),
                mint_token_events: account::new_event_handle&lt;MintTokenEvent&gt;(creator),
            &#125;,
        )
    &#125;;

    let collection_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(account_addr).collection_data;

    assert!(
        !table::contains(collection_data, name),
        error::already_exists(ECOLLECTION_ALREADY_EXISTS),
    );

    let mutability_config &#61; create_collection_mutability_config(&amp;mutate_setting);
    let collection &#61; CollectionData &#123;
        description,
        name: name,
        uri,
        supply: 0,
        maximum,
        mutability_config
    &#125;;

    table::add(collection_data, name, collection);
    let collection_handle &#61; borrow_global_mut&lt;Collections&gt;(account_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CreateCollection &#123;
                creator: account_addr,
                collection_name: name,
                uri,
                description,
                maximum,
            &#125;
        );
    &#125;;
    event::emit_event&lt;CreateCollectionEvent&gt;(
        &amp;mut collection_handle.create_collection_events,
        CreateCollectionEvent &#123;
            creator: account_addr,
            collection_name: name,
            uri,
            description,
            maximum,
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code>public fun check_collection_exists(creator: address, name: string::String): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun check_collection_exists(creator: address, name: String): bool acquires Collections &#123;
    assert!(
        exists&lt;Collections&gt;(creator),
        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
    );

    let collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).collection_data;
    table::contains(collection_data, name)
&#125;
</code></pre>



</details>

<a id="0x3_token_check_tokendata_exists"></a>

## Function `check_tokendata_exists`



<pre><code>public fun check_tokendata_exists(creator: address, collection_name: string::String, token_name: string::String): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun check_tokendata_exists(creator: address, collection_name: String, token_name: String): bool acquires Collections &#123;
    assert!(
        exists&lt;Collections&gt;(creator),
        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
    );

    let token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).token_data;
    let token_data_id &#61; create_token_data_id(creator, collection_name, token_name);
    table::contains(token_data, token_data_id)
&#125;
</code></pre>



</details>

<a id="0x3_token_create_tokendata"></a>

## Function `create_tokendata`



<pre><code>public fun create_tokendata(account: &amp;signer, collection: string::String, name: string::String, description: string::String, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: token::TokenMutabilityConfig, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;): token::TokenDataId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_tokendata(
    account: &amp;signer,
    collection: String,
    name: String,
    description: String,
    maximum: u64,
    uri: String,
    royalty_payee_address: address,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    token_mutate_config: TokenMutabilityConfig,
    property_keys: vector&lt;String&gt;,
    property_values: vector&lt;vector&lt;u8&gt;&gt;,
    property_types: vector&lt;String&gt;
): TokenDataId acquires Collections &#123;
    assert!(string::length(&amp;name) &lt;&#61; MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));
    assert!(string::length(&amp;collection) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
    assert!(string::length(&amp;uri) &lt;&#61; MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
    assert!(royalty_points_numerator &lt;&#61; royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));

    let account_addr &#61; signer::address_of(account);
    assert!(
        exists&lt;Collections&gt;(account_addr),
        error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
    );
    let collections &#61; borrow_global_mut&lt;Collections&gt;(account_addr);

    let token_data_id &#61; create_token_data_id(account_addr, collection, name);

    assert!(
        table::contains(&amp;collections.collection_data, token_data_id.collection),
        error::not_found(ECOLLECTION_NOT_PUBLISHED),
    );
    assert!(
        !table::contains(&amp;collections.token_data, token_data_id),
        error::already_exists(ETOKEN_DATA_ALREADY_EXISTS),
    );

    let collection &#61; table::borrow_mut(&amp;mut collections.collection_data, token_data_id.collection);

    // if collection maximum &#61;&#61; 0, user don&apos;t want to enforce supply constraint.
    // we don&apos;t track supply to make token creation parallelizable
    if (collection.maximum &gt; 0) &#123;
        collection.supply &#61; collection.supply &#43; 1;
        assert!(
            collection.maximum &gt;&#61; collection.supply,
            error::invalid_argument(ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM),
        );
    &#125;;

    let token_data &#61; TokenData &#123;
        maximum,
        largest_property_version: 0,
        supply: 0,
        uri,
        royalty: create_royalty(royalty_points_numerator, royalty_points_denominator, royalty_payee_address),
        name,
        description,
        default_properties: property_map::new(property_keys, property_values, property_types),
        mutability_config: token_mutate_config,
    &#125;;

    table::add(&amp;mut collections.token_data, token_data_id, token_data);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(
            CreateTokenData &#123;
                id: token_data_id,
                description,
                maximum,
                uri,
                royalty_payee_address,
                royalty_points_denominator,
                royalty_points_numerator,
                name,
                mutability_config: token_mutate_config,
                property_keys,
                property_values,
                property_types,
            &#125;
        );
    &#125;;

    event::emit_event&lt;CreateTokenDataEvent&gt;(
        &amp;mut collections.create_token_data_events,
        CreateTokenDataEvent &#123;
            id: token_data_id,
            description,
            maximum,
            uri,
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_numerator,
            name,
            mutability_config: token_mutate_config,
            property_keys,
            property_values,
            property_types,
        &#125;,
    );
    token_data_id
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_supply"></a>

## Function `get_collection_supply`

return the number of distinct token_data_id created under this collection


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: string::String): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: String): Option&lt;u64&gt; acquires Collections &#123;
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);

    if (collection_data.maximum &gt; 0) &#123;
        option::some(collection_data.supply)
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_description"></a>

## Function `get_collection_description`



<pre><code>public fun get_collection_description(creator_address: address, collection_name: string::String): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_description(creator_address: address, collection_name: String): String acquires Collections &#123;
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    collection_data.description
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_uri"></a>

## Function `get_collection_uri`



<pre><code>public fun get_collection_uri(creator_address: address, collection_name: string::String): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_uri(creator_address: address, collection_name: String): String acquires Collections &#123;
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    collection_data.uri
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_maximum"></a>

## Function `get_collection_maximum`



<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: string::String): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: String): u64 acquires Collections &#123;
    assert_collection_exists(creator_address, collection_name);
    let collection_data &#61; table::borrow_mut(&amp;mut borrow_global_mut&lt;Collections&gt;(creator_address).collection_data, collection_name);
    collection_data.maximum
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_supply"></a>

## Function `get_token_supply`

return the number of distinct token_id created under this TokenData


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: token::TokenDataId): option::Option&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: TokenDataId): Option&lt;u64&gt; acquires Collections &#123;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    let token_data &#61; table::borrow(all_token_data, token_data_id);

    if (token_data.maximum &gt; 0) &#123;
        option::some(token_data.supply)
    &#125; else &#123;
        option::none&lt;u64&gt;()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_largest_property_version"></a>

## Function `get_tokendata_largest_property_version`

return the largest_property_version of this TokenData


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: token::TokenDataId): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: TokenDataId): u64 acquires Collections &#123;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    table::borrow(all_token_data, token_data_id).largest_property_version
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_id"></a>

## Function `get_token_id`

return the TokenId for a given Token


<pre><code>public fun get_token_id(token: &amp;token::Token): token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_id(token: &amp;Token): TokenId &#123;
    token.id
&#125;
</code></pre>



</details>

<a id="0x3_token_get_direct_transfer"></a>

## Function `get_direct_transfer`



<pre><code>public fun get_direct_transfer(receiver: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_direct_transfer(receiver: address): bool acquires TokenStore &#123;
    if (!exists&lt;TokenStore&gt;(receiver)) &#123;
        return false
    &#125;;

    borrow_global&lt;TokenStore&gt;(receiver).direct_transfer
&#125;
</code></pre>



</details>

<a id="0x3_token_create_token_mutability_config"></a>

## Function `create_token_mutability_config`



<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::TokenMutabilityConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): TokenMutabilityConfig &#123;
    TokenMutabilityConfig &#123;
        maximum: &#42;vector::borrow(mutate_setting, TOKEN_MAX_MUTABLE_IND),
        uri: &#42;vector::borrow(mutate_setting, TOKEN_URI_MUTABLE_IND),
        royalty: &#42;vector::borrow(mutate_setting, TOKEN_ROYALTY_MUTABLE_IND),
        description: &#42;vector::borrow(mutate_setting, TOKEN_DESCRIPTION_MUTABLE_IND),
        properties: &#42;vector::borrow(mutate_setting, TOKEN_PROPERTY_MUTABLE_IND),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_create_collection_mutability_config"></a>

## Function `create_collection_mutability_config`



<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::CollectionMutabilityConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): CollectionMutabilityConfig &#123;
    CollectionMutabilityConfig &#123;
        description: &#42;vector::borrow(mutate_setting, COLLECTION_DESCRIPTION_MUTABLE_IND),
        uri: &#42;vector::borrow(mutate_setting, COLLECTION_URI_MUTABLE_IND),
        maximum: &#42;vector::borrow(mutate_setting, COLLECTION_MAX_MUTABLE_IND),
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_mint_token"></a>

## Function `mint_token`



<pre><code>public fun mint_token(account: &amp;signer, token_data_id: token::TokenDataId, amount: u64): token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token(
    account: &amp;signer,
    token_data_id: TokenDataId,
    amount: u64,
): TokenId acquires Collections, TokenStore &#123;
    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
    let creator_addr &#61; token_data_id.creator;
    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);

    if (token_data.maximum &gt; 0) &#123;
        assert!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
        token_data.supply &#61; token_data.supply &#43; amount;
    &#125;;

    // we add more tokens with property_version 0
    let token_id &#61; create_token_id(token_data_id, 0);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(MintToken &#123; id: token_data_id, amount &#125;)
    &#125;;
    event::emit_event&lt;MintTokenEvent&gt;(
        &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).mint_token_events,
        MintTokenEvent &#123;
            id: token_data_id,
            amount,
        &#125;
    );

    deposit_token(account,
        Token &#123;
            id: token_id,
            amount,
            token_properties: property_map::empty(), // same as default properties no need to store
        &#125;
    );

    token_id
&#125;
</code></pre>



</details>

<a id="0x3_token_mint_token_to"></a>

## Function `mint_token_to`

create tokens and directly deposite to receiver's address. The receiver should opt-in direct transfer


<pre><code>public fun mint_token_to(account: &amp;signer, receiver: address, token_data_id: token::TokenDataId, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun mint_token_to(
    account: &amp;signer,
    receiver: address,
    token_data_id: TokenDataId,
    amount: u64,
) acquires Collections, TokenStore &#123;
    assert!(exists&lt;TokenStore&gt;(receiver), error::not_found(ETOKEN_STORE_NOT_PUBLISHED));
    let opt_in_transfer &#61; borrow_global&lt;TokenStore&gt;(receiver).direct_transfer;
    assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));

    assert!(token_data_id.creator &#61;&#61; signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
    let creator_addr &#61; token_data_id.creator;
    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    let token_data &#61; table::borrow_mut(all_token_data, token_data_id);

    if (token_data.maximum &gt; 0) &#123;
        assert!(token_data.supply &#43; amount &lt;&#61; token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
        token_data.supply &#61; token_data.supply &#43; amount;
    &#125;;

    // we add more tokens with property_version 0
    let token_id &#61; create_token_id(token_data_id, 0);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(MintToken &#123; id: token_data_id, amount &#125;)
    &#125;;
    event::emit_event&lt;MintTokenEvent&gt;(
        &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).mint_token_events,
        MintTokenEvent &#123;
            id: token_data_id,
            amount,
        &#125;
    );

    direct_deposit(receiver,
        Token &#123;
            id: token_id,
            amount,
            token_properties: property_map::empty(), // same as default properties no need to store
        &#125;
    );
&#125;
</code></pre>



</details>

<a id="0x3_token_create_token_id"></a>

## Function `create_token_id`



<pre><code>public fun create_token_id(token_data_id: token::TokenDataId, property_version: u64): token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_id(token_data_id: TokenDataId, property_version: u64): TokenId &#123;
    TokenId &#123;
        token_data_id,
        property_version,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_create_token_data_id"></a>

## Function `create_token_data_id`



<pre><code>public fun create_token_data_id(creator: address, collection: string::String, name: string::String): token::TokenDataId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_data_id(
    creator: address,
    collection: String,
    name: String,
): TokenDataId &#123;
    assert!(string::length(&amp;collection) &lt;&#61; MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
    assert!(string::length(&amp;name) &lt;&#61; MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));
    TokenDataId &#123; creator, collection, name &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_create_token_id_raw"></a>

## Function `create_token_id_raw`



<pre><code>public fun create_token_id_raw(creator: address, collection: string::String, name: string::String, property_version: u64): token::TokenId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_token_id_raw(
    creator: address,
    collection: String,
    name: String,
    property_version: u64,
): TokenId &#123;
    TokenId &#123;
        token_data_id: create_token_data_id(creator, collection, name),
        property_version,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_balance_of"></a>

## Function `balance_of`



<pre><code>public fun balance_of(owner: address, id: token::TokenId): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance_of(owner: address, id: TokenId): u64 acquires TokenStore &#123;
    if (!exists&lt;TokenStore&gt;(owner)) &#123;
        return 0
    &#125;;
    let token_store &#61; borrow_global&lt;TokenStore&gt;(owner);
    if (table::contains(&amp;token_store.tokens, id)) &#123;
        table::borrow(&amp;token_store.tokens, id).amount
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_has_token_store"></a>

## Function `has_token_store`



<pre><code>public fun has_token_store(owner: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun has_token_store(owner: address): bool &#123;
    exists&lt;TokenStore&gt;(owner)
&#125;
</code></pre>



</details>

<a id="0x3_token_get_royalty"></a>

## Function `get_royalty`



<pre><code>public fun get_royalty(token_id: token::TokenId): token::Royalty
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty(token_id: TokenId): Royalty acquires Collections &#123;
    let token_data_id &#61; token_id.token_data_id;
    get_tokendata_royalty(token_data_id)
&#125;
</code></pre>



</details>

<a id="0x3_token_get_royalty_numerator"></a>

## Function `get_royalty_numerator`



<pre><code>public fun get_royalty_numerator(royalty: &amp;token::Royalty): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_numerator(royalty: &amp;Royalty): u64 &#123;
    royalty.royalty_points_numerator
&#125;
</code></pre>



</details>

<a id="0x3_token_get_royalty_denominator"></a>

## Function `get_royalty_denominator`



<pre><code>public fun get_royalty_denominator(royalty: &amp;token::Royalty): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_denominator(royalty: &amp;Royalty): u64 &#123;
    royalty.royalty_points_denominator
&#125;
</code></pre>



</details>

<a id="0x3_token_get_royalty_payee"></a>

## Function `get_royalty_payee`



<pre><code>public fun get_royalty_payee(royalty: &amp;token::Royalty): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_royalty_payee(royalty: &amp;Royalty): address &#123;
    royalty.payee_address
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_amount"></a>

## Function `get_token_amount`



<pre><code>public fun get_token_amount(token: &amp;token::Token): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_amount(token: &amp;Token): u64 &#123;
    token.amount
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_id_fields"></a>

## Function `get_token_id_fields`

return the creator address, collection name, token name and property_version


<pre><code>public fun get_token_id_fields(token_id: &amp;token::TokenId): (address, string::String, string::String, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_id_fields(token_id: &amp;TokenId): (address, String, String, u64) &#123;
    (
        token_id.token_data_id.creator,
        token_id.token_data_id.collection,
        token_id.token_data_id.name,
        token_id.property_version,
    )
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_data_id_fields"></a>

## Function `get_token_data_id_fields`



<pre><code>public fun get_token_data_id_fields(token_data_id: &amp;token::TokenDataId): (address, string::String, string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_data_id_fields(token_data_id: &amp;TokenDataId): (address, String, String) &#123;
    (
        token_data_id.creator,
        token_data_id.collection,
        token_data_id.name,
    )
&#125;
</code></pre>



</details>

<a id="0x3_token_get_property_map"></a>

## Function `get_property_map`

return a copy of the token property map.
if property_version = 0, return the default property map
if property_version > 0, return the property value stored at owner's token store


<pre><code>public fun get_property_map(owner: address, token_id: token::TokenId): property_map::PropertyMap
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_property_map(owner: address, token_id: TokenId): PropertyMap acquires Collections, TokenStore &#123;
    assert!(balance_of(owner, token_id) &gt; 0, error::not_found(EINSUFFICIENT_BALANCE));
    // if property_version &#61; 0, return default property map
    if (token_id.property_version &#61;&#61; 0) &#123;
        let creator_addr &#61; token_id.token_data_id.creator;
        let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_addr).token_data;
        assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        let token_data &#61; table::borrow(all_token_data, token_id.token_data_id);
        token_data.default_properties
    &#125; else &#123;
        let tokens &#61; &amp;borrow_global&lt;TokenStore&gt;(owner).tokens;
        table::borrow(tokens, token_id).token_properties
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_maximum"></a>

## Function `get_tokendata_maximum`



<pre><code>public fun get_tokendata_maximum(token_data_id: token::TokenDataId): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_maximum(token_data_id: TokenDataId): u64 acquires Collections &#123;
    let creator_address &#61; token_data_id.creator;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

    let token_data &#61; table::borrow(all_token_data, token_data_id);
    token_data.maximum
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_uri"></a>

## Function `get_tokendata_uri`



<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: token::TokenDataId): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: TokenDataId): String acquires Collections &#123;
    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

    let token_data &#61; table::borrow(all_token_data, token_data_id);
    token_data.uri
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_description"></a>

## Function `get_tokendata_description`



<pre><code>public fun get_tokendata_description(token_data_id: token::TokenDataId): string::String
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_description(token_data_id: TokenDataId): String acquires Collections &#123;
    let creator_address &#61; token_data_id.creator;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

    let token_data &#61; table::borrow(all_token_data, token_data_id);
    token_data.description
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_royalty"></a>

## Function `get_tokendata_royalty`



<pre><code>public fun get_tokendata_royalty(token_data_id: token::TokenDataId): token::Royalty
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_royalty(token_data_id: TokenDataId): Royalty acquires Collections &#123;
    let creator_address &#61; token_data_id.creator;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

    let token_data &#61; table::borrow(all_token_data, token_data_id);
    token_data.royalty
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_id"></a>

## Function `get_tokendata_id`

return the token_data_id from the token_id


<pre><code>public fun get_tokendata_id(token_id: token::TokenId): token::TokenDataId
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_id(token_id: TokenId): TokenDataId &#123;
    token_id.token_data_id
&#125;
</code></pre>



</details>

<a id="0x3_token_get_tokendata_mutability_config"></a>

## Function `get_tokendata_mutability_config`

return the mutation setting of the token


<pre><code>public fun get_tokendata_mutability_config(token_data_id: token::TokenDataId): token::TokenMutabilityConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_tokendata_mutability_config(token_data_id: TokenDataId): TokenMutabilityConfig acquires Collections &#123;
    let creator_addr &#61; token_data_id.creator;
    assert!(exists&lt;Collections&gt;(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_addr).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    table::borrow(all_token_data, token_data_id).mutability_config
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_maximum"></a>

## Function `get_token_mutability_maximum`

return if the token's maximum is mutable


<pre><code>public fun get_token_mutability_maximum(config: &amp;token::TokenMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_maximum(config: &amp;TokenMutabilityConfig): bool &#123;
    config.maximum
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_royalty"></a>

## Function `get_token_mutability_royalty`

return if the token royalty is mutable with a token mutability config


<pre><code>public fun get_token_mutability_royalty(config: &amp;token::TokenMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_royalty(config: &amp;TokenMutabilityConfig): bool &#123;
    config.royalty
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_uri"></a>

## Function `get_token_mutability_uri`

return if the token uri is mutable with a token mutability config


<pre><code>public fun get_token_mutability_uri(config: &amp;token::TokenMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_uri(config: &amp;TokenMutabilityConfig): bool &#123;
    config.uri
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_description"></a>

## Function `get_token_mutability_description`

return if the token description is mutable with a token mutability config


<pre><code>public fun get_token_mutability_description(config: &amp;token::TokenMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_description(config: &amp;TokenMutabilityConfig): bool &#123;
    config.description
&#125;
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_default_properties"></a>

## Function `get_token_mutability_default_properties`

return if the tokendata's default properties is mutable with a token mutability config


<pre><code>public fun get_token_mutability_default_properties(config: &amp;token::TokenMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_token_mutability_default_properties(config: &amp;TokenMutabilityConfig): bool &#123;
    config.properties
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_config"></a>

## Function `get_collection_mutability_config`

return the collection mutation setting


<pre><code>&#35;[view]
public fun get_collection_mutability_config(creator: address, collection_name: string::String): token::CollectionMutabilityConfig
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_config(
    creator: address,
    collection_name: String
): CollectionMutabilityConfig acquires Collections &#123;
    assert!(exists&lt;Collections&gt;(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator).collection_data;
    assert!(table::contains(all_collection_data, collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));
    table::borrow(all_collection_data, collection_name).mutability_config
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_description"></a>

## Function `get_collection_mutability_description`

return if the collection description is mutable with a collection mutability config


<pre><code>public fun get_collection_mutability_description(config: &amp;token::CollectionMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_description(config: &amp;CollectionMutabilityConfig): bool &#123;
    config.description
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_uri"></a>

## Function `get_collection_mutability_uri`

return if the collection uri is mutable with a collection mutability config


<pre><code>public fun get_collection_mutability_uri(config: &amp;token::CollectionMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_uri(config: &amp;CollectionMutabilityConfig): bool &#123;
    config.uri
&#125;
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_maximum"></a>

## Function `get_collection_mutability_maximum`

return if the collection maximum is mutable with collection mutability config


<pre><code>public fun get_collection_mutability_maximum(config: &amp;token::CollectionMutabilityConfig): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collection_mutability_maximum(config: &amp;CollectionMutabilityConfig): bool &#123;
    config.maximum
&#125;
</code></pre>



</details>

<a id="0x3_token_destroy_token_data"></a>

## Function `destroy_token_data`



<pre><code>fun destroy_token_data(token_data: token::TokenData)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_token_data(token_data: TokenData) &#123;
    let TokenData &#123;
        maximum: _,
        largest_property_version: _,
        supply: _,
        uri: _,
        royalty: _,
        name: _,
        description: _,
        default_properties: _,
        mutability_config: _,
    &#125; &#61; token_data;
&#125;
</code></pre>



</details>

<a id="0x3_token_destroy_collection_data"></a>

## Function `destroy_collection_data`



<pre><code>fun destroy_collection_data(collection_data: token::CollectionData)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun destroy_collection_data(collection_data: CollectionData) &#123;
    let CollectionData &#123;
        description: _,
        name: _,
        uri: _,
        supply: _,
        maximum: _,
        mutability_config: _,
    &#125; &#61; collection_data;
&#125;
</code></pre>



</details>

<a id="0x3_token_withdraw_with_event_internal"></a>

## Function `withdraw_with_event_internal`



<pre><code>fun withdraw_with_event_internal(account_addr: address, id: token::TokenId, amount: u64): token::Token
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun withdraw_with_event_internal(
    account_addr: address,
    id: TokenId,
    amount: u64,
): Token acquires TokenStore &#123;
    // It does not make sense to withdraw 0 tokens.
    assert!(amount &gt; 0, error::invalid_argument(EWITHDRAW_ZERO));
    // Make sure the account has sufficient tokens to withdraw.
    assert!(balance_of(account_addr, id) &gt;&#61; amount, error::invalid_argument(EINSUFFICIENT_BALANCE));

    assert!(
        exists&lt;TokenStore&gt;(account_addr),
        error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
    );

    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(account_addr);
    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(Withdraw &#123; id, amount &#125;)
    &#125;;
    event::emit_event&lt;WithdrawEvent&gt;(
        &amp;mut token_store.withdraw_events,
        WithdrawEvent &#123; id, amount &#125;
    );
    let tokens &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(account_addr).tokens;
    assert!(
        table::contains(tokens, id),
        error::not_found(ENO_TOKEN_IN_TOKEN_STORE),
    );
    // balance &gt; amount and amount &gt; 0 indirectly asserted that balance &gt; 0.
    let balance &#61; &amp;mut table::borrow_mut(tokens, id).amount;
    if (&#42;balance &gt; amount) &#123;
        &#42;balance &#61; &#42;balance &#45; amount;
        Token &#123; id, amount, token_properties: property_map::empty() &#125;
    &#125; else &#123;
        table::remove(tokens, id)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x3_token_update_token_property_internal"></a>

## Function `update_token_property_internal`



<pre><code>fun update_token_property_internal(token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_token_property_internal(
    token_owner: address,
    token_id: TokenId,
    keys: vector&lt;String&gt;,
    values: vector&lt;vector&lt;u8&gt;&gt;,
    types: vector&lt;String&gt;,
) acquires TokenStore &#123;
    let tokens &#61; &amp;mut borrow_global_mut&lt;TokenStore&gt;(token_owner).tokens;
    assert!(table::contains(tokens, token_id), error::not_found(ENO_TOKEN_IN_TOKEN_STORE));

    let value &#61; &amp;mut table::borrow_mut(tokens, token_id).token_properties;
    assert_non_standard_reserved_property(&amp;keys);
    property_map::update_property_map(value, keys, values, types);
&#125;
</code></pre>



</details>

<a id="0x3_token_direct_deposit"></a>

## Function `direct_deposit`

Deposit the token balance into the recipients account and emit an event.


<pre><code>fun direct_deposit(account_addr: address, token: token::Token)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun direct_deposit(account_addr: address, token: Token) acquires TokenStore &#123;
    assert!(token.amount &gt; 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));
    let token_store &#61; borrow_global_mut&lt;TokenStore&gt;(account_addr);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(Deposit &#123; id: token.id, amount: token.amount &#125;);
    &#125;;
    event::emit_event&lt;DepositEvent&gt;(
        &amp;mut token_store.deposit_events,
        DepositEvent &#123; id: token.id, amount: token.amount &#125;,
    );

    assert!(
        exists&lt;TokenStore&gt;(account_addr),
        error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
    );

    if (!table::contains(&amp;token_store.tokens, token.id)) &#123;
        table::add(&amp;mut token_store.tokens, token.id, token);
    &#125; else &#123;
        let recipient_token &#61; table::borrow_mut(&amp;mut token_store.tokens, token.id);
        merge(recipient_token, token);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x3_token_assert_collection_exists"></a>

## Function `assert_collection_exists`



<pre><code>fun assert_collection_exists(creator_address: address, collection_name: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_collection_exists(creator_address: address, collection_name: String) acquires Collections &#123;
    assert!(exists&lt;Collections&gt;(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_collection_data &#61; &amp;borrow_global&lt;Collections&gt;(creator_address).collection_data;
    assert!(table::contains(all_collection_data, collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));
&#125;
</code></pre>



</details>

<a id="0x3_token_assert_tokendata_exists"></a>

## Function `assert_tokendata_exists`



<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: token::TokenDataId)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: TokenDataId) acquires Collections &#123;
    let creator_addr &#61; token_data_id.creator;
    assert!(signer::address_of(creator) &#61;&#61; creator_addr, error::permission_denied(ENO_MUTATE_CAPABILITY));
    assert!(exists&lt;Collections&gt;(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
    let all_token_data &#61; &amp;mut borrow_global_mut&lt;Collections&gt;(creator_addr).token_data;
    assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
&#125;
</code></pre>



</details>

<a id="0x3_token_assert_non_standard_reserved_property"></a>

## Function `assert_non_standard_reserved_property`



<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;string::String&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;String&gt;) &#123;
    vector::for_each_ref(keys, &#124;key&#124; &#123;
        let key: &amp;String &#61; key;
        let length &#61; string::length(key);
        if (length &gt;&#61; 6) &#123;
            let prefix &#61; string::sub_string(&amp;&#42;key, 0, 6);
            assert!(prefix !&#61; string::utf8(b&quot;TOKEN_&quot;), error::permission_denied(EPROPERTY_RESERVED_BY_STANDARD));
        &#125;;
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x3_token_initialize_token_script"></a>

## Function `initialize_token_script`



<pre><code>public entry fun initialize_token_script(_account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun initialize_token_script(_account: &amp;signer) &#123;
    abort 0
&#125;
</code></pre>



</details>

<a id="0x3_token_initialize_token"></a>

## Function `initialize_token`



<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: token::TokenId)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: TokenId) &#123;
    abort 0
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_create_collection_script"></a>

### Function `create_collection_script`


<pre><code>public entry fun create_collection_script(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)
</code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;


<pre><code>pragma aborts_if_is_partial;
include CreateCollectionAbortsIf;
</code></pre>



<a id="@Specification_1_create_token_script"></a>

### Function `create_token_script`


<pre><code>public entry fun create_token_script(account: &amp;signer, collection: string::String, name: string::String, description: string::String, balance: u64, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: vector&lt;bool&gt;, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;)
</code></pre>


the length of 'mutate_setting' should maore than five.
The creator of the TokenDataId is signer.
The token_data_id should exist in the creator's collections..
The sum of supply and mint Token is less than maximum.


<pre><code>pragma aborts_if_is_partial;
let addr &#61; signer::address_of(account);
let token_data_id &#61; spec_create_tokendata(addr, collection, name);
let creator_addr &#61; token_data_id.creator;
let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
aborts_if token_data_id.creator !&#61; addr;
aborts_if !exists&lt;Collections&gt;(creator_addr);
aborts_if balance &lt;&#61; 0;
include CreateTokenMutabilityConfigAbortsIf;
include CreateTokenMutabilityConfigAbortsIf;
</code></pre>




<a id="0x3_token_spec_create_tokendata"></a>


<pre><code>fun spec_create_tokendata(
   creator: address,
   collection: String,
   name: String): TokenDataId &#123;
   TokenDataId &#123; creator, collection, name &#125;
&#125;
</code></pre>



<a id="@Specification_1_mint_script"></a>

### Function `mint_script`


<pre><code>public entry fun mint_script(account: &amp;signer, token_data_address: address, collection: string::String, name: string::String, amount: u64)
</code></pre>


only creator of the tokendata can mint tokens


<pre><code>pragma aborts_if_is_partial;
let token_data_id &#61; spec_create_token_data_id(
    token_data_address,
    collection,
    name,
);
let addr &#61; signer::address_of(account);
let creator_addr &#61; token_data_id.creator;
let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
aborts_if token_data_id.creator !&#61; signer::address_of(account);
include CreateTokenDataIdAbortsIf&#123;
creator: token_data_address,
collection: collection,
name: name
&#125;;
include MintTokenAbortsIf &#123;
token_data_id: token_data_id
&#125;;
</code></pre>



<a id="@Specification_1_mutate_token_properties"></a>

### Function `mutate_token_properties`


<pre><code>public entry fun mutate_token_properties(account: &amp;signer, token_owner: address, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, amount: u64, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>


The signer is creator.


<pre><code>pragma aborts_if_is_partial;
let addr &#61; signer::address_of(account);
aborts_if addr !&#61; creator;
include CreateTokenDataIdAbortsIf &#123;
    creator: creator,
    collection: collection_name,
    name: token_name
&#125;;
</code></pre>



<a id="@Specification_1_direct_transfer_script"></a>

### Function `direct_transfer_script`


<pre><code>public entry fun direct_transfer_script(sender: &amp;signer, receiver: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
include CreateTokenDataIdAbortsIf&#123;
    creator: creators_address,
    collection: collection,
    name: name
&#125;;
</code></pre>



<a id="@Specification_1_opt_in_direct_transfer"></a>

### Function `opt_in_direct_transfer`


<pre><code>public entry fun opt_in_direct_transfer(account: &amp;signer, opt_in: bool)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let addr &#61; signer::address_of(account);
let account_addr &#61; global&lt;account::Account&gt;(addr);
aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 9 &gt; MAX_U64;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
</code></pre>



<a id="@Specification_1_transfer_with_opt_in"></a>

### Function `transfer_with_opt_in`


<pre><code>public entry fun transfer_with_opt_in(from: &amp;signer, creator: address, collection_name: string::String, token_name: string::String, token_property_version: u64, to: address, amount: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
include CreateTokenDataIdAbortsIf&#123;
    creator: creator,
    collection: collection_name,
    name: token_name
&#125;;
</code></pre>



<a id="@Specification_1_burn_by_creator"></a>

### Function `burn_by_creator`


<pre><code>public entry fun burn_by_creator(creator: &amp;signer, owner: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let creator_address &#61; signer::address_of(creator);
let token_id &#61; spec_create_token_id_raw(creator_address, collection, name, property_version);
let creator_addr &#61; token_id.token_data_id.creator;
let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_address);
let token_data &#61; table::spec_get(
    collections.token_data,
    token_id.token_data_id,
);
aborts_if amount &lt;&#61; 0;
aborts_if !exists&lt;Collections&gt;(creator_addr);
aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);
aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_CREATOR));
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code>public entry fun burn(owner: &amp;signer, creators_address: address, collection: string::String, name: string::String, property_version: u64, amount: u64)
</code></pre>


The token_data_id should exist in token_data.


<pre><code>pragma aborts_if_is_partial;
let token_id &#61; spec_create_token_id_raw(creators_address, collection, name, property_version);
let creator_addr &#61; token_id.token_data_id.creator;
let collections &#61; borrow_global_mut&lt;Collections&gt;(creator_addr);
let token_data &#61; table::spec_get(
    collections.token_data,
    token_id.token_data_id,
);
include CreateTokenDataIdAbortsIf &#123;
creator: creators_address
&#125;;
aborts_if amount &lt;&#61; 0;
aborts_if !exists&lt;Collections&gt;(creator_addr);
aborts_if !table::spec_contains(collections.token_data, token_id.token_data_id);
aborts_if !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(BURNABLE_BY_OWNER));
aborts_if !string::spec_internal_check_utf8(BURNABLE_BY_OWNER);
</code></pre>




<a id="0x3_token_spec_create_token_id_raw"></a>


<pre><code>fun spec_create_token_id_raw(
   creator: address,
   collection: String,
   name: String,
   property_version: u64,
): TokenId &#123;
   let token_data_id &#61; TokenDataId &#123; creator, collection, name &#125;;
   TokenId &#123;
       token_data_id,
       property_version
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_mutate_collection_description"></a>

### Function `mutate_collection_description`


<pre><code>public fun mutate_collection_description(creator: &amp;signer, collection_name: string::String, description: string::String)
</code></pre>


The description of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);
include AssertCollectionExistsAbortsIf &#123;
    creator_address: addr,
    collection_name: collection_name
&#125;;
aborts_if !collection_data.mutability_config.description;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_collection_uri"></a>

### Function `mutate_collection_uri`


<pre><code>public fun mutate_collection_uri(creator: &amp;signer, collection_name: string::String, uri: string::String)
</code></pre>


The uri of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);
aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;
include AssertCollectionExistsAbortsIf &#123;
    creator_address: addr,
    collection_name: collection_name
&#125;;
aborts_if !collection_data.mutability_config.uri;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_collection_maximum"></a>

### Function `mutate_collection_maximum`


<pre><code>public fun mutate_collection_maximum(creator: &amp;signer, collection_name: string::String, maximum: u64)
</code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The maxium of Collection is mutable.


<pre><code>let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let collection_data &#61; table::spec_get(global&lt;Collections&gt;(addr).collection_data, collection_name);
include AssertCollectionExistsAbortsIf &#123;
    creator_address: addr,
    collection_name: collection_name
&#125;;
aborts_if collection_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;
aborts_if maximum &lt; collection_data.supply;
aborts_if !collection_data.mutability_config.maximum;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_tokendata_maximum"></a>

### Function `mutate_tokendata_maximum`


<pre><code>public fun mutate_tokendata_maximum(creator: &amp;signer, token_data_id: token::TokenDataId, maximum: u64)
</code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The token maximum is mutable


<pre><code>let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
include AssertTokendataExistsAbortsIf;
aborts_if token_data.maximum &#61;&#61; 0 &#124;&#124; maximum &#61;&#61; 0;
aborts_if maximum &lt; token_data.supply;
aborts_if !token_data.mutability_config.maximum;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_tokendata_uri"></a>

### Function `mutate_tokendata_uri`


<pre><code>public fun mutate_tokendata_uri(creator: &amp;signer, token_data_id: token::TokenDataId, uri: string::String)
</code></pre>


The length of uri should less than MAX_URI_LENGTH.
The  creator of token_data_id should exist in Collections.
The token uri is mutable


<pre><code>let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
include AssertTokendataExistsAbortsIf;
aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;
aborts_if !token_data.mutability_config.uri;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_tokendata_royalty"></a>

### Function `mutate_tokendata_royalty`


<pre><code>public fun mutate_tokendata_royalty(creator: &amp;signer, token_data_id: token::TokenDataId, royalty: token::Royalty)
</code></pre>


The token royalty is mutable


<pre><code>include AssertTokendataExistsAbortsIf;
let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
aborts_if !token_data.mutability_config.royalty;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_tokendata_description"></a>

### Function `mutate_tokendata_description`


<pre><code>public fun mutate_tokendata_description(creator: &amp;signer, token_data_id: token::TokenDataId, description: string::String)
</code></pre>


The token description is mutable


<pre><code>include AssertTokendataExistsAbortsIf;
let addr &#61; signer::address_of(creator);
let account &#61; global&lt;account::Account&gt;(addr);
let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
aborts_if !token_data.mutability_config.description;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt;&#61; account::MAX_GUID_CREATION_NUM;
aborts_if !exists&lt;token_event_store::TokenEventStoreV1&gt;(addr) &amp;&amp; account.guid_creation_num &#43; 9 &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_mutate_tokendata_property"></a>

### Function `mutate_tokendata_property`


<pre><code>public fun mutate_tokendata_property(creator: &amp;signer, token_data_id: token::TokenDataId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>


The property map is mutable


<pre><code>pragma aborts_if_is_partial;
let all_token_data &#61; global&lt;Collections&gt;(token_data_id.creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
include AssertTokendataExistsAbortsIf;
aborts_if len(keys) !&#61; len(values);
aborts_if len(keys) !&#61; len(types);
aborts_if !token_data.mutability_config.properties;
</code></pre>



<a id="@Specification_1_mutate_one_token"></a>

### Function `mutate_one_token`


<pre><code>public fun mutate_one_token(account: &amp;signer, token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;): token::TokenId
</code></pre>


The signer is creator.
The token_data_id should exist in token_data.
The property map is mutable.


<pre><code>pragma aborts_if_is_partial;
let creator &#61; token_id.token_data_id.creator;
let addr &#61; signer::address_of(account);
let all_token_data &#61; global&lt;Collections&gt;(creator).token_data;
let token_data &#61; table::spec_get(all_token_data, token_id.token_data_id);
aborts_if addr !&#61; creator;
aborts_if !exists&lt;Collections&gt;(creator);
aborts_if !table::spec_contains(all_token_data, token_id.token_data_id);
aborts_if !token_data.mutability_config.properties &amp;&amp; !simple_map::spec_contains_key(token_data.default_properties.map, std::string::spec_utf8(TOKEN_PROPERTY_MUTABLE));
</code></pre>



<a id="@Specification_1_create_royalty"></a>

### Function `create_royalty`


<pre><code>public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): token::Royalty
</code></pre>




<pre><code>include CreateRoyaltyAbortsIf;
</code></pre>


The royalty_points_numerator should less than royalty_points_denominator.


<a id="0x3_token_CreateRoyaltyAbortsIf"></a>


<pre><code>schema CreateRoyaltyAbortsIf &#123;
    royalty_points_numerator: u64;
    royalty_points_denominator: u64;
    payee_address: address;
    aborts_if royalty_points_numerator &gt; royalty_points_denominator;
    aborts_if !exists&lt;account::Account&gt;(payee_address);
&#125;
</code></pre>



<a id="@Specification_1_deposit_token"></a>

### Function `deposit_token`


<pre><code>public fun deposit_token(account: &amp;signer, token: token::Token)
</code></pre>




<pre><code>pragma verify &#61; false;
pragma aborts_if_is_partial;
let account_addr &#61; signer::address_of(account);
include !exists&lt;TokenStore&gt;(account_addr) &#61;&#61;&gt; InitializeTokenStore;
let token_id &#61; token.id;
let token_amount &#61; token.amount;
include DirectDepositAbortsIf;
</code></pre>



<a id="@Specification_1_direct_deposit_with_opt_in"></a>

### Function `direct_deposit_with_opt_in`


<pre><code>public fun direct_deposit_with_opt_in(account_addr: address, token: token::Token)
</code></pre>


The token can direct_transfer.


<pre><code>let opt_in_transfer &#61; global&lt;TokenStore&gt;(account_addr).direct_transfer;
aborts_if !exists&lt;TokenStore&gt;(account_addr);
aborts_if !opt_in_transfer;
let token_id &#61; token.id;
let token_amount &#61; token.amount;
include DirectDepositAbortsIf;
</code></pre>



<a id="@Specification_1_direct_transfer"></a>

### Function `direct_transfer`


<pre><code>public fun direct_transfer(sender: &amp;signer, receiver: &amp;signer, token_id: token::TokenId, amount: u64)
</code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_initialize_token_store"></a>

### Function `initialize_token_store`


<pre><code>public fun initialize_token_store(account: &amp;signer)
</code></pre>




<pre><code>include InitializeTokenStore;
</code></pre>




<a id="0x3_token_InitializeTokenStore"></a>


<pre><code>schema InitializeTokenStore &#123;
    account: signer;
    let addr &#61; signer::address_of(account);
    let account_addr &#61; global&lt;account::Account&gt;(addr);
    aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; !exists&lt;account::Account&gt;(addr);
    aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if !exists&lt;TokenStore&gt;(addr) &amp;&amp; account_addr.guid_creation_num &#43; 4 &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code>public fun merge(dst_token: &amp;mut token::Token, source_token: token::Token)
</code></pre>




<pre><code>aborts_if dst_token.id !&#61; source_token.id;
aborts_if dst_token.amount &#43; source_token.amount &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_split"></a>

### Function `split`


<pre><code>public fun split(dst_token: &amp;mut token::Token, amount: u64): token::Token
</code></pre>




<pre><code>aborts_if dst_token.id.property_version !&#61; 0;
aborts_if dst_token.amount &lt;&#61; amount;
aborts_if amount &lt;&#61; 0;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code>public fun transfer(from: &amp;signer, id: token::TokenId, to: address, amount: u64)
</code></pre>




<pre><code>let opt_in_transfer &#61; global&lt;TokenStore&gt;(to).direct_transfer;
let account_addr &#61; signer::address_of(from);
aborts_if !opt_in_transfer;
pragma aborts_if_is_partial;
include WithdrawWithEventInternalAbortsIf;
</code></pre>



<a id="@Specification_1_withdraw_with_capability"></a>

### Function `withdraw_with_capability`


<pre><code>public fun withdraw_with_capability(withdraw_proof: token::WithdrawCapability): token::Token
</code></pre>




<pre><code>let now_seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds;
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR &gt; withdraw_proof.expiration_sec;
include WithdrawWithEventInternalAbortsIf&#123;
account_addr: withdraw_proof.token_owner,
id: withdraw_proof.token_id,
amount: withdraw_proof.amount&#125;;
</code></pre>



<a id="@Specification_1_partial_withdraw_with_capability"></a>

### Function `partial_withdraw_with_capability`


<pre><code>public fun partial_withdraw_with_capability(withdraw_proof: token::WithdrawCapability, withdraw_amount: u64): (token::Token, option::Option&lt;token::WithdrawCapability&gt;)
</code></pre>




<pre><code>let now_seconds &#61; global&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework).microseconds;
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
aborts_if now_seconds / timestamp::MICRO_CONVERSION_FACTOR &gt; withdraw_proof.expiration_sec;
aborts_if withdraw_amount &gt; withdraw_proof.amount;
include WithdrawWithEventInternalAbortsIf&#123;
    account_addr: withdraw_proof.token_owner,
    id: withdraw_proof.token_id,
    amount: withdraw_amount
&#125;;
</code></pre>



<a id="@Specification_1_withdraw_token"></a>

### Function `withdraw_token`


<pre><code>public fun withdraw_token(account: &amp;signer, id: token::TokenId, amount: u64): token::Token
</code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code>let account_addr &#61; signer::address_of(account);
include WithdrawWithEventInternalAbortsIf;
</code></pre>



<a id="@Specification_1_create_collection"></a>

### Function `create_collection`


<pre><code>public fun create_collection(creator: &amp;signer, name: string::String, description: string::String, uri: string::String, maximum: u64, mutate_setting: vector&lt;bool&gt;)
</code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;
The collection_data should not exist before you create it.


<pre><code>pragma aborts_if_is_partial;
let account_addr &#61; signer::address_of(creator);
aborts_if len(name.bytes) &gt; 128;
aborts_if len(uri.bytes) &gt; 512;
include CreateCollectionAbortsIf;
</code></pre>




<a id="0x3_token_CreateCollectionAbortsIf"></a>


<pre><code>schema CreateCollectionAbortsIf &#123;
    creator: signer;
    name: String;
    description: String;
    uri: String;
    maximum: u64;
    mutate_setting: vector&lt;bool&gt;;
    let addr &#61; signer::address_of(creator);
    let account &#61; global&lt;account::Account&gt;(addr);
    let collection &#61; global&lt;Collections&gt;(addr);
    let b &#61; !exists&lt;Collections&gt;(addr);
    let collection_data &#61; global&lt;Collections&gt;(addr).collection_data;
    aborts_if b &amp;&amp; !exists&lt;account::Account&gt;(addr);
    aborts_if len(name.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;
    aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;
    aborts_if b &amp;&amp; account.guid_creation_num &#43; 3 &gt;&#61; account::MAX_GUID_CREATION_NUM;
    aborts_if b &amp;&amp; account.guid_creation_num &#43; 3 &gt; MAX_U64;
    include CreateCollectionMutabilityConfigAbortsIf;
&#125;
</code></pre>



<a id="@Specification_1_check_collection_exists"></a>

### Function `check_collection_exists`


<pre><code>public fun check_collection_exists(creator: address, name: string::String): bool
</code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator);
</code></pre>



<a id="@Specification_1_check_tokendata_exists"></a>

### Function `check_tokendata_exists`


<pre><code>public fun check_tokendata_exists(creator: address, collection_name: string::String, token_name: string::String): bool
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>aborts_if !exists&lt;Collections&gt;(creator);
include CreateTokenDataIdAbortsIf &#123;
    creator: creator,
    collection: collection_name,
    name: token_name
&#125;;
</code></pre>



<a id="@Specification_1_create_tokendata"></a>

### Function `create_tokendata`


<pre><code>public fun create_tokendata(account: &amp;signer, collection: string::String, name: string::String, description: string::String, maximum: u64, uri: string::String, royalty_payee_address: address, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: token::TokenMutabilityConfig, property_keys: vector&lt;string::String&gt;, property_values: vector&lt;vector&lt;u8&gt;&gt;, property_types: vector&lt;string::String&gt;): token::TokenDataId
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>pragma verify &#61; false;
pragma aborts_if_is_partial;
let account_addr &#61; signer::address_of(account);
let collections &#61; global&lt;Collections&gt;(account_addr);
let token_data_id &#61; spec_create_token_data_id(account_addr, collection, name);
let Collection &#61; table::spec_get(collections.collection_data, token_data_id.collection);
let length &#61; len(property_keys);
aborts_if len(name.bytes) &gt; MAX_NFT_NAME_LENGTH;
aborts_if len(collection.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;
aborts_if len(uri.bytes) &gt; MAX_URI_LENGTH;
aborts_if royalty_points_numerator &gt; royalty_points_denominator;
aborts_if !exists&lt;Collections&gt;(account_addr);
include CreateTokenDataIdAbortsIf &#123;
    creator: account_addr,
    collection: collection,
    name: name
&#125;;
aborts_if !table::spec_contains(collections.collection_data, collection);
aborts_if table::spec_contains(collections.token_data, token_data_id);
aborts_if Collection.maximum &gt; 0 &amp;&amp; Collection.supply &#43; 1 &gt; MAX_U64;
aborts_if Collection.maximum &gt; 0 &amp;&amp; Collection.maximum &lt; Collection.supply &#43; 1;
include CreateRoyaltyAbortsIf &#123;
    payee_address: royalty_payee_address
&#125;;
aborts_if length &gt; property_map::MAX_PROPERTY_MAP_SIZE;
aborts_if length !&#61; len(property_values);
aborts_if length !&#61; len(property_types);
</code></pre>




<a id="0x3_token_spec_create_token_data_id"></a>


<pre><code>fun spec_create_token_data_id(
   creator: address,
   collection: String,
   name: String,
): TokenDataId &#123;
   TokenDataId &#123; creator, collection, name &#125;
&#125;
</code></pre>



<a id="@Specification_1_get_collection_supply"></a>

### Function `get_collection_supply`


<pre><code>public fun get_collection_supply(creator_address: address, collection_name: string::String): option::Option&lt;u64&gt;
</code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;
</code></pre>



<a id="@Specification_1_get_collection_description"></a>

### Function `get_collection_description`


<pre><code>public fun get_collection_description(creator_address: address, collection_name: string::String): string::String
</code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;
</code></pre>



<a id="@Specification_1_get_collection_uri"></a>

### Function `get_collection_uri`


<pre><code>public fun get_collection_uri(creator_address: address, collection_name: string::String): string::String
</code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;
</code></pre>



<a id="@Specification_1_get_collection_maximum"></a>

### Function `get_collection_maximum`


<pre><code>public fun get_collection_maximum(creator_address: address, collection_name: string::String): u64
</code></pre>




<pre><code>include AssertCollectionExistsAbortsIf;
</code></pre>



<a id="@Specification_1_get_token_supply"></a>

### Function `get_token_supply`


<pre><code>public fun get_token_supply(creator_address: address, token_data_id: token::TokenDataId): option::Option&lt;u64&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator_address);
let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_largest_property_version"></a>

### Function `get_tokendata_largest_property_version`


<pre><code>public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: token::TokenDataId): u64
</code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator_address);
let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_create_token_mutability_config"></a>

### Function `create_token_mutability_config`


<pre><code>public fun create_token_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::TokenMutabilityConfig
</code></pre>


The length of 'mutate_setting' should more than five.
The mutate_setting shuold have a value.


<pre><code>include CreateTokenMutabilityConfigAbortsIf;
</code></pre>




<a id="0x3_token_CreateTokenMutabilityConfigAbortsIf"></a>


<pre><code>schema CreateTokenMutabilityConfigAbortsIf &#123;
    mutate_setting: vector&lt;bool&gt;;
    aborts_if len(mutate_setting) &lt; 5;
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_MAX_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_URI_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_ROYALTY_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_DESCRIPTION_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[TOKEN_PROPERTY_MUTABLE_IND]);
&#125;
</code></pre>



<a id="@Specification_1_create_collection_mutability_config"></a>

### Function `create_collection_mutability_config`


<pre><code>public fun create_collection_mutability_config(mutate_setting: &amp;vector&lt;bool&gt;): token::CollectionMutabilityConfig
</code></pre>




<pre><code>include CreateCollectionMutabilityConfigAbortsIf;
</code></pre>




<a id="0x3_token_CreateCollectionMutabilityConfigAbortsIf"></a>


<pre><code>schema CreateCollectionMutabilityConfigAbortsIf &#123;
    mutate_setting: vector&lt;bool&gt;;
    aborts_if len(mutate_setting) &lt; 3;
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_DESCRIPTION_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_URI_MUTABLE_IND]);
    aborts_if !vector::spec_contains(mutate_setting, mutate_setting[COLLECTION_MAX_MUTABLE_IND]);
&#125;
</code></pre>



<a id="@Specification_1_mint_token"></a>

### Function `mint_token`


<pre><code>public fun mint_token(account: &amp;signer, token_data_id: token::TokenDataId, amount: u64): token::TokenId
</code></pre>


The creator of the TokenDataId is signer.
The token_data_id should exist in the creator's collections..
The sum of supply and the amount of mint Token is less than maximum.


<pre><code>pragma verify &#61; false;
</code></pre>




<a id="0x3_token_MintTokenAbortsIf"></a>


<pre><code>schema MintTokenAbortsIf &#123;
    account: signer;
    token_data_id: TokenDataId;
    amount: u64;
    let addr &#61; signer::address_of(account);
    let creator_addr &#61; token_data_id.creator;
    let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
    let token_data &#61; table::spec_get(all_token_data, token_data_id);
    aborts_if token_data_id.creator !&#61; addr;
    aborts_if !table::spec_contains(all_token_data, token_data_id);
    aborts_if token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;
    aborts_if !exists&lt;Collections&gt;(creator_addr);
    aborts_if amount &lt;&#61; 0;
    include InitializeTokenStore;
    let token_id &#61; create_token_id(token_data_id, 0);
&#125;
</code></pre>



<a id="@Specification_1_mint_token_to"></a>

### Function `mint_token_to`


<pre><code>public fun mint_token_to(account: &amp;signer, receiver: address, token_data_id: token::TokenDataId, amount: u64)
</code></pre>




<pre><code>let addr &#61; signer::address_of(account);
let opt_in_transfer &#61; global&lt;TokenStore&gt;(receiver).direct_transfer;
let creator_addr &#61; token_data_id.creator;
let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
let token_data &#61; table::spec_get(all_token_data, token_data_id);
aborts_if !exists&lt;TokenStore&gt;(receiver);
aborts_if !opt_in_transfer;
aborts_if token_data_id.creator !&#61; addr;
aborts_if !table::spec_contains(all_token_data, token_data_id);
aborts_if token_data.maximum &gt; 0 &amp;&amp; token_data.supply &#43; amount &gt; token_data.maximum;
aborts_if amount &lt;&#61; 0;
aborts_if !exists&lt;Collections&gt;(creator_addr);
let token_id &#61; create_token_id(token_data_id, 0);
include DirectDepositAbortsIf &#123;
    account_addr: receiver,
    token_id: token_id,
    token_amount: amount,
&#125;;
</code></pre>



<a id="@Specification_1_create_token_data_id"></a>

### Function `create_token_data_id`


<pre><code>public fun create_token_data_id(creator: address, collection: string::String, name: string::String): token::TokenDataId
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>include CreateTokenDataIdAbortsIf;
</code></pre>




<a id="0x3_token_CreateTokenDataIdAbortsIf"></a>


<pre><code>schema CreateTokenDataIdAbortsIf &#123;
    creator: address;
    collection: String;
    name: String;
    aborts_if len(collection.bytes) &gt; MAX_COLLECTION_NAME_LENGTH;
    aborts_if len(name.bytes) &gt; MAX_NFT_NAME_LENGTH;
&#125;
</code></pre>



<a id="@Specification_1_create_token_id_raw"></a>

### Function `create_token_id_raw`


<pre><code>public fun create_token_id_raw(creator: address, collection: string::String, name: string::String, property_version: u64): token::TokenId
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code>include CreateTokenDataIdAbortsIf;
</code></pre>




<a id="0x3_token_spec_balance_of"></a>


<pre><code>fun spec_balance_of(owner: address, id: TokenId): u64 &#123;
   let token_store &#61; borrow_global&lt;TokenStore&gt;(owner);
   if (!exists&lt;TokenStore&gt;(owner)) &#123;
       0
   &#125;
   else if (table::spec_contains(token_store.tokens, id)) &#123;
       table::spec_get(token_store.tokens, id).amount
   &#125; else &#123;
       0
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_get_royalty"></a>

### Function `get_royalty`


<pre><code>public fun get_royalty(token_id: token::TokenId): token::Royalty
</code></pre>




<pre><code>include GetTokendataRoyaltyAbortsIf &#123;
    token_data_id: token_id.token_data_id
&#125;;
</code></pre>



<a id="@Specification_1_get_property_map"></a>

### Function `get_property_map`


<pre><code>public fun get_property_map(owner: address, token_id: token::TokenId): property_map::PropertyMap
</code></pre>




<pre><code>let creator_addr &#61; token_id.token_data_id.creator;
let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
aborts_if spec_balance_of(owner, token_id) &lt;&#61; 0;
aborts_if token_id.property_version &#61;&#61; 0 &amp;&amp; !table::spec_contains(all_token_data, token_id.token_data_id);
aborts_if token_id.property_version &#61;&#61; 0 &amp;&amp; !exists&lt;Collections&gt;(creator_addr);
</code></pre>



<a id="@Specification_1_get_tokendata_maximum"></a>

### Function `get_tokendata_maximum`


<pre><code>public fun get_tokendata_maximum(token_data_id: token::TokenDataId): u64
</code></pre>




<pre><code>let creator_address &#61; token_data_id.creator;
aborts_if !exists&lt;Collections&gt;(creator_address);
let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_uri"></a>

### Function `get_tokendata_uri`


<pre><code>public fun get_tokendata_uri(creator: address, token_data_id: token::TokenDataId): string::String
</code></pre>




<pre><code>aborts_if !exists&lt;Collections&gt;(creator);
let all_token_data &#61; global&lt;Collections&gt;(creator).token_data;
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_description"></a>

### Function `get_tokendata_description`


<pre><code>public fun get_tokendata_description(token_data_id: token::TokenDataId): string::String
</code></pre>




<pre><code>let creator_address &#61; token_data_id.creator;
aborts_if !exists&lt;Collections&gt;(creator_address);
let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_royalty"></a>

### Function `get_tokendata_royalty`


<pre><code>public fun get_tokendata_royalty(token_data_id: token::TokenDataId): token::Royalty
</code></pre>




<pre><code>include GetTokendataRoyaltyAbortsIf;
</code></pre>




<a id="0x3_token_GetTokendataRoyaltyAbortsIf"></a>


<pre><code>schema GetTokendataRoyaltyAbortsIf &#123;
    token_data_id: TokenDataId;
    let creator_address &#61; token_data_id.creator;
    let all_token_data &#61; global&lt;Collections&gt;(creator_address).token_data;
    aborts_if !exists&lt;Collections&gt;(creator_address);
    aborts_if !table::spec_contains(all_token_data, token_data_id);
&#125;
</code></pre>



<a id="@Specification_1_get_tokendata_mutability_config"></a>

### Function `get_tokendata_mutability_config`


<pre><code>public fun get_tokendata_mutability_config(token_data_id: token::TokenDataId): token::TokenMutabilityConfig
</code></pre>




<pre><code>let creator_addr &#61; token_data_id.creator;
let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
aborts_if !exists&lt;Collections&gt;(creator_addr);
aborts_if !table::spec_contains(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_collection_mutability_config"></a>

### Function `get_collection_mutability_config`


<pre><code>&#35;[view]
public fun get_collection_mutability_config(creator: address, collection_name: string::String): token::CollectionMutabilityConfig
</code></pre>




<pre><code>let all_collection_data &#61; global&lt;Collections&gt;(creator).collection_data;
aborts_if !exists&lt;Collections&gt;(creator);
aborts_if !table::spec_contains(all_collection_data, collection_name);
</code></pre>



<a id="@Specification_1_withdraw_with_event_internal"></a>

### Function `withdraw_with_event_internal`


<pre><code>fun withdraw_with_event_internal(account_addr: address, id: token::TokenId, amount: u64): token::Token
</code></pre>




<pre><code>include WithdrawWithEventInternalAbortsIf;
</code></pre>




<a id="0x3_token_WithdrawWithEventInternalAbortsIf"></a>


<pre><code>schema WithdrawWithEventInternalAbortsIf &#123;
    account_addr: address;
    id: TokenId;
    amount: u64;
    let tokens &#61; global&lt;TokenStore&gt;(account_addr).tokens;
    aborts_if amount &lt;&#61; 0;
    aborts_if spec_balance_of(account_addr, id) &lt; amount;
    aborts_if !exists&lt;TokenStore&gt;(account_addr);
    aborts_if !table::spec_contains(tokens, id);
&#125;
</code></pre>



<a id="@Specification_1_update_token_property_internal"></a>

### Function `update_token_property_internal`


<pre><code>fun update_token_property_internal(token_owner: address, token_id: token::TokenId, keys: vector&lt;string::String&gt;, values: vector&lt;vector&lt;u8&gt;&gt;, types: vector&lt;string::String&gt;)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
let tokens &#61; global&lt;TokenStore&gt;(token_owner).tokens;
aborts_if !exists&lt;TokenStore&gt;(token_owner);
aborts_if !table::spec_contains(tokens, token_id);
</code></pre>



<a id="@Specification_1_direct_deposit"></a>

### Function `direct_deposit`


<pre><code>fun direct_deposit(account_addr: address, token: token::Token)
</code></pre>




<pre><code>let token_id &#61; token.id;
let token_amount &#61; token.amount;
include DirectDepositAbortsIf;
</code></pre>




<a id="0x3_token_DirectDepositAbortsIf"></a>


<pre><code>schema DirectDepositAbortsIf &#123;
    account_addr: address;
    token_id: TokenId;
    token_amount: u64;
    let token_store &#61; global&lt;TokenStore&gt;(account_addr);
    let recipient_token &#61; table::spec_get(token_store.tokens, token_id);
    let b &#61; table::spec_contains(token_store.tokens, token_id);
    aborts_if token_amount &lt;&#61; 0;
    aborts_if !exists&lt;TokenStore&gt;(account_addr);
    aborts_if b &amp;&amp; recipient_token.id !&#61; token_id;
    aborts_if b &amp;&amp; recipient_token.amount &#43; token_amount &gt; MAX_U64;
&#125;
</code></pre>



<a id="@Specification_1_assert_collection_exists"></a>

### Function `assert_collection_exists`


<pre><code>fun assert_collection_exists(creator_address: address, collection_name: string::String)
</code></pre>


The collection_name should exist in collection_data of the creator_address's Collections.


<pre><code>include AssertCollectionExistsAbortsIf;
</code></pre>




<a id="0x3_token_AssertCollectionExistsAbortsIf"></a>


<pre><code>schema AssertCollectionExistsAbortsIf &#123;
    creator_address: address;
    collection_name: String;
    let all_collection_data &#61; global&lt;Collections&gt;(creator_address).collection_data;
    aborts_if !exists&lt;Collections&gt;(creator_address);
    aborts_if !table::spec_contains(all_collection_data, collection_name);
&#125;
</code></pre>



<a id="@Specification_1_assert_tokendata_exists"></a>

### Function `assert_tokendata_exists`


<pre><code>fun assert_tokendata_exists(creator: &amp;signer, token_data_id: token::TokenDataId)
</code></pre>


The creator of token_data_id should be signer.
The  creator of token_data_id exists in Collections.
The token_data_id is in the all_token_data.


<pre><code>include AssertTokendataExistsAbortsIf;
</code></pre>




<a id="0x3_token_AssertTokendataExistsAbortsIf"></a>


<pre><code>schema AssertTokendataExistsAbortsIf &#123;
    creator: signer;
    token_data_id: TokenDataId;
    let creator_addr &#61; token_data_id.creator;
    let addr &#61; signer::address_of(creator);
    aborts_if addr !&#61; creator_addr;
    aborts_if !exists&lt;Collections&gt;(creator_addr);
    let all_token_data &#61; global&lt;Collections&gt;(creator_addr).token_data;
    aborts_if !table::spec_contains(all_token_data, token_data_id);
&#125;
</code></pre>



<a id="@Specification_1_assert_non_standard_reserved_property"></a>

### Function `assert_non_standard_reserved_property`


<pre><code>fun assert_non_standard_reserved_property(keys: &amp;vector&lt;string::String&gt;)
</code></pre>




<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_initialize_token_script"></a>

### Function `initialize_token_script`


<pre><code>public entry fun initialize_token_script(_account: &amp;signer)
</code></pre>


Deprecated function


<pre><code>pragma verify &#61; false;
</code></pre>



<a id="@Specification_1_initialize_token"></a>

### Function `initialize_token`


<pre><code>public fun initialize_token(_account: &amp;signer, _token_id: token::TokenId)
</code></pre>


Deprecated function


<pre><code>pragma verify &#61; false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
