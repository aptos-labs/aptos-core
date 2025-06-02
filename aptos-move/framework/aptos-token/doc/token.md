
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
-  [Struct `TokenDeposit`](#0x3_token_TokenDeposit)
-  [Struct `Deposit`](#0x3_token_Deposit)
-  [Struct `WithdrawEvent`](#0x3_token_WithdrawEvent)
-  [Struct `Withdraw`](#0x3_token_Withdraw)
-  [Struct `TokenWithdraw`](#0x3_token_TokenWithdraw)
-  [Struct `CreateTokenDataEvent`](#0x3_token_CreateTokenDataEvent)
-  [Struct `CreateTokenData`](#0x3_token_CreateTokenData)
-  [Struct `TokenDataCreation`](#0x3_token_TokenDataCreation)
-  [Struct `MintTokenEvent`](#0x3_token_MintTokenEvent)
-  [Struct `MintToken`](#0x3_token_MintToken)
-  [Struct `Mint`](#0x3_token_Mint)
-  [Struct `BurnTokenEvent`](#0x3_token_BurnTokenEvent)
-  [Struct `BurnToken`](#0x3_token_BurnToken)
-  [Struct `Burn`](#0x3_token_Burn)
-  [Struct `MutateTokenPropertyMapEvent`](#0x3_token_MutateTokenPropertyMapEvent)
-  [Struct `MutateTokenPropertyMap`](#0x3_token_MutateTokenPropertyMap)
-  [Struct `MutatePropertyMap`](#0x3_token_MutatePropertyMap)
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


<pre><code><b>use</b> <a href="../../aptos-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="property_map.md#0x3_property_map">0x3::property_map</a>;
<b>use</b> <a href="token_event_store.md#0x3_token_event_store">0x3::token_event_store</a>;
</code></pre>



<a id="0x3_token_Token"></a>

## Struct `Token`



<pre><code><b>struct</b> <a href="token.md#0x3_token_Token">Token</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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
<code>token_properties: <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenId">TokenId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>
 The address of the creator, eg: 0xcafe
</dd>
<dt>
<code>collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of collection; this is unique under the same account, eg: "Aptos Animal Collection"
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of the token; this is the same as the name field of TokenData
</dd>
</dl>


</details>

<a id="0x3_token_TokenData"></a>

## Struct `TokenData`

The shared TokenData by tokens with different property_version


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenData">TokenData</a> <b>has</b> store
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
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul-7Mv6vzm7JwE69O-I-vd6Bz2QriJO1niwCh4
</dd>
<dt>
<code>royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a></code>
</dt>
<dd>
 The denominator and numerator for calculating the royalty fee; it also contains payee account address for depositing the Royalty
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: "Aptos Animal #1234"
</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Describes this Token
</dd>
<dt>
<code>default_properties: <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a></code>
</dt>
<dd>
 The properties are stored in the TokenData that are shared by all tokens
</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>
 Control the TokenData field mutability
</dd>
</dl>


</details>

<a id="0x3_token_Royalty"></a>

## Struct `Royalty`

The royalty of a token


<pre><code><b>struct</b> <a href="token.md#0x3_token_Royalty">Royalty</a> <b>has</b> <b>copy</b>, drop, store
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
<code>payee_address: <b>address</b></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> <b>has</b> <b>copy</b>, drop, store
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>tokens: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenId">token::TokenId</a>, <a href="token.md#0x3_token_Token">token::Token</a>&gt;</code>
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
<code>deposit_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_DepositEvent">token::DepositEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>withdraw_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">token::WithdrawEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">token::BurnTokenEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mutate_token_property_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">token::MutateTokenPropertyMapEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionMutabilityConfig"></a>

## Struct `CollectionMutabilityConfig`

This config specifies which fields in the Collection are mutable


<pre><code><b>struct</b> <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> <b>has</b> <b>copy</b>, drop, store
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_Collections">Collections</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>collection_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="token.md#0x3_token_CollectionData">token::CollectionData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>token_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, <a href="token.md#0x3_token_TokenData">token::TokenData</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_collection_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">token::CreateCollectionEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>create_token_data_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">token::CreateTokenDataEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>mint_token_events: <a href="../../aptos-framework/doc/event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">token::MintTokenEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CollectionData"></a>

## Struct `CollectionData`

Represent the collection metadata


<pre><code><b>struct</b> <a href="token.md#0x3_token_CollectionData">CollectionData</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 A description for the token collection Eg: "Aptos Toad Overload"
</dd>
<dt>
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: "Animal Collection"
</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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
<code>mutability_config: <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a></code>
</dt>
<dd>
 control which collectionData field is mutable
</dd>
</dl>


</details>

<a id="0x3_token_WithdrawCapability"></a>

## Struct `WithdrawCapability`

capability to withdraw without signer, this struct should be non-copyable


<pre><code><b>struct</b> <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>token_owner: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_DepositEvent">DepositEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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

<a id="0x3_token_TokenDeposit"></a>

## Struct `TokenDeposit`

Set of data sent to the event stream during a receive


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_TokenDeposit">TokenDeposit</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_Deposit">Deposit</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_Withdraw">Withdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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

<a id="0x3_token_TokenWithdraw"></a>

## Struct `TokenWithdraw`

Set of data sent to the event stream during a withdrawal


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_TokenWithdraw">TokenWithdraw</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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


<pre><code><b>struct</b> <a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: <b>address</b></code>
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
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateTokenData"></a>

## Struct `CreateTokenData`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_CreateTokenData">CreateTokenData</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: <b>address</b></code>
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
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_TokenDataCreation"></a>

## Struct `TokenDataCreation`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_TokenDataCreation">TokenDataCreation</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>maximum: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>royalty_payee_address: <b>address</b></code>
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
<code>name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>mutability_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MintTokenEvent"></a>

## Struct `MintTokenEvent`

mint token event. This event triggered when creator adds more supply to existing token


<pre><code><b>struct</b> <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
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



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_MintToken">MintToken</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
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

<a id="0x3_token_Mint"></a>

## Struct `Mint`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_Mint">Mint</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a></code>
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



<pre><code><b>struct</b> <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_BurnToken">BurnToken</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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

<a id="0x3_token_Burn"></a>

## Struct `Burn`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_Burn">Burn</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
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



<pre><code><b>struct</b> <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MutateTokenPropertyMap"></a>

## Struct `MutateTokenPropertyMap`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
#[deprecated]
<b>struct</b> <a href="token.md#0x3_token_MutateTokenPropertyMap">MutateTokenPropertyMap</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_MutatePropertyMap"></a>

## Struct `MutatePropertyMap`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_MutatePropertyMap">MutatePropertyMap</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a></code>
</dt>
<dd>

</dd>
<dt>
<code>keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x3_token_CreateCollectionEvent"></a>

## Struct `CreateCollectionEvent`

create collection event with creator address and collection name


<pre><code><b>struct</b> <a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="token.md#0x3_token_CreateCollection">CreateCollection</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
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


<pre><code><b>const</b> <a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>: u64 = 5;
</code></pre>



<a id="0x3_token_EURI_TOO_LONG"></a>

The URI is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>: u64 = 27;
</code></pre>



<a id="0x3_token_MAX_URI_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>: u64 = 512;
</code></pre>



<a id="0x3_token_BURNABLE_BY_CREATOR"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 67, 82, 69, 65, 84, 79, 82];
</code></pre>



<a id="0x3_token_BURNABLE_BY_OWNER"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [84, 79, 75, 69, 78, 95, 66, 85, 82, 78, 65, 66, 76, 69, 95, 66, 89, 95, 79, 87, 78, 69, 82];
</code></pre>



<a id="0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>: u64 = 0;
</code></pre>



<a id="0x3_token_COLLECTION_MAX_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>: u64 = 2;
</code></pre>



<a id="0x3_token_COLLECTION_URI_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>: u64 = 1;
</code></pre>



<a id="0x3_token_EALREADY_HAS_BALANCE"></a>

The token has balance and cannot be initialized


<pre><code><b>const</b> <a href="token.md#0x3_token_EALREADY_HAS_BALANCE">EALREADY_HAS_BALANCE</a>: u64 = 0;
</code></pre>



<a id="0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY"></a>

Reserved fields for token contract
Cannot be updated by user


<pre><code><b>const</b> <a href="token.md#0x3_token_ECANNOT_UPDATE_RESERVED_PROPERTY">ECANNOT_UPDATE_RESERVED_PROPERTY</a>: u64 = 32;
</code></pre>



<a id="0x3_token_ECOLLECTIONS_NOT_PUBLISHED"></a>

There isn't any collection under this account


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>: u64 = 1;
</code></pre>



<a id="0x3_token_ECOLLECTION_ALREADY_EXISTS"></a>

The collection already exists


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_ALREADY_EXISTS">ECOLLECTION_ALREADY_EXISTS</a>: u64 = 3;
</code></pre>



<a id="0x3_token_ECOLLECTION_NAME_TOO_LONG"></a>

The collection name is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>: u64 = 25;
</code></pre>



<a id="0x3_token_ECOLLECTION_NOT_PUBLISHED"></a>

Cannot find collection in creator's account


<pre><code><b>const</b> <a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>: u64 = 2;
</code></pre>



<a id="0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM"></a>

Exceeds the collection's maximal number of token_data


<pre><code><b>const</b> <a href="token.md#0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM">ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM</a>: u64 = 4;
</code></pre>



<a id="0x3_token_ECREATOR_CANNOT_BURN_TOKEN"></a>

Token is not burnable by creator


<pre><code><b>const</b> <a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>: u64 = 31;
</code></pre>



<a id="0x3_token_EFIELD_NOT_MUTABLE"></a>

The field is not mutable


<pre><code><b>const</b> <a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>: u64 = 13;
</code></pre>



<a id="0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT"></a>

Withdraw capability doesn't have sufficient amount


<pre><code><b>const</b> <a href="token.md#0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT">EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT</a>: u64 = 38;
</code></pre>



<a id="0x3_token_EINVALID_MAXIMUM"></a>

Collection or tokendata maximum must be larger than supply


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>: u64 = 36;
</code></pre>



<a id="0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR"></a>

Royalty invalid if the numerator is larger than the denominator


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>: u64 = 34;
</code></pre>



<a id="0x3_token_EINVALID_TOKEN_MERGE"></a>

Cannot merge the two tokens with different token id


<pre><code><b>const</b> <a href="token.md#0x3_token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>: u64 = 6;
</code></pre>



<a id="0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM"></a>

Exceed the token data maximal allowed


<pre><code><b>const</b> <a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>: u64 = 7;
</code></pre>



<a id="0x3_token_ENFT_NAME_TOO_LONG"></a>

The NFT name is too long


<pre><code><b>const</b> <a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>: u64 = 26;
</code></pre>



<a id="0x3_token_ENFT_NOT_SPLITABLE"></a>

Cannot split a token that only has 1 amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ENFT_NOT_SPLITABLE">ENFT_NOT_SPLITABLE</a>: u64 = 18;
</code></pre>



<a id="0x3_token_ENO_BURN_CAPABILITY"></a>

No burn capability


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_BURN_CAPABILITY">ENO_BURN_CAPABILITY</a>: u64 = 8;
</code></pre>



<a id="0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot burn 0 Token


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>: u64 = 29;
</code></pre>



<a id="0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT"></a>

Cannot deposit a Token with 0 amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT">ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT</a>: u64 = 28;
</code></pre>



<a id="0x3_token_ENO_MINT_CAPABILITY"></a>

No mint capability


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>: u64 = 19;
</code></pre>



<a id="0x3_token_ENO_MUTATE_CAPABILITY"></a>

Not authorized to mutate


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>: u64 = 14;
</code></pre>



<a id="0x3_token_ENO_TOKEN_IN_TOKEN_STORE"></a>

Token not in the token store


<pre><code><b>const</b> <a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>: u64 = 15;
</code></pre>



<a id="0x3_token_EOWNER_CANNOT_BURN_TOKEN"></a>

Token is not burnable by owner


<pre><code><b>const</b> <a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>: u64 = 30;
</code></pre>



<a id="0x3_token_EPROPERTY_RESERVED_BY_STANDARD"></a>

The property is reserved by token standard


<pre><code><b>const</b> <a href="token.md#0x3_token_EPROPERTY_RESERVED_BY_STANDARD">EPROPERTY_RESERVED_BY_STANDARD</a>: u64 = 40;
</code></pre>



<a id="0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST"></a>

Royalty payee account does not exist


<pre><code><b>const</b> <a href="token.md#0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST">EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST</a>: u64 = 35;
</code></pre>



<a id="0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT"></a>

TOKEN with 0 amount is not allowed


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>: u64 = 33;
</code></pre>



<a id="0x3_token_ETOKEN_DATA_ALREADY_EXISTS"></a>

TokenData already exists


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_DATA_ALREADY_EXISTS">ETOKEN_DATA_ALREADY_EXISTS</a>: u64 = 9;
</code></pre>



<a id="0x3_token_ETOKEN_DATA_NOT_PUBLISHED"></a>

TokenData not published


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>: u64 = 10;
</code></pre>



<a id="0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH"></a>

Token Properties count doesn't match


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>: u64 = 37;
</code></pre>



<a id="0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT"></a>

Cannot split token to an amount larger than its amount


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT">ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT</a>: u64 = 12;
</code></pre>



<a id="0x3_token_ETOKEN_STORE_NOT_PUBLISHED"></a>

TokenStore doesn't exist


<pre><code><b>const</b> <a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>: u64 = 11;
</code></pre>



<a id="0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER"></a>

User didn't opt-in direct transfer


<pre><code><b>const</b> <a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>: u64 = 16;
</code></pre>



<a id="0x3_token_EWITHDRAW_PROOF_EXPIRES"></a>

Withdraw proof expires


<pre><code><b>const</b> <a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>: u64 = 39;
</code></pre>



<a id="0x3_token_EWITHDRAW_ZERO"></a>

Cannot withdraw 0 token


<pre><code><b>const</b> <a href="token.md#0x3_token_EWITHDRAW_ZERO">EWITHDRAW_ZERO</a>: u64 = 17;
</code></pre>



<a id="0x3_token_MAX_COLLECTION_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a id="0x3_token_MAX_NFT_NAME_LENGTH"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>: u64 = 128;
</code></pre>



<a id="0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>: u64 = 3;
</code></pre>



<a id="0x3_token_TOKEN_MAX_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>: u64 = 0;
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [84, 79, 75, 69, 78, 95, 80, 82, 79, 80, 69, 82, 84, 89, 95, 77, 85, 84, 65, 84, 66, 76, 69];
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>: u64 = 4;
</code></pre>



<a id="0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_PROPERTY_VALUE_MUTABLE_IND">TOKEN_PROPERTY_VALUE_MUTABLE_IND</a>: u64 = 5;
</code></pre>



<a id="0x3_token_TOKEN_ROYALTY_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>: u64 = 2;
</code></pre>



<a id="0x3_token_TOKEN_URI_MUTABLE_IND"></a>



<pre><code><b>const</b> <a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>: u64 = 1;
</code></pre>



<a id="0x3_token_create_collection_script"></a>

## Function `create_collection_script`

create a empty token collection with parameters


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    name: String,
    description: String,
    uri: String,
    maximum: u64,
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_create_collection">create_collection</a>(
        creator,
        name,
        description,
        uri,
        maximum,
        mutate_setting
    );
}
</code></pre>



</details>

<a id="0x3_token_create_token_script"></a>

## Function `create_token_script`

create token with raw inputs


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, balance: u64, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    name: String,
    description: String,
    balance: u64,
    maximum: u64,
    uri: String,
    royalty_payee_address: <b>address</b>,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> token_mut_config = <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(&mutate_setting);
    <b>let</b> tokendata_id = <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
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

    <a href="token.md#0x3_token_mint_token">mint_token</a>(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        tokendata_id,
        balance,
    );
}
</code></pre>



</details>

<a id="0x3_token_mint_script"></a>

## Function `mint_script`

Mint more token from an existing token_data. Mint only adds more token to property_version 0


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_data_address: <b>address</b>,
    collection: String,
    name: String,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> token_data_id = <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(
        token_data_address,
        collection,
        name,
    );
    // only creator of the tokendata can mint more tokens for now
    <b>assert</b>!(token_data_id.creator == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));
    <a href="token.md#0x3_token_mint_token">mint_token</a>(
        <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        token_data_id,
        amount,
    );
}
</code></pre>



</details>

<a id="0x3_token_mutate_token_properties"></a>

## Function `mutate_token_properties`

mutate the token property and save the new property in TokenStore
if the token property_version is 0, we will create a new property_version per token to generate a new token_id per token
if the token property_version is not 0, we will just update the propertyMap and use the existing token_id (property_version)


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, amount: u64, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_owner: <b>address</b>,
    creator: <b>address</b>,
    collection_name: String,
    token_name: String,
    token_property_version: u64,
    amount: u64,
    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>) == creator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(
        creator,
        collection_name,
        token_name,
        token_property_version,
    );
    // give a new property_version for each <a href="token.md#0x3_token">token</a>
    for (i in 0..amount) {
        <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, token_owner, token_id, keys, values, types);
    };
}
</code></pre>



</details>

<a id="0x3_token_direct_transfer_script"></a>

## Function `direct_transfer_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creators_address: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creators_address, collection, name, property_version);
    <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender, receiver, token_id, amount);
}
</code></pre>



</details>

<a id="0x3_token_opt_in_direct_transfer"></a>

## Function `opt_in_direct_transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> opt_in_flag = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[addr].direct_transfer;
    *opt_in_flag = opt_in;
    <a href="token_event_store.md#0x3_token_event_store_emit_token_opt_in_event">token_event_store::emit_token_opt_in_event</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, opt_in);
}
</code></pre>



</details>

<a id="0x3_token_transfer_with_opt_in"></a>

## Function `transfer_with_opt_in`

Transfers <code>amount</code> of tokens from <code>from</code> to <code><b>to</b></code>.
The receiver <code><b>to</b></code> has to opt-in direct transfer first


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(
    from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creator: <b>address</b>,
    collection_name: String,
    token_name: String,
    token_property_version: u64,
    <b>to</b>: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator, collection_name, token_name, token_property_version);
    <a href="token.md#0x3_token_transfer">transfer</a>(from, token_id, <b>to</b>, amount);
}
</code></pre>



</details>

<a id="0x3_token_burn_by_creator"></a>

## Function `burn_by_creator`

Burn a token by creator when the token's BURNABLE_BY_CREATOR is true
The token is owned at address owner


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    owner: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>));
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator_address, collection, name, property_version);
    <b>let</b> creator_addr = token_id.token_data_id.creator;
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),
    );

    <b>let</b> collections = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_address];
    <b>assert</b>!(
        collections.token_data.contains(token_id.token_data_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>),
    );

    <b>let</b> token_data = collections.token_data.borrow_mut(token_id.token_data_id);

    // The property should be explicitly set in the <a href="property_map.md#0x3_property_map">property_map</a> for creator <b>to</b> burn the <a href="token.md#0x3_token">token</a>
    <b>assert</b>!(
        token_data.default_properties.contains_key(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>)),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>)
    );

    <b>let</b> burn_by_creator_flag = token_data.default_properties.read_bool(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>));
    <b>assert</b>!(burn_by_creator_flag, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ECREATOR_CANNOT_BURN_TOKEN">ECREATOR_CANNOT_BURN_TOKEN</a>));

    // <a href="token.md#0x3_token_Burn">Burn</a> the tokens.
    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> { id: _, amount: burned_amount, token_properties: _ } = <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(owner, token_id, amount);
    <b>let</b> token_store = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[owner];
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Burn">Burn</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: owner, id: token_id, amount: burned_amount });
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(
            &<b>mut</b> token_store.burn_events,
            <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> { id: token_id, amount: burned_amount }
        );
    };

    <b>if</b> (token_data.maximum &gt; 0) {
        token_data.supply -= burned_amount;

        // Delete the token_data <b>if</b> supply drops <b>to</b> 0.
        <b>if</b> (token_data.supply == 0) {
            <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(collections.token_data.remove(token_id.token_data_id));

            // <b>update</b> the collection supply
            <b>let</b> collection_data = collections.collection_data.borrow_mut(token_id.token_data_id.collection);
            <b>if</b> (collection_data.maximum &gt; 0) {
                collection_data.supply -= 1;
                // delete the collection data <b>if</b> the collection supply equals 0
                <b>if</b> (collection_data.supply == 0) {
                    <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collections.collection_data.remove(collection_data.name));
                };
            };
        };
    };
}
</code></pre>



