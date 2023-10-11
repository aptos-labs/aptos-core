
<a name="0x1_config_for_next_epoch"></a>

# Module `0x1::config_for_next_epoch`

This wrapper helps store an on-chain config for the next epoch.


-  [Resource `ForNextEpoch`](#0x1_config_for_next_epoch_ForNextEpoch)
-  [Resource `UpsertLock`](#0x1_config_for_next_epoch_UpsertLock)
-  [Resource `ExtractPermit`](#0x1_config_for_next_epoch_ExtractPermit)
-  [Constants](#@Constants_0)
-  [Function `extracts_enabled`](#0x1_config_for_next_epoch_extracts_enabled)
-  [Function `enable_extracts`](#0x1_config_for_next_epoch_enable_extracts)
-  [Function `disable_extracts`](#0x1_config_for_next_epoch_disable_extracts)
-  [Function `upserts_enabled`](#0x1_config_for_next_epoch_upserts_enabled)
-  [Function `disable_upserts`](#0x1_config_for_next_epoch_disable_upserts)
-  [Function `enable_upserts`](#0x1_config_for_next_epoch_enable_upserts)
-  [Function `does_exist`](#0x1_config_for_next_epoch_does_exist)
-  [Function `copied`](#0x1_config_for_next_epoch_copied)
-  [Function `upsert`](#0x1_config_for_next_epoch_upsert)
-  [Function `extract`](#0x1_config_for_next_epoch_extract)
-  [Function `upsert_lock_state`](#0x1_config_for_next_epoch_upsert_lock_state)
-  [Function `latest_upsert_lock_state`](#0x1_config_for_next_epoch_latest_upsert_lock_state)
-  [Function `set_upsert_lock_state`](#0x1_config_for_next_epoch_set_upsert_lock_state)
-  [Function `abort_unless_vm_or_std`](#0x1_config_for_next_epoch_abort_unless_vm_or_std)


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

<a name="0x1_config_for_next_epoch_UpsertLock"></a>

## Resource `UpsertLock`

We need to temporarily reject on-chain config changes during DKG.
<code>0x0::UpdateLock</code> or <code>0x1::UpdateLock</code>, whichever has the higher <code>seq_num</code>, represents whether we should reject.


<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> <b>has</b> <b>copy</b>, drop, key
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

<a name="0x1_config_for_next_epoch_ExtractPermit"></a>

## Resource `ExtractPermit`

We need to allow extraction of pending configs ONLY when we are at the end of a reconfiguration.


<pre><code><b>struct</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a> <b>has</b> <b>copy</b>, drop, key
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

<a name="@Constants_0"></a>

## Constants


<a name="0x1_config_for_next_epoch_EPERMISSION_DENIED"></a>



<pre><code><b>const</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_EPERMISSION_DENIED">EPERMISSION_DENIED</a>: u64 = 5;
</code></pre>



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



<a name="0x1_config_for_next_epoch_extracts_enabled"></a>

## Function `extracts_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extracts_enabled">extracts_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extracts_enabled">extracts_enabled</a>(): bool {
    <b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a>&gt;(@vm) || <b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a>&gt;(@std)
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_enable_extracts"></a>

## Function `enable_extracts`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_extracts">enable_extracts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_extracts">enable_extracts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(account, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a> {});
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_disable_extracts"></a>

## Function `disable_extracts`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_extracts">disable_extracts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_extracts">disable_extracts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a> {
    <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ExtractPermit">ExtractPermit</a>&gt;(address_of(account));
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_upserts_enabled"></a>

## Function `upserts_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upserts_enabled">upserts_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upserts_enabled">upserts_enabled</a>(): bool <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    !<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_upsert_lock_state">latest_upsert_lock_state</a>().locked
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_disable_upserts"></a>

## Function `disable_upserts`

Disable on-chain config updates. Only needed when a reconfiguration with DKG starts.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_upserts">disable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_disable_upserts">disable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_set_upsert_lock_state">set_upsert_lock_state</a>(account, <b>true</b>);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_enable_upserts"></a>

## Function `enable_upserts`

Enable on-chain config updates. Only needed when a reconfiguration with DKG finishes.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_upserts">enable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_enable_upserts">enable_upserts</a>(account: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_set_upsert_lock_state">set_upsert_lock_state</a>(account, <b>false</b>);
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

<a name="0x1_config_for_next_epoch_copied"></a>

## Function `copied`

Return a copy of the buffered on-chain config. Abort if the buffer is empty.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_copied">copied</a>&lt;T: <b>copy</b>, store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_copied">copied</a>&lt;T: <b>copy</b> + store&gt;(): T <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <b>borrow_global</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std).payload
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_upsert"></a>

## Function `upsert`

Save an on-chain config to the buffer for the next epoch.
If the buffer is not empty, put in the new one and discard the old one.
Typically followed by a <code>aptos_framework::reconfigure::start_reconfigure_with_dkg()</code> to make it effective as soon as possible.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop, store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert">upsert</a>&lt;T: drop + store&gt;(std: &<a href="signer.md#0x1_signer">signer</a>, config: T) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <b>assert</b>!(address_of(std) == @std, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>));
    <b>assert</b>!(!<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_upsert_lock_state">latest_upsert_lock_state</a>().locked, std::error::invalid_state(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ERESOURCE_BUSY">ERESOURCE_BUSY</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std)) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    };
    <b>move_to</b>(std, <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> { payload: config });
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_extract"></a>

## Function `extract`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

NOTE: The caller has to ensure updates are enabled using <code>enable_updates()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extract">extract</a>&lt;T: store&gt;(): T <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a> {
    <b>assert</b>!(!<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_extracts_enabled">extracts_enabled</a>(), std::error::invalid_state(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_EPERMISSION_DENIED">EPERMISSION_DENIED</a>));
    <b>let</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt; { payload } = <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ForNextEpoch">ForNextEpoch</a>&lt;T&gt;&gt;(@std);
    payload
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_upsert_lock_state"></a>

## Function `upsert_lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert_lock_state">upsert_lock_state</a>(addr: <b>address</b>): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">config_for_next_epoch::UpsertLock</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert_lock_state">upsert_lock_state</a>(addr: <b>address</b>): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a>&gt;(addr)) {
        *<b>borrow_global</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a>&gt;(addr)
    } <b>else</b> {
        <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
            seq_num: 0,
            locked: <b>false</b>,
        }
    }
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_latest_upsert_lock_state"></a>

## Function `latest_upsert_lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_upsert_lock_state">latest_upsert_lock_state</a>(): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">config_for_next_epoch::UpsertLock</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_upsert_lock_state">latest_upsert_lock_state</a>(): <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <b>let</b> state_0 = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert_lock_state">upsert_lock_state</a>(@vm);
    <b>let</b> state_1 = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_upsert_lock_state">upsert_lock_state</a>(@std);
    <b>if</b> (state_0.seq_num &gt; state_1.seq_num) {
        state_0
    } <b>else</b> {
        state_1
    }
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_set_upsert_lock_state"></a>

## Function `set_upsert_lock_state`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_set_upsert_lock_state">set_upsert_lock_state</a>(account: &<a href="signer.md#0x1_signer">signer</a>, locked: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_set_upsert_lock_state">set_upsert_lock_state</a>(account: &<a href="signer.md#0x1_signer">signer</a>, locked: bool) <b>acquires</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
    <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(account);

    <b>let</b> latest_state = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_latest_upsert_lock_state">latest_upsert_lock_state</a>();

    <b>if</b> (<b>exists</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a>&gt;(address_of(account))) {
        <b>move_from</b>&lt;<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a>&gt;(address_of(account));
    };

    <b>let</b> new_state = <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_UpsertLock">UpsertLock</a> {
        seq_num: latest_state.seq_num + 1,
        locked,
    };
    <b>move_to</b>(account, new_state);
}
</code></pre>



</details>

<a name="0x1_config_for_next_epoch_abort_unless_vm_or_std"></a>

## Function `abort_unless_vm_or_std`



<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="config_for_next_epoch.md#0x1_config_for_next_epoch_abort_unless_vm_or_std">abort_unless_vm_or_std</a>(account: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = std::signer::address_of(account);
    <b>assert</b>!(addr == @std || addr == @vm, std::error::permission_denied(<a href="config_for_next_epoch.md#0x1_config_for_next_epoch_ESTD_OR_VM_SIGNER_NEEDED">ESTD_OR_VM_SIGNER_NEEDED</a>));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
