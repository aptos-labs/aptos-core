
<a name="0x1_fungible_source"></a>

# Module `0x1::fungible_source`

This module defines the extension called <code><a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a></code> that any object must equip with to make it fungible.


-  [Resource `FungibleSource`](#0x1_fungible_source_FungibleSource)
-  [Constants](#@Constants_0)
-  [Function `init_fungible_source`](#0x1_fungible_source_init_fungible_source)
-  [Function `get_current_supply`](#0x1_fungible_source_get_current_supply)
-  [Function `get_maximum_supply`](#0x1_fungible_source_get_maximum_supply)
-  [Function `get_name`](#0x1_fungible_source_get_name)
-  [Function `get_symbol`](#0x1_fungible_source_get_symbol)
-  [Function `get_decimals`](#0x1_fungible_source_get_decimals)
-  [Function `increase_supply`](#0x1_fungible_source_increase_supply)
-  [Function `decrease_supply`](#0x1_fungible_source_decrease_supply)
-  [Function `verify`](#0x1_fungible_source_verify)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x1_fungible_source_FungibleSource"></a>

## Resource `FungibleSource`

Define the metadata required of an asset to be fungible.


<pre><code><b>struct</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_supply: u64</code>
</dt>
<dd>
 Self-explanatory.
</dd>
<dt>
<code>maximum_supply: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>
 The max supply limit where <code><a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()</code> means no limit.
</dd>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the fungible asset, i.e., "USDT".
</dd>
<dt>
<code>symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Symbol of the fungible asset, usually a shorter version of the name.
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_fungible_source_ECURRENT_SUPPLY_OVERFLOW"></a>

Current supply overflow


<pre><code><b>const</b> <a href="fungible_source.md#0x1_fungible_source_ECURRENT_SUPPLY_OVERFLOW">ECURRENT_SUPPLY_OVERFLOW</a>: u64 = 2;
</code></pre>



<a name="0x1_fungible_source_ECURRENT_SUPPLY_UNDERFLOW"></a>

Current supply underflow


<pre><code><b>const</b> <a href="fungible_source.md#0x1_fungible_source_ECURRENT_SUPPLY_UNDERFLOW">ECURRENT_SUPPLY_UNDERFLOW</a>: u64 = 3;
</code></pre>



<a name="0x1_fungible_source_EZERO_AMOUNT"></a>

Amount cannot be zero.


<pre><code><b>const</b> <a href="fungible_source.md#0x1_fungible_source_EZERO_AMOUNT">EZERO_AMOUNT</a>: u64 = 1;
</code></pre>



<a name="0x1_fungible_source_init_fungible_source"></a>

## Function `init_fungible_source`

The initialization of an object with <code><a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_init_fungible_source">init_fungible_source</a>(constructor_ref: &<a href="object.md#0x1_object_ConstructorRef">object::ConstructorRef</a>, maximum_supply: u64, name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, symbol: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, decimals: u8): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_init_fungible_source">init_fungible_source</a>(
    constructor_ref: &ConstructorRef,
    maximum_supply: u64,
    name: String,
    symbol: String,
    decimals: u8,
): Object&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a>&gt; {
    <b>let</b> asset_object_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(constructor_ref);
    <b>let</b> converted_maximum = <b>if</b> (maximum_supply == 0) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(maximum_supply)
    };
    <b>move_to</b>(&asset_object_signer,
        <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
            current_supply: 0,
            maximum_supply: converted_maximum,
            name,
            symbol,
            decimals,
        }
    );
    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a>&gt;(constructor_ref)
}
</code></pre>



</details>

<a name="0x1_fungible_source_get_current_supply"></a>

## Function `get_current_supply`

Self-explanatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_current_supply">get_current_supply</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_current_supply">get_current_supply</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): u64 <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    borrow_fungible_source(asset).current_supply
}
</code></pre>



</details>

<a name="0x1_fungible_source_get_maximum_supply"></a>

## Function `get_maximum_supply`

Self-explanatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_maximum_supply">get_maximum_supply</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_maximum_supply">get_maximum_supply</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): Option&lt;u64&gt; <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    borrow_fungible_source(asset).maximum_supply
}
</code></pre>



</details>

<a name="0x1_fungible_source_get_name"></a>

## Function `get_name`

Self-explanatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_name">get_name</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_name">get_name</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    borrow_fungible_source(asset).name
}
</code></pre>



</details>

<a name="0x1_fungible_source_get_symbol"></a>

## Function `get_symbol`

Self-explanatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_symbol">get_symbol</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_symbol">get_symbol</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): String <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    borrow_fungible_source(asset).symbol
}
</code></pre>



</details>

<a name="0x1_fungible_source_get_decimals"></a>

## Function `get_decimals`

Self-explanatory.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_decimals">get_decimals</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_get_decimals">get_decimals</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): u8 <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    borrow_fungible_source(asset).decimals
}
</code></pre>



</details>

<a name="0x1_fungible_source_increase_supply"></a>

## Function `increase_supply`

Increase the supply of a fungible asset by minting.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_increase_supply">increase_supply</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_increase_supply">increase_supply</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_source.md#0x1_fungible_source_EZERO_AMOUNT">EZERO_AMOUNT</a>));
    <b>let</b> <a href="fungible_source.md#0x1_fungible_source">fungible_source</a> = borrow_fungible_source_mut(asset);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.maximum_supply)) {
        <b>let</b> max = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.maximum_supply);
        <b>assert</b>!(max - <a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_source.md#0x1_fungible_source_ECURRENT_SUPPLY_OVERFLOW">ECURRENT_SUPPLY_OVERFLOW</a>))
    };
    <a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply = <a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply + amount;
}
</code></pre>



</details>

<a name="0x1_fungible_source_decrease_supply"></a>

## Function `decrease_supply`

Increase the supply of a fungible asset by burning.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_decrease_supply">decrease_supply</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_decrease_supply">decrease_supply</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;, amount: u64) <b>acquires</b> <a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a> {
    <b>assert</b>!(amount != 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_source.md#0x1_fungible_source_EZERO_AMOUNT">EZERO_AMOUNT</a>));
    <b>let</b> <a href="fungible_source.md#0x1_fungible_source">fungible_source</a> = borrow_fungible_source_mut(asset);
    <b>assert</b>!(<a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply &gt;= amount, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fungible_source.md#0x1_fungible_source_ECURRENT_SUPPLY_UNDERFLOW">ECURRENT_SUPPLY_UNDERFLOW</a>));
    <a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply = <a href="fungible_source.md#0x1_fungible_source">fungible_source</a>.current_supply - amount;
}
</code></pre>



</details>

<a name="0x1_fungible_source_verify"></a>

## Function `verify`

Verify any object is equipped with <code><a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a></code> and return its address.


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_verify">verify</a>&lt;T: key&gt;(asset: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;T&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">fungible_source::FungibleSource</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fungible_source.md#0x1_fungible_source_verify">verify</a>&lt;T: key&gt;(asset: &Object&lt;T&gt;): Object&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a>&gt; {
    <b>let</b> addr = <a href="object.md#0x1_object_object_address">object::object_address</a>(asset);
    <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="fungible_source.md#0x1_fungible_source_FungibleSource">FungibleSource</a>&gt;(addr)
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
