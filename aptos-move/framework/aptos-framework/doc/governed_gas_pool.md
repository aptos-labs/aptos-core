
<a id="0x1_governed_gas_pool"></a>

# Module `0x1::governed_gas_pool`



-  [Resource `GovernedGasPool`](#0x1_governed_gas_pool_GovernedGasPool)
-  [Constants](#@Constants_0)
-  [Function `primary_fungible_store_address`](#0x1_governed_gas_pool_primary_fungible_store_address)
-  [Function `create_resource_account_seed`](#0x1_governed_gas_pool_create_resource_account_seed)
-  [Function `initialize`](#0x1_governed_gas_pool_initialize)
-  [Function `init_module`](#0x1_governed_gas_pool_init_module)
-  [Function `governed_gas_signer`](#0x1_governed_gas_pool_governed_gas_signer)
-  [Function `governed_gas_pool_address`](#0x1_governed_gas_pool_governed_gas_pool_address)
-  [Function `fund`](#0x1_governed_gas_pool_fund)
-  [Function `deposit`](#0x1_governed_gas_pool_deposit)
-  [Function `deposit_from`](#0x1_governed_gas_pool_deposit_from)
-  [Function `deposit_from_fungible_store`](#0x1_governed_gas_pool_deposit_from_fungible_store)
-  [Function `deposit_gas_fee`](#0x1_governed_gas_pool_deposit_gas_fee)
-  [Function `deposit_gas_fee_v2`](#0x1_governed_gas_pool_deposit_gas_fee_v2)
-  [Function `get_balance`](#0x1_governed_gas_pool_get_balance)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `fund`](#@Specification_1_fund)
    -  [Function `deposit`](#@Specification_1_deposit)
    -  [Function `deposit_gas_fee`](#@Specification_1_deposit_gas_fee)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_account.md#0x1_aptos_account">0x1::aptos_account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_governed_gas_pool_GovernedGasPool"></a>

## Resource `GovernedGasPool`

The Governed Gas Pool
Internally, this is a simply wrapper around a resource account.


<pre><code><b>struct</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signer_capability: <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a></code>
</dt>
<dd>
 The signer capability of the resource account.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_governed_gas_pool_MODULE_SALT"></a>



<pre><code><b>const</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_MODULE_SALT">MODULE_SALT</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 103, 111, 118, 101, 114, 110, 101, 100, 95, 103, 97, 115, 95, 112, 111, 111, 108];
</code></pre>



<a id="0x1_governed_gas_pool_primary_fungible_store_address"></a>

## Function `primary_fungible_store_address`

Address of APT Primary Fungible Store


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>): <b>address</b> {
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(<a href="account.md#0x1_account">account</a>, @aptos_fungible_asset)
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_create_resource_account_seed"></a>

## Function `create_resource_account_seed`

Create the seed to derive the resource account address.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_create_resource_account_seed">create_resource_account_seed</a>(
    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> seed = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    // <b>include</b> <b>module</b> salt (before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> subseeds) <b>to</b> avoid conflicts <b>with</b> other modules creating resource accounts
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, <a href="governed_gas_pool.md#0x1_governed_gas_pool_MODULE_SALT">MODULE_SALT</a>);
    // <b>include</b> an additional salt in case the same resource <a href="account.md#0x1_account">account</a> <b>has</b> already been created
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> seed, delegation_pool_creation_seed);
    seed
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_initialize"></a>

## Function `initialize`

Initializes the governed gas pool around a resource account creation seed.
@param aptos_framework The signer of the aptos_framework module.
@param delegation_pool_creation_seed The seed to be used to create the resource account hosting the delegation pool.


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_initialize">initialize</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    // <b>return</b> <b>if</b> the governed gas pool <b>has</b> already been initialized
    <b>if</b> (<b>exists</b>&lt;<a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework))) {
        <b>return</b>
    };

    // generate a seed <b>to</b> be used <b>to</b> create the resource <a href="account.md#0x1_account">account</a> hosting the delegation pool
    <b>let</b> seed = <a href="governed_gas_pool.md#0x1_governed_gas_pool_create_resource_account_seed">create_resource_account_seed</a>(delegation_pool_creation_seed);

    <b>let</b> (governed_gas_pool_signer, governed_gas_pool_signer_cap) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(aptos_framework, seed);

    // register apt
    <a href="aptos_account.md#0x1_aptos_account_register_apt">aptos_account::register_apt</a>(&governed_gas_pool_signer);

    <b>move_to</b>(aptos_framework, <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a>{
        signer_capability: governed_gas_pool_signer_cap,
    });
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_init_module"></a>

## Function `init_module`

Initialize the governed gas pool as a module
@param aptos_framework The signer of the aptos_framework module.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_init_module">init_module</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_init_module">init_module</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    // Initialize the governed gas pool
    <b>let</b> seed : <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = b"aptos_framework::governed_gas_pool";
    <a href="governed_gas_pool.md#0x1_governed_gas_pool_initialize">initialize</a>(aptos_framework, seed);
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_governed_gas_signer"></a>

## Function `governed_gas_signer`

Borrows the signer of the governed gas pool.
@return The signer of the governed gas pool.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_signer">governed_gas_signer</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_signer">governed_gas_signer</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    <b>let</b> signer_cap = &<b>borrow_global</b>&lt;<a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a>&gt;(@aptos_framework).signer_capability;
    create_signer_with_capability(signer_cap)
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_governed_gas_pool_address"></a>

## Function `governed_gas_pool_address`

Gets the address of the governed gas pool.
@return The address of the governed gas pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>(): <b>address</b> <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_signer">governed_gas_signer</a>())
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_fund"></a>

## Function `fund`

Funds the destination account with a given amount of coin.
@param account The account to be funded.
@param amount The amount of coin to be funded.


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_fund">fund</a>&lt;CoinType&gt;(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_fund">fund</a>&lt;CoinType&gt;(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    // Check that the Aptos framework is the caller
    // This is what <b>ensures</b> that funding can only be done by the Aptos framework,
    // i.e., via a governance proposal.
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> governed_gas_signer = &<a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_signer">governed_gas_signer</a>();
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(<a href="account.md#0x1_account">account</a>, <a href="coin.md#0x1_coin_withdraw">coin::withdraw</a>&lt;CoinType&gt;(governed_gas_signer, amount));
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_deposit"></a>

## Function `deposit`

Deposits some coin into the governed gas pool.
@param coin The coin to be deposited.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit">deposit</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit">deposit</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: Coin&lt;CoinType&gt;) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    <b>let</b> governed_gas_pool_address = <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>();
    <a href="coin.md#0x1_coin_deposit">coin::deposit</a>(governed_gas_pool_address, <a href="coin.md#0x1_coin">coin</a>);
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_deposit_from"></a>

## Function `deposit_from`

Deposits some coin from an account to the governed gas pool.
@param account The account from which the coin is to be deposited.
@param amount The amount of coin to be deposited.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from">deposit_from</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from">deposit_from</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
   <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit">deposit</a>(<a href="coin.md#0x1_coin_withdraw_from">coin::withdraw_from</a>&lt;CoinType&gt;(<a href="account.md#0x1_account">account</a>, amount));
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_deposit_from_fungible_store"></a>

## Function `deposit_from_fungible_store`

Deposits some FA from the fungible store.
@param aptos_framework The signer of the aptos_framework module.
@param account The account from which the FA is to be deposited.
@param amount The amount of FA to be deposited.


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from_fungible_store">deposit_from_fungible_store</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from_fungible_store">deposit_from_fungible_store</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    <b>if</b> (amount &gt; 0){
        // compute the governed gas pool store <b>address</b>
        <b>let</b> governed_gas_pool_address = <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>();
        <b>let</b> governed_gas_pool_store_address = <a href="governed_gas_pool.md#0x1_governed_gas_pool_primary_fungible_store_address">primary_fungible_store_address</a>(governed_gas_pool_address);

        // compute the <a href="account.md#0x1_account">account</a> store <b>address</b>
        <b>let</b> account_store_address = <a href="governed_gas_pool.md#0x1_governed_gas_pool_primary_fungible_store_address">primary_fungible_store_address</a>(<a href="account.md#0x1_account">account</a>);
        <a href="fungible_asset.md#0x1_fungible_asset_deposit_internal">fungible_asset::deposit_internal</a>(
            governed_gas_pool_store_address,
            <a href="fungible_asset.md#0x1_fungible_asset_withdraw_internal">fungible_asset::withdraw_internal</a>(
                account_store_address,
                amount
            )
        );
    }
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_deposit_gas_fee"></a>

## Function `deposit_gas_fee`

Deposits gas fees into the governed gas pool.
@param gas_payer The address of the account that paid the gas fees.
@param gas_fee The amount of gas fees to be deposited.


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_gas_fee">deposit_gas_fee</a>(gas_payer: <b>address</b>, gas_fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_gas_fee">deposit_gas_fee</a>(gas_payer: <b>address</b>, gas_fee: u64) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    // get the sender <b>to</b> preserve the signature but do nothing
    <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>();
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_deposit_gas_fee_v2"></a>

## Function `deposit_gas_fee_v2`

Deposits gas fees into the governed gas pool.
@param gas_payer The address of the account that paid the gas fees.
@param gas_fee The amount of gas fees to be deposited.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_gas_fee_v2">deposit_gas_fee_v2</a>(gas_payer: <b>address</b>, gas_fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_gas_fee_v2">deposit_gas_fee_v2</a>(gas_payer: <b>address</b>, gas_fee: u64) <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
   <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_operations_default_to_fa_apt_store_enabled">features::operations_default_to_fa_apt_store_enabled</a>()) {
        <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from_fungible_store">deposit_from_fungible_store</a>(gas_payer, gas_fee);
    } <b>else</b> {
        <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_from">deposit_from</a>&lt;AptosCoin&gt;(gas_payer, gas_fee);
    };
}
</code></pre>



</details>

<a id="0x1_governed_gas_pool_get_balance"></a>

## Function `get_balance`

Gets the balance of a specified coin type in the governed gas pool.
@return The balance of the coin in the pool.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_get_balance">get_balance</a>&lt;CoinType&gt;(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_get_balance">get_balance</a>&lt;CoinType&gt;(): u64 <b>acquires</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a> {
    <b>let</b> pool_address = <a href="governed_gas_pool.md#0x1_governed_gas_pool_governed_gas_pool_address">governed_gas_pool_address</a>();
    <a href="coin.md#0x1_coin_balance">coin::balance</a>&lt;CoinType&gt;(pool_address)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>invariant</b> <b>exists</b>&lt;<a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegation_pool_creation_seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>requires</b> <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="governed_gas_pool.md#0x1_governed_gas_pool_GovernedGasPool">GovernedGasPool</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_fund"></a>

### Function `fund`


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_fund">fund</a>&lt;CoinType&gt;(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account">account</a>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial = <b>true</b>;
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));
</code></pre>


Abort if the governed gas pool has insufficient funds


<pre><code><b>aborts_with</b> <a href="coin.md#0x1_coin_EINSUFFICIENT_BALANCE">coin::EINSUFFICIENT_BALANCE</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(EINSUFFICIENT_BALANCE), 0x1, 0x5, 0x7;
</code></pre>



<a id="@Specification_1_deposit"></a>

### Function `deposit`


<pre><code><b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit">deposit</a>&lt;CoinType&gt;(<a href="coin.md#0x1_coin">coin</a>: <a href="coin.md#0x1_coin_Coin">coin::Coin</a>&lt;CoinType&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial = <b>true</b>;
</code></pre>



<a id="@Specification_1_deposit_gas_fee"></a>

### Function `deposit_gas_fee`


<pre><code><b>public</b> <b>fun</b> <a href="governed_gas_pool.md#0x1_governed_gas_pool_deposit_gas_fee">deposit_gas_fee</a>(gas_payer: <b>address</b>, gas_fee: u64)
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
