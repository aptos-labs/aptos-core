
<a id="0x1_ethereum"></a>

# Module `0x1::ethereum`



-  [Struct `EthereumAddress`](#0x1_ethereum_EthereumAddress)
-  [Constants](#@Constants_0)
-  [Function `ethereum_address`](#0x1_ethereum_ethereum_address)
-  [Function `ethereum_address_no_eip55`](#0x1_ethereum_ethereum_address_no_eip55)
-  [Function `ethereum_address_20_bytes`](#0x1_ethereum_ethereum_address_20_bytes)
-  [Function `get_inner_ethereum_address`](#0x1_ethereum_get_inner_ethereum_address)
-  [Function `to_lowercase`](#0x1_ethereum_to_lowercase)
-  [Function `to_eip55_checksumed_address`](#0x1_ethereum_to_eip55_checksumed_address)
-  [Function `get_inner`](#0x1_ethereum_get_inner)
-  [Function `assert_eip55`](#0x1_ethereum_assert_eip55)
-  [Function `assert_40_char_hex`](#0x1_ethereum_assert_40_char_hex)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
</code></pre>



<a id="0x1_ethereum_EthereumAddress"></a>

## Struct `EthereumAddress`

Represents an Ethereum address within Aptos smart contracts.
Provides structured handling, storage, and validation of Ethereum addresses.


<pre><code><b>struct</b> <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>inner: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ethereum_ASCII_A"></a>

Constants for ASCII character codes


<pre><code><b>const</b> <a href="ethereum.md#0x1_ethereum_ASCII_A">ASCII_A</a>: u8 = 65;
</code></pre>



<a id="0x1_ethereum_ASCII_A_LOWERCASE"></a>



<pre><code><b>const</b> <a href="ethereum.md#0x1_ethereum_ASCII_A_LOWERCASE">ASCII_A_LOWERCASE</a>: u8 = 97;
</code></pre>



<a id="0x1_ethereum_ASCII_F_LOWERCASE"></a>



<pre><code><b>const</b> <a href="ethereum.md#0x1_ethereum_ASCII_F_LOWERCASE">ASCII_F_LOWERCASE</a>: u8 = 102;
</code></pre>



<a id="0x1_ethereum_ASCII_Z"></a>



<pre><code><b>const</b> <a href="ethereum.md#0x1_ethereum_ASCII_Z">ASCII_Z</a>: u8 = 90;
</code></pre>



<a id="0x1_ethereum_EINVALID_LENGTH"></a>



<pre><code><b>const</b> <a href="ethereum.md#0x1_ethereum_EINVALID_LENGTH">EINVALID_LENGTH</a>: u64 = 1;
</code></pre>



<a id="0x1_ethereum_ethereum_address"></a>

## Function `ethereum_address`

Validates an Ethereum address against EIP-55 checksum rules and returns a new <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code>.

@param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
@return A validated <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code> struct.
@abort If the address does not conform to EIP-55 standards.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address">ethereum_address</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address">ethereum_address</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> {
    <a href="ethereum.md#0x1_ethereum_assert_eip55">assert_eip55</a>(&ethereum_address);
    <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> { inner: ethereum_address }
}
</code></pre>



</details>

<a id="0x1_ethereum_ethereum_address_no_eip55"></a>

## Function `ethereum_address_no_eip55`

Returns a new <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code> without EIP-55 validation.

@param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
@return A validated <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code> struct.
@abort If the address does not conform to EIP-55 standards.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address_no_eip55">ethereum_address_no_eip55</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address_no_eip55">ethereum_address_no_eip55</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> {
    <a href="ethereum.md#0x1_ethereum_assert_40_char_hex">assert_40_char_hex</a>(&ethereum_address);
    <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> { inner: ethereum_address }
}
</code></pre>



</details>

<a id="0x1_ethereum_ethereum_address_20_bytes"></a>

## Function `ethereum_address_20_bytes`

Returns a new 20-byte <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code> without EIP-55 validation.

@param ethereum_address A 20-byte vector of unsigned 8-bit bytes.
@return An <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code> struct.
@abort If the address does not conform to EIP-55 standards.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address_20_bytes">ethereum_address_20_bytes</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_ethereum_address_20_bytes">ethereum_address_20_bytes</a>(ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ethereum_address) == 20, <a href="ethereum.md#0x1_ethereum_EINVALID_LENGTH">EINVALID_LENGTH</a>);
    <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a> { inner: ethereum_address }
}
</code></pre>



</details>

<a id="0x1_ethereum_get_inner_ethereum_address"></a>

## Function `get_inner_ethereum_address`

Gets the inner vector of an <code><a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a></code>.

@param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
@return The vector<u8> inner value of the EthereumAddress


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_get_inner_ethereum_address">get_inner_ethereum_address</a>(ethereum_address: <a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_get_inner_ethereum_address">get_inner_ethereum_address</a>(ethereum_address: <a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    ethereum_address.inner
}
</code></pre>



</details>

<a id="0x1_ethereum_to_lowercase"></a>

## Function `to_lowercase`

Converts uppercase ASCII characters in a vector to their lowercase equivalents.

@param input A reference to a vector of ASCII characters.
@return A new vector with lowercase equivalents of the input characters.
@note Only affects ASCII letters; non-alphabetic characters are unchanged.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_to_lowercase">to_lowercase</a>(input: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_to_lowercase">to_lowercase</a>(input: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> lowercase_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(input, |_i, element| {
        <b>let</b> lower_byte = <b>if</b> (*element &gt;= <a href="ethereum.md#0x1_ethereum_ASCII_A">ASCII_A</a> && *element &lt;= <a href="ethereum.md#0x1_ethereum_ASCII_Z">ASCII_Z</a>) {
            *element + 32
        } <b>else</b> {
            *element
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>&lt;u8&gt;(&<b>mut</b> lowercase_bytes, lower_byte);
    });
    lowercase_bytes
}
</code></pre>



</details>

<a id="0x1_ethereum_to_eip55_checksumed_address"></a>

## Function `to_eip55_checksumed_address`

Converts an Ethereum address to EIP-55 checksummed format.

@param ethereum_address A 40-character vector representing the Ethereum address in hexadecimal format.
@return The EIP-55 checksummed version of the input address.
@abort If the input address does not have exactly 40 characters.
@note Assumes input address is valid and in lowercase hexadecimal format.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_to_eip55_checksumed_address">to_eip55_checksumed_address</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_to_eip55_checksumed_address">to_eip55_checksumed_address</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(ethereum_address) == 40, 0);
    <b>let</b> lowercase = <a href="ethereum.md#0x1_ethereum_to_lowercase">to_lowercase</a>(ethereum_address);
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = keccak256(lowercase);
    <b>let</b> output = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();

    for (index in 0..40) {
        <b>let</b> item = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(ethereum_address, index);
        <b>if</b> (item &gt;= <a href="ethereum.md#0x1_ethereum_ASCII_A_LOWERCASE">ASCII_A_LOWERCASE</a> && item &lt;= <a href="ethereum.md#0x1_ethereum_ASCII_F_LOWERCASE">ASCII_F_LOWERCASE</a>) {
            <b>let</b> hash_item = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>, index / 2);
            <b>if</b> ((hash_item &gt;&gt; ((4 * (1 - (index % 2))) <b>as</b> u8)) & 0xF &gt;= 8) {
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> output, item - 32);
            } <b>else</b> {
                <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> output, item);
            }
        } <b>else</b> {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> output, item);
        }
    };
    output
}
</code></pre>



</details>

<a id="0x1_ethereum_get_inner"></a>

## Function `get_inner`



<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_get_inner">get_inner</a>(eth_address: &<a href="ethereum.md#0x1_ethereum_EthereumAddress">ethereum::EthereumAddress</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_get_inner">get_inner</a>(eth_address: &<a href="ethereum.md#0x1_ethereum_EthereumAddress">EthereumAddress</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    eth_address.inner
}
</code></pre>



</details>

<a id="0x1_ethereum_assert_eip55"></a>

## Function `assert_eip55`

Checks if an Ethereum address conforms to the EIP-55 checksum standard.

@param ethereum_address A reference to a 40-character vector of an Ethereum address in hexadecimal format.
@abort If the address does not match its EIP-55 checksummed version.
@note Assumes the address is correctly formatted as a 40-character hexadecimal string.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_assert_eip55">assert_eip55</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_assert_eip55">assert_eip55</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> eip55 = <a href="ethereum.md#0x1_ethereum_to_eip55_checksumed_address">to_eip55_checksumed_address</a>(ethereum_address);
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&eip55);
    for (index in 0..len) {
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&eip55, index) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(ethereum_address, index), 0);
    };
}
</code></pre>



