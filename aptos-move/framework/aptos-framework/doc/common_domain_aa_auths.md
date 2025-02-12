
<a id="0x1_common_domain_aa_auths"></a>

# Module `0x1::common_domain_aa_auths`



-  [Constants](#@Constants_0)
-  [Function `authenticate_ed25519_hex`](#0x1_common_domain_aa_auths_authenticate_ed25519_hex)
-  [Function `nibble_to_char`](#0x1_common_domain_aa_auths_nibble_to_char)
-  [Function `bytes_to_hex`](#0x1_common_domain_aa_auths_bytes_to_hex)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream">0x1::bcs_stream</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_common_domain_aa_auths_EINVALID_ACCOUNT_IDENTITY"></a>



<pre><code><b>const</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_ACCOUNT_IDENTITY">EINVALID_ACCOUNT_IDENTITY</a>: u64 = 1;
</code></pre>



<a id="0x1_common_domain_aa_auths_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 2;
</code></pre>



<a id="0x1_common_domain_aa_auths_authenticate_ed25519_hex"></a>

## Function `authenticate_ed25519_hex`



<pre><code><b>public</b> <b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_authenticate_ed25519_hex">authenticate_ed25519_hex</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_authenticate_ed25519_hex">authenticate_ed25519_hex</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);

    <b>let</b> hex_digest = <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_bytes_to_hex">bytes_to_hex</a>(aa_auth_data.digest());
    <b>let</b> stream = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_new">bcs_stream::new</a>(*aa_auth_data.authenticator());
    <b>let</b> public_key_bytes = <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x));

    <b>assert</b>!(
        aa_auth_data.account_identity() == &public_key_bytes,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_ACCOUNT_IDENTITY">EINVALID_ACCOUNT_IDENTITY</a>)
    );

    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(public_key_bytes);
    <b>let</b> signature = new_signature_from_bytes(
        <a href="../../aptos-stdlib/doc/bcs_stream.md#0x1_bcs_stream_deserialize_vector">bcs_stream::deserialize_vector</a>&lt;u8&gt;(&<b>mut</b> stream, |x| deserialize_u8(x))
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key,
            hex_digest,
        ),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );

    <a href="account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x1_common_domain_aa_auths_nibble_to_char"></a>

## Function `nibble_to_char`



<pre><code><b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_nibble_to_char">nibble_to_char</a>(nibble: u8): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_nibble_to_char">nibble_to_char</a>(nibble: u8): u8 {
    <b>if</b> (nibble &lt; 10) {
        48 + nibble  // '0' <b>to</b> '9'
    } <b>else</b> {
        87 + nibble  // 'a' <b>to</b> 'f' (87 = 'a' - 10)
    }
}
</code></pre>



</details>

<a id="0x1_common_domain_aa_auths_bytes_to_hex"></a>

## Function `bytes_to_hex`



<pre><code><b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_bytes_to_hex">bytes_to_hex</a>(data: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_bytes_to_hex">bytes_to_hex</a>(data: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> hex_chars = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();

    <b>let</b> i = 0;
    <b>while</b> (i &lt; data.length()) {
        <b>let</b> cur = *data.borrow(i);
        <b>let</b> high_nibble = cur / 16;
        <b>let</b> low_nibble = cur % 16;

        hex_chars.push_back(<a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_nibble_to_char">nibble_to_char</a>(high_nibble));
        hex_chars.push_back(<a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_nibble_to_char">nibble_to_char</a>(low_nibble));

        i = i + 1;
    };

    hex_chars
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
