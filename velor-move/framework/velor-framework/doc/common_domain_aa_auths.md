
<a id="0x1_common_domain_aa_auths"></a>

# Module `0x1::common_domain_aa_auths`



-  [Constants](#@Constants_0)
-  [Function `authenticate_ed25519_hex`](#0x1_common_domain_aa_auths_authenticate_ed25519_hex)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../velor-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../velor-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_common_domain_aa_auths_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 1;
</code></pre>



<a id="0x1_common_domain_aa_auths_authenticate_ed25519_hex"></a>

## Function `authenticate_ed25519_hex`



<pre><code><b>public</b> <b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_authenticate_ed25519_hex">authenticate_ed25519_hex</a>(<a href="account.md#0x1_account">account</a>: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_authenticate_ed25519_hex">authenticate_ed25519_hex</a>(<a href="account.md#0x1_account">account</a>: <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData): <a href="../../velor-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> hex_digest = <a href="../../velor-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest());

    <b>let</b> public_key = new_unvalidated_public_key_from_bytes(*aa_auth_data.domain_account_identity());
    <b>let</b> signature = new_signature_from_bytes(*aa_auth_data.domain_authenticator());
    <b>assert</b>!(
        <a href="../../velor-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature,
            &public_key,
            *hex_digest.bytes(),
        ),
        <a href="../../velor-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="common_domain_aa_auths.md#0x1_common_domain_aa_auths_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );

    <a href="account.md#0x1_account">account</a>
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