</details>

<a id="0x1_ethereum_assert_40_char_hex"></a>

## Function `assert_40_char_hex`

Checks if an Ethereum address is a nonzero 40-character hexadecimal string.

@param ethereum_address A reference to a vector of bytes representing the Ethereum address as characters.
@abort If the address is not 40 characters long, contains invalid characters, or is all zeros.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_assert_40_char_hex">assert_40_char_hex</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum.md#0x1_ethereum_assert_40_char_hex">assert_40_char_hex</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(ethereum_address);

    // Ensure the <b>address</b> is exactly 40 characters long
    <b>assert</b>!(len == 40, 1);

    // Ensure the <b>address</b> contains only valid hexadecimal characters
    <b>let</b> is_zero = <b>true</b>;
    for (index in 0..len) {
        <b>let</b> char = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(ethereum_address, index);

        // Check <b>if</b> the character is a valid hexadecimal character (0-9, a-f, A-F)
        <b>assert</b>!(
            (char &gt;= 0x30 && char &lt;= 0x39) || // '0' <b>to</b> '9'
            (char &gt;= 0x41 && char &lt;= 0x46) || // 'A' <b>to</b> 'F'
            (char &gt;= 0x61 && char &lt;= 0x66),  // 'a' <b>to</b> 'f'
            2
        );

        // Check <b>if</b> the <b>address</b> is nonzero
        <b>if</b> (char != 0x30) { // '0'
            is_zero = <b>false</b>;
        };
    };

    // Abort <b>if</b> the <b>address</b> is all zeros
    <b>assert</b>!(!is_zero, 3);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
