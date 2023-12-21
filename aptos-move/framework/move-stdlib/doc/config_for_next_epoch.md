
<a id="0x1_config_for_next_epoch"></a>

# Module `0x1::config_for_next_epoch`

This wrapper helps store an on-chain config for the next epoch.

Once reconfigure with DKG is introduced, every on-chain config <code>C</code> should do the following.
- Support async update when DKG is enabled. This is typically done by 3 steps below.
- Implement <code>C::set_for_next_epoch()</code> using <code><a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>()</code> function in this module.
- Implement <code>C::on_new_epoch()</code> using <code><a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>()</code> function in this module.
- Update <code>0x1::reconfiguration_with_dkg::finish()</code> to call <code>C::on_new_epoch()</code>.
- Support sychronous update when DKG is disabled.
This is typically done by implementing <code>C::set()</code> to update the config resource directly.

NOTE: on-chain config <code>0x1::state::ValidatorSet</code> implemented its own buffer.


-  [Resource `ForNextEpoch`](#0x1_config_for_next_epoch_ForNextEpoch)
-  [Resource `UpsertLocked`](#0x1_config_for_next_epoch_UpsertLocked)
-  [Constants](#@Constants_0)
-  [Function `upserts_enabled`](#0x1_config_for_next_epoch_upserts_enabled)
-  [Function `disable_upserts`](#0x1_config_for_next_epoch_disable_upserts)
-  [Function `enable_upserts`](#0x1_config_for_next_epoch_enable_upserts)
-  [Function `does_exist`](#0x1_config_for_next_epoch_does_exist)
-  [Function `upsert`](#0x1_config_for_next_epoch_upsert)
-  [Function `extract`](#0x1_config_for_next_epoch_extract)
-  [Function `abort_unless_std`](#0x1_config_for_next_epoch_abort_unless_std)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_config_for_next_epoch_ForNextEpoch"></a>

## Resource `ForNextEpoch`

<code><a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;</code> under account 0x1 holds the config payload for the next epoch, where <code>T</code> can be <code>ConsnsusConfig</code>, <code>Features</code>, etc.


<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt; <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>payload: T</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_config_for_next_epoch_UpsertLocked"></a>

## Resource `UpsertLocked`

This flag exists under account 0x1 if and only if any on-chain config change for the next epoch should be rejected.


<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_config_for_next_epoch_ERESOURCE_BUSY"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>: u64 = 2;
</code></pre>



<a id="0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_config_for_next_epoch_upserts_enabled"></a>

## Function `upserts_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upserts_enabled">upserts_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upserts_enabled">upserts_enabled</a>(): bool {
    !<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a>&gt;(@std)
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_disable_upserts"></a>

## Function `disable_upserts`

Disable on-chain config updates. Called by the system when a reconfiguration with DKG starts.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_upserts">disable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_upserts">disable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a>&gt;(@std)) {
        <b>move_to</b>(account, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a> {})
    }
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_enable_upserts"></a>

## Function `enable_upserts`

Enable on-chain config updates. Called by the system when a reconfiguration with DKG finishes.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_upserts">enable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_upserts">enable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account);
    <b>if</b> (!<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a>&gt;(@std)) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLocked">UpsertLocked</a>&gt;(address_of(account));
    }
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_does_exist"></a>

## Function `does_exist`

Check whether there is a pending config payload for <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_does_exist">does_exist</a>&lt;T: store&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_does_exist">does_exist</a>&lt;T: store&gt;(): bool {
    <b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std)
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_upsert"></a>

## Function `upsert`

Upsert an on-chain config to the buffer for the next epoch.

Typically used in <code>X::set_for_next_epoch()</code> where X is an on-chaon config.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop, store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>, config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop + store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>, config: T) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account);
    <b>assert</b>!(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upserts_enabled">upserts_enabled</a>(), std::error::invalid_state(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std)) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    };
    <b>move_to</b>(account, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> { payload: config });
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_extract"></a>

## Function `extract`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

Typically used in <code>X::on_new_epoch()</code> where X is an on-chaon config.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account);
    <b>let</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt; { payload } = <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    payload
}
</code></pre>



</details>

<a id="0x1_config_for_next_epoch_abort_unless_std"></a>

## Function `abort_unless_std`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(account: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = std::signer::address_of(account);
    <b>assert</b>!(addr == @std, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