</details>

<a id="0x3_token_burn"></a>

## Function `burn`

Burn a token by the token owner


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creators_address: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
    amount: u64
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENO_BURN_TOKEN_WITH_ZERO_AMOUNT">ENO_BURN_TOKEN_WITH_ZERO_AMOUNT</a>));
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creators_address, collection, name, property_version);
    <b>let</b> creator_addr = token_id.token_data_id.creator;
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),
    );

    <b>let</b> collections = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_addr];
    <b>assert</b>!(
        collections.token_data.contains(token_id.token_data_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>),
    );

    <b>let</b> token_data = collections.token_data.borrow_mut(token_id.token_data_id);

    <b>assert</b>!(
        token_data.default_properties.contains_key(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>)),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>)
    );
    <b>let</b> burn_by_owner_flag = token_data.default_properties.read_bool(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>));
    <b>assert</b>!(burn_by_owner_flag, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EOWNER_CANNOT_BURN_TOKEN">EOWNER_CANNOT_BURN_TOKEN</a>));

    // <a href="token.md#0x3_token_Burn">Burn</a> the tokens.
    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> { id: _, amount: burned_amount, token_properties: _ } = <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(owner, token_id, amount);
    <b>let</b> token_store = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)];
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Burn">Burn</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), id: token_id, amount: burned_amount });
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(
            &<b>mut</b> token_store.burn_events,
            <a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a> { id: token_id, amount: burned_amount }
        );
    };

    // Decrease the supply correspondingly by the amount of tokens burned.
    <b>let</b> token_data = collections.token_data.borrow_mut(token_id.token_data_id);

    // only <b>update</b> the supply <b>if</b> we tracking the supply and maximal
    // maximal == 0 is reserved for unlimited <a href="token.md#0x3_token">token</a> and collection <b>with</b> no tracking info.
    <b>if</b> (token_data.maximum &gt; 0) {
        token_data.supply -= burned_amount;

        // Delete the token_data <b>if</b> supply drops <b>to</b> 0.
        <b>if</b> (token_data.supply == 0) {
            <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(collections.token_data.remove(token_id.token_data_id));

            // <b>update</b> the collection supply
            <b>let</b> collection_data = collections.collection_data.borrow_mut(token_id.token_data_id.collection);

            // only <b>update</b> and check the supply for unlimited collection
            <b>if</b> (collection_data.maximum &gt; 0){
                collection_data.supply -= 1;
                // delete the collection data <b>if</b> the collection supply equals 0
                <b>if</b> (collection_data.supply == 0) {
                    <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collections.collection_data.remove(collection_data.name));
                };
            };
        };
    };
}
</code></pre>



</details>

<a id="0x3_token_mutate_collection_description"></a>

## Function `mutate_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, description: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    <b>assert</b>!(collection_data.mutability_config.description, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_collection_description_mutate_event">token_event_store::emit_collection_description_mutate_event</a>(creator, collection_name, collection_data.description, description);
    collection_data.description = description;
}
</code></pre>



</details>

<a id="0x3_token_mutate_collection_uri"></a>

