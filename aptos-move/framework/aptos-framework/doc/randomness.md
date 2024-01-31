
<a id="0x1_randomness"></a>

# Module `0x1::randomness`

On-chain randomness utils.


-  [Resource `PerBlockRandomness`](#0x1_randomness_PerBlockRandomness)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_randomness_initialize)
-  [Function `on_new_block`](#0x1_randomness_on_new_block)
-  [Function `next_blob`](#0x1_randomness_next_blob)
-  [Function `u64_integer`](#0x1_randomness_u64_integer)
-  [Function `u256_integer`](#0x1_randomness_u256_integer)
-  [Function `u64_range`](#0x1_randomness_u64_range)
-  [Function `u256_range`](#0x1_randomness_u256_range)
-  [Function `permutation`](#0x1_randomness_permutation)
-  [Function `safe_add_mod`](#0x1_randomness_safe_add_mod)
-  [Function `get_and_add_txn_local_state`](#0x1_randomness_get_and_add_txn_local_state)
-  [Specification](#@Specification_1)
    -  [Function `get_and_add_txn_local_state`](#@Specification_1_get_and_add_txn_local_state)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="transaction_context.md#0x1_transaction_context">0x1::transaction_context</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_randomness_PerBlockRandomness"></a>

## Resource `PerBlockRandomness`

Per-block randomness seed.
This resource is updated in every block prologue.


<pre><code><b>struct</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> <b>has</b> drop, key
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
<code>seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_randomness_DST"></a>



<pre><code><b>const</b> <a href="randomness.md#0x1_randomness_DST">DST</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 80, 84, 79, 83, 95, 82, 65, 78, 68, 79, 77, 78, 69, 83, 83];
</code></pre>



<a id="0x1_randomness_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>move_to</b>(framework, <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
        epoch: 0,
        round: 0,
        seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_randomness_on_new_block"></a>

## Function `on_new_block`

Invoked in block prologues to update the block-level randomness seed.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch: u64, round: u64, seed_for_new_block: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, epoch: u64, round: u64, seed_for_new_block: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>let</b> <a href="randomness.md#0x1_randomness">randomness</a> = <b>borrow_global_mut</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(@aptos_framework);
    <a href="randomness.md#0x1_randomness">randomness</a>.epoch = epoch;
    <a href="randomness.md#0x1_randomness">randomness</a>.round = round;
    <a href="randomness.md#0x1_randomness">randomness</a>.seed = seed_for_new_block;
}
</code></pre>



</details>

<a id="0x1_randomness_next_blob"></a>

## Function `next_blob`

Generate 32 random bytes.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_next_blob">next_blob</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_next_blob">next_blob</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> input = <a href="randomness.md#0x1_randomness_DST">DST</a>;
    <b>let</b> seed_holder = <b>borrow_global</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(@aptos_framework);
    <b>let</b> seed = *<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&seed_holder.seed);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> input, seed);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> input, <a href="transaction_context.md#0x1_transaction_context_get_transaction_hash">transaction_context::get_transaction_hash</a>());
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> input, <a href="randomness.md#0x1_randomness_get_and_add_txn_local_state">get_and_add_txn_local_state</a>());
    <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(input)
}
</code></pre>



</details>

<a id="0x1_randomness_u64_integer"></a>

## Function `u64_integer`

Generates a u64 uniformly at random.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u64_integer">u64_integer</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u64_integer">u64_integer</a>(): u64 <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> raw = <a href="randomness.md#0x1_randomness_next_blob">next_blob</a>();
    <b>let</b> i = 0;
    <b>let</b> ret: u64 = 0;
    <b>while</b> (i &lt; 8) {
        ret = ret * 256 + (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> raw) <b>as</b> u64);
        i = i + 1;
    };
    ret
}
</code></pre>



</details>

<a id="0x1_randomness_u256_integer"></a>

## Function `u256_integer`

Generates a u256 uniformly at random.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>(): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>(): u256 <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> raw = <a href="randomness.md#0x1_randomness_next_blob">next_blob</a>();
    <b>let</b> i = 0;
    <b>let</b> ret: u256 = 0;
    <b>while</b> (i &lt; 32) {
        ret = ret * 256 + (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> raw) <b>as</b> u256);
        i = i + 1;
    };
    ret
}
</code></pre>



</details>

<a id="0x1_randomness_u64_range"></a>

## Function `u64_range`

Generates a number $n \in [min_incl, max_excl)$ uniformly at random.

NOTE: the uniformity is not perfect, but it can be proved that the probability error is no more than 1/2^192.
If you need perfect uniformty, consider implement your own with <code><a href="randomness.md#0x1_randomness_u64_integer">u64_integer</a>()</code> + rejection sampling.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u64_range">u64_range</a>(min_incl: u64, max_excl: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u64_range">u64_range</a>(min_incl: u64, max_excl: u64): u64 <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> range = ((max_excl - min_incl) <b>as</b> u256);
    <b>let</b> sample = ((<a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>() % range) <b>as</b> u64);
    min_incl + sample
}
</code></pre>



