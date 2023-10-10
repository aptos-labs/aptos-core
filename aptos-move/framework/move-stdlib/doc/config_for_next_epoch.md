
<a name="0x1_config_for_next_epoch"></a>

# Module `0x1::config_for_next_epoch`

This wrapper helps store an on-chain config for the next epoch.


-  [Resource `ForNextEpoch`](#0x1_config_for_next_epoch_ForNextEpoch)
-  [Resource `UpdateLock`](#0x1_config_for_next_epoch_UpdateLock)
-  [Constants](#@Constants_0)
-  [Function `updates_enabled`](#0x1_config_for_next_epoch_updates_enabled)
-  [Function `disable_updates`](#0x1_config_for_next_epoch_disable_updates)
-  [Function `enable_updates`](#0x1_config_for_next_epoch_enable_updates)
-  [Function `does_exist`](#0x1_config_for_next_epoch_does_exist)
-  [Function `upsert`](#0x1_config_for_next_epoch_upsert)
-  [Function `extract`](#0x1_config_for_next_epoch_extract)
-  [Function `abort_unless_system_account`](#0x1_config_for_next_epoch_abort_unless_system_account)
-  [Function `abort_if_updates_disabled`](#0x1_config_for_next_epoch_abort_if_updates_disabled)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a name="0x1_config_for_next_epoch_ForNextEpoch"></a>

## Resource `ForNextEpoch`



<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt; <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payload: <a href="option.md#0x1_option_Option">option::Option</a>&lt;T&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_config_for_next_epoch_UpdateLock"></a>

## Resource `UpdateLock`



<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>locked: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_config_for_next_epoch_ERESOURCE_BUSY"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>: u64 = 2;
</code></pre>



<a name="0x1_config_for_next_epoch_ESYSTEM_SIGNER_NEEDED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESYSTEM_SIGNER_NEEDED">ESYSTEM_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a name="0x1_config_for_next_epoch_updates_enabled"></a>

## Function `updates_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_updates_enabled">updates_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_updates_enabled">updates_enabled</a>(): bool <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <b>borrow_global</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(@std).locked
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_disable_updates"></a>

## Function `disable_updates`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_updates">disable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_updates">disable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(account);
    <b>borrow_global_mut</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(@std).locked = <b>true</b>;
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_enable_updates"></a>

## Function `enable_updates`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_updates">enable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_updates">enable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(account);
    <b>borrow_global_mut</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(@std).locked = <b>false</b>;
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_does_exist"></a>

## Function `does_exist`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_does_exist">does_exist</a>&lt;T: store&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_does_exist">does_exist</a>&lt;T: store&gt;(): bool <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std) && <a href="option.md#0x1_option_is_some">option::is_some</a>(&<b>borrow_global</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std).payload)
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_upsert"></a>

## Function `upsert`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop, store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop + store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(std);
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_if_updates_disabled">abort_if_updates_disabled</a>();
    <b>borrow_global_mut</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std).payload = <a href="option.md#0x1_option_some">option::some</a>(config);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_extract"></a>

## Function `extract`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(account);
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_if_updates_disabled">abort_if_updates_disabled</a>();
    <a href="option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std).payload)
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_unless_system_account"></a>

## Function `abort_unless_system_account`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(std: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_system_account">abort_unless_system_account</a>(std: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = std::signer::address_of(std);
    <b>assert</b>!(addr == @std || addr == @vm, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESYSTEM_SIGNER_NEEDED">ESYSTEM_SIGNER_NEEDED</a>));
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_if_updates_disabled"></a>

## Function `abort_if_updates_disabled`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_if_updates_disabled">abort_if_updates_disabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_if_updates_disabled">abort_if_updates_disabled</a>() {
    <b>assert</b>!(!<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(@std), std::error::invalid_state(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