## Function `mutate_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, uri: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(uri.length() &lt;= <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    <b>assert</b>!(collection_data.mutability_config.uri, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_collection_uri_mutate_event">token_event_store::emit_collection_uri_mutate_event</a>(creator, collection_name, collection_data.uri , uri);
    collection_data.uri = uri;
}
</code></pre>



</details>

<a id="0x3_token_mutate_collection_maximum"></a>

## Function `mutate_collection_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: String, maximum: u64) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    // cannot change maximum from 0 and cannot change maximum <b>to</b> 0
    <b>assert</b>!(collection_data.maximum != 0 && maximum != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));
    <b>assert</b>!(maximum &gt;= collection_data.supply, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));
    <b>assert</b>!(collection_data.mutability_config.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_collection_maximum_mutate_event">token_event_store::emit_collection_maximum_mutate_event</a>(creator, collection_name, collection_data.maximum, maximum);
    collection_data.maximum = maximum;
}
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_maximum"></a>

## Function `mutate_tokendata_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, maximum: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, maximum: u64) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);
    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[token_data_id.creator].token_data;
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);
    // cannot change maximum from 0 and cannot change maximum <b>to</b> 0
    <b>assert</b>!(token_data.maximum != 0 && maximum != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));
    <b>assert</b>!(maximum &gt;= token_data.supply, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_MAXIMUM">EINVALID_MAXIMUM</a>));
    <b>assert</b>!(token_data.mutability_config.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_token_maximum_mutate_event">token_event_store::emit_token_maximum_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.maximum, maximum);
    token_data.maximum = maximum;
}
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_uri"></a>

## Function `mutate_tokendata_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,
    uri: String
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(uri.length() &lt;= <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);

    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[token_data_id.creator].token_data;
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);
    <b>assert</b>!(token_data.mutability_config.uri, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_token_uri_mutate_event">token_event_store::emit_token_uri_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.uri ,uri);
    token_data.uri = uri;
}
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_royalty"></a>

## Function `mutate_tokendata_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">Royalty</a>) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);

    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[token_data_id.creator].token_data;
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);
    <b>assert</b>!(token_data.mutability_config.royalty, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));

    <a href="token_event_store.md#0x3_token_event_store_emit_token_royalty_mutate_event">token_event_store::emit_token_royalty_mutate_event</a>(
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
    token_data.royalty = royalty;
}
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_description"></a>

## Function `mutate_tokendata_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, description: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);

    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[token_data_id.creator].token_data;
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);
    <b>assert</b>!(token_data.mutability_config.description, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <a href="token_event_store.md#0x3_token_event_store_emit_token_descrition_mutate_event">token_event_store::emit_token_descrition_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, token_data.description, description);
    token_data.description = description;
}
</code></pre>



</details>

<a id="0x3_token_mutate_tokendata_property"></a>

## Function `mutate_tokendata_property`

Allow creator to mutate the default properties in TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,
    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator, token_data_id);
    <b>let</b> key_len = keys.length();
    <b>let</b> val_len = values.length();
    <b>let</b> typ_len = types.length();
    <b>assert</b>!(key_len == val_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>));
    <b>assert</b>!(key_len == typ_len, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ETOKEN_PROPERTIES_COUNT_NOT_MATCH">ETOKEN_PROPERTIES_COUNT_NOT_MATCH</a>));

    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[token_data_id.creator].token_data;
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);
    <b>assert</b>!(token_data.mutability_config.properties, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    <b>let</b> old_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Option&lt;PropertyValue&gt;&gt; = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <b>let</b> new_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;PropertyValue&gt; = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(&keys);
    for (i in 0..keys.length()){
        <b>let</b> key = keys.borrow(i);
        <b>let</b> old_pv = <b>if</b> (token_data.default_properties.contains_key(key)) {
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*token_data.default_properties.borrow(key))
        } <b>else</b> {
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;PropertyValue&gt;()
        };
        old_values.push_back(old_pv);
        <b>let</b> new_pv = <a href="property_map.md#0x3_property_map_create_property_value_raw">property_map::create_property_value_raw</a>(values[i], types[i]);
        new_values.push_back(new_pv);
        <b>if</b> (old_pv.is_some()) {
            token_data.default_properties.update_property_value(key, new_pv);
        } <b>else</b> {
            token_data.default_properties.add(*key, new_pv);
        };
    };
    <a href="token_event_store.md#0x3_token_event_store_emit_default_property_mutate_event">token_event_store::emit_default_property_mutate_event</a>(creator, token_data_id.collection, token_data_id.name, keys, old_values, new_values);
}
</code></pre>



</details>

<a id="0x3_token_mutate_one_token"></a>

## Function `mutate_one_token`

Mutate the token_properties of one token.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_owner: <b>address</b>,
    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
): <a href="token.md#0x3_token_TokenId">TokenId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> creator = token_id.token_data_id.creator;
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>) == creator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));
    // validate <b>if</b> the properties is mutable
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[
        creator
    ].token_data;

    <b>assert</b>!(all_token_data.contains(token_id.token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    <b>let</b> token_data = all_token_data.borrow_mut(token_id.token_data_id);

    // <b>if</b> default property is mutatable, <a href="token.md#0x3_token">token</a> property is always mutable
    // we only need <b>to</b> check <a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a> when default property is immutable
    <b>if</b> (!token_data.mutability_config.properties) {
        <b>assert</b>!(
            token_data.default_properties.contains_key(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>)),
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>)
        );

        <b>let</b> token_prop_mutable = token_data.default_properties.read_bool(&<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>));
        <b>assert</b>!(token_prop_mutable, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EFIELD_NOT_MUTABLE">EFIELD_NOT_MUTABLE</a>));
    };

    // check <b>if</b> the property_version is 0 <b>to</b> determine <b>if</b> we need <b>to</b> <b>update</b> the property_version
    <b>if</b> (token_id.property_version == 0) {
        <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(token_owner, token_id, 1);
        // give a new property_version for each <a href="token.md#0x3_token">token</a>
        <b>let</b> cur_property_version = token_data.largest_property_version + 1;
        <b>let</b> new_token_id = <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_id.token_data_id, cur_property_version);
        <b>let</b> new_token = <a href="token.md#0x3_token_Token">Token</a> {
            id: new_token_id,
            amount: 1,
            token_properties: token_data.default_properties,
        };
        <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(token_owner, new_token);
        <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner, new_token_id, keys, values, types);
        <b>if</b> (std::features::module_event_migration_enabled()) {
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MutatePropertyMap">MutatePropertyMap</a> {
                <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: token_owner,
                old_id: token_id,
                new_id: new_token_id,
                keys,
                values,
                types
            });
        } <b>else</b> {
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(
                &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[token_owner].mutate_token_property_events,
                <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> {
                    old_id: token_id,
                    new_id: new_token_id,
                    keys,
                    values,
                    types
                },
            );
        };

        token_data.largest_property_version = cur_property_version;
        // burn the orignial property_version 0 <a href="token.md#0x3_token">token</a> after mutation
        <b>let</b> <a href="token.md#0x3_token_Token">Token</a> { id: _, amount: _, token_properties: _ } = <a href="token.md#0x3_token">token</a>;
        new_token_id
    } <b>else</b> {
        // only 1 <b>copy</b> for the <a href="token.md#0x3_token">token</a> <b>with</b> property verion bigger than 0
        <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner, token_id, keys, values, types);
        <b>if</b> (std::features::module_event_migration_enabled()) {
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_MutatePropertyMap">MutatePropertyMap</a> {
                <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: token_owner,
                old_id: token_id,
                new_id: token_id,
                keys,
                values,
                types
            });
        } <b>else</b> {
            <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(
                &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[token_owner].mutate_token_property_events,
                <a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a> {
                    old_id: token_id,
                    new_id: token_id,
                    keys,
                    values,
                    types
                },
            );
        };
        token_id
    }
}
</code></pre>



</details>

<a id="0x3_token_create_royalty"></a>

## Function `create_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">Royalty</a> {
    <b>assert</b>!(royalty_points_numerator &lt;= royalty_points_denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>));
    // Question[Orderless]: Is it okay <b>to</b> remove this check <b>to</b> accommodate stateless accounts?
    // <b>assert</b>!(<a href="../../aptos-framework/doc/account.md#0x1_account_exists_at">account::exists_at</a>(payee_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST">EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST</a>));
    <a href="token.md#0x3_token_Royalty">Royalty</a> {
        royalty_points_numerator,
        royalty_points_denominator,
        payee_address
    }
}
</code></pre>



</details>

<a id="0x3_token_deposit_token"></a>

## Function `deposit_token`

Deposit the token balance into the owner's account and emit an event.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr, <a href="token.md#0x3_token">token</a>)
}
</code></pre>



</details>

<a id="0x3_token_direct_deposit_with_opt_in"></a>

## Function `direct_deposit_with_opt_in`

direct deposit if user opt in direct transfer


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> opt_in_transfer = <a href="token.md#0x3_token_TokenStore">TokenStore</a>[account_addr].direct_transfer;
    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));
    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr, <a href="token.md#0x3_token">token</a>);
}
</code></pre>



</details>

<a id="0x3_token_direct_transfer"></a>

## Function `direct_transfer`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(sender, token_id, amount);
    <a href="token.md#0x3_token_deposit_token">deposit_token</a>(receiver, <a href="token.md#0x3_token">token</a>);
}
</code></pre>



</details>

<a id="0x3_token_initialize_token_store"></a>

## Function `initialize_token_store`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>))) {
        <b>move_to</b>(
            <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
            <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
                tokens: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
                direct_transfer: <b>false</b>,
                deposit_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_DepositEvent">DepositEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),
                withdraw_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),
                burn_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_BurnTokenEvent">BurnTokenEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),
                mutate_token_property_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_MutateTokenPropertyMapEvent">MutateTokenPropertyMapEvent</a>&gt;(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>),
            },
        );
    }
}
</code></pre>



</details>

<a id="0x3_token_merge"></a>

## Function `merge`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, source_token: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">Token</a>, source_token: <a href="token.md#0x3_token_Token">Token</a>) {
    <b>assert</b>!(&dst_token.id == &source_token.id, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_TOKEN_MERGE">EINVALID_TOKEN_MERGE</a>));
    dst_token.amount += source_token.amount;
    <b>let</b> <a href="token.md#0x3_token_Token">Token</a> { id: _, amount: _, token_properties: _ } = source_token;
}
</code></pre>



</details>

<a id="0x3_token_split"></a>

## Function `split`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">Token</a>, amount: u64): <a href="token.md#0x3_token_Token">Token</a> {
    <b>assert</b>!(dst_token.id.property_version == 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="token.md#0x3_token_ENFT_NOT_SPLITABLE">ENFT_NOT_SPLITABLE</a>));
    <b>assert</b>!(dst_token.amount &gt; amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT">ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT</a>));
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>));
    dst_token.amount -= amount;
    <a href="token.md#0x3_token_Token">Token</a> {
        id: dst_token.id,
        amount,
        token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(),
    }
}
</code></pre>



</details>

<a id="0x3_token_token_id"></a>

## Function `token_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_token_id">token_id</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">token::Token</a>): &<a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_token_id">token_id</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">Token</a>): &<a href="token.md#0x3_token_TokenId">TokenId</a> {
    &<a href="token.md#0x3_token">token</a>.id
}
</code></pre>



</details>

<a id="0x3_token_transfer"></a>

## Function `transfer`

Transfers <code>amount</code> of tokens from <code>from</code> to <code><b>to</b></code>.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(
    from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    <b>to</b>: <b>address</b>,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> opt_in_transfer = <a href="token.md#0x3_token_TokenStore">TokenStore</a>[<b>to</b>].direct_transfer;
    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));
    <b>let</b> <a href="token.md#0x3_token">token</a> = <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(from, id, amount);
    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(<b>to</b>, <a href="token.md#0x3_token">token</a>);
}
</code></pre>



</details>

<a id="0x3_token_create_withdraw_capability"></a>

## Function `create_withdraw_capability`

