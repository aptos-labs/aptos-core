
<a id="0x1_sui_derivable_account"></a>

# Module `0x1::sui_derivable_account`

Derivable account abstraction that verifies a message signed by
Sui wallet.
1. The message format is as follows:

<domain> wants you to sign in with your Sui account:
<sui_account_address>

Please confirm you explicitly initiated this request from <domain>. You are approving to execute transaction <entry_function_name> on Aptos blockchain (<network_name>).

Nonce: <digest>

2. The abstract public key is a BCS serialized <code><a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">SuiAbstractPublicKey</a></code>.
3. The abstract signature is a BCS serialized <code><a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractSignature">SuiAbstractSignature</a></code>.
4. This module has been tested for the following wallets:
- Slush
- Phantom
- Nightly


-  [Enum `SuiAbstractSignature`](#0x1_sui_derivable_account_SuiAbstractSignature)
-  [Struct `SuiAbstractPublicKey`](#0x1_sui_derivable_account_SuiAbstractPublicKey)
-  [Enum `SuiSigningScheme`](#0x1_sui_derivable_account_SuiSigningScheme)
-  [Struct `IntentMessage`](#0x1_sui_derivable_account_IntentMessage)
-  [Struct `Intent`](#0x1_sui_derivable_account_Intent)
-  [Enum `IntentScope`](#0x1_sui_derivable_account_IntentScope)
-  [Enum `IntentVersion`](#0x1_sui_derivable_account_IntentVersion)
-  [Enum `AppId`](#0x1_sui_derivable_account_AppId)
-  [Constants](#@Constants_0)
-  [Function `construct_message`](#0x1_sui_derivable_account_construct_message)
-  [Function `get_signing_scheme`](#0x1_sui_derivable_account_get_signing_scheme)
-  [Function `deserialize_abstract_public_key`](#0x1_sui_derivable_account_deserialize_abstract_public_key)
-  [Function `deserialize_abstract_signature`](#0x1_sui_derivable_account_deserialize_abstract_signature)
-  [Function `split_signature_bytes`](#0x1_sui_derivable_account_split_signature_bytes)
-  [Function `derive_account_address_from_public_key`](#0x1_sui_derivable_account_derive_account_address_from_public_key)
-  [Function `authenticate_auth_data`](#0x1_sui_derivable_account_authenticate_auth_data)
-  [Function `authenticate`](#0x1_sui_derivable_account_authenticate)
-  [Specification](#@Specification_1)
    -  [Function `authenticate_auth_data`](#@Specification_1_authenticate_auth_data)
    -  [Function `authenticate`](#@Specification_1_authenticate)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="common_account_abstractions_utils.md#0x1_common_account_abstractions_utils">0x1::common_account_abstractions_utils</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_sui_derivable_account_SuiAbstractSignature"></a>

## Enum `SuiAbstractSignature`



<pre><code>enum <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractSignature">SuiAbstractSignature</a> <b>has</b> drop
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
 The signature of the message in raw bytes
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_sui_derivable_account_SuiAbstractPublicKey"></a>

## Struct `SuiAbstractPublicKey`

Sui abstract public key defined with the


<pre><code><b>struct</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">SuiAbstractPublicKey</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sui_account_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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

<a id="0x1_sui_derivable_account_SuiSigningScheme"></a>

## Enum `SuiSigningScheme`

Sui signing scheme as defined in
https://github.com/MystenLabs/ts-sdks/blob/main/packages/typescript/src/cryptography/signature-scheme.ts#L19


<pre><code>enum <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiSigningScheme">SuiSigningScheme</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>ED25519</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_sui_derivable_account_IntentMessage"></a>

## Struct `IntentMessage`

A wrapper struct that defines a message with its signing context (intent).
https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L168


<pre><code><b>struct</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentMessage">IntentMessage</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>intent: <a href="sui_derivable_account.md#0x1_sui_derivable_account_Intent">sui_derivable_account::Intent</a></code>
</dt>
<dd>

</dd>
<dt>
<code>value: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_sui_derivable_account_Intent"></a>

## Struct `Intent`

Metadata specifying the scope, version, and application domain of the message.
https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L86


<pre><code><b>struct</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_Intent">Intent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>scope: <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentScope">sui_derivable_account::IntentScope</a></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="version.md#0x1_version">version</a>: <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentVersion">sui_derivable_account::IntentVersion</a></code>
</dt>
<dd>

</dd>
<dt>
<code>app_id: <a href="sui_derivable_account.md#0x1_sui_derivable_account_AppId">sui_derivable_account::AppId</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_sui_derivable_account_IntentScope"></a>

## Enum `IntentScope`

https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L60


<pre><code>enum <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentScope">IntentScope</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>TransactionData</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>TransactionEffects</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>CheckpointSummary</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>PersonalMessage</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_sui_derivable_account_IntentVersion"></a>

## Enum `IntentVersion`

https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L18


<pre><code>enum <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentVersion">IntentVersion</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V0</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="0x1_sui_derivable_account_AppId"></a>

## Enum `AppId`

https://github.com/MystenLabs/sui/blob/main/crates/shared-crypto/src/intent.rs#L35


<pre><code>enum <a href="sui_derivable_account.md#0x1_sui_derivable_account_AppId">AppId</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>Sui</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_sui_derivable_account_EINVALID_PUBLIC_KEY"></a>

Invalid public key.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>: u64 = 6;
</code></pre>



<a id="0x1_sui_derivable_account_EINVALID_SIGNATURE"></a>

Invalid signature.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 5;
</code></pre>



<a id="0x1_sui_derivable_account_EINVALID_SIGNATURE_TYPE"></a>

Invalid signature type.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>: u64 = 2;
</code></pre>



<a id="0x1_sui_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD"></a>

Entry function payload is missing.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>: u64 = 1;
</code></pre>



<a id="0x1_sui_derivable_account_EACCOUNT_ADDRESS_MISMATCH"></a>

Account address mismatch.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EACCOUNT_ADDRESS_MISMATCH">EACCOUNT_ADDRESS_MISMATCH</a>: u64 = 7;
</code></pre>



<a id="0x1_sui_derivable_account_EINVALID_SIGNATURE_LENGTH"></a>

Invalid signature length.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE_LENGTH">EINVALID_SIGNATURE_LENGTH</a>: u64 = 4;
</code></pre>



<a id="0x1_sui_derivable_account_EINVALID_SIGNING_SCHEME_TYPE"></a>

Invalid signing scheme type.


<pre><code><b>const</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNING_SCHEME_TYPE">EINVALID_SIGNING_SCHEME_TYPE</a>: u64 = 3;
</code></pre>



<a id="0x1_sui_derivable_account_construct_message"></a>

## Function `construct_message`



<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_construct_message">construct_message</a>(sui_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_construct_message">construct_message</a>(
    sui_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    domain: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    digest_utf8: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> message = &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    message.append(*domain);
    message.append(b" wants you <b>to</b> sign in <b>with</b> your Sui <a href="account.md#0x1_account">account</a>:\n");
    message.append(*sui_public_key);
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

<a id="0x1_sui_derivable_account_get_signing_scheme"></a>

## Function `get_signing_scheme`

Returns the signing scheme for the given value.


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_get_signing_scheme">get_signing_scheme</a>(value: u8): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiSigningScheme">sui_derivable_account::SuiSigningScheme</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_get_signing_scheme">get_signing_scheme</a>(value: u8): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiSigningScheme">SuiSigningScheme</a> {
    <b>if</b> (value == 0) SuiSigningScheme::ED25519
    <b>else</b> <b>abort</b>(<a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNING_SCHEME_TYPE">EINVALID_SIGNING_SCHEME_TYPE</a>)
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_deserialize_abstract_public_key"></a>

## Function `deserialize_abstract_public_key`

Deserializes the abstract public key which is supposed to be a bcs
serialized <code><a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">SuiAbstractPublicKey</a></code>.


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">sui_derivable_account::SuiAbstractPublicKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(abstract_public_key: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">SuiAbstractPublicKey</a> {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_public_key);
    <b>let</b> sui_account_address = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    <b>let</b> domain = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
    <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractPublicKey">SuiAbstractPublicKey</a> { sui_account_address, domain }
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_deserialize_abstract_signature"></a>

## Function `deserialize_abstract_signature`

Returns a tuple of the signature.


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractSignature">sui_derivable_account::SuiAbstractSignature</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(abstract_signature: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sui_derivable_account.md#0x1_sui_derivable_account_SuiAbstractSignature">SuiAbstractSignature</a> {
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*abstract_signature);
    <b>let</b> signature_type = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_u8">bcs_stream::deserialize_u8</a>(&<b>mut</b> stream);
    <b>if</b> (signature_type == 0x00) {
        <b>let</b> signature = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));
        SuiAbstractSignature::MessageV1 { signature }
    } <b>else</b> {
        <b>abort</b>(<a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE_TYPE">EINVALID_SIGNATURE_TYPE</a>)
    }
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_split_signature_bytes"></a>

## Function `split_signature_bytes`

Splits raw signature bytes containing <code>scheme flag (1 byte), signature (64 bytes) and <b>public</b> key (32 bytes)</code>
to a tuple of (signing_scheme, signature, public_key)


<pre><code><b>public</b> <b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_split_signature_bytes">split_signature_bytes</a>(bytes: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (u8, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_split_signature_bytes">split_signature_bytes</a>(bytes: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (u8, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    // 1 + 64 + 32 = 97 bytes
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(bytes) == 97, <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE_LENGTH">EINVALID_SIGNATURE_LENGTH</a>);

    <b>let</b> signing_scheme = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bytes, 0);
    <b>let</b> abstract_signature_signature = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>let</b> abstract_signature_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();

    // Extract signature (64 bytes)
    <b>let</b> i = 1;
    <b>while</b> (i &lt; 65) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> abstract_signature_signature, *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bytes, i));
        i = i + 1;
    };

    // Extract <b>public</b> key (32 bytes)
    <b>while</b> (i &lt; 97) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> abstract_signature_public_key, *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(bytes, i));
        i = i + 1;
    };

    (signing_scheme, abstract_signature_signature, abstract_signature_public_key)
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_derive_account_address_from_public_key"></a>

## Function `derive_account_address_from_public_key`

Derives the account address from the public key and returns it is a hex string with "0x" prefix


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_derive_account_address_from_public_key">derive_account_address_from_public_key</a>(signing_scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_derive_account_address_from_public_key">derive_account_address_from_public_key</a>(signing_scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    // Create a <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> <b>with</b> signing scheme and <b>public</b> key bytes
    <b>let</b> data_to_hash = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_singleton">vector::singleton</a>(signing_scheme);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> data_to_hash, public_key_bytes);

    // Compute blake2b <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>
    <b>let</b> sui_account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash_blake2b_256">aptos_hash::blake2b_256</a>(data_to_hash);

    // Convert the <b>address</b> bytes <b>to</b> a hex <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">string</a> <b>with</b> "0x" prefix
    <b>let</b> sui_account_address_hex = b"0x";
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&sui_account_address)) {
        <b>let</b> byte = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&sui_account_address, i);
        // Convert each byte <b>to</b> two hex characters
        <b>let</b> hex_chars = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
            <b>if</b> ((byte &gt;&gt; 4) &lt; 10) ((byte &gt;&gt; 4) + 0x30) <b>else</b> ((byte &gt;&gt; 4) - 10 + 0x61),
            <b>if</b> ((byte & 0xf) &lt; 10) ((byte & 0xf) + 0x30) <b>else</b> ((byte & 0xf) - 10 + 0x61)
        ];
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> sui_account_address_hex, hex_chars);
        i = i + 1;
    };

    // Return the <a href="account.md#0x1_account">account</a> <b>address</b> <b>as</b> hex <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">string</a>
    sui_account_address_hex
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_authenticate_auth_data"></a>

## Function `authenticate_auth_data`



<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(
    aa_auth_data: AbstractionAuthData,
    entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    <b>let</b> abstract_signature = <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_signature">deserialize_abstract_signature</a>(aa_auth_data.derivable_abstract_signature());
    <b>let</b> (signing_scheme, abstract_signature_signature, abstract_signature_public_key) = <a href="sui_derivable_account.md#0x1_sui_derivable_account_split_signature_bytes">split_signature_bytes</a>(&abstract_signature.signature);

    // Check siging scheme is ED25519 <b>as</b> we currently only support this scheme
    <b>assert</b>!(<a href="sui_derivable_account.md#0x1_sui_derivable_account_get_signing_scheme">get_signing_scheme</a>(signing_scheme) == SuiSigningScheme::ED25519, <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNING_SCHEME_TYPE">EINVALID_SIGNING_SCHEME_TYPE</a>);

    // Derive the <a href="account.md#0x1_account">account</a> <b>address</b> from the <b>public</b> key
    <b>let</b> sui_account_address = <a href="sui_derivable_account.md#0x1_sui_derivable_account_derive_account_address_from_public_key">derive_account_address_from_public_key</a>(signing_scheme, abstract_signature_public_key);

    <b>let</b> derivable_abstract_public_key = aa_auth_data.derivable_abstract_public_key();
    <b>let</b> abstract_public_key = <a href="sui_derivable_account.md#0x1_sui_derivable_account_deserialize_abstract_public_key">deserialize_abstract_public_key</a>(derivable_abstract_public_key);

    // Check the <a href="account.md#0x1_account">account</a> <b>address</b> matches the abstract <b>public</b> key
    <b>assert</b>!(&sui_account_address == &abstract_public_key.sui_account_address, <a href="sui_derivable_account.md#0x1_sui_derivable_account_EACCOUNT_ADDRESS_MISMATCH">EACCOUNT_ADDRESS_MISMATCH</a>);

    <b>let</b> public_key = new_validated_public_key_from_bytes(abstract_signature_public_key);
    <b>assert</b>!(public_key.is_some(), <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_PUBLIC_KEY">EINVALID_PUBLIC_KEY</a>);

    <b>let</b> digest_utf8 = <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest()).bytes();

    // Build the raw message
    <b>let</b> raw_message = <a href="sui_derivable_account.md#0x1_sui_derivable_account_construct_message">construct_message</a>(&sui_account_address, &abstract_public_key.domain, entry_function_name, digest_utf8);

    // Prepend <a href="sui_derivable_account.md#0x1_sui_derivable_account_Intent">Intent</a> <b>to</b> the message
    <b>let</b> intent = <a href="sui_derivable_account.md#0x1_sui_derivable_account_Intent">Intent</a> {
        scope: PersonalMessage,
        <a href="version.md#0x1_version">version</a>: V0,
        app_id: Sui,
    };
    <b>let</b> msg = <a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentMessage">IntentMessage</a> {
        intent,
        value: raw_message,
    };
    // Serialize the whole <b>struct</b>
    <b>let</b> bcs_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>&lt;<a href="sui_derivable_account.md#0x1_sui_derivable_account_IntentMessage">IntentMessage</a>&gt;(&msg);

    // Hash full_message <b>with</b> blake2b256
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash_blake2b_256">aptos_hash::blake2b_256</a>(bcs_bytes);

    <b>let</b> signature = new_signature_from_bytes(abstract_signature_signature);

    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key_into_unvalidated(public_key.destroy_some()),
            <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a>,
        ),
        <a href="sui_derivable_account.md#0x1_sui_derivable_account_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>
    );
}
</code></pre>



</details>

<a id="0x1_sui_derivable_account_authenticate"></a>

## Function `authenticate`

Authorization function for domain account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> maybe_entry_function_payload = <a href="transaction_context.md#0x1_transaction_context_entry_function_payload">transaction_context::entry_function_payload</a>();
    <b>if</b> (maybe_entry_function_payload.is_some()) {
        <b>let</b> entry_function_payload = maybe_entry_function_payload.destroy_some();
        <b>let</b> entry_function_name = entry_function_name(&entry_function_payload);
        <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data, &entry_function_name);
        <a href="account.md#0x1_account">account</a>
    } <b>else</b> {
        <b>abort</b>(<a href="sui_derivable_account.md#0x1_sui_derivable_account_EMISSING_ENTRY_FUNCTION_PAYLOAD">EMISSING_ENTRY_FUNCTION_PAYLOAD</a>)
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_authenticate_auth_data"></a>

### Function `authenticate_auth_data`


<pre><code><b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate_auth_data">authenticate_auth_data</a>(aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, entry_function_name: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_authenticate"></a>

### Function `authenticate`


<pre><code><b>public</b> <b>fun</b> <a href="sui_derivable_account.md#0x1_sui_derivable_account_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
