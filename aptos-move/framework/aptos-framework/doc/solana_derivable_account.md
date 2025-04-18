
<a id="0x1_solana_derivable_account"></a>

# Module `0x1::solana_derivable_account`

Derivable account abstraction that verifies a message signed by
SIWS.
1. The message format is as follows:

<domain> wants you to sign in with your Solana account:
<base58_public_key>

Please confirm you explicitly initiated this request from <domain>. You are approving to execute transaction <entry_function_name> on Aptos blockchain (<network_name>).

Nonce: <aptos_txn_digest>

2. The abstract public key is a BCS serialized <code>SIWSAbstractPublicKey</code>.
3. The abstract signature is a BCS serialized <code><a href="solana_derivable_account.md#0x1_solana_derivable_account_SIWSAbstractSignature">SIWSAbstractSignature</a></code>.
4. This module has been tested for the following wallets:
- Phantom
- Solflare
- Backpack
- OKX


-  [Enum `SIWSAbstractSignature`](#0x1_solana_derivable_account_SIWSAbstractSignature)
-  [Constants](#@Constants_0)
-  [Function `deserialize_abstract_public_key`](#0x1_solana_derivable_account_deserialize_abstract_public_key)
-  [Function `deserialize_abstract_signature`](#0x1_solana_derivable_account_deserialize_abstract_signature)
-  [Function `construct_message`](#0x1_solana_derivable_account_construct_message)
-  [Function `to_public_key_bytes`](#0x1_solana_derivable_account_to_public_key_bytes)
-  [Function `entry_function_name`](#0x1_solana_derivable_account_entry_function_name)
-  [Function `authenticate_auth_data`](#0x1_solana_derivable_account_authenticate_auth_data)
-  [Function `authenticate`](#0x1_solana_derivable_account_authenticate)
-  [Specification](#@Specification_1)
    -  [Function `to_public_key_bytes`](#@Specification_1_to_public_key_bytes)
    -  [Function `authenticate_auth_data`](#@Specification_1_authenticate_auth_data)
    -  [Function `authenticate`](#@Specification_1_authenticate)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils">0x1::common_account_abstractions_utils</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_solana_derivable_account_SIWSAbstractSignature"></a>

## Enum `SIWSAbstractSignature`



<pre><code>enum <a href="solana_derivable_account.md#0x1_solana_derivable_account_SIWSAbstractSignature">SIWSAbstractSignature</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>MessageV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_solana_derivable_account_PUBLIC_KEY_NUM_BYTES"></a>



<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_PUBLIC_KEY_NUM_BYTES">PUBLIC_KEY_NUM_BYTES</a>: u64 = 32;
</code></pre>



<a id="0x1_solana_derivable_account_EINVALID_PUBLIC_KEY"></a>

Invalid public key.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>: u64 = 5;
</code></pre>



<a id="0x1_solana_derivable_account_EINVALID_SIGNATURE"></a>

Signature failed to verify.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 1;
</code></pre>



<a id="0x1_solana_derivable_account_EINVALID_SIGNATURE_TYPE"></a>

Invalid signature type.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>: u64 = 4;
</code></pre>



<a id="0x1_solana_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD"></a>

Entry function payload is missing.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>: u64 = 3;
</code></pre>



<a id="0x1_solana_derivable_account_BASE_58_ALPHABET"></a>



<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_BASE_58_ALPHABET">BASE_58_ALPHABET</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [49, 50, 51, 52, 53, 54, 55, 56, 57, 65, 66, 67, 68, 69, 70, 71, 72, 74, 75, 76, 77, 78, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122];
</code></pre>



<a id="0x1_solana_derivable_account_EINVALID_BASE_58_PUBLIC_KEY"></a>

Non base58 character found in public key.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_BASE_58_PUBLIC_KEY">EINVALID_BASE_58_PUBLIC_KEY</a>: u64 = 2;
</code></pre>



<a id="0x1_solana_derivable_account_EINVALID_PUBLIC_KEY_LENGTH"></a>

Invalid public key length.


<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_PUBLIC_KEY_LENGTH">EINVALID_PUBLIC_KEY_LENGTH</a>: u64 = 6;
</code></pre>



<a id="0x1_solana_derivable_account_HEX_ALPHABET"></a>



<pre><code><b>const</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_HEX_ALPHABET">HEX_ALPHABET</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 97, 98, 99, 100, 101, 102];
</code></pre>



<a id="0x1_solana_derivable_account_deserialize_abstract_public_key"></a>

## Function `deserialize_abstract_public_key`

Deserializes the abstract public key which is supposed to be a bcs
serialized <code>SIWSAbstractPublicKey</code>.  The base58_public_key is
represented in UTF8. We prefer this format because it's computationally
cheaper to decode a base58 string than to encode from raw bytes.  We
require both the base58 public key in UTF8 to construct the message and
the raw bytes version to do signature verification.


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;):
(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_public_key);
    <b>let</b> base58_public_key = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    <b>let</b> domain = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    (base58_public_key, domain)
}
</code></pre>



</details>

<a id="0x1_solana_derivable_account_deserialize_abstract_signature"></a>

## Function `deserialize_abstract_signature`

Returns a tuple of the signature type and the signature.


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="solana_derivable_account.md#0x1_solana_derivable_account_SIWSAbstractSignature">solana_derivable_account::SIWSAbstractSignature</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="solana_derivable_account.md#0x1_solana_derivable_account_SIWSAbstractSignature">SIWSAbstractSignature</a> {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_signature);
    <b>let</b> signature_type = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_u8">bcs_stream::deserialize_u8</a>(&<b>mut</b> stream);
    <b>if</b> (signature_type == 0x00) {
        <b>let</b> signature = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
        SIWSAbstractSignature::MessageV1 { signature }
    } <b>else</b> {
        <b>abort</b>(<a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>)
    }
}
</code></pre>



</details>

<a id="0x1_solana_derivable_account_construct_message"></a>

## Function `construct_message`



<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_construct_message">construct_message</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_construct_message">construct_message</a>(
    base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    message.append(*domain);
    message.append(b" wants you <b>to</b> sign in <b>with</b> your Solana <a href="account.md#0x1_account">account</a>:\n");
    message.append(*base58_public_key);
    message.append(b"\n\nPlease confirm you explicitly initiated this request from ");
    message.append(*domain);
    message.append(b".");
    message.append(b" You are approving <b>to</b> execute transaction ");
    message.append(*entry_function_name);
    message.append(b" on Aptos blockchain");
    <b>let</b> network_name = network_name();
    message.append(b" (");
    message.append(network_name);
    message.append(b")");
    message.append(b".");
    message.append(b"\n\nNonce: ");
    message.append(*digest_utf8);
    *message
}
</code></pre>



</details>

<a id="0x1_solana_derivable_account_to_public_key_bytes"></a>

## Function `to_public_key_bytes`



<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_to_public_key_bytes">to_public_key_bytes</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_to_public_key_bytes">to_public_key_bytes</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0u8];
    <b>let</b> base = 58u16;

    <b>let</b> i = 0;
    <b>while</b> (i &lt; base58_public_key.length()) {
        <b>let</b> char = base58_public_key[i];
        <b>let</b> (found, char_index) = <a href="solana_derivable_account.md#0x1_solana_derivable_account_BASE_58_ALPHABET">BASE_58_ALPHABET</a>.index_of(&char);
        <b>assert</b>!(found, <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_BASE_58_PUBLIC_KEY">EINVALID_BASE_58_PUBLIC_KEY</a>);

        <b>let</b> j = 0;
        <b>let</b> carry = (char_index <b>as</b> u16);

        // For each existing byte, multiply by 58 and add carry
        <b>while</b> (j &lt; bytes.length()) {
            <b>let</b> current = (bytes[j] <b>as</b> u16);
            <b>let</b> new_carry = current * base + carry;
            bytes[j] = ((new_carry & 0xff) <b>as</b> u8);
            carry = new_carry &gt;&gt; 8;
            j = j + 1;
        };

        // Add <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> remaining carry <b>as</b> new bytes
        <b>while</b> (carry &gt; 0) {
            bytes.push_back((carry & 0xff) <b>as</b> u8);
            carry = carry &gt;&gt; 8;
        };

        i = i + 1;
    };

    // Handle leading zeros (1's in Base58)
    <b>let</b> i = 0;
    <b>while</b> (i &lt; base58_public_key.length() && base58_public_key[i] == 49) { // '1' is 49 in ASCII
        bytes.push_back(0);
        i = i + 1;
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> bytes);
    <b>assert</b>!(bytes.length() == <a href="solana_derivable_account.md#0x1_solana_derivable_account_PUBLIC_KEY_NUM_BYTES">PUBLIC_KEY_NUM_BYTES</a>, <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_PUBLIC_KEY_LENGTH">EINVALID_PUBLIC_KEY_LENGTH</a>);
    bytes
}
</code></pre>



</details>

<a id="0x1_solana_derivable_account_entry_function_name"></a>

## Function `entry_function_name`



<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_entry_function_name">entry_function_name</a>(entry_function_payload: &<a href="transaction_context.md#0x1_transaction_context_EntryFunctionPayload">transaction_context::EntryFunctionPayload</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_entry_function_name">entry_function_name</a>(entry_function_payload: &EntryFunctionPayload): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
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

<a id="0x1_solana_derivable_account_authenticate_auth_data"></a>

## Function `authenticate_auth_data`



<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(
    aa_auth_data: AbstractionAuthData,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    <b>let</b> abstract_public_key = aa_auth_data.derivable_abstract_public_key();
    <b>let</b> (base58_public_key, domain) = <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key);
    <b>let</b> digest_utf8 = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest()).bytes();

    <b>let</b> public_key_bytes = <a href="solana_derivable_account.md#0x1_solana_derivable_account_to_public_key_bytes">to_public_key_bytes</a>(&base58_public_key);
    <b>let</b> public_key = new_validated_public_key_from_bytes(public_key_bytes);
    <b>assert</b>!(public_key.is_some(), <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>);
    <b>let</b> abstract_signature = <a href="solana_derivable_account.md#0x1_solana_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(aa_auth_data.derivable_abstract_signature());
    match (abstract_signature) {
        SIWSAbstractSignature::MessageV1 { signature: signature_bytes } =&gt; {
            <b>let</b> message = <a href="solana_derivable_account.md#0x1_solana_derivable_account_construct_message">construct_message</a>(&base58_public_key, &domain, entry_function_name, digest_utf8);

            <b>let</b> signature = new_signature_from_bytes(signature_bytes);
            <b>assert</b>!(
                <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
                    &signature,
                    &public_key_into_unvalidated(public_key.destroy_some()),
                    message,
                ),
                <a href="solana_derivable_account.md#0x1_solana_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>
            );
        },
    };
}
</code></pre>



</details>

<a id="0x1_solana_derivable_account_authenticate"></a>

## Function `authenticate`

Authorization function for domain account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> maybe_entry_function_payload = <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">transaction_context::entry_function_payload</a>();
    <b>if</b> (maybe_entry_function_payload.is_some()) {
        <b>let</b> entry_function_payload = maybe_entry_function_payload.destroy_some();
        <b>let</b> entry_function_name = <a href="solana_derivable_account.md#0x1_solana_derivable_account_entry_function_name">entry_function_name</a>(&entry_function_payload);
        <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data, &entry_function_name);
        <a href="account.md#0x1_account">account</a>
    } <b>else</b> {
        <b>abort</b>(<a href="solana_derivable_account.md#0x1_solana_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>)
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_to_public_key_bytes"></a>

### Function `to_public_key_bytes`


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_to_public_key_bytes">to_public_key_bytes</a>(base58_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>ensures</b> result.length() == <a href="solana_derivable_account.md#0x1_solana_derivable_account_PUBLIC_KEY_NUM_BYTES">PUBLIC_KEY_NUM_BYTES</a>;
</code></pre>



<a id="@Specification_1_authenticate_auth_data"></a>

### Function `authenticate_auth_data`


<pre><code><b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_authenticate"></a>

### Function `authenticate`


<pre><code><b>public</b> <b>fun</b> <a href="solana_derivable_account.md#0x1_solana_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