Token owner can create this one-time withdraw capability with an expiration time


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_withdraw_capability">create_withdraw_capability</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64, expiration_sec: u64): <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_withdraw_capability">create_withdraw_capability</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    amount: u64,
    expiration_sec: u64,
): <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> {
    <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> {
        token_owner: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner),
        token_id,
        amount,
        expiration_sec,
    }
}
</code></pre>



</details>

<a id="0x3_token_withdraw_with_capability"></a>

## Function `withdraw_with_capability`

Withdraw the token with a capability


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(
    withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>,
): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    // verify the delegation hasn't expired yet
    <b>assert</b>!(<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;= withdraw_proof.expiration_sec, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>));

    <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(
        withdraw_proof.token_owner,
        withdraw_proof.token_id,
        withdraw_proof.amount,
    )
}
</code></pre>



</details>

<a id="0x3_token_partial_withdraw_with_capability"></a>

## Function `partial_withdraw_with_capability`

Withdraw the token with a capability.


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>, withdraw_amount: u64): (<a href="token.md#0x3_token_Token">token::Token</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(
    withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>,
    withdraw_amount: u64,
): (<a href="token.md#0x3_token_Token">Token</a>, Option&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt;) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    // verify the delegation hasn't expired yet
    <b>assert</b>!(<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;= withdraw_proof.expiration_sec, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_PROOF_EXPIRES">EWITHDRAW_PROOF_EXPIRES</a>));

    <b>assert</b>!(withdraw_amount &lt;= withdraw_proof.amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT">EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT</a>));

    <b>let</b> res: Option&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt; = <b>if</b> (withdraw_amount == withdraw_proof.amount) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a>&gt;()
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
            <a href="token.md#0x3_token_WithdrawCapability">WithdrawCapability</a> {
                token_owner: withdraw_proof.token_owner,
                token_id: withdraw_proof.token_id,
                amount: withdraw_proof.amount - withdraw_amount,
                expiration_sec: withdraw_proof.expiration_sec,
            }
        )
    };

    (
        <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(
            withdraw_proof.token_owner,
            withdraw_proof.token_id,
            withdraw_amount,
        ),
        res
    )

}
</code></pre>



</details>

<a id="0x3_token_withdraw_token"></a>

## Function `withdraw_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    amount: u64,
): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr, id, amount)
}
</code></pre>



</details>

<a id="0x3_token_create_collection"></a>

## Function `create_collection`

Create a new collection to hold tokens


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(
    creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    name: String,
    description: String,
    uri: String,
    maximum: u64,
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(name.length() &lt;= <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));
    <b>assert</b>!(uri.length() &lt;= <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr)) {
        <b>move_to</b>(
            creator,
            <a href="token.md#0x3_token_Collections">Collections</a> {
                collection_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
                token_data: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
                create_collection_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a>&gt;(creator),
                create_token_data_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a>&gt;(creator),
                mint_token_events: <a href="../../aptos-framework/doc/account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(creator),
            },
        )
    };

    <b>let</b> collection_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[account_addr].collection_data;

    <b>assert</b>!(
        !collection_data.contains(name),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="token.md#0x3_token_ECOLLECTION_ALREADY_EXISTS">ECOLLECTION_ALREADY_EXISTS</a>),
    );

    <b>let</b> mutability_config = <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(&mutate_setting);
    <b>let</b> collection = <a href="token.md#0x3_token_CollectionData">CollectionData</a> {
        description,
        name,
        uri,
        supply: 0,
        maximum,
        mutability_config
    };

    collection_data.add(name, collection);
    <b>let</b> collection_handle = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[account_addr];
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="token.md#0x3_token_CreateCollection">CreateCollection</a> {
                creator: account_addr,
                collection_name: name,
                uri,
                description,
                maximum,
            }
        );
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a>&gt;(
            &<b>mut</b> collection_handle.create_collection_events,
            <a href="token.md#0x3_token_CreateCollectionEvent">CreateCollectionEvent</a> {
                creator: account_addr,
                collection_name: name,
                uri,
                description,
                maximum,
            }
        );
    };
}
</code></pre>



</details>

<a id="0x3_token_check_collection_exists"></a>

## Function `check_collection_exists`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: String): bool <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),
    );

    <b>let</b> collection_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator].collection_data;
    collection_data.contains(name)
}
</code></pre>



</details>

<a id="0x3_token_check_tokendata_exists"></a>

## Function `check_tokendata_exists`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: String, token_name: String): bool <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),
    );

    <b>let</b> token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator].token_data;
    <b>let</b> token_data_id = <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator, collection_name, token_name);
    token_data.contains(token_data_id)
}
</code></pre>



</details>

<a id="0x3_token_create_tokendata"></a>

## Function `create_tokendata`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    collection: String,
    name: String,
    description: String,
    maximum: u64,
    uri: String,
    royalty_payee_address: <b>address</b>,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>,
    property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;
): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(name.length() &lt;= <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>));
    <b>assert</b>!(collection.length() &lt;= <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));
    <b>assert</b>!(uri.length() &lt;= <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EURI_TOO_LONG">EURI_TOO_LONG</a>));
    <b>assert</b>!(royalty_points_numerator &lt;= royalty_points_denominator, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINVALID_ROYALTY_NUMERATOR_DENOMINATOR">EINVALID_ROYALTY_NUMERATOR_DENOMINATOR</a>));

    <b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>),
    );
    <b>let</b> collections = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[account_addr];

    <b>let</b> token_data_id = <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(account_addr, collection, name);

    <b>assert</b>!(
        collections.collection_data.contains(token_data_id.collection),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>),
    );
    <b>assert</b>!(
        !collections.token_data.contains(token_data_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="token.md#0x3_token_ETOKEN_DATA_ALREADY_EXISTS">ETOKEN_DATA_ALREADY_EXISTS</a>),
    );

    <b>let</b> collection = collections.collection_data.borrow_mut(token_data_id.collection);

    // <b>if</b> collection maximum == 0, user don't want <b>to</b> enforce supply constraint.
    // we don't track supply <b>to</b> make <a href="token.md#0x3_token">token</a> creation parallelizable
    <b>if</b> (collection.maximum &gt; 0) {
        collection.supply += 1;
        <b>assert</b>!(
            collection.maximum &gt;= collection.supply,
            <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM">ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM</a>),
        );
    };

    <b>let</b> token_data = <a href="token.md#0x3_token_TokenData">TokenData</a> {
        maximum,
        largest_property_version: 0,
        supply: 0,
        uri,
        royalty: <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator, royalty_points_denominator, royalty_payee_address),
        name,
        description,
        default_properties: <a href="property_map.md#0x3_property_map_new">property_map::new</a>(property_keys, property_values, property_types),
        mutability_config: token_mutate_config,
    };

    collections.token_data.add(token_data_id, token_data);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            <a href="token.md#0x3_token_TokenDataCreation">TokenDataCreation</a> {
                creator: account_addr,
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
            }
        );
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a>&gt;(
            &<b>mut</b> collections.create_token_data_events,
            <a href="token.md#0x3_token_CreateTokenDataEvent">CreateTokenDataEvent</a> {
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
            },
        );
    };

    token_data_id
}
</code></pre>



</details>

<a id="0x3_token_get_collection_supply"></a>

## Function `get_collection_supply`

return the number of distinct token_data_id created under this collection


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: String): Option&lt;u64&gt; <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );

    <b>if</b> (collection_data.maximum &gt; 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(collection_data.supply)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x3_token_get_collection_description"></a>

## Function `get_collection_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: String): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    collection_data.description
}
</code></pre>



</details>

<a id="0x3_token_get_collection_uri"></a>

## Function `get_collection_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: String): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    collection_data.uri
}
</code></pre>



</details>

<a id="0x3_token_get_collection_maximum"></a>

## Function `get_collection_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: String): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address, collection_name);
    <b>let</b> collection_data = <a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data.borrow_mut(
        collection_name
    );
    collection_data.maximum
}
</code></pre>



</details>

<a id="0x3_token_get_token_supply"></a>

## Function `get_token_supply`

return the number of distinct token_id created under this TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): Option&lt;u64&gt; <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    <b>let</b> token_data = all_token_data.borrow(token_data_id);

    <b>if</b> (token_data.maximum &gt; 0) {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(token_data.supply)
    } <b>else</b> {
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>&lt;u64&gt;()
    }
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_largest_property_version"></a>

## Function `get_tokendata_largest_property_version`

return the largest_property_version of this TokenData


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    all_token_data.borrow(token_data_id).largest_property_version
}
</code></pre>



</details>

<a id="0x3_token_get_token_id"></a>

## Function `get_token_id`

return the TokenId for a given Token


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id">get_token_id</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">token::Token</a>): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id">get_token_id</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">Token</a>): <a href="token.md#0x3_token_TokenId">TokenId</a> {
    <a href="token.md#0x3_token">token</a>.id
}
</code></pre>



</details>

<a id="0x3_token_get_direct_transfer"></a>

## Function `get_direct_transfer`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_direct_transfer">get_direct_transfer</a>(receiver: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_direct_transfer">get_direct_transfer</a>(receiver: <b>address</b>): bool <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver)) {
        <b>return</b> <b>false</b>
    };

    <a href="token.md#0x3_token_TokenStore">TokenStore</a>[receiver].direct_transfer
}
</code></pre>



</details>

<a id="0x3_token_create_token_mutability_config"></a>

## Function `create_token_mutability_config`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> {
    <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> {
        maximum: mutate_setting[<a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>],
        uri: mutate_setting[<a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>],
        royalty: mutate_setting[<a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>],
        description: mutate_setting[<a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>],
        properties: mutate_setting[<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>],
    }
}
</code></pre>



</details>

<a id="0x3_token_create_collection_mutability_config"></a>

## Function `create_collection_mutability_config`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> {
    <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> {
        description: mutate_setting[<a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>],
        uri: mutate_setting[<a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>],
        maximum: mutate_setting[<a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>],
    }
}
</code></pre>



</details>

<a id="0x3_token_mint_token"></a>

## Function `mint_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,
    amount: u64,
): <a href="token.md#0x3_token_TokenId">TokenId</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(token_data_id.creator == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));
    <b>let</b> creator_addr = token_data_id.creator;
    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);

    <b>if</b> (token_data.maximum &gt; 0) {
        <b>assert</b>!(token_data.supply + amount &lt;= token_data.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>));
        token_data.supply += amount;
    };

    // we add more tokens <b>with</b> property_version 0
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Mint">Mint</a> { creator: creator_addr, id: token_data_id, amount })
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(
            &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].mint_token_events,
            <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> {
                id: token_data_id,
                amount,
            }
        );
    };

    <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
        <a href="token.md#0x3_token_Token">Token</a> {
            id: token_id,
            amount,
            token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(), // same <b>as</b> default properties no need <b>to</b> store
        }
    );

    token_id
}
</code></pre>



</details>

<a id="0x3_token_mint_token_to"></a>

## Function `mint_token_to`

create tokens and directly deposite to receiver's address. The receiver should opt-in direct transfer


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    receiver: <b>address</b>,
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>,
    amount: u64,
) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>));
    <b>let</b> opt_in_transfer = <a href="token.md#0x3_token_TokenStore">TokenStore</a>[receiver].direct_transfer;
    <b>assert</b>!(opt_in_transfer, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EUSER_NOT_OPT_IN_DIRECT_TRANSFER">EUSER_NOT_OPT_IN_DIRECT_TRANSFER</a>));

    <b>assert</b>!(token_data_id.creator == <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MINT_CAPABILITY">ENO_MINT_CAPABILITY</a>));
    <b>let</b> creator_addr = token_data_id.creator;
    <b>let</b> all_token_data = &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    <b>let</b> token_data = all_token_data.borrow_mut(token_data_id);

    <b>if</b> (token_data.maximum &gt; 0) {
        <b>assert</b>!(token_data.supply + amount &lt;= token_data.maximum, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EMINT_WOULD_EXCEED_TOKEN_MAXIMUM">EMINT_WOULD_EXCEED_TOKEN_MAXIMUM</a>));
        token_data.supply += amount;
    };

    // we add more tokens <b>with</b> property_version 0
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_Mint">Mint</a> { creator: creator_addr, id: token_data_id, amount })
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a>&gt;(
            &<b>mut</b> <a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].mint_token_events,
            <a href="token.md#0x3_token_MintTokenEvent">MintTokenEvent</a> {
                id: token_data_id,
                amount,
            }
        );
    };

    <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(receiver,
        <a href="token.md#0x3_token_Token">Token</a> {
            id: token_id,
            amount,
            token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>(), // same <b>as</b> default properties no need <b>to</b> store
        }
    );
}
</code></pre>



