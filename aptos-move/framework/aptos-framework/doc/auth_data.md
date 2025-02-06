
<a id="0x1_auth_data"></a>

# Module `0x1::auth_data`



-  [Enum `DomainAccount`](#0x1_auth_data_DomainAccount)
-  [Enum `AbstractionAuthData`](#0x1_auth_data_AbstractionAuthData)
-  [Function `digest`](#0x1_auth_data_digest)
-  [Function `authenticator`](#0x1_auth_data_authenticator)
-  [Function `is_domain`](#0x1_auth_data_is_domain)
-  [Function `domain_name`](#0x1_auth_data_domain_name)
-  [Function `account_authentication_key`](#0x1_auth_data_account_authentication_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
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
<code>domain_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>account_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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

<a id="0x1_auth_data_domain_name"></a>

## Function `domain_name`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_name">domain_name</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_domain_name">domain_name</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &String {
    &self.<a href="account.md#0x1_account">account</a>.domain_name
}
</code></pre>



</details>

<a id="0x1_auth_data_account_authentication_key"></a>

## Function `account_authentication_key`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_account_authentication_key">account_authentication_key</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_account_authentication_key">account_authentication_key</a>(self: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &self.<a href="account.md#0x1_account">account</a>.account_authentication_key
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