</details>

<a id="0x1_randomness_u256_range"></a>

## Function `u256_range`

Generates a number $n \in [min_incl, max_excl)$ uniformly at random.

NOTE: the uniformity is not perfect, but it can be proved that the probability error is no more than 1/2^256.
If you need perfect uniformty, consider implement your own with <code><a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>()</code> + rejection sampling.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u256_range">u256_range</a>(min_incl: u256, max_excl: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_u256_range">u256_range</a>(min_incl: u256, max_excl: u256): u256 <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> range = max_excl - min_incl;
    <b>let</b> r0 = <a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>();
    <b>let</b> r1 = <a href="randomness.md#0x1_randomness_u256_integer">u256_integer</a>();

    // Will compute sample := (r0 + r1*2^256) % range.

    <b>let</b> sample = r1 % range;
    <b>let</b> i = 0;
    <b>while</b> (i &lt; 256) {
        sample = <a href="randomness.md#0x1_randomness_safe_add_mod">safe_add_mod</a>(sample, sample, range);
        i = i + 1;
    };

    <b>let</b> sample = <a href="randomness.md#0x1_randomness_safe_add_mod">safe_add_mod</a>(sample, r0 % range, range);

    min_incl + sample
}
</code></pre>



</details>

<a id="0x1_randomness_permutation"></a>

## Function `permutation`

Generate a permutation of <code>[0, 1, ..., n-1]</code> uniformly at random.


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_permutation">permutation</a>(n: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness.md#0x1_randomness_permutation">permutation</a>(n: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <b>let</b> values = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];

    // Initialize into [0, 1, ..., n-1].
    <b>let</b> i = 0;
    <b>while</b> (i &lt; n) {
        std::vector::push_back(&<b>mut</b> values, i);
        i = i + 1;
    };

    // Shuffle.
    <b>let</b> tail = n - 1;
    <b>while</b> (tail &gt; 0) {
        <b>let</b> pop_position = <a href="randomness.md#0x1_randomness_u64_range">u64_range</a>(0, tail + 1);
        std::vector::swap(&<b>mut</b> values, pop_position, tail);
        tail = tail - 1;
    };

    values
}
</code></pre>



</details>

<a id="0x1_randomness_safe_add_mod"></a>

## Function `safe_add_mod`

Compute <code>(a + b) % m</code>, assuming <code>m &gt;= 1, 0 &lt;= a &lt; m, 0&lt;= b &lt; m</code>.


<pre><code><b>fun</b> <a href="randomness.md#0x1_randomness_safe_add_mod">safe_add_mod</a>(a: u256, b: u256, m: u256): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="randomness.md#0x1_randomness_safe_add_mod">safe_add_mod</a>(a: u256, b: u256, m: u256): u256 {
    <b>let</b> neg_b = m - b;
    <b>if</b> (a &lt; neg_b) {
        a + b
    } <b>else</b> {
        a - neg_b
    }
}
</code></pre>



</details>

<a id="0x1_randomness_get_and_add_txn_local_state"></a>

## Function `get_and_add_txn_local_state`



<pre><code><b>fun</b> <a href="randomness.md#0x1_randomness_get_and_add_txn_local_state">get_and_add_txn_local_state</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="randomness.md#0x1_randomness_get_and_add_txn_local_state">get_and_add_txn_local_state</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_get_and_add_txn_local_state"></a>

### Function `get_and_add_txn_local_state`


<pre><code><b>fun</b> <a href="randomness.md#0x1_randomness_get_and_add_txn_local_state">get_and_add_txn_local_state</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