</details>

<a id="0x3_token_create_token_id"></a>

## Function `create_token_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">TokenId</a> {
    <a href="token.md#0x3_token_TokenId">TokenId</a> {
        token_data_id,
        property_version,
    }
}
</code></pre>



</details>

<a id="0x3_token_create_token_data_id"></a>

## Function `create_token_data_id`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(
    creator: <b>address</b>,
    collection: String,
    name: String,
): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> {
    <b>assert</b>!(collection.length() &lt;= <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ECOLLECTION_NAME_TOO_LONG">ECOLLECTION_NAME_TOO_LONG</a>));
    <b>assert</b>!(name.length() &lt;= <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ENFT_NAME_TOO_LONG">ENFT_NAME_TOO_LONG</a>));
    <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> { creator, collection, name }
}
</code></pre>



</details>

<a id="0x3_token_create_token_id_raw"></a>

## Function `create_token_id_raw`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(
    creator: <b>address</b>,
    collection: String,
    name: String,
    property_version: u64,
): <a href="token.md#0x3_token_TokenId">TokenId</a> {
    <a href="token.md#0x3_token_TokenId">TokenId</a> {
        token_data_id: <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator, collection, name),
        property_version,
    }
}
</code></pre>



</details>

<a id="0x3_token_balance_of"></a>

## Function `balance_of`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_balance_of">balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_balance_of">balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">TokenId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)) {
        <b>return</b> 0
    };
    <b>let</b> token_store = &<a href="token.md#0x3_token_TokenStore">TokenStore</a>[owner];
    <b>if</b> (token_store.tokens.contains(id)) {
        token_store.tokens.borrow(id).amount
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x3_token_has_token_store"></a>

## Function `has_token_store`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_has_token_store">has_token_store</a>(owner: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_has_token_store">has_token_store</a>(owner: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)
}
</code></pre>



</details>

<a id="0x3_token_get_royalty"></a>

## Function `get_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): <a href="token.md#0x3_token_Royalty">Royalty</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> token_data_id = token_id.token_data_id;
    <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id)
}
</code></pre>



</details>

<a id="0x3_token_get_royalty_numerator"></a>

## Function `get_royalty_numerator`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_numerator">get_royalty_numerator</a>(royalty: &<a href="token.md#0x3_token_Royalty">token::Royalty</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_numerator">get_royalty_numerator</a>(royalty: &<a href="token.md#0x3_token_Royalty">Royalty</a>): u64 {
    royalty.royalty_points_numerator
}
</code></pre>



</details>

<a id="0x3_token_get_royalty_denominator"></a>

## Function `get_royalty_denominator`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_denominator">get_royalty_denominator</a>(royalty: &<a href="token.md#0x3_token_Royalty">token::Royalty</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_denominator">get_royalty_denominator</a>(royalty: &<a href="token.md#0x3_token_Royalty">Royalty</a>): u64 {
    royalty.royalty_points_denominator
}
</code></pre>



</details>

<a id="0x3_token_get_royalty_payee"></a>

## Function `get_royalty_payee`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_payee">get_royalty_payee</a>(royalty: &<a href="token.md#0x3_token_Royalty">token::Royalty</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty_payee">get_royalty_payee</a>(royalty: &<a href="token.md#0x3_token_Royalty">Royalty</a>): <b>address</b> {
    royalty.payee_address
}
</code></pre>



</details>

<a id="0x3_token_get_token_amount"></a>

## Function `get_token_amount`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_amount">get_token_amount</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">token::Token</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_amount">get_token_amount</a>(<a href="token.md#0x3_token">token</a>: &<a href="token.md#0x3_token_Token">Token</a>): u64 {
    <a href="token.md#0x3_token">token</a>.amount
}
</code></pre>



</details>

<a id="0x3_token_get_token_id_fields"></a>

## Function `get_token_id_fields`

return the creator address, collection name, token name and property_version


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id_fields">get_token_id_fields</a>(token_id: &<a href="token.md#0x3_token_TokenId">token::TokenId</a>): (<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_id_fields">get_token_id_fields</a>(token_id: &<a href="token.md#0x3_token_TokenId">TokenId</a>): (<b>address</b>, String, String, u64) {
    (
        token_id.token_data_id.creator,
        token_id.token_data_id.collection,
        token_id.token_data_id.name,
        token_id.property_version,
    )
}
</code></pre>



</details>

<a id="0x3_token_get_token_data_id_fields"></a>

## Function `get_token_data_id_fields`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_data_id_fields">get_token_data_id_fields</a>(token_data_id: &<a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): (<b>address</b>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_data_id_fields">get_token_data_id_fields</a>(token_data_id: &<a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): (<b>address</b>, String, String) {
    (
        token_data_id.creator,
        token_data_id.collection,
        token_data_id.name,
    )
}
</code></pre>



</details>

<a id="0x3_token_get_property_map"></a>

## Function `get_property_map`

return a copy of the token property map.
if property_version = 0, return the default property map
if property_version > 0, return the property value stored at owner's token store


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): PropertyMap <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a>, <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(<a href="token.md#0x3_token_balance_of">balance_of</a>(owner, token_id) &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));
    // <b>if</b> property_version = 0, <b>return</b> default property map
    <b>if</b> (token_id.property_version == 0) {
        <b>let</b> creator_addr = token_id.token_data_id.creator;
        <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].token_data;
        <b>assert</b>!(all_token_data.contains(token_id.token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
        <b>let</b> token_data = all_token_data.borrow(token_id.token_data_id);
        token_data.default_properties
    } <b>else</b> {
        <b>let</b> tokens = &<a href="token.md#0x3_token_TokenStore">TokenStore</a>[owner].tokens;
        tokens.borrow(token_id).token_properties
    }
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_maximum"></a>

## Function `get_tokendata_maximum`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): u64 <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_address = token_data_id.creator;
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));

    <b>let</b> token_data = all_token_data.borrow(token_data_id);
    token_data.maximum
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_uri"></a>

## Function `get_tokendata_uri`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));

    <b>let</b> token_data = all_token_data.borrow(token_data_id);
    token_data.uri
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_description"></a>

## Function `get_tokendata_description`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): String <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_address = token_data_id.creator;
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));

    <b>let</b> token_data = all_token_data.borrow(token_data_id);
    token_data.description
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_royalty"></a>

## Function `get_tokendata_royalty`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): <a href="token.md#0x3_token_Royalty">Royalty</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_address = token_data_id.creator;
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));

    <b>let</b> token_data = all_token_data.borrow(token_data_id);
    token_data.royalty
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_id"></a>

## Function `get_tokendata_id`

return the token_data_id from the token_id


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_id">get_tokendata_id</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_id">get_tokendata_id</a>(token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> {
    token_id.token_data_id
}
</code></pre>



</details>

<a id="0x3_token_get_tokendata_mutability_config"></a>

## Function `get_tokendata_mutability_config`

return the mutation setting of the token


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_addr = token_data_id.creator;
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
    all_token_data.borrow(token_data_id).mutability_config
}
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_maximum"></a>

## Function `get_token_mutability_maximum`

return if the token's maximum is mutable


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_maximum">get_token_mutability_maximum</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_maximum">get_token_mutability_maximum</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool {
    config.maximum
}
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_royalty"></a>

## Function `get_token_mutability_royalty`

return if the token royalty is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_royalty">get_token_mutability_royalty</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_royalty">get_token_mutability_royalty</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool {
    config.royalty
}
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_uri"></a>

## Function `get_token_mutability_uri`

return if the token uri is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_uri">get_token_mutability_uri</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_uri">get_token_mutability_uri</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool {
    config.uri
}
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_description"></a>

## Function `get_token_mutability_description`

return if the token description is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_description">get_token_mutability_description</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_description">get_token_mutability_description</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool {
    config.description
}
</code></pre>



</details>

<a id="0x3_token_get_token_mutability_default_properties"></a>

## Function `get_token_mutability_default_properties`

return if the tokendata's default properties is mutable with a token mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_default_properties">get_token_mutability_default_properties</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_mutability_default_properties">get_token_mutability_default_properties</a>(config: &<a href="token.md#0x3_token_TokenMutabilityConfig">TokenMutabilityConfig</a>): bool {
    config.properties
}
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_config"></a>

## Function `get_collection_mutability_config`

return the collection mutation setting


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(
    creator: <b>address</b>,
    collection_name: String
): <a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a> <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_collection_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator].collection_data;
    <b>assert</b>!(all_collection_data.contains(collection_name), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>));
    all_collection_data.borrow(collection_name).mutability_config
}
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_description"></a>

## Function `get_collection_mutability_description`

return if the collection description is mutable with a collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_description">get_collection_mutability_description</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_description">get_collection_mutability_description</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool {
    config.description
}
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_uri"></a>

## Function `get_collection_mutability_uri`

return if the collection uri is mutable with a collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_uri">get_collection_mutability_uri</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_uri">get_collection_mutability_uri</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool {
    config.uri
}
</code></pre>



</details>

<a id="0x3_token_get_collection_mutability_maximum"></a>

## Function `get_collection_mutability_maximum`

return if the collection maximum is mutable with collection mutability config


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_maximum">get_collection_mutability_maximum</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_maximum">get_collection_mutability_maximum</a>(config: &<a href="token.md#0x3_token_CollectionMutabilityConfig">CollectionMutabilityConfig</a>): bool {
    config.maximum
}
</code></pre>



</details>

<a id="0x3_token_destroy_token_data"></a>

## Function `destroy_token_data`



<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(token_data: <a href="token.md#0x3_token_TokenData">token::TokenData</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_token_data">destroy_token_data</a>(token_data: <a href="token.md#0x3_token_TokenData">TokenData</a>) {
    <b>let</b> <a href="token.md#0x3_token_TokenData">TokenData</a> {
        maximum: _,
        largest_property_version: _,
        supply: _,
        uri: _,
        royalty: _,
        name: _,
        description: _,
        default_properties: _,
        mutability_config: _,
    } = token_data;
}
</code></pre>



</details>

<a id="0x3_token_destroy_collection_data"></a>

## Function `destroy_collection_data`



<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collection_data: <a href="token.md#0x3_token_CollectionData">token::CollectionData</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_destroy_collection_data">destroy_collection_data</a>(collection_data: <a href="token.md#0x3_token_CollectionData">CollectionData</a>) {
    <b>let</b> <a href="token.md#0x3_token_CollectionData">CollectionData</a> {
        description: _,
        name: _,
        uri: _,
        supply: _,
        maximum: _,
        mutability_config: _,
    } = collection_data;
}
</code></pre>



</details>

<a id="0x3_token_withdraw_with_event_internal"></a>

## Function `withdraw_with_event_internal`



