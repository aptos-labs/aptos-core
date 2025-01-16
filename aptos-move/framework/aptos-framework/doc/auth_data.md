
<a id="0x1_auth_data"></a>

# Module `0x1::auth_data`



-  [Enum `AbstractionAuthData`](#0x1_auth_data_AbstractionAuthData)
-  [Function `digest`](#0x1_auth_data_digest)
-  [Function `authenticator`](#0x1_auth_data_authenticator)


<pre><code></code></pre>



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

</details>

<a id="0x1_auth_data_digest"></a>

## Function `digest`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_digest">digest</a>(signing_data: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_digest">digest</a>(signing_data: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &signing_data.digest
}
</code></pre>



</details>

<a id="0x1_auth_data_authenticator"></a>

## Function `authenticator`



<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_authenticator">authenticator</a>(signing_data: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="auth_data.md#0x1_auth_data_authenticator">authenticator</a>(signing_data: &<a href="auth_data.md#0x1_auth_data_AbstractionAuthData">AbstractionAuthData</a>): &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &signing_data.authenticator
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
