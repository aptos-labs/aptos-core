
<a id="0x1_ethereum_derivable_account"></a>

# Module `0x1::ethereum_derivable_account`

Derivable account abstraction that verifies a message signed by
SIWE.
1. The message format is as follows:

<domain> wants you to sign in with your Ethereum account:
<ethereum_address>

Please confirm you explicitly initiated this request from <domain>. You are approving to execute transaction <entry_function_name> on Aptos blockchain (<network_name>).

URI: <domain>
Version: 1
Chain ID: <chain_id>
Nonce: <digest>
Issued At: <issued_at>

2. The abstract public key is a BCS serialized <code><a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">SIWEAbstractPublicKey</a></code>.
3. The abstract signature is a BCS serialized <code><a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractSignature">SIWEAbstractSignature</a></code>.
4. This module has been tested for the following wallets:
- Metamask
- Phantom
- Coinbase
- OKX
- Exodus
- Backpack


-  [Enum `SIWEAbstractSignature`](#0x1_ethereum_derivable_account_SIWEAbstractSignature)
-  [Struct `SIWEAbstractPublicKey`](#0x1_ethereum_derivable_account_SIWEAbstractPublicKey)
-  [Constants](#@Constants_0)
-  [Function `deserialize_abstract_public_key`](#0x1_ethereum_derivable_account_deserialize_abstract_public_key)
-  [Function `deserialize_abstract_signature`](#0x1_ethereum_derivable_account_deserialize_abstract_signature)
-  [Function `construct_message`](#0x1_ethereum_derivable_account_construct_message)
-  [Function `recover_public_key`](#0x1_ethereum_derivable_account_recover_public_key)
-  [Function `authenticate_auth_data`](#0x1_ethereum_derivable_account_authenticate_auth_data)
-  [Function `authenticate`](#0x1_ethereum_derivable_account_authenticate)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/base16.md#0x1_base16">0x1::base16</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils">0x1::common_account_abstractions_utils</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/secp256k1.md#0x1_secp256k1">0x1::secp256k1</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_ethereum_derivable_account_SIWEAbstractSignature"></a>

## Enum `SIWEAbstractSignature`



<pre><code>enum <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractSignature">SIWEAbstractSignature</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>EIP1193DerivedSignature</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>issued_at: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_ethereum_derivable_account_SIWEAbstractPublicKey"></a>

## Struct `SIWEAbstractPublicKey`



<pre><code><b>struct</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">SIWEAbstractPublicKey</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ethereum_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>domain: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ethereum_derivable_account_EADDR_MISMATCH"></a>

Address mismatch.


<pre><code><b>const</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EADDR_MISMATCH">EADDR_MISMATCH</a>: u64 = 4;
</code></pre>



<a id="0x1_ethereum_derivable_account_EINVALID_SIGNATURE"></a>

Signature failed to verify.


<pre><code><b>const</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 1;
</code></pre>



<a id="0x1_ethereum_derivable_account_EINVALID_SIGNATURE_TYPE"></a>

Invalid signature type.


<pre><code><b>const</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>: u64 = 3;
</code></pre>



<a id="0x1_ethereum_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD"></a>

Entry function payload is missing.


<pre><code><b>const</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>: u64 = 2;
</code></pre>



<a id="0x1_ethereum_derivable_account_EUNEXPECTED_V"></a>

Unexpected v value.


<pre><code><b>const</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EUNEXPECTED_V">EUNEXPECTED_V</a>: u64 = 5;
</code></pre>



<a id="0x1_ethereum_derivable_account_deserialize_abstract_public_key"></a>

## Function `deserialize_abstract_public_key`

Deserializes the abstract public key which is supposed to be a bcs
serialized <code><a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">SIWEAbstractPublicKey</a></code>.


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">ethereum_derivable_account::SIWEAbstractPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">SIWEAbstractPublicKey</a> {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_public_key);
    <b>let</b> ethereum_address = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    <b>let</b> domain = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractPublicKey">SIWEAbstractPublicKey</a> { ethereum_address, domain }
}
</code></pre>



</details>

<a id="0x1_ethereum_derivable_account_deserialize_abstract_signature"></a>

## Function `deserialize_abstract_signature`

Returns a tuple of the signature type and the signature.
We include the issued_at in the signature as it is a required field in the SIWE standard.


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractSignature">ethereum_derivable_account::SIWEAbstractSignature</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_SIWEAbstractSignature">SIWEAbstractSignature</a> {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_signature);
    <b>let</b> signature_type = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_u8">bcs_stream::deserialize_u8</a>(&<b>mut</b> stream);
    <b>if</b> (signature_type == 0x00) {
        <b>let</b> issued_at = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
        <b>let</b> signature = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
        SIWEAbstractSignature::EIP1193DerivedSignature { issued_at: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(issued_at), signature }
    } <b>else</b> {
        <b>abort</b>(<a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>)
    }
}
</code></pre>



</details>

<a id="0x1_ethereum_derivable_account_construct_message"></a>

## Function `construct_message`



<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_construct_message">construct_message</a>(ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, issued_at: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_construct_message">construct_message</a>(
    ethereum_address: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    issued_at: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    message.append(*domain);
    message.append(b" wants you <b>to</b> sign in <b>with</b> your Ethereum <a href="account.md#0x1_account">account</a>:\n");
    message.append(*ethereum_address);
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
    message.append(b"\n\nURI: ");
    message.append(*domain);
    message.append(b"\nVersion: 1");
    message.append(b"\nChain ID: ");
    message.append(*<a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(&<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>()).bytes());
    message.append(b"\nNonce: ");
    message.append(*digest_utf8);
    message.append(b"\nIssued At: ");
    message.append(*issued_at);

    <b>let</b> msg_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(message);

    <b>let</b> prefix = b"\x19Ethereum Signed Message:\n";
    <b>let</b> msg_len_string = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(&msg_len); // returns <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">string</a>
    <b>let</b> msg_len_bytes = msg_len_string.bytes(); // <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;

    <b>let</b> full_message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    full_message.append(prefix);
    full_message.append(*msg_len_bytes);
    full_message.append(*message);

    *full_message
}
</code></pre>



</details>

<a id="0x1_ethereum_derivable_account_recover_public_key"></a>

## Function `recover_public_key`



<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_recover_public_key">recover_public_key</a>(signature_bytes: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_recover_public_key">recover_public_key</a>(signature_bytes: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> rs = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(signature_bytes, 0, 64);
    <b>let</b> v = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(signature_bytes, 64);
    <b>assert</b>!(v == 27 || v == 28, <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EUNEXPECTED_V">EUNEXPECTED_V</a>);
    <b>let</b> signature = <a href="../../aptos-stdlib/doc/secp256k1.md#0x1_secp256k1_ecdsa_signature_from_bytes">secp256k1::ecdsa_signature_from_bytes</a>(rs);

    <b>let</b> maybe_recovered = <a href="../../aptos-stdlib/doc/secp256k1.md#0x1_secp256k1_ecdsa_recover">secp256k1::ecdsa_recover</a>(*message, v - 27, &signature);

    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&maybe_recovered),
        <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>
    );

    <b>let</b> pubkey = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&maybe_recovered);

    <b>let</b> pubkey_bytes = <a href="../../aptos-stdlib/doc/secp256k1.md#0x1_secp256k1_ecdsa_raw_public_key_to_bytes">secp256k1::ecdsa_raw_public_key_to_bytes</a>(pubkey);

    // Add 0x04 prefix <b>to</b> the <b>public</b> key, <b>to</b> match the
    // full uncompressed format from ethers.js
    <b>let</b> full_pubkey = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(full_pubkey, 4u8);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(full_pubkey, pubkey_bytes);

    *full_pubkey
}
</code></pre>



</details>

<a id="0x1_ethereum_derivable_account_authenticate_auth_data"></a>

## Function `authenticate_auth_data`



<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(
    aa_auth_data: AbstractionAuthData,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    <b>let</b> derivable_abstract_public_key = aa_auth_data.derivable_abstract_public_key();
    <b>let</b> abstract_public_key = <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(derivable_abstract_public_key);
    <b>let</b> digest_utf8 = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest()).bytes();
    <b>let</b> abstract_signature = <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(aa_auth_data.derivable_abstract_signature());
    <b>let</b> issued_at = abstract_signature.issued_at.bytes();
    <b>let</b> message = <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_construct_message">construct_message</a>(&abstract_public_key.ethereum_address, &abstract_public_key.domain, entry_function_name, digest_utf8, issued_at);
    <b>let</b> hashed_message = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash_keccak256">aptos_hash::keccak256</a>(message);
    <b>let</b> public_key_bytes = <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_recover_public_key">recover_public_key</a>(&abstract_signature.signature, &hashed_message);

    // 1. Skip the 0x04 prefix (take the bytes after the first byte)
    <b>let</b> public_key_without_prefix = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&public_key_bytes, 1, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&public_key_bytes));
    // 2. Run Keccak256 on the <b>public</b> key (without the 0x04 prefix)
    <b>let</b> kexHash = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash_keccak256">aptos_hash::keccak256</a>(public_key_without_prefix);
    // 3. Slice the last 20 bytes (this is the Ethereum <b>address</b>)
    <b>let</b> recovered_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&kexHash, 12, 32);
    // 4. Remove the 0x prefix from the utf8 <a href="account.md#0x1_account">account</a> <b>address</b>
    <b>let</b> ethereum_address_without_prefix = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&abstract_public_key.ethereum_address, 2, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&abstract_public_key.ethereum_address));

    <b>let</b> account_address_vec = base16_utf8_to_vec_u8(ethereum_address_without_prefix);
    // Verify that the recovered <b>address</b> matches the domain <a href="account.md#0x1_account">account</a> identity
    <b>assert</b>!(recovered_addr == account_address_vec, <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EADDR_MISMATCH">EADDR_MISMATCH</a>);
}
</code></pre>



</details>

<a id="0x1_ethereum_derivable_account_authenticate"></a>

## Function `authenticate`

Authorization function for domain account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> maybe_entry_function_payload = <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">transaction_context::entry_function_payload</a>();
    <b>if</b> (maybe_entry_function_payload.is_some()) {
        <b>let</b> entry_function_payload = maybe_entry_function_payload.destroy_some();
        <b>let</b> entry_function_name = entry_function_name(&entry_function_payload);
        <a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data, &entry_function_name);
        <a href="account.md#0x1_account">account</a>
    } <b>else</b> {
        <b>abort</b>(<a href="ethereum_derivable_account.md#0x1_ethereum_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>)
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
