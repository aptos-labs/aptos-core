
<a id="0x1_auth_data"></a>

# Module `0x1::auth_data`



-  [Enum `AbstractionAuthData`](#0x1_auth_data_AbstractionAuthData)
-  [Constants](#@Constants_0)
-  [Function `digest`](#0x1_auth_data_digest)
-  [Function `authenticator`](#0x1_auth_data_authenticator)
-  [Function `is_derivable`](#0x1_auth_data_is_derivable)
-  [Function `derivable_abstract_signature`](#0x1_auth_data_derivable_abstract_signature)
-  [Function `derivable_abstract_public_key`](#0x1_auth_data_derivable_abstract_public_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
</code></pre>



<a id="0x1_auth_data_AbstractionAuthData"></a>

## Enum `AbstractionAuthData`



<pre><code>enum <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>digest: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>authenticator: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

<details>
<summary>DerivableV1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>digest: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>abstract_signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>abstract_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_auth_data_ENOT_DERIVABLE_AUTH_DATA"></a>



<pre><code><b>const</b> <a href="auth_data.md#0x1_auth_data_ENOT_DERIVABLE_AUTH_DATA">ENOT_DERIVABLE_AUTH_DATA</a>: u64 = 2;
</code></pre>



<a id="0x1_auth_data_ENOT_REGULAR_AUTH_DATA"></a>



<pre><code><b>const</b> <a href="auth_data.md#0x1_auth_data_ENOT_REGULAR_AUTH_DATA">ENOT_REGULAR_AUTH_DATA</a>: u64 = 1;
</code></pre>



<a id="0x1_auth_data_digest"></a>

## Function `digest`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_digest">digest</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_digest">digest</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &self.digest
}
</code></pre>



</details>

<a id="0x1_auth_data_authenticator"></a>

## Function `authenticator`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_authenticator">authenticator</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_authenticator">authenticator</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(self is V1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="auth_data.md#0x1_auth_data_ENOT_REGULAR_AUTH_DATA">ENOT_REGULAR_AUTH_DATA</a>));
    &self.authenticator
}
</code></pre>



</details>

<a id="0x1_auth_data_is_derivable"></a>

## Function `is_derivable`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_is_derivable">is_derivable</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_is_derivable">is_derivable</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): bool {
    self is DerivableV1
}
</code></pre>



</details>

<a id="0x1_auth_data_derivable_abstract_signature"></a>

## Function `derivable_abstract_signature`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_derivable_abstract_signature">derivable_abstract_signature</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_derivable_abstract_signature">derivable_abstract_signature</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(self is DerivableV1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="auth_data.md#0x1_auth_data_ENOT_REGULAR_AUTH_DATA">ENOT_REGULAR_AUTH_DATA</a>));
    &self.abstract_signature
}
</code></pre>



</details>

<a id="0x1_auth_data_derivable_abstract_public_key"></a>

## Function `derivable_abstract_public_key`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_derivable_abstract_public_key">derivable_abstract_public_key</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_derivable_abstract_public_key">derivable_abstract_public_key</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(self is DerivableV1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="auth_data.md#0x1_auth_data_ENOT_DERIVABLE_AUTH_DATA">ENOT_DERIVABLE_AUTH_DATA</a>));
    &self.abstract_public_key
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
