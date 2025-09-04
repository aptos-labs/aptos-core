
<a id="0x1_signing_data"></a>

# Module `0x1::signing_data`



-  [Enum `SigningData`](#0x1_signing_data_SigningData)
-  [Function `digest`](#0x1_signing_data_digest)
-  [Function `authenticator`](#0x1_signing_data_authenticator)


<pre><code></code></pre>



<a id="0x1_signing_data_SigningData"></a>

## Enum `SigningData`



<pre><code>enum <a href="signing_data.md#0x1_signing_data_SigningData">SigningData</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>digest: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>authenticator: <a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_signing_data_digest"></a>

## Function `digest`



<pre><code><b>public</b> <b>fun</b> <a href="signing_data.md#0x1_signing_data_digest">digest</a>(<a href="signing_data.md#0x1_signing_data">signing_data</a>: &<a href="signing_data.md#0x1_signing_data_SigningData">signing_data::SigningData</a>): &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signing_data.md#0x1_signing_data_digest">digest</a>(<a href="signing_data.md#0x1_signing_data">signing_data</a>: &<a href="signing_data.md#0x1_signing_data_SigningData">SigningData</a>): &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &<a href="signing_data.md#0x1_signing_data">signing_data</a>.digest
}
</code></pre>



</details>

<a id="0x1_signing_data_authenticator"></a>

## Function `authenticator`



<pre><code><b>public</b> <b>fun</b> <a href="signing_data.md#0x1_signing_data_authenticator">authenticator</a>(<a href="signing_data.md#0x1_signing_data">signing_data</a>: &<a href="signing_data.md#0x1_signing_data_SigningData">signing_data::SigningData</a>): &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="signing_data.md#0x1_signing_data_authenticator">authenticator</a>(<a href="signing_data.md#0x1_signing_data">signing_data</a>: &<a href="signing_data.md#0x1_signing_data_SigningData">SigningData</a>): &<a href="../../velor-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    &<a href="signing_data.md#0x1_signing_data">signing_data</a>.authenticator
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
