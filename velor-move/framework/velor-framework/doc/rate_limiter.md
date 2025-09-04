
<a id="0x1_rate_limiter"></a>

# Module `0x1::rate_limiter`



-  [Enum Resource `RateLimiter`](#0x1_rate_limiter_RateLimiter)
-  [Function `initialize`](#0x1_rate_limiter_initialize)
-  [Function `request`](#0x1_rate_limiter_request)
-  [Function `refill`](#0x1_rate_limiter_refill)


<pre><code><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_rate_limiter_RateLimiter"></a>

## Enum Resource `RateLimiter`



<pre><code>enum <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">RateLimiter</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>TokenBucket</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>capacity: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>current_amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>refill_interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_refill_timestamp: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>fractional_accumulated: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_rate_limiter_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_initialize">initialize</a>(capacity: u64, refill_interval: u64): <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">rate_limiter::RateLimiter</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_initialize">initialize</a>(capacity: u64, refill_interval: u64): <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">RateLimiter</a> {
    RateLimiter::TokenBucket {
        capacity,
        current_amount: capacity, // Start <b>with</b> a full bucket (full capacity of transactions allowed)
        refill_interval,
        last_refill_timestamp: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>(),
        fractional_accumulated: 0, // Start <b>with</b> no fractional accumulated
    }
}
</code></pre>



</details>

<a id="0x1_rate_limiter_request"></a>

## Function `request`



<pre><code><b>public</b> <b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_request">request</a>(limiter: &<b>mut</b> <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">rate_limiter::RateLimiter</a>, num_token_requested: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_request">request</a>(limiter: &<b>mut</b> <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">RateLimiter</a>, num_token_requested: u64): bool {
    <a href="rate_limiter.md#0x1_rate_limiter_refill">refill</a>(limiter);
    <b>if</b> (limiter.current_amount &gt;= num_token_requested) {
        limiter.current_amount = limiter.current_amount - num_token_requested;
        <b>true</b>
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_rate_limiter_refill"></a>

## Function `refill`



<pre><code><b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_refill">refill</a>(limiter: &<b>mut</b> <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">rate_limiter::RateLimiter</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="rate_limiter.md#0x1_rate_limiter_refill">refill</a>(limiter: &<b>mut</b> <a href="rate_limiter.md#0x1_rate_limiter_RateLimiter">RateLimiter</a>) {
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>();
    <b>let</b> time_passed = current_time - limiter.last_refill_timestamp;
    // Calculate the full tokens that can be added
    <b>let</b> accumulated_amount = time_passed * limiter.capacity + limiter.fractional_accumulated;
    <b>let</b> new_tokens = accumulated_amount / limiter.refill_interval;
    <b>if</b> (limiter.current_amount + new_tokens &gt;= limiter.capacity) {
        limiter.current_amount = limiter.capacity;
        limiter.fractional_accumulated = 0;
    } <b>else</b> {
        limiter.current_amount = limiter.current_amount + new_tokens;
        // Update the fractional amount accumulated for the next refill cycle
        limiter.fractional_accumulated = accumulated_amount % limiter.refill_interval;
    };
    limiter.last_refill_timestamp = current_time;
}
</code></pre>



</details>


[move-book]: https://velor.dev/move/book/SUMMARY
