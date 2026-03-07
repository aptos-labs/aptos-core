
<a id="0x7_sigma_protocol_fiat_shamir"></a>

# Module `0x7::sigma_protocol_fiat_shamir`



-  [Enum `DomainSeparator`](#0x7_sigma_protocol_fiat_shamir_DomainSeparator)
-  [Struct `FiatShamirInputs`](#0x7_sigma_protocol_fiat_shamir_FiatShamirInputs)
-  [Constants](#@Constants_0)
-  [Function `new_domain_separator`](#0x7_sigma_protocol_fiat_shamir_new_domain_separator)
-  [Function `fiat_shamir`](#0x7_sigma_protocol_fiat_shamir_fiat_shamir)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
</code></pre>



<a id="0x7_sigma_protocol_fiat_shamir_DomainSeparator"></a>

## Enum `DomainSeparator`

A domain separator prevents replay attacks in $\Sigma$ protocols and consists of 5 things.

1. The contract address (defense in depth: binds the proof to a specific deployed contract)

2. The chain ID (defense in depth: binds the proof to a specific Aptos network)

3. A protocol identifier, which is typically split up into two things:
- A higher-level protocol: "Confidential Assets v1 on Aptos"
- A lower-level relation identifier (e.g., "PedEq", "Schnorr", "DLEQ", etc.)

4. Statement (i.e., the public statement in the NP relation being proved)
- This is captured implicitly in our <code>prove</code> and <code>verify</code> functions ==> it is not part of this struct.

5. Session identifier
- Chosen by user
- specifies the "context" in which this proof is valid
- e.g., "Alice (0x1) is paying Bob (0x2) at time <code>t</code>
- together with the protocol identifier, prevents replay attacks across the same protocol or different protocols

Note: The session identifier can be tricky, since in some settings the "session" accumulates implicitly in the
statement being proven. For confidential assets, it does not AFAICT: the "session" is represented at least by
the confidential balances of the users & their addresses.

TODO(Security): We may want to add more here (like some sort of account TXN counter). I suspect that the
ciphertext randomness in the public statement would act as enough of a "session ID", but I would prefer
to avoid reasoning about that.


<pre><code>enum <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">DomainSeparator</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>contract_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">chain_id</a>: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>protocol_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_sigma_protocol_fiat_shamir_FiatShamirInputs"></a>

## Struct `FiatShamirInputs`

Unfortunately, we cannot directly use the <code>Statement</code> struct here because its <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;</code>
will not serialize correctly via <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a></code>, since a <code>RistrettoPoint</code> stores a Move VM "handle" rather than
an actual point.


<pre><code><b>struct</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_FiatShamirInputs">FiatShamirInputs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dst: <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">sigma_protocol_fiat_shamir::DomainSeparator</a></code>
</dt>
<dd>

</dd>
<dt>
<code>type_name: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The fully-qualified type name of the phantom marker type <code>P</code> in <code>Statement&lt;P&gt;</code>.
 E.g., <code>"<a href="sigma_protocol_registration.md#0x7_sigma_protocol_registration_Registration">0x7::sigma_protocol_registration::Registration</a>"</code>.
 This binds the Fiat-Shamir challenge to the specific protocol type for defense in depth.
</dd>
<dt>
<code>k: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>stmt_X: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>stmt_x: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proof_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_sigma_protocol_fiat_shamir_E_INTERNAL_INVARIANT_FAILED"></a>

One of our internal invariants was broken. There is likely a logical error in the code.


<pre><code><b>const</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_sigma_protocol_fiat_shamir_E_PROOF_COMMITMENT_EMPTY"></a>

The length of the <code>A</code> field in <code>Proof</code> should NOT be zero


<pre><code><b>const</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_E_PROOF_COMMITMENT_EMPTY">E_PROOF_COMMITMENT_EMPTY</a>: u64 = 1;
</code></pre>



<a id="0x7_sigma_protocol_fiat_shamir_new_domain_separator"></a>

## Function `new_domain_separator`



<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_new_domain_separator">new_domain_separator</a>(contract_address: <b>address</b>, <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">chain_id</a>: u8, protocol_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">sigma_protocol_fiat_shamir::DomainSeparator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_new_domain_separator">new_domain_separator</a>(contract_address: <b>address</b>, <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">chain_id</a>: u8, protocol_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">DomainSeparator</a> {
    DomainSeparator::V1 {
        contract_address,
        <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">chain_id</a>,
        protocol_id,
        session_id
    }
}
</code></pre>



</details>

<a id="0x7_sigma_protocol_fiat_shamir_fiat_shamir"></a>

## Function `fiat_shamir`

Returns the Sigma protocol challenge $e$ and $1,\beta,\beta^2,\ldots, \beta^{m-1}$


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_fiat_shamir">fiat_shamir</a>&lt;P&gt;(dst: <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">sigma_protocol_fiat_shamir::DomainSeparator</a>, stmt: &<a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement_Statement">sigma_protocol_statement::Statement</a>&lt;P&gt;, compressed_A: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, k: u64): (<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_fiat_shamir">fiat_shamir</a>&lt;P&gt;(
    dst: <a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_DomainSeparator">DomainSeparator</a>,
    stmt: &Statement&lt;P&gt;,
    compressed_A: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    k: u64): (Scalar, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;)
{
    <b>let</b> m = compressed_A.length();
    <b>assert</b>!(m != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_E_PROOF_COMMITMENT_EMPTY">E_PROOF_COMMITMENT_EMPTY</a>));

    // We will <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> an application-specific domain separator and the (full) <b>public</b> statement,
    // which will <b>include</b> <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> <b>public</b> parameters like group generators $G$, $H$.

    // Note: A hardcodes $m$, the statement hardcodes $n_1$ and $n_2$, and $k$ is specified manually!
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_FiatShamirInputs">FiatShamirInputs</a> {
        dst,
        type_name: <a href="../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;P&gt;(),
        k,
        stmt_X: *stmt.get_compressed_points(),
        stmt_x: *stmt.get_scalars(),
        proof_A: *compressed_A
    });

    // TODO(Security): A bit ad-hoc.
    <b>let</b> seed = sha2_512(bytes);

    // e = SHA2-512(SHA2-512(bytes) || 0x00)
    seed.push_back(0u8);
    <b>let</b> e_hash = sha2_512(seed);

    // beta = SHA2-512(SHA2-512(bytes) || 0x01)
    <b>let</b> len = seed.length();
    seed[len - 1] = 1u8;
    <b>let</b> beta_hash = sha2_512(seed);

    <b>let</b> e = new_scalar_uniform_from_64_bytes(e_hash).extract();
    <b>let</b> beta = new_scalar_uniform_from_64_bytes(beta_hash).extract();

    <b>let</b> betas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> prev_beta = scalar_one();
    betas.push_back(prev_beta);
    for (_i in 1..m) {
        prev_beta = scalar_mul(&prev_beta, &beta);
        betas.push_back(prev_beta);
    };

    // This will only fail when our logic above for generating the `\beta_i`'s is broken
    <b>assert</b>!(betas.length() == m, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="sigma_protocol_fiat_shamir.md#0x7_sigma_protocol_fiat_shamir_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    (e, betas)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
