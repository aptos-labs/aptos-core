
<a id="0x7_test_derivable_account_abstraction_ed25519_hex"></a>

# Module `0x7::test_derivable_account_abstraction_ed25519_hex`

Domain account abstraction using ed25519 hex for signing.

Authentication takes digest, converts to hex (prefixed with 0x, with lowercase letters),
and then expects that to be signed.
authenticator is expected to be signature: vector<u8>
account_identity is raw public_key.


-  [Constants](#@Constants_0)
-  [Function `authenticate`](#0x7_test_derivable_account_abstraction_ed25519_hex_authenticate)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x7_test_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="test_derivable_account_abstraction_ed25519_hex.md#0x7_test_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 1;
</code></pre>



<a id="0x7_test_derivable_account_abstraction_ed25519_hex_authenticate"></a>

## Function `authenticate`

Authorization function for domain account abstraction.


<pre><code><b>public</b> <b>fun</b> <a href="test_derivable_account_abstraction_ed25519_hex.md#0x7_test_derivable_account_abstraction_ed25519_hex_authenticate">authenticate</a>(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: <a href="../../aptos-framework/doc/auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="test_derivable_account_abstraction_ed25519_hex.md#0x7_test_derivable_account_abstraction_ed25519_hex_authenticate">authenticate</a>(
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aa_auth_data: AbstractionAuthData
): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> hex_digest = <a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils_to_string">string_utils::to_string</a>(aa_auth_data.digest());

    <b>let</b> public_key =
        new_unvalidated_public_key_from_bytes(
            *aa_auth_data.derivable_abstract_public_key()
        );
    <b>let</b> signature =
        new_signature_from_bytes(*aa_auth_data.derivable_abstract_signature());
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict">ed25519::signature_verify_strict</a>(
            &signature, &public_key, *hex_digest.bytes()
        ),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="test_derivable_account_abstraction_ed25519_hex.md#0x7_test_derivable_account_abstraction_ed25519_hex_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>)
    );

    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