<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(
    account_addr: <b>address</b>,
    id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    amount: u64,
): <a href="token.md#0x3_token_Token">Token</a> <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    // It does not make sense <b>to</b> withdraw 0 tokens.
    <b>assert</b>!(amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EWITHDRAW_ZERO">EWITHDRAW_ZERO</a>));
    // Make sure the <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> <b>has</b> sufficient tokens <b>to</b> withdraw.
    <b>assert</b>!(<a href="token.md#0x3_token_balance_of">balance_of</a>(account_addr, id) &gt;= amount, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_EINSUFFICIENT_BALANCE">EINSUFFICIENT_BALANCE</a>));

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>),
    );

    <b>let</b> token_store = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[account_addr];
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_TokenWithdraw">TokenWithdraw</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: account_addr, id, amount })
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a>&gt;(
            &<b>mut</b> token_store.withdraw_events,
            <a href="token.md#0x3_token_WithdrawEvent">WithdrawEvent</a> { id, amount }
        );
    };

    <b>let</b> tokens = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[account_addr].tokens;
    <b>assert</b>!(
        tokens.contains(id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>),
    );
    // balance &gt; amount and amount &gt; 0 indirectly asserted that balance &gt; 0.
    <b>let</b> balance = &<b>mut</b> tokens.borrow_mut(id).amount;
    <b>if</b> (*balance &gt; amount) {
        *balance -= amount;
        <a href="token.md#0x3_token_Token">Token</a> { id, amount, token_properties: <a href="property_map.md#0x3_property_map_empty">property_map::empty</a>() }
    } <b>else</b> {
        tokens.remove(id)
    }
}
</code></pre>



</details>

<a id="0x3_token_update_token_property_internal"></a>

## Function `update_token_property_internal`



<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(
    token_owner: <b>address</b>,
    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>,
    keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>let</b> tokens = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[token_owner].tokens;
    <b>assert</b>!(tokens.contains(token_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ENO_TOKEN_IN_TOKEN_STORE">ENO_TOKEN_IN_TOKEN_STORE</a>));

    <b>let</b> value = &<b>mut</b> tokens.borrow_mut(token_id).token_properties;
    <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(&keys);
    value.update_property_map(keys, values, types);
}
</code></pre>



</details>

<a id="0x3_token_direct_deposit"></a>

## Function `direct_deposit`

Deposit the token balance into the recipients account and emit an event.


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">Token</a>) <b>acquires</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a> {
    <b>assert</b>!(<a href="token.md#0x3_token">token</a>.amount &gt; 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="token.md#0x3_token_ETOKEN_CANNOT_HAVE_ZERO_AMOUNT">ETOKEN_CANNOT_HAVE_ZERO_AMOUNT</a>));
    <b>let</b> token_store = &<b>mut</b> <a href="token.md#0x3_token_TokenStore">TokenStore</a>[account_addr];

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="token.md#0x3_token_TokenDeposit">TokenDeposit</a> { <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: account_addr, id: <a href="token.md#0x3_token">token</a>.id, amount: <a href="token.md#0x3_token">token</a>.amount });
    } <b>else</b> {
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="token.md#0x3_token_DepositEvent">DepositEvent</a>&gt;(
            &<b>mut</b> token_store.deposit_events,
            <a href="token.md#0x3_token_DepositEvent">DepositEvent</a> { id: <a href="token.md#0x3_token">token</a>.id, amount: <a href="token.md#0x3_token">token</a>.amount },
        );
    };

    <b>assert</b>!(
        <b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_STORE_NOT_PUBLISHED">ETOKEN_STORE_NOT_PUBLISHED</a>),
    );

    <b>if</b> (!token_store.tokens.contains(<a href="token.md#0x3_token">token</a>.id)) {
        token_store.tokens.add(<a href="token.md#0x3_token">token</a>.id, <a href="token.md#0x3_token">token</a>);
    } <b>else</b> {
        <b>let</b> recipient_token = token_store.tokens.borrow_mut(<a href="token.md#0x3_token">token</a>.id);
        <a href="token.md#0x3_token_merge">merge</a>(recipient_token, <a href="token.md#0x3_token">token</a>);
    };
}
</code></pre>



</details>

<a id="0x3_token_assert_collection_exists"></a>

## Function `assert_collection_exists`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: String) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_collection_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_address].collection_data;
    <b>assert</b>!(all_collection_data.contains(collection_name), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTION_NOT_PUBLISHED">ECOLLECTION_NOT_PUBLISHED</a>));
}
</code></pre>



</details>

<a id="0x3_token_assert_tokendata_exists"></a>

## Function `assert_tokendata_exists`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>) <b>acquires</b> <a href="token.md#0x3_token_Collections">Collections</a> {
    <b>let</b> creator_addr = token_data_id.creator;
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator) == creator_addr, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_ENO_MUTATE_CAPABILITY">ENO_MUTATE_CAPABILITY</a>));
    <b>assert</b>!(<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ECOLLECTIONS_NOT_PUBLISHED">ECOLLECTIONS_NOT_PUBLISHED</a>));
    <b>let</b> all_token_data = &<a href="token.md#0x3_token_Collections">Collections</a>[creator_addr].token_data;
    <b>assert</b>!(all_token_data.contains(token_data_id), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="token.md#0x3_token_ETOKEN_DATA_NOT_PUBLISHED">ETOKEN_DATA_NOT_PUBLISHED</a>));
}
</code></pre>



</details>

<a id="0x3_token_assert_non_standard_reserved_property"></a>

## Function `assert_non_standard_reserved_property`



<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;) {
    keys.for_each_ref(|key| {
        <b>let</b> key: &String = key;
        <b>let</b> length = key.length();
        <b>if</b> (length &gt;= 6) {
            <b>let</b> prefix = key.sub_string(0, 6);
            <b>assert</b>!(prefix != <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"TOKEN_"), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="token.md#0x3_token_EPROPERTY_RESERVED_BY_STANDARD">EPROPERTY_RESERVED_BY_STANDARD</a>));
        };
    });
}
</code></pre>



</details>

<a id="0x3_token_initialize_token_script"></a>

## Function `initialize_token_script`



<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>abort</b> 0
}
</code></pre>



</details>

<a id="0x3_token_initialize_token"></a>

## Function `initialize_token`



<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>) {
    <b>abort</b> 0
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_partial;
</code></pre>



<a id="@Specification_1_create_collection_script"></a>

### Function `create_collection_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_collection_script">create_collection_script</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)
</code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a>;
</code></pre>



<a id="@Specification_1_create_token_script"></a>

### Function `create_token_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_create_token_script">create_token_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, balance: u64, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>


the length of 'mutate_setting' should maore than five.
The creator of the TokenDataId is signer.
The token_data_id should exist in the creator's collections..
The sum of supply and mint Token is less than maximum.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> token_data_id = <a href="token.md#0x3_token_spec_create_tokendata">spec_create_tokendata</a>(addr, collection, name);
<b>let</b> creator_addr = token_data_id.creator;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>aborts_if</b> token_data_id.creator != addr;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>aborts_if</b> balance &lt;= 0;
<b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;
<b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;
</code></pre>




<a id="0x3_token_spec_create_tokendata"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_tokendata">spec_create_tokendata</a>(
   creator: <b>address</b>,
   collection: String,
   name: String): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> {
   <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> { creator, collection, name }
}
</code></pre>



<a id="@Specification_1_mint_script"></a>

### Function `mint_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mint_script">mint_script</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, amount: u64)
</code></pre>


only creator of the tokendata can mint tokens


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> token_data_id = <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(
    token_data_address,
    collection,
    name,
);
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> creator_addr = token_data_id.creator;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>aborts_if</b> token_data_id.creator != <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>{
creator: token_data_address,
collection,
name
};
<b>include</b> <a href="token.md#0x3_token_MintTokenAbortsIf">MintTokenAbortsIf</a> {
token_data_id
};
</code></pre>



<a id="@Specification_1_mutate_token_properties"></a>

### Function `mutate_token_properties`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_mutate_token_properties">mutate_token_properties</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, amount: u64, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>


The signer is creator.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>aborts_if</b> addr != creator;
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> {
    creator,
    collection: collection_name,
    name: token_name
};
</code></pre>



<a id="@Specification_1_direct_transfer_script"></a>

### Function `direct_transfer_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_direct_transfer_script">direct_transfer_script</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>{
    creator: creators_address,
    collection,
    name
};
</code></pre>



<a id="@Specification_1_opt_in_direct_transfer"></a>

### Function `opt_in_direct_transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_opt_in_direct_transfer">opt_in_direct_transfer</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, opt_in: bool)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> account_addr = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_transfer_with_opt_in"></a>

### Function `transfer_with_opt_in`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_transfer_with_opt_in">transfer_with_opt_in</a>(from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_property_version: u64, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>{
    creator,
    collection: collection_name,
    name: token_name
};
</code></pre>



<a id="@Specification_1_burn_by_creator"></a>

### Function `burn_by_creator`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn_by_creator">burn_by_creator</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> creator_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> token_id = <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(creator_address, collection, name, property_version);
<b>let</b> creator_addr = token_id.token_data_id.creator;
<b>let</b> collections = <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(
    collections.token_data,
    token_id.token_data_id,
);
<b>aborts_if</b> amount &lt;= 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_id.token_data_id);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_BURNABLE_BY_CREATOR">BURNABLE_BY_CREATOR</a>));
</code></pre>



<a id="@Specification_1_burn"></a>

### Function `burn`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_burn">burn</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creators_address: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64, amount: u64)
</code></pre>


The token_data_id should exist in token_data.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> token_id = <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(creators_address, collection, name, property_version);
<b>let</b> creator_addr = token_id.token_data_id.creator;
<b>let</b> collections = <b>borrow_global_mut</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(
    collections.token_data,
    token_id.token_data_id,
);
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> {
creator: creators_address
};
<b>aborts_if</b> amount &lt;= 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_id.token_data_id);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>));
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_spec_internal_check_utf8">string::spec_internal_check_utf8</a>(<a href="token.md#0x3_token_BURNABLE_BY_OWNER">BURNABLE_BY_OWNER</a>);
</code></pre>




<a id="0x3_token_spec_create_token_id_raw"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_token_id_raw">spec_create_token_id_raw</a>(
   creator: <b>address</b>,
   collection: String,
   name: String,
   property_version: u64,
): <a href="token.md#0x3_token_TokenId">TokenId</a> {
   <b>let</b> token_data_id = <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> { creator, collection, name };
   <a href="token.md#0x3_token_TokenId">TokenId</a> {
       token_data_id,
       property_version
   }
}
</code></pre>



<a id="@Specification_1_mutate_collection_description"></a>

### Function `mutate_collection_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_description">mutate_collection_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>


The description of Collection is mutable.


<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> collection_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);
<b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> {
    creator_address: addr,
    collection_name
};
<b>aborts_if</b> !collection_data.mutability_config.description;
</code></pre>



<a id="@Specification_1_mutate_collection_uri"></a>

### Function `mutate_collection_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_uri">mutate_collection_uri</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>


The uri of Collection is mutable.


<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> collection_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);
<b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;
<b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> {
    creator_address: addr,
    collection_name
};
<b>aborts_if</b> !collection_data.mutability_config.uri;
</code></pre>



<a id="@Specification_1_mutate_collection_maximum"></a>

### Function `mutate_collection_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_collection_maximum">mutate_collection_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64)
</code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The maxium of Collection is mutable.


<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> collection_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(<b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data, collection_name);
<b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> {
    creator_address: addr,
    collection_name
};
<b>aborts_if</b> collection_data.maximum == 0 || maximum == 0;
<b>aborts_if</b> maximum &lt; collection_data.supply;
<b>aborts_if</b> !collection_data.mutability_config.maximum;
</code></pre>



<a id="@Specification_1_mutate_tokendata_maximum"></a>

### Function `mutate_tokendata_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_maximum">mutate_tokendata_maximum</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, maximum: u64)
</code></pre>


Cannot change maximum from 0 and cannot change maximum to 0.
The maximum should more than suply.
The token maximum is mutable


<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
<b>aborts_if</b> token_data.maximum == 0 || maximum == 0;
<b>aborts_if</b> maximum &lt; token_data.supply;
<b>aborts_if</b> !token_data.mutability_config.maximum;
</code></pre>



<a id="@Specification_1_mutate_tokendata_uri"></a>

### Function `mutate_tokendata_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_uri">mutate_tokendata_uri</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>


