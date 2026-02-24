
<a id="0x1_decryption"></a>

# Module `0x1::decryption`

This module provides a decryption key unique to every block. This resource
is updated in every block prologue. The decryption key is the key used to
decrypt the encrypted transactions in the block.


-  [Resource `PerBlockDecryptionKey`](#0x1_decryption_PerBlockDecryptionKey)
-  [Resource `PerEpochEncryptionKey`](#0x1_decryption_PerEpochEncryptionKey)
-  [Function `initialize`](#0x1_decryption_initialize)
-  [Function `on_new_block`](#0x1_decryption_on_new_block)
-  [Function `set_for_next_epoch`](#0x1_decryption_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_decryption_on_new_epoch)


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_decryption_PerBlockDecryptionKey"></a>

## Resource `PerBlockDecryptionKey`

Decryption key unique to every block.
This resource is updated in every block prologue.


<pre><code><b>struct</b> <a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>round: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_decryption_PerEpochEncryptionKey"></a>

## Resource `PerEpochEncryptionKey`

Encryption key derived from the DKG result, valid for one epoch.


<pre><code><b>struct</b> <a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>encryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_decryption_initialize"></a>

## Function `initialize`

Called during genesis initialization.


<pre><code><b>public</b> <b>fun</b> <a href="decryption.md#0x1_decryption_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="decryption.md#0x1_decryption_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(
            framework,
            <a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a> { epoch: 0, round: 0, decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        );
    };
    <b>if</b> (!<b>exists</b>&lt;<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(
            framework,
            <a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a> { epoch: 0, encryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
        );
    }
}
</code></pre>



</details>

<a id="0x1_decryption_on_new_block"></a>

## Function `on_new_block`

Invoked in block prologues to update the block decryption key.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch: u64, round: u64, decryption_key_for_new_block: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_on_new_block">on_new_block</a>(
    vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    epoch: u64,
    round: u64,
    decryption_key_for_new_block: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>if</b> (<b>exists</b>&lt;<a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a>&gt;(@aptos_framework)) {
        <b>let</b> decryption_key =
            <b>borrow_global_mut</b>&lt;<a href="decryption.md#0x1_decryption_PerBlockDecryptionKey">PerBlockDecryptionKey</a>&gt;(@aptos_framework);
        decryption_key.epoch = epoch;
        decryption_key.round = round;
        decryption_key.decryption_key = decryption_key_for_new_block;
    }
}
</code></pre>



</details>

<a id="0x1_decryption_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Buffer the encryption key for the next epoch.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_set_for_next_epoch">set_for_next_epoch</a>(epoch: u64, encryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_set_for_next_epoch">set_for_next_epoch</a>(epoch: u64, encryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a> {
        epoch,
        encryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(encryption_key)
    });
}
</code></pre>



</details>

<a id="0x1_decryption_on_new_epoch"></a>

## Function `on_new_epoch`

Apply buffered PerEpochEncryptionKey on epoch transition.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="decryption.md#0x1_decryption_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a>&gt;()) {
        <b>let</b> new_key = <a href="config_buffer.md#0x1_config_buffer_extract_v2">config_buffer::extract_v2</a>&lt;<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a>&gt;();
        <b>if</b> (<b>exists</b>&lt;<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="decryption.md#0x1_decryption_PerEpochEncryptionKey">PerEpochEncryptionKey</a>&gt;(@aptos_framework) = new_key;
        } <b>else</b> {
            <b>move_to</b>(framework, new_key);
        }
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
