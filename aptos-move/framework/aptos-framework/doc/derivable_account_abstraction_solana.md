
<a id="0x1_derivable_account_abstraction_ed25519_hex"></a>

# Module `0x1::derivable_account_abstraction_ed25519_hex`

Domain account abstraction using ed25519 hex for signing.

Authentication takes digest, converts to hex (prefixed with 0x, with lowercase letters),
and then expects that to be signed.
authenticator is expected to be signature: vector<u8>
account_identity is raw public_key.


-  [Constants](#@Constants_0)
-  [Function `split_abstract_public_key`](#0x1_derivable_account_abstraction_ed25519_hex_split_abstract_public_key)
-  [Function `network_name`](#0x1_derivable_account_abstraction_ed25519_hex_network_name)
-  [Function `construct_message`](#0x1_derivable_account_abstraction_ed25519_hex_construct_message)
-  [Function `to_public_key_bytes`](#0x1_derivable_account_abstraction_ed25519_hex_to_public_key_bytes)
-  [Function `entry_function_name`](#0x1_derivable_account_abstraction_ed25519_hex_entry_function_name)
-  [Function `authenticate_auth_data`](#0x1_derivable_account_abstraction_ed25519_hex_authenticate_auth_data)
-  [Function `authenticate`](#0x1_derivable_account_abstraction_ed25519_hex_authenticate)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_derivable_account_abstraction_ed25519_hex_BASE_58_ALPHABET"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_BASE_58_ALPHABET">BASE_58_ALPHABET</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [49, 50, 51, 52, 53, 54, 55, 56, 57, 65, 66, 67, 68, 69, 70, 71, 72, 74, 75, 76, 77, 78, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122];
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_EINVALID_BASE_58_PUBLIC_KEY"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EINVALID_BASE_58_PUBLIC_KEY">EINVALID_BASE_58_PUBLIC_KEY</a>: u64 = 2;
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 1;
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_EMISSING_ENTRY_FUNCTION_PAYLOAD"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>: u64 = 4;
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_EUNSUPPORTED_CHAIN_ID"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EUNSUPPORTED_CHAIN_ID">EUNSUPPORTED_CHAIN_ID</a>: u64 = 3;
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_HEX_ALPHABET"></a>



<pre><code><b>const</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_HEX_ALPHABET">HEX_ALPHABET</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102];
</code></pre>



<a id="0x1_derivable_account_abstraction_ed25519_hex_split_abstract_public_key"></a>

## Function `split_abstract_public_key`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_split_abstract_public_key">split_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_split_abstract_public_key">split_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    // First 44 bytes are the base58 utf8 encoded <b>public</b> key
    <b>let</b> base58_public_key = abstract_public_key.slice(0, 44);
    <b>let</b> domain = abstract_public_key.slice(44, abstract_public_key.length());
    (base58_public_key, domain)
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_network_name"></a>

## Function `network_name`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_network_name">network_name</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_network_name">network_name</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> <a href="chain_id.md#0x1_chain_id">chain_id</a> = <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>();
    <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 1) {
        b"mainnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 2) {
        b"testnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 3) {
        b"devnet"
    } <b>else</b> <b>if</b> (<a href="chain_id.md#0x1_chain_id">chain_id</a> == 4) {
        b"<b>local</b>"
    } <b>else</b> {
        <b>abort</b>(<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EUNSUPPORTED_CHAIN_ID">EUNSUPPORTED_CHAIN_ID</a>)
    }
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_construct_message"></a>

## Function `construct_message`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_construct_message">construct_message</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_construct_message">construct_message</a>(
    base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    message.append(*domain);
    message.append(b" wants you <b>to</b> sign in <b>with</b> your Solana <a href="account.md#0x1_account">account</a>:\n");
    message.append(*base58_public_key);
    message.append(b"\n\nTo execute transaction ");
    message.append(*entry_function_name);
    message.append(b" on Aptos blockchain (");
    message.append(<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_network_name">network_name</a>());
    message.append(b").");
    message.append(b"\n\nNonce: ");
    message.append(*digest_utf8);
    *message
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_to_public_key_bytes"></a>

## Function `to_public_key_bytes`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_to_public_key_bytes">to_public_key_bytes</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_to_public_key_bytes">to_public_key_bytes</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0u8];
    <b>let</b> base = 58u16;  // Using u16 <b>to</b> handle multiplication without overflow

    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(base58_public_key)) {
        <b>let</b> char = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(base58_public_key, i);
        <b>let</b> (found, char_index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_BASE_58_ALPHABET">BASE_58_ALPHABET</a>, &char);
        <b>assert</b>!(found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EINVALID_BASE_58_PUBLIC_KEY">EINVALID_BASE_58_PUBLIC_KEY</a>));

        <b>let</b> mut_bytes = &<b>mut</b> bytes;
        <b>let</b> j = 0;
        <b>let</b> carry = (char_index <b>as</b> u16);

        // For each existing byte, multiply by 58 and add carry
        <b>while</b> (j &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(mut_bytes)) {
            <b>let</b> current = (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(mut_bytes, j) <b>as</b> u16);
            <b>let</b> new_carry = current * base + carry;
            *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(mut_bytes, j) = ((new_carry & 0xff) <b>as</b> u8);
            carry = new_carry &gt;&gt; 8;
            j = j + 1;
        };

        // Add <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> remaining carry <b>as</b> new bytes
        <b>while</b> (carry &gt; 0) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(mut_bytes, ((carry & 0xff) <b>as</b> u8));
            carry = carry &gt;&gt; 8;
        };

        i = i + 1;
    };

    // Handle leading zeros (1's in Base58)
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(base58_public_key) && *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(base58_public_key, i) == 49) { // '1' is 49 in ASCII
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> bytes, 0);
        i = i + 1;
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> bytes);
    bytes
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_entry_function_name"></a>

## Function `entry_function_name`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_entry_function_name">entry_function_name</a>(entry_function_payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_entry_function_name">entry_function_name</a>(entry_function_payload: &EntryFunctionPayload): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> entry_function_name = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> addr_str = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(
        &<a href="transaction_context.md#0x1_transaction_context_account_address">transaction_context::account_address</a>(entry_function_payload)
    ).bytes();
    // .slice(1) <b>to</b> remove the leading '@' char
    entry_function_name.append(addr_str.slice(1, addr_str.length()));
    entry_function_name.append(b"::");
    entry_function_name.append(
        *<a href="transaction_context.md#0x1_transaction_context_module_name">transaction_context::module_name</a>(entry_function_payload).bytes()
    );
    entry_function_name.append(b"::");
    entry_function_name.append(
        *<a href="transaction_context.md#0x1_transaction_context_function_name">transaction_context::function_name</a>(entry_function_payload).bytes()
    );
    *entry_function_name
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_authenticate_auth_data"></a>

## Function `authenticate_auth_data`



<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_authenticate_auth_data">authenticate_auth_data</a>(
    aa_auth_data: AbstractionAuthData,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    <b>let</b> abstract_public_key = aa_auth_data.derivable_abstract_public_key();
    <b>let</b> (base58_public_key, domain) = <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_split_abstract_public_key">split_abstract_public_key</a>(abstract_public_key);
    <b>let</b> digest_utf8 = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest()).bytes();
    <b>let</b> message = <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_construct_message">construct_message</a>(&base58_public_key, &domain, entry_function_name, digest_utf8);

    <b>let</b> public_key_bytes = <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_to_public_key_bytes">to_public_key_bytes</a>(&base58_public_key);
    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(public_key_bytes);
    <b>let</b> signature = new_signature_from_bytes(*aa_auth_data.derivable_abstract_signature());
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key,
            message,
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );
}
</code></pre>



</details>

<a id="0x1_derivable_account_abstraction_ed25519_hex_authenticate"></a>

## Function `authenticate`

Authorization function for domain account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> maybe_entry_function_payload = <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">transaction_context::entry_function_payload</a>();
    <b>if</b> (maybe_entry_function_payload.is_some()) {
        <b>let</b> entry_function_payload = maybe_entry_function_payload.destroy_some();
        <b>let</b> entry_function_name = <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_entry_function_name">entry_function_name</a>(&entry_function_payload);
        <a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data, &entry_function_name);
        <a href="account.md#0x1_account">account</a>
    } <b>else</b> {
        <b>abort</b>(<a href="derivable_account_abstraction_solana.md#0x1_derivable_account_abstraction_ed25519_hex_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>)
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