The length of uri should less than MAX_URI_LENGTH.
The  creator of token_data_id should exist in Collections.
The token uri is mutable


<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
<b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;
<b>aborts_if</b> !token_data.mutability_config.uri;
</code></pre>



<a id="@Specification_1_mutate_tokendata_royalty"></a>

### Function `mutate_tokendata_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_royalty">mutate_tokendata_royalty</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, royalty: <a href="token.md#0x3_token_Royalty">token::Royalty</a>)
</code></pre>


The token royalty is mutable


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>aborts_if</b> !token_data.mutability_config.royalty;
</code></pre>



<a id="@Specification_1_mutate_tokendata_description"></a>

### Function `mutate_tokendata_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_description">mutate_tokendata_description</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>


The token description is mutable


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>aborts_if</b> !token_data.mutability_config.description;
</code></pre>



<a id="@Specification_1_mutate_tokendata_property"></a>

### Function `mutate_tokendata_property`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_tokendata_property">mutate_tokendata_property</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>


The property map is mutable


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(token_data_id.creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
<b>aborts_if</b> len(keys) != len(values);
<b>aborts_if</b> len(keys) != len(types);
<b>aborts_if</b> !token_data.mutability_config.properties;
</code></pre>



<a id="@Specification_1_mutate_one_token"></a>

### Function `mutate_one_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mutate_one_token">mutate_one_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>


The signer is creator.
The token_data_id should exist in token_data.
The property map is mutable.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> creator = token_id.token_data_id.creator;
<b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_id.token_data_id);
<b>aborts_if</b> addr != creator;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_id.token_data_id);
<b>aborts_if</b> !token_data.mutability_config.properties && !<a href="../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(token_data.default_properties.map, std::string::spec_utf8(<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE">TOKEN_PROPERTY_MUTABLE</a>));
</code></pre>



<a id="@Specification_1_create_royalty"></a>

### Function `create_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_royalty">create_royalty</a>(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: <b>address</b>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a>;
</code></pre>


The royalty_points_numerator should less than royalty_points_denominator.


<a id="0x3_token_CreateRoyaltyAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a> {
    royalty_points_numerator: u64;
    royalty_points_denominator: u64;
    payee_address: <b>address</b>;
    <b>aborts_if</b> royalty_points_numerator &gt; royalty_points_denominator;
}
</code></pre>



<a id="@Specification_1_deposit_token"></a>

### Function `deposit_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_deposit_token">deposit_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial;
<b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>include</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr) ==&gt; <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;
<b>let</b> token_id = <a href="token.md#0x3_token">token</a>.id;
<b>let</b> token_amount = <a href="token.md#0x3_token">token</a>.amount;
<b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;
</code></pre>



<a id="@Specification_1_direct_deposit_with_opt_in"></a>

### Function `direct_deposit_with_opt_in`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_deposit_with_opt_in">direct_deposit_with_opt_in</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>


The token can direct_transfer.


<pre><code><b>let</b> opt_in_transfer = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).direct_transfer;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);
<b>aborts_if</b> !opt_in_transfer;
<b>let</b> token_id = <a href="token.md#0x3_token">token</a>.id;
<b>let</b> token_amount = <a href="token.md#0x3_token">token</a>.amount;
<b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;
</code></pre>



<a id="@Specification_1_direct_transfer"></a>

### Function `direct_transfer`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_direct_transfer">direct_transfer</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64)
</code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_initialize_token_store"></a>

### Function `initialize_token_store`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token_store">initialize_token_store</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;
</code></pre>




<a id="0x3_token_InitializeTokenStore"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a> {
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> account_addr = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
}
</code></pre>



<a id="@Specification_1_merge"></a>

### Function `merge`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_merge">merge</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, source_token: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>




<pre><code><b>aborts_if</b> dst_token.id != source_token.id;
<b>aborts_if</b> dst_token.amount + source_token.amount &gt; MAX_U64;
</code></pre>



<a id="@Specification_1_split"></a>

### Function `split`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_split">split</a>(dst_token: &<b>mut</b> <a href="token.md#0x3_token_Token">token::Token</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>




<pre><code><b>aborts_if</b> dst_token.id.property_version != 0;
<b>aborts_if</b> dst_token.amount &lt;= amount;
<b>aborts_if</b> amount &lt;= 0;
</code></pre>



<a id="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_transfer">transfer</a>(from: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>let</b> opt_in_transfer = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(<b>to</b>).direct_transfer;
<b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(from);
<b>aborts_if</b> !opt_in_transfer;
<b>pragma</b> aborts_if_is_partial;
<b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;
</code></pre>



<a id="@Specification_1_withdraw_with_capability"></a>

### Function `withdraw_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_with_capability">withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>




<pre><code><b>let</b> now_seconds = <b>global</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);
<b>aborts_if</b> now_seconds / <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">timestamp::MICRO_CONVERSION_FACTOR</a> &gt; withdraw_proof.expiration_sec;
<b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>{
account_addr: withdraw_proof.token_owner,
id: withdraw_proof.token_id,
amount: withdraw_proof.amount};
</code></pre>



<a id="@Specification_1_partial_withdraw_with_capability"></a>

### Function `partial_withdraw_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_partial_withdraw_with_capability">partial_withdraw_with_capability</a>(withdraw_proof: <a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>, withdraw_amount: u64): (<a href="token.md#0x3_token_Token">token::Token</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="token.md#0x3_token_WithdrawCapability">token::WithdrawCapability</a>&gt;)
</code></pre>




<pre><code><b>let</b> now_seconds = <b>global</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework).microseconds;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);
<b>aborts_if</b> now_seconds / <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp_MICRO_CONVERSION_FACTOR">timestamp::MICRO_CONVERSION_FACTOR</a> &gt; withdraw_proof.expiration_sec;
<b>aborts_if</b> withdraw_amount &gt; withdraw_proof.amount;
<b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>{
    account_addr: withdraw_proof.token_owner,
    id: withdraw_proof.token_id,
    amount: withdraw_amount
};
</code></pre>



<a id="@Specification_1_withdraw_token"></a>

### Function `withdraw_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_withdraw_token">withdraw_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>


Cannot withdraw 0 tokens.
Make sure the account has sufficient tokens to withdraw.


<pre><code><b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;
</code></pre>



<a id="@Specification_1_create_collection"></a>

### Function `create_collection`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection">create_collection</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;)
</code></pre>


The length of the name is up to MAX_COLLECTION_NAME_LENGTH;
The length of the uri is up to MAX_URI_LENGTH;
The collection_data should not exist before you create it.


<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
<b>aborts_if</b> len(name.bytes) &gt; 128;
<b>aborts_if</b> len(uri.bytes) &gt; 512;
<b>include</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a>;
</code></pre>




<a id="0x3_token_CreateCollectionAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateCollectionAbortsIf">CreateCollectionAbortsIf</a> {
    creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    name: String;
    description: String;
    uri: String;
    maximum: u64;
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>let</b> <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="../../aptos-framework/doc/account.md#0x1_account_Account">account::Account</a>&gt;(addr);
    <b>let</b> collection = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr);
    <b>let</b> b = !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr);
    <b>let</b> collection_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(addr).collection_data;
    <b>include</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a>;
}
</code></pre>



<a id="@Specification_1_check_collection_exists"></a>

### Function `check_collection_exists`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_collection_exists">check_collection_exists</a>(creator: <b>address</b>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);
</code></pre>



<a id="@Specification_1_check_tokendata_exists"></a>

### Function `check_tokendata_exists`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_check_tokendata_exists">check_tokendata_exists</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, token_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): bool
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> {
    creator,
    collection: collection_name,
    name: token_name
};
</code></pre>



<a id="@Specification_1_create_tokendata"></a>

### Function `create_tokendata`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_tokendata">create_tokendata</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, description: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, maximum: u64, uri: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, royalty_payee_address: <b>address</b>, royalty_points_denominator: u64, royalty_points_numerator: u64, token_mutate_config: <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>, property_keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, property_values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, property_types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>pragma</b> verify = <b>false</b>;
<b>pragma</b> aborts_if_is_partial;
<b>let</b> account_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> collections = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);
<b>let</b> token_data_id = <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(account_addr, collection, name);
<b>let</b> Collection = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(collections.collection_data, token_data_id.collection);
<b>let</b> length = len(property_keys);
<b>aborts_if</b> len(name.bytes) &gt; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>;
<b>aborts_if</b> len(collection.bytes) &gt; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>;
<b>aborts_if</b> len(uri.bytes) &gt; <a href="token.md#0x3_token_MAX_URI_LENGTH">MAX_URI_LENGTH</a>;
<b>aborts_if</b> royalty_points_numerator &gt; royalty_points_denominator;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(account_addr);
<b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> {
    creator: account_addr,
    collection,
    name
};
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.collection_data, collection);
<b>aborts_if</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(collections.token_data, token_data_id);
<b>aborts_if</b> Collection.maximum &gt; 0 && Collection.supply + 1 &gt; MAX_U64;
<b>aborts_if</b> Collection.maximum &gt; 0 && Collection.maximum &lt; Collection.supply + 1;
<b>include</b> <a href="token.md#0x3_token_CreateRoyaltyAbortsIf">CreateRoyaltyAbortsIf</a> {
    payee_address: royalty_payee_address
};
<b>aborts_if</b> length &gt; <a href="property_map.md#0x3_property_map_MAX_PROPERTY_MAP_SIZE">property_map::MAX_PROPERTY_MAP_SIZE</a>;
<b>aborts_if</b> length != len(property_values);
<b>aborts_if</b> length != len(property_types);
</code></pre>




<a id="0x3_token_spec_create_token_data_id"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_create_token_data_id">spec_create_token_data_id</a>(
   creator: <b>address</b>,
   collection: String,
   name: String,
): <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> {
   <a href="token.md#0x3_token_TokenDataId">TokenDataId</a> { creator, collection, name }
}
</code></pre>



<a id="@Specification_1_get_collection_supply"></a>

### Function `get_collection_supply`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_supply">get_collection_supply</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;
</code></pre>



<a id="@Specification_1_get_collection_description"></a>

### Function `get_collection_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_description">get_collection_description</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;
</code></pre>



<a id="@Specification_1_get_collection_uri"></a>

### Function `get_collection_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_uri">get_collection_uri</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;
</code></pre>



<a id="@Specification_1_get_collection_maximum"></a>

### Function `get_collection_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_maximum">get_collection_maximum</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): u64
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;
</code></pre>



<a id="@Specification_1_get_token_supply"></a>

### Function `get_token_supply`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_token_supply">get_token_supply</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_largest_property_version"></a>

### Function `get_tokendata_largest_property_version`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_largest_property_version">get_tokendata_largest_property_version</a>(creator_address: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_create_token_mutability_config"></a>

### Function `create_token_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_mutability_config">create_token_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>
</code></pre>


The length of 'mutate_setting' should more than five.
The mutate_setting shuold have a value.


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a>;
</code></pre>




<a id="0x3_token_CreateTokenMutabilityConfigAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateTokenMutabilityConfigAbortsIf">CreateTokenMutabilityConfigAbortsIf</a> {
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;
    <b>aborts_if</b> len(mutate_setting) &lt; 5;
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_MAX_MUTABLE_IND">TOKEN_MAX_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_URI_MUTABLE_IND">TOKEN_URI_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_ROYALTY_MUTABLE_IND">TOKEN_ROYALTY_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_DESCRIPTION_MUTABLE_IND">TOKEN_DESCRIPTION_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_TOKEN_PROPERTY_MUTABLE_IND">TOKEN_PROPERTY_MUTABLE_IND</a>]);
}
</code></pre>



<a id="@Specification_1_create_collection_mutability_config"></a>

### Function `create_collection_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_collection_mutability_config">create_collection_mutability_config</a>(mutate_setting: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a>;
</code></pre>




