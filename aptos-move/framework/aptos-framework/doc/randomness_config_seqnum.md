
<a id="0x1_randomness_config_seqnum"></a>

# Module `0x1::randomness_config_seqnum`

Randomness stall recovery utils.

When randomness generation is stuck due to a bug, the chain is also stuck. Below is the recovery procedure.
1. Ensure more than 2/3 stakes are stuck at the same version.
1. Every validator restarts with <code>randomness_override_seq_num</code> set to <code>X&#43;1</code> in the node config file,
where <code>X</code> is the current <code>RandomnessConfigSeqNum</code> on chain.
1. The chain should then be unblocked.
1. Once the bug is fixed and the binary + framework have been patched,
a governance proposal is needed to set <code>RandomnessConfigSeqNum</code> to be <code>X&#43;2</code>.


-  [Resource `RandomnessConfigSeqNum`](#0x1_randomness_config_seqnum_RandomnessConfigSeqNum)
-  [Function `set_for_next_epoch`](#0x1_randomness_config_seqnum_set_for_next_epoch)
-  [Function `initialize`](#0x1_randomness_config_seqnum_initialize)
-  [Function `on_new_epoch`](#0x1_randomness_config_seqnum_on_new_epoch)
-  [Specification](#@Specification_0)
    -  [Function `on_new_epoch`](#@Specification_0_on_new_epoch)


<pre><code>use 0x1::config_buffer;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_randomness_config_seqnum_RandomnessConfigSeqNum"></a>

## Resource `RandomnessConfigSeqNum`

If this seqnum is smaller than a validator local override, the on-chain <code>RandomnessConfig</code> will be ignored.
Useful in a chain recovery from randomness stall.


<pre><code>struct RandomnessConfigSeqNum has drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>seq_num: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_config_seqnum_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Update <code>RandomnessConfigSeqNum</code>.
Used when re-enable randomness after an emergency randomness disable via local override.


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, seq_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, seq_num: u64) &#123;
    system_addresses::assert_aptos_framework(framework);
    config_buffer::upsert(RandomnessConfigSeqNum &#123; seq_num &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_seqnum_initialize"></a>

## Function `initialize`

Initialize the configuration. Used in genesis or governance.


<pre><code>public fun initialize(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(framework);
    if (!exists&lt;RandomnessConfigSeqNum&gt;(@aptos_framework)) &#123;
        move_to(framework, RandomnessConfigSeqNum &#123; seq_num: 0 &#125;)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_randomness_config_seqnum_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>RandomnessConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires RandomnessConfigSeqNum &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;RandomnessConfigSeqNum&gt;()) &#123;
        let new_config &#61; config_buffer::extract&lt;RandomnessConfigSeqNum&gt;();
        if (exists&lt;RandomnessConfigSeqNum&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;RandomnessConfigSeqNum&gt;(@aptos_framework) &#61; new_config;
        &#125; else &#123;
            move_to(framework, new_config);
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);
include config_buffer::OnNewEpochRequirement&lt;RandomnessConfigSeqNum&gt;;
aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
