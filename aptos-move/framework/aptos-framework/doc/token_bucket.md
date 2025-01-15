
<a id="0x1_token_bucket"></a>

# Module `0x1::token_bucket`



-  [Resource `Bucket`](#0x1_token_bucket_Bucket)
-  [Function `initialize_bucket`](#0x1_token_bucket_initialize_bucket)
-  [Function `request`](#0x1_token_bucket_request)
-  [Function `refill`](#0x1_token_bucket_refill)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/math64.md#0x1_math64">0x1::math64</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_token_bucket_Bucket"></a>

## Resource `Bucket`



<pre><code><b>struct</b> <a href="token_bucket.md#0x1_token_bucket_Bucket">Bucket</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>capacity: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>tokens: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>refill_rate_per_minute: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_refill_timestamp: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>fractional_time_accumulated: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_token_bucket_initialize_bucket"></a>

## Function `initialize_bucket`



<pre><code><b>public</b> <b>fun</b> <a href="token_bucket.md#0x1_token_bucket_initialize_bucket">initialize_bucket</a>(capacity: u64): <a href="token_bucket.md#0x1_token_bucket_Bucket">token_bucket::Bucket</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_bucket.md#0x1_token_bucket_initialize_bucket">initialize_bucket</a>(capacity: u64): <a href="token_bucket.md#0x1_token_bucket_Bucket">Bucket</a> {
    <b>let</b> bucket = <a href="token_bucket.md#0x1_token_bucket_Bucket">Bucket</a> {
        capacity,
        tokens: capacity, // Start <b>with</b> a full bucket (full capacity of transactions allowed)
        refill_rate_per_minute: capacity,
        last_refill_timestamp: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        fractional_time_accumulated: 0, // Start <b>with</b> no fractional time accumulated
    };
    bucket
}
</code></pre>



</details>

<a id="0x1_token_bucket_request"></a>

## Function `request`



<pre><code><b>public</b> <b>fun</b> <a href="token_bucket.md#0x1_token_bucket_request">request</a>(bucket: &<b>mut</b> <a href="token_bucket.md#0x1_token_bucket_Bucket">token_bucket::Bucket</a>, num_token_requested: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="token_bucket.md#0x1_token_bucket_request">request</a>(bucket: &<b>mut</b> <a href="token_bucket.md#0x1_token_bucket_Bucket">Bucket</a>, num_token_requested: u64): bool {
    <a href="token_bucket.md#0x1_token_bucket_refill">refill</a>(bucket);
    <b>if</b> (bucket.tokens &gt;= num_token_requested) {
        bucket.tokens = bucket.tokens - num_token_requested;
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_token_bucket_refill"></a>

## Function `refill`



<pre><code><b>fun</b> <a href="token_bucket.md#0x1_token_bucket_refill">refill</a>(bucket: &<b>mut</b> <a href="token_bucket.md#0x1_token_bucket_Bucket">token_bucket::Bucket</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="token_bucket.md#0x1_token_bucket_refill">refill</a>(bucket: &<b>mut</b> <a href="token_bucket.md#0x1_token_bucket_Bucket">Bucket</a>) {
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> time_passed = current_time - bucket.last_refill_timestamp;

    // Total time passed including fractional accumulated time
    <b>let</b> total_time = time_passed + bucket.fractional_time_accumulated;

    // Calculate the full tokens that can be added
    <b>let</b> new_tokens = total_time * bucket.refill_rate_per_minute / 60;

    // Calculate the remaining fractional time
    <b>let</b> remaining_fractional_time = total_time % 60;

    // Refill the bucket <b>with</b> the full tokens
    <b>if</b> (new_tokens &gt; 0) {
        bucket.tokens = <a href="../../aptos-stdlib/doc/math64.md#0x1_math64_min">math64::min</a>(bucket.tokens + new_tokens, bucket.capacity);
        bucket.last_refill_timestamp = current_time;
    };

    // Update the fractional time accumulated for the next refill cycle
    bucket.fractional_time_accumulated = remaining_fractional_time;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
