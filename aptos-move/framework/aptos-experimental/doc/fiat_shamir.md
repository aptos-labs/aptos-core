
<a id="0x7_fiat_shamir"></a>

# Module `0x7::fiat_shamir`



-  [Struct `DomainSeparator`](#0x7_fiat_shamir_DomainSeparator)
-  [Struct `FiatShamirInputs`](#0x7_fiat_shamir_FiatShamirInputs)
-  [Constants](#@Constants_0)
-  [Function `new_domain_separator`](#0x7_fiat_shamir_new_domain_separator)
-  [Function `fiat_shamir`](#0x7_fiat_shamir_fiat_shamir)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="public_statement.md#0x7_public_statement">0x7::public_statement</a>;
</code></pre>



<a id="0x7_fiat_shamir_DomainSeparator"></a>

## Struct `DomainSeparator`

A domain separator prevents replay attacks in $\Sigma$ protocols and consists of 3 things.

1. A protocol identifier, which is typically split up into two things:
- A higher-level protocol: "Confidential Assets v1 on Aptos"
- A lower-level relation identifier (e.g., "PedEq", "Schnorr", "DLEQ", etc.)

2. Statement (i.e., the public statement in the NP relation being proved)
- This is captured implicitly in our <code>prove</code> and <code>verify</code> functions ==> it is not part of this struct.

3. Session identifier
- Chosen by user
- specifies the "context" in which this proof is valid
- e.g., "Alice (0x1) is paying Bob (0x2) at time <code>t</code>
- together with the protocol identifier, prevents replay attacks across the same protocol or different protocols

Note: The session identifier can be tricky, since in some settings the "session" accumulates implicitly in the
statement being proven. For confidential assets, it does not AFAICT: the "session" is represented at least by
the confidential balances of the users & their addresses.


<pre><code><b>struct</b> <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">DomainSeparator</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
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

<a id="0x7_fiat_shamir_FiatShamirInputs"></a>

## Struct `FiatShamirInputs`

Unfortunately, we cannot directly use the <code>PublicStatement</code> struct here because its <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;</code>
will not serialize correctly via <code><a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a></code>, since a <code>RistrettoPoint</code> stores a Move VM "handle" rather than
an actual point.


<pre><code><b>struct</b> <a href="fiat_shamir.md#0x7_fiat_shamir_FiatShamirInputs">FiatShamirInputs</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dst: <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">fiat_shamir::DomainSeparator</a></code>
</dt>
<dd>

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
<code>A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_fiat_shamir_E_INTERNAL_INVARIANT_FAILED"></a>

One of our internal invariants was broken. There is likely a logical error in the code.


<pre><code><b>const</b> <a href="fiat_shamir.md#0x7_fiat_shamir_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>: u64 = 2;
</code></pre>



<a id="0x7_fiat_shamir_E_PROOF_COMMITMENT_EMPTY"></a>

The length of the <code>A</code> field in <code>Proof</code> should NOT be zero


<pre><code><b>const</b> <a href="fiat_shamir.md#0x7_fiat_shamir_E_PROOF_COMMITMENT_EMPTY">E_PROOF_COMMITMENT_EMPTY</a>: u64 = 1;
</code></pre>



<a id="0x7_fiat_shamir_new_domain_separator"></a>

## Function `new_domain_separator`



<pre><code><b>public</b> <b>fun</b> <a href="fiat_shamir.md#0x7_fiat_shamir_new_domain_separator">new_domain_separator</a>(protocol_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">fiat_shamir::DomainSeparator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fiat_shamir.md#0x7_fiat_shamir_new_domain_separator">new_domain_separator</a>(protocol_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, session_id: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">DomainSeparator</a> {
    <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">DomainSeparator</a> {
        protocol_id,
        session_id
    }
}
</code></pre>



</details>

<a id="0x7_fiat_shamir_fiat_shamir"></a>

## Function `fiat_shamir`

Returns the Sigma protocol challenge $e$ and $1,\beta,\beta^2,\ldots, \beta^{m-1}$


<pre><code><b>public</b> <b>fun</b> <a href="fiat_shamir.md#0x7_fiat_shamir">fiat_shamir</a>(dst: <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">fiat_shamir::DomainSeparator</a>, stmt: &<a href="public_statement.md#0x7_public_statement_PublicStatement">public_statement::PublicStatement</a>, _A: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;, k: u64): (<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_Scalar">ristretto255::Scalar</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="fiat_shamir.md#0x7_fiat_shamir">fiat_shamir</a>(
    dst: <a href="fiat_shamir.md#0x7_fiat_shamir_DomainSeparator">DomainSeparator</a>,
    stmt: &PublicStatement,
    _A: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;RistrettoPoint&gt;,
    k: u64): (Scalar, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;Scalar&gt;)
{
    <b>let</b> m = _A.length();
    <b>assert</b>!(m != 0, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="fiat_shamir.md#0x7_fiat_shamir_E_PROOF_COMMITMENT_EMPTY">E_PROOF_COMMITMENT_EMPTY</a>));

    // We will <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> an application-specific domain separator and the (full) <b>public</b> statement,
    // which will <b>include</b> <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> <b>public</b> parameters like group generators $G$, $H$.

    // Note: A hardcodes $m$, the statement hardcodes $n_1$ and $n_2$, and $k$ is specified manually!
    <b>let</b> bytes = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&<a href="fiat_shamir.md#0x7_fiat_shamir_FiatShamirInputs">FiatShamirInputs</a> {
        dst,
        k,
        stmt_X: compress_points(stmt.get_points()),
        stmt_x: *stmt.get_scalars(),
        A: compress_points(_A)
    });

    // TODO(Security): A bit ad-hoc.
    <b>let</b> e_hash = sha2_512(bytes);
    <b>let</b> beta_hash = sha2_512(e_hash);

    <b>let</b> e = new_scalar_uniform_from_64_bytes(e_hash).extract();
    <b>let</b> beta = new_scalar_uniform_from_64_bytes(beta_hash).extract();

    <b>let</b> betas = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> prev_beta = scalar_one();
    betas.push_back(prev_beta);
    <b>let</b> i = 1;
    <b>while</b> (i &lt; m) {
        <b>let</b> new_beta = scalar_mul(&prev_beta, &beta);

        // \beta^i &lt;- \beta^{i-1} * \beta
        betas.push_back(new_beta);

        prev_beta = new_beta;
        i += 1;
    };

    // This will only fail when our logic above for generating the `\beta_i`'s is broken
    <b>assert</b>!(betas.length() == m, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="fiat_shamir.md#0x7_fiat_shamir_E_INTERNAL_INVARIANT_FAILED">E_INTERNAL_INVARIANT_FAILED</a>));

    (e, betas)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
