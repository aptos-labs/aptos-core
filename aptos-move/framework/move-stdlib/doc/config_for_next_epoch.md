
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
-  [Function `lock_state`](#0x1_config_for_next_epoch_lock_state)
-  [Function `latest_lock_state`](#0x1_config_for_next_epoch_latest_lock_state)
-  [Function `update_lock_state`](#0x1_config_for_next_epoch_update_lock_state)
-  [Function `abort_unless_vm_or_std`](#0x1_config_for_next_epoch_abort_unless_vm_or_std)
-  [Function `abort_unless_std`](#0x1_config_for_next_epoch_abort_unless_std)
-  [Function `abort_unless_updates_enabled`](#0x1_config_for_next_epoch_abort_unless_updates_enabled)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a name="0x1_config_for_next_epoch_ForNextEpoch"></a>

## Resource `ForNextEpoch`

<code>0x1::ForNextEpoch&lt;T&gt;</code> holds the config payload for the next epoch, where <code>T</code> can be <code>ConsnsusConfig</code>, <code>Features</code>, etc.


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

<a name="0x1_config_for_next_epoch_UpdateLock"></a>

## Resource `UpdateLock`

We need to temporarily reject on-chain config changes during DKG.
<code>0x0::UpdateLock</code> or <code>0x1::UpdateLock</code>, whichever has the higher <code>seq_num</code>, represents whether we should reject.


<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>seq_num: u64</code>
</dt>
<dd>

</dd>
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



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>: u64 = 4;
</code></pre>



<a name="0x1_config_for_next_epoch_ESTD_OR_VM_SIGNER_NEEDED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_OR_VM_SIGNER_NEEDED">ESTD_OR_VM_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a name="0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>: u64 = 2;
</code></pre>



<a name="0x1_config_for_next_epoch_EVM_SIGNER_NEEDED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_EVM_SIGNER_NEEDED">EVM_SIGNER_NEEDED</a>: u64 = 3;
</code></pre>



<a name="0x1_config_for_next_epoch_updates_enabled"></a>

## Function `updates_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_updates_enabled">updates_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_updates_enabled">updates_enabled</a>(): bool <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    !<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_lock_state">latest_lock_state</a>().locked
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_disable_updates"></a>

## Function `disable_updates`

Disable on-chain config updates. Only needed when a reconfiguration with DKG starts.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_updates">disable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_updates">disable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_update_lock_state">update_lock_state</a>(account, <b>true</b>);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_enable_updates"></a>

## Function `enable_updates`

Enable on-chain config updates. Only needed when a reconfiguration with DKG finishes.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_updates">enable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_updates">enable_updates</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_update_lock_state">update_lock_state</a>(account, <b>false</b>);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_does_exist"></a>

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

<a name="0x1_config_for_next_epoch_upsert"></a>

## Function `upsert`

Save an on-chain config to be used in the next epoch.
Typically followed by a <code>aptos_framework::reconfigure::start_reconfigure_with_dkg()</code> to make it effective as soon as possible.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop, store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop + store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(std);
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_updates_enabled">abort_unless_updates_enabled</a>();
    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std)) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    };
    <b>move_to</b>(std, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> { payload: config });
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_extract"></a>

## Function `extract`

Extract the config payload. Should be called at the end of a reconfiguration with DKG.
It is assumed that the caller has checked existence using <code><a href="config_for_next_epoch.md#0x1_config_for_next_epoch_does_exist">does_exist</a>()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(account: &<a href="signer.md#0x1_signer">signer</a>): T <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(account);
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_updates_enabled">abort_unless_updates_enabled</a>();
    <b>let</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt; { payload } = <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    payload
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_lock_state"></a>

## Function `lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_lock_state">lock_state</a>(addr: <b>address</b>): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">config_for_next_epoch::UpdateLock</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_lock_state">lock_state</a>(addr: <b>address</b>): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(addr)) {
        *<b>borrow_global</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(addr)
    } <b>else</b> {
        <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
            seq_num: 0,
            locked: <b>false</b>,
        }
    }
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_latest_lock_state"></a>

## Function `latest_lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_lock_state">latest_lock_state</a>(): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">config_for_next_epoch::UpdateLock</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_lock_state">latest_lock_state</a>(): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <b>let</b> state_0 = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_lock_state">lock_state</a>(@vm);
    <b>let</b> state_1 = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_lock_state">lock_state</a>(@std);
    <b>if</b> (state_0.seq_num &gt; state_1.seq_num) {
        state_0
    } <b>else</b> {
        state_1
    }
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_update_lock_state"></a>

## Function `update_lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_update_lock_state">update_lock_state</a>(account: &<a href="signer.md#0x1_signer">signer</a>, locked: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_update_lock_state">update_lock_state</a>(account: &<a href="signer.md#0x1_signer">signer</a>, locked: bool) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(account);

    <b>let</b> latest_lock_state = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_lock_state">latest_lock_state</a>();

    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(address_of(account))) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a>&gt;(address_of(account));
    };

    <b>let</b> new_state = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
        seq_num: latest_lock_state.seq_num + 1,
        locked,
    };
    <b>move_to</b>(account, new_state);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_unless_vm_or_std"></a>

## Function `abort_unless_vm_or_std`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(std: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(std: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = std::signer::address_of(std);
    <b>assert</b>!(addr == @std || addr == @vm, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_OR_VM_SIGNER_NEEDED">ESTD_OR_VM_SIGNER_NEEDED</a>));
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_unless_std"></a>

## Function `abort_unless_std`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(std: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_std">abort_unless_std</a>(std: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = std::signer::address_of(std);
    <b>assert</b>!(addr == @std, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>));
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_unless_updates_enabled"></a>

## Function `abort_unless_updates_enabled`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_updates_enabled">abort_unless_updates_enabled</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_updates_enabled">abort_unless_updates_enabled</a>() <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpdateLock">UpdateLock</a> {
    <b>assert</b>!(!<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_lock_state">latest_lock_state</a>().locked, std::error::invalid_state(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
