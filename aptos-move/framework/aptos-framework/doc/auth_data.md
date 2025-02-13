
<a id="0x1_auth_data"></a>

# Module `0x1::auth_data`



-  [Enum `DomainAccount`](#0x1_auth_data_DomainAccount)
-  [Enum `AbstractionAuthData`](#0x1_auth_data_AbstractionAuthData)
-  [Constants](#@Constants_0)
-  [Function `digest`](#0x1_auth_data_digest)
-  [Function `authenticator`](#0x1_auth_data_authenticator)
-  [Function `is_domain`](#0x1_auth_data_is_domain)
-  [Function `domain_authenticator`](#0x1_auth_data_domain_authenticator)
-  [Function `domain_account_identity`](#0x1_auth_data_domain_account_identity)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
</code></pre>



<a id="0x1_auth_data_DomainAccount"></a>

## Enum `DomainAccount`



<pre><code>enum <a href="auth_data.md#0x1_auth_data_DomainAccount">DomainAccount</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_identity: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

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
<summary>DomainV1</summary>


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
<dt>
<code><a href="account.md#0x1_account">account</a>: <a href="auth_data.md#0x1_auth_data_DomainAccount">auth_data::DomainAccount</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_auth_data_ENOT_DOMAIN_AUTH_DATA"></a>



<pre><code><b>const</b> <a href="auth_data.md#0x1_auth_data_ENOT_DOMAIN_AUTH_DATA">ENOT_DOMAIN_AUTH_DATA</a>: u64 = 2;
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

<a id="0x1_auth_data_is_domain"></a>

## Function `is_domain`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_is_domain">is_domain</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_is_domain">is_domain</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): bool {
    self is DomainV1
}
</code></pre>



</details>

<a id="0x1_auth_data_domain_authenticator"></a>

## Function `domain_authenticator`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_authenticator">domain_authenticator</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_authenticator">domain_authenticator</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(self is DomainV1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="auth_data.md#0x1_auth_data_ENOT_REGULAR_AUTH_DATA">ENOT_REGULAR_AUTH_DATA</a>));
    &self.authenticator
}
</code></pre>



</details>

<a id="0x1_auth_data_domain_account_identity"></a>

## Function `domain_account_identity`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_account_identity">domain_account_identity</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_account_identity">domain_account_identity</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>assert</b>!(self is DomainV1, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="auth_data.md#0x1_auth_data_ENOT_DOMAIN_AUTH_DATA">ENOT_DOMAIN_AUTH_DATA</a>));
    &self.<a href="account.md#0x1_account">account</a>.account_identity
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