<a id="0x3_token_CreateCollectionMutabilityConfigAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateCollectionMutabilityConfigAbortsIf">CreateCollectionMutabilityConfigAbortsIf</a> {
    mutate_setting: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;bool&gt;;
    <b>aborts_if</b> len(mutate_setting) &lt; 3;
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_DESCRIPTION_MUTABLE_IND">COLLECTION_DESCRIPTION_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_URI_MUTABLE_IND">COLLECTION_URI_MUTABLE_IND</a>]);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(mutate_setting, mutate_setting[<a href="token.md#0x3_token_COLLECTION_MAX_MUTABLE_IND">COLLECTION_MAX_MUTABLE_IND</a>]);
}
</code></pre>



<a id="@Specification_1_mint_token"></a>

### Function `mint_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token">mint_token</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>


The creator of the TokenDataId is signer.
The token_data_id should exist in the creator's collections..
The sum of supply and the amount of mint Token is less than maximum.


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x3_token_MintTokenAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_MintTokenAbortsIf">MintTokenAbortsIf</a> {
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;
    amount: u64;
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> creator_addr = token_data_id.creator;
    <b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
    <b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
    <b>aborts_if</b> token_data_id.creator != addr;
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
    <b>aborts_if</b> token_data.maximum &gt; 0 && token_data.supply + amount &gt; token_data.maximum;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
    <b>aborts_if</b> amount &lt;= 0;
    <b>include</b> <a href="token.md#0x3_token_InitializeTokenStore">InitializeTokenStore</a>;
    <b>let</b> token_id = <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);
}
</code></pre>



<a id="@Specification_1_mint_token_to"></a>

### Function `mint_token_to`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_mint_token_to">mint_token_to</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, receiver: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>, amount: u64)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
<b>let</b> opt_in_transfer = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver).direct_transfer;
<b>let</b> creator_addr = token_data_id.creator;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
<b>let</b> token_data = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(all_token_data, token_data_id);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(receiver);
<b>aborts_if</b> !opt_in_transfer;
<b>aborts_if</b> token_data_id.creator != addr;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
<b>aborts_if</b> token_data.maximum &gt; 0 && token_data.supply + amount &gt; token_data.maximum;
<b>aborts_if</b> amount &lt;= 0;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>let</b> token_id = <a href="token.md#0x3_token_create_token_id">create_token_id</a>(token_data_id, 0);
<b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a> {
    account_addr: receiver,
    token_id,
    token_amount: amount,
};
</code></pre>



<a id="@Specification_1_create_token_data_id"></a>

### Function `create_token_data_id`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_data_id">create_token_data_id</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>;
</code></pre>




<a id="0x3_token_CreateTokenDataIdAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a> {
    creator: <b>address</b>;
    collection: String;
    name: String;
    <b>aborts_if</b> len(collection.bytes) &gt; <a href="token.md#0x3_token_MAX_COLLECTION_NAME_LENGTH">MAX_COLLECTION_NAME_LENGTH</a>;
    <b>aborts_if</b> len(name.bytes) &gt; <a href="token.md#0x3_token_MAX_NFT_NAME_LENGTH">MAX_NFT_NAME_LENGTH</a>;
}
</code></pre>



<a id="@Specification_1_create_token_id_raw"></a>

### Function `create_token_id_raw`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_create_token_id_raw">create_token_id_raw</a>(creator: <b>address</b>, collection: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, property_version: u64): <a href="token.md#0x3_token_TokenId">token::TokenId</a>
</code></pre>


The length of collection should less than MAX_COLLECTION_NAME_LENGTH
The length of name should less than MAX_NFT_NAME_LENGTH


<pre><code><b>include</b> <a href="token.md#0x3_token_CreateTokenDataIdAbortsIf">CreateTokenDataIdAbortsIf</a>;
</code></pre>




<a id="0x3_token_spec_balance_of"></a>


<pre><code><b>fun</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(owner: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">TokenId</a>): u64 {
   <b>let</b> token_store = <b>borrow_global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner);
   <b>if</b> (!<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(owner)) {
       0
   }
   <b>else</b> <b>if</b> (<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, id)) {
       <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, id).amount
   } <b>else</b> {
       0
   }
}
</code></pre>



<a id="@Specification_1_get_royalty"></a>

### Function `get_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_royalty">get_royalty</a>(token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a> {
    token_data_id: token_id.token_data_id
};
</code></pre>



<a id="@Specification_1_get_property_map"></a>

### Function `get_property_map`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_property_map">get_property_map</a>(owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>): <a href="property_map.md#0x3_property_map_PropertyMap">property_map::PropertyMap</a>
</code></pre>




<pre><code><b>let</b> creator_addr = token_id.token_data_id.creator;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
<b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(owner, token_id) &lt;= 0;
<b>aborts_if</b> token_id.property_version == 0 && !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_id.token_data_id);
<b>aborts_if</b> token_id.property_version == 0 && !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
</code></pre>



<a id="@Specification_1_get_tokendata_maximum"></a>

### Function `get_tokendata_maximum`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_maximum">get_tokendata_maximum</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): u64
</code></pre>




<pre><code><b>let</b> creator_address = token_data_id.creator;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_uri"></a>

### Function `get_tokendata_uri`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_uri">get_tokendata_uri</a>(creator: <b>address</b>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).token_data;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_description"></a>

### Function `get_tokendata_description`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_description">get_tokendata_description</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>




<pre><code><b>let</b> creator_address = token_data_id.creator;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_tokendata_royalty"></a>

### Function `get_tokendata_royalty`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_royalty">get_tokendata_royalty</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_Royalty">token::Royalty</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a>;
</code></pre>




<a id="0x3_token_GetTokendataRoyaltyAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_GetTokendataRoyaltyAbortsIf">GetTokendataRoyaltyAbortsIf</a> {
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;
    <b>let</b> creator_address = token_data_id.creator;
    <b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).token_data;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
}
</code></pre>



<a id="@Specification_1_get_tokendata_mutability_config"></a>

### Function `get_tokendata_mutability_config`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_tokendata_mutability_config">get_tokendata_mutability_config</a>(token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>): <a href="token.md#0x3_token_TokenMutabilityConfig">token::TokenMutabilityConfig</a>
</code></pre>




<pre><code><b>let</b> creator_addr = token_data_id.creator;
<b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
</code></pre>



<a id="@Specification_1_get_collection_mutability_config"></a>

### Function `get_collection_mutability_config`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="token.md#0x3_token_get_collection_mutability_config">get_collection_mutability_config</a>(creator: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>): <a href="token.md#0x3_token_CollectionMutabilityConfig">token::CollectionMutabilityConfig</a>
</code></pre>




<pre><code><b>let</b> all_collection_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator).collection_data;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_collection_data, collection_name);
</code></pre>



<a id="@Specification_1_withdraw_with_event_internal"></a>

### Function `withdraw_with_event_internal`


<pre><code><b>fun</b> <a href="token.md#0x3_token_withdraw_with_event_internal">withdraw_with_event_internal</a>(account_addr: <b>address</b>, id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, amount: u64): <a href="token.md#0x3_token_Token">token::Token</a>
</code></pre>




<pre><code><b>include</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a>;
</code></pre>




<a id="0x3_token_WithdrawWithEventInternalAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_WithdrawWithEventInternalAbortsIf">WithdrawWithEventInternalAbortsIf</a> {
    account_addr: <b>address</b>;
    id: <a href="token.md#0x3_token_TokenId">TokenId</a>;
    amount: u64;
    <b>let</b> tokens = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr).tokens;
    <b>aborts_if</b> amount &lt;= 0;
    <b>aborts_if</b> <a href="token.md#0x3_token_spec_balance_of">spec_balance_of</a>(account_addr, id) &lt; amount;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, id);
}
</code></pre>



<a id="@Specification_1_update_token_property_internal"></a>

### Function `update_token_property_internal`


<pre><code><b>fun</b> <a href="token.md#0x3_token_update_token_property_internal">update_token_property_internal</a>(token_owner: <b>address</b>, token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>, keys: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, values: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, types: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>let</b> tokens = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner).tokens;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(token_owner);
<b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(tokens, token_id);
</code></pre>



<a id="@Specification_1_direct_deposit"></a>

### Function `direct_deposit`


<pre><code><b>fun</b> <a href="token.md#0x3_token_direct_deposit">direct_deposit</a>(account_addr: <b>address</b>, <a href="token.md#0x3_token">token</a>: <a href="token.md#0x3_token_Token">token::Token</a>)
</code></pre>




<pre><code><b>let</b> token_id = <a href="token.md#0x3_token">token</a>.id;
<b>let</b> token_amount = <a href="token.md#0x3_token">token</a>.amount;
<b>include</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a>;
</code></pre>




<a id="0x3_token_DirectDepositAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_DirectDepositAbortsIf">DirectDepositAbortsIf</a> {
    account_addr: <b>address</b>;
    token_id: <a href="token.md#0x3_token_TokenId">TokenId</a>;
    token_amount: u64;
    <b>let</b> token_store = <b>global</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);
    <b>let</b> recipient_token = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(token_store.tokens, token_id);
    <b>let</b> b = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(token_store.tokens, token_id);
    <b>aborts_if</b> token_amount &lt;= 0;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_TokenStore">TokenStore</a>&gt;(account_addr);
    <b>aborts_if</b> b && recipient_token.id != token_id;
    <b>aborts_if</b> b && recipient_token.amount + token_amount &gt; MAX_U64;
}
</code></pre>



<a id="@Specification_1_assert_collection_exists"></a>

### Function `assert_collection_exists`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_collection_exists">assert_collection_exists</a>(creator_address: <b>address</b>, collection_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>


The collection_name should exist in collection_data of the creator_address's Collections.


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a>;
</code></pre>




<a id="0x3_token_AssertCollectionExistsAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_AssertCollectionExistsAbortsIf">AssertCollectionExistsAbortsIf</a> {
    creator_address: <b>address</b>;
    collection_name: String;
    <b>let</b> all_collection_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address).collection_data;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_address);
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_collection_data, collection_name);
}
</code></pre>



<a id="@Specification_1_assert_tokendata_exists"></a>

### Function `assert_tokendata_exists`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_tokendata_exists">assert_tokendata_exists</a>(creator: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, token_data_id: <a href="token.md#0x3_token_TokenDataId">token::TokenDataId</a>)
</code></pre>


The creator of token_data_id should be signer.
The  creator of token_data_id exists in Collections.
The token_data_id is in the all_token_data.


<pre><code><b>include</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a>;
</code></pre>




<a id="0x3_token_AssertTokendataExistsAbortsIf"></a>


<pre><code><b>schema</b> <a href="token.md#0x3_token_AssertTokendataExistsAbortsIf">AssertTokendataExistsAbortsIf</a> {
    creator: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    token_data_id: <a href="token.md#0x3_token_TokenDataId">TokenDataId</a>;
    <b>let</b> creator_addr = token_data_id.creator;
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(creator);
    <b>aborts_if</b> addr != creator_addr;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr);
    <b>let</b> all_token_data = <b>global</b>&lt;<a href="token.md#0x3_token_Collections">Collections</a>&gt;(creator_addr).token_data;
    <b>aborts_if</b> !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(all_token_data, token_data_id);
}
</code></pre>



<a id="@Specification_1_assert_non_standard_reserved_property"></a>

### Function `assert_non_standard_reserved_property`


<pre><code><b>fun</b> <a href="token.md#0x3_token_assert_non_standard_reserved_property">assert_non_standard_reserved_property</a>(keys: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_initialize_token_script"></a>

### Function `initialize_token_script`


<pre><code><b>public</b> entry <b>fun</b> <a href="token.md#0x3_token_initialize_token_script">initialize_token_script</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>


Deprecated function


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_initialize_token"></a>

### Function `initialize_token`


<pre><code><b>public</b> <b>fun</b> <a href="token.md#0x3_token_initialize_token">initialize_token</a>(_account: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _token_id: <a href="token.md#0x3_token_TokenId">token::TokenId</a>)
</code></pre>


Deprecated function


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
