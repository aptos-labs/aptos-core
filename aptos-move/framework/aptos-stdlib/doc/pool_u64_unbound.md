
<a id="0x1_pool_u64_unbound"></a>

# Module `0x1::pool_u64_unbound`


Simple module for tracking and calculating shares of a pool of coins. The shares are worth more as the total coins in
the pool increases. New shareholder can buy more shares or redeem their existing shares.

Example flow:
1. Pool start outs empty.
2. Shareholder A buys in with 1000 coins. A will receive 1000 shares in the pool. Pool now has 1000 total coins and
1000 total shares.
3. Pool appreciates in value from rewards and now has 2000 coins. A&apos;s 1000 shares are now worth 2000 coins.
4. Shareholder B now buys in with 1000 coins. Since before the buy in, each existing share is worth 2 coins, B will
receive 500 shares in exchange for 1000 coins. Pool now has 1500 shares and 3000 coins.
5. Pool appreciates in value from rewards and now has 6000 coins.
6. A redeems 500 shares. Each share is worth 6000 / 1500 &#61; 4. A receives 2000 coins. Pool has 4000 coins and 1000
shares left.


-  [Struct `Pool`](#0x1_pool_u64_unbound_Pool)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_pool_u64_unbound_new)
-  [Function `create`](#0x1_pool_u64_unbound_create)
-  [Function `create_with_scaling_factor`](#0x1_pool_u64_unbound_create_with_scaling_factor)
-  [Function `destroy_empty`](#0x1_pool_u64_unbound_destroy_empty)
-  [Function `total_coins`](#0x1_pool_u64_unbound_total_coins)
-  [Function `total_shares`](#0x1_pool_u64_unbound_total_shares)
-  [Function `contains`](#0x1_pool_u64_unbound_contains)
-  [Function `shares`](#0x1_pool_u64_unbound_shares)
-  [Function `balance`](#0x1_pool_u64_unbound_balance)
-  [Function `shareholders_count`](#0x1_pool_u64_unbound_shareholders_count)
-  [Function `update_total_coins`](#0x1_pool_u64_unbound_update_total_coins)
-  [Function `buy_in`](#0x1_pool_u64_unbound_buy_in)
-  [Function `add_shares`](#0x1_pool_u64_unbound_add_shares)
-  [Function `redeem_shares`](#0x1_pool_u64_unbound_redeem_shares)
-  [Function `transfer_shares`](#0x1_pool_u64_unbound_transfer_shares)
-  [Function `deduct_shares`](#0x1_pool_u64_unbound_deduct_shares)
-  [Function `amount_to_shares`](#0x1_pool_u64_unbound_amount_to_shares)
-  [Function `amount_to_shares_with_total_coins`](#0x1_pool_u64_unbound_amount_to_shares_with_total_coins)
-  [Function `shares_to_amount`](#0x1_pool_u64_unbound_shares_to_amount)
-  [Function `shares_to_amount_with_total_coins`](#0x1_pool_u64_unbound_shares_to_amount_with_total_coins)
-  [Function `shares_to_amount_with_total_stats`](#0x1_pool_u64_unbound_shares_to_amount_with_total_stats)
-  [Function `multiply_then_divide`](#0x1_pool_u64_unbound_multiply_then_divide)
-  [Function `to_u128`](#0x1_pool_u64_unbound_to_u128)
-  [Function `to_u256`](#0x1_pool_u64_unbound_to_u256)
-  [Specification](#@Specification_1)
    -  [Struct `Pool`](#@Specification_1_Pool)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `shares`](#@Specification_1_shares)
    -  [Function `balance`](#@Specification_1_balance)
    -  [Function `buy_in`](#@Specification_1_buy_in)
    -  [Function `add_shares`](#@Specification_1_add_shares)
    -  [Function `redeem_shares`](#@Specification_1_redeem_shares)
    -  [Function `transfer_shares`](#@Specification_1_transfer_shares)
    -  [Function `deduct_shares`](#@Specification_1_deduct_shares)
    -  [Function `amount_to_shares_with_total_coins`](#@Specification_1_amount_to_shares_with_total_coins)
    -  [Function `shares_to_amount_with_total_coins`](#@Specification_1_shares_to_amount_with_total_coins)
    -  [Function `multiply_then_divide`](#@Specification_1_multiply_then_divide)
    -  [Function `to_u128`](#@Specification_1_to_u128)
    -  [Function `to_u256`](#@Specification_1_to_u256)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;<br /></code></pre>



<a id="0x1_pool_u64_unbound_Pool"></a>

## Struct `Pool`



<pre><code><b>struct</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> <b>has</b> store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total_coins: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_shares: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>shares: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;<b>address</b>, u128&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scaling_factor: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_pool_u64_unbound_MAX_U64"></a>



<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>: u64 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_pool_u64_unbound_MAX_U128"></a>



<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>: u128 &#61; 340282366920938463463374607431768211455;<br /></code></pre>



<a id="0x1_pool_u64_unbound_EINSUFFICIENT_SHARES"></a>

Cannot redeem more shares than the shareholder has in the pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY"></a>

Cannot destroy non&#45;empty pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW"></a>

Pool&apos;s total coins cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW"></a>

Pool&apos;s total shares cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW">EPOOL_TOTAL_SHARES_OVERFLOW</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND"></a>

Shareholder not present in pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW"></a>

Shareholder cannot have more than u64.max shares.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_pool_u64_unbound_ETOO_MANY_SHAREHOLDERS"></a>

There are too many shareholders in the pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ETOO_MANY_SHAREHOLDERS">ETOO_MANY_SHAREHOLDERS</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_pool_u64_unbound_new"></a>

## Function `new`

Create a new pool.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> &#123;<br />    // Default <b>to</b> a scaling factor of 1 (effectively no scaling).<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(1)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_create"></a>

## Function `create`

Deprecated. Use <code>new</code> instead.
Create a new pool.


<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create">create</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create">create</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> &#123;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>()<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_create_with_scaling_factor"></a>

## Function `create_with_scaling_factor`

Create a new pool with custom <code>scaling_factor</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(scaling_factor: u64): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(scaling_factor: u64): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> &#123;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> &#123;<br />        total_coins: 0,<br />        total_shares: 0,<br />        shares: <a href="table.md#0x1_table_new">table::new</a>&lt;<b>address</b>, u128&gt;(),<br />        scaling_factor,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty pool. This will fail if the pool has any balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_destroy_empty">destroy_empty</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_destroy_empty">destroy_empty</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>) &#123;<br />    <b>assert</b>!(pool.total_coins &#61;&#61; 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>));<br />    <b>let</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> &#123;<br />        total_coins: _,<br />        total_shares: _,<br />        shares,<br />        scaling_factor: _,<br />    &#125; &#61; pool;<br />    table::destroy_empty&lt;<b>address</b>, u128&gt;(shares);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_coins"></a>

## Function `total_coins`

Return <code>pool</code>&apos;s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u64 &#123;<br />    pool.total_coins<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_shares"></a>

## Function `total_shares`

Return the total number of shares across all shareholders in <code>pool</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u128 &#123;<br />    pool.total_shares<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_contains"></a>

## Function `contains`

Return true if <code>shareholder</code> is in <code>pool</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): bool &#123;<br />    <a href="table.md#0x1_table_contains">table::contains</a>(&amp;pool.shares, shareholder)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares"></a>

## Function `shares`

Return the number of shares of <code>stakeholder</code> in <code>pool</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u128 &#123;<br />    <b>if</b> (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool, shareholder)) &#123;<br />        &#42;<a href="table.md#0x1_table_borrow">table::borrow</a>(&amp;pool.shares, shareholder)<br />    &#125; <b>else</b> &#123;<br />        0<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_balance"></a>

## Function `balance`

Return the balance in coins of <code>shareholder</code> in <code>pool.</code>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u64 &#123;<br />    <b>let</b> num_shares &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool, shareholder);<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(pool, num_shares)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_shareholders_count"></a>

## Function `shareholders_count`

Return the number of shareholders in <code>pool</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shareholders_count">shareholders_count</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shareholders_count">shareholders_count</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u64 &#123;<br />    table::length(&amp;pool.shares)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_update_total_coins"></a>

## Function `update_total_coins`

Update <code>pool</code>&apos;s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_update_total_coins">update_total_coins</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, new_total_coins: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_update_total_coins">update_total_coins</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, new_total_coins: u64) &#123;<br />    pool.total_coins &#61; new_total_coins;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_buy_in"></a>

## Function `buy_in`

Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128 &#123;<br />    <b>if</b> (coins_amount &#61;&#61; 0) <b>return</b> 0;<br /><br />    <b>let</b> new_shares &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(pool, coins_amount);<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a> &#45; pool.total_coins &gt;&#61; coins_amount, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>));<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a> &#45; pool.total_shares &gt;&#61; new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW">EPOOL_TOTAL_SHARES_OVERFLOW</a>));<br /><br />    pool.total_coins &#61; pool.total_coins &#43; coins_amount;<br />    pool.total_shares &#61; pool.total_shares &#43; new_shares;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(pool, shareholder, new_shares);<br />    new_shares<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_add_shares"></a>

## Function `add_shares`

Add the number of shares directly for <code>shareholder</code> in <code>pool</code>.
This would dilute other shareholders if the pool&apos;s balance of coins didn&apos;t change.


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128 &#123;<br />    <b>if</b> (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool, shareholder)) &#123;<br />        <b>let</b> existing_shares &#61; <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> pool.shares, shareholder);<br />        <b>let</b> current_shares &#61; &#42;existing_shares;<br />        <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a> &#45; current_shares &gt;&#61; new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>));<br /><br />        &#42;existing_shares &#61; current_shares &#43; new_shares;<br />        &#42;existing_shares<br />    &#125; <b>else</b> <b>if</b> (new_shares &gt; 0) &#123;<br />        <a href="table.md#0x1_table_add">table::add</a>(&amp;<b>mut</b> pool.shares, shareholder, new_shares);<br />        new_shares<br />    &#125; <b>else</b> &#123;<br />        new_shares<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_redeem_shares"></a>

## Function `redeem_shares`

Allow <code>shareholder</code> to redeem their shares in <code>pool</code> for coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64 &#123;<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool, shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool, shareholder) &gt;&#61; shares_to_redeem, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));<br /><br />    <b>if</b> (shares_to_redeem &#61;&#61; 0) <b>return</b> 0;<br /><br />    <b>let</b> redeemed_coins &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(pool, shares_to_redeem);<br />    pool.total_coins &#61; pool.total_coins &#45; redeemed_coins;<br />    pool.total_shares &#61; pool.total_shares &#45; shares_to_redeem;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(pool, shareholder, shares_to_redeem);<br /><br />    redeemed_coins<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_transfer_shares"></a>

## Function `transfer_shares`

Transfer shares from <code>shareholder_1</code> to <code>shareholder_2</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u128)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(<br />    pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>,<br />    shareholder_1: <b>address</b>,<br />    shareholder_2: <b>address</b>,<br />    shares_to_transfer: u128,<br />) &#123;<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool, shareholder_1), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool, shareholder_1) &gt;&#61; shares_to_transfer, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));<br />    <b>if</b> (shares_to_transfer &#61;&#61; 0) <b>return</b>;<br /><br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(pool, shareholder_1, shares_to_transfer);<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(pool, shareholder_2, shares_to_transfer);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_deduct_shares"></a>

## Function `deduct_shares`

Directly deduct <code>shareholder</code>&apos;s number of shares in <code>pool</code> and return the number of remaining shares.


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128 &#123;<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool, shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));<br />    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool, shareholder) &gt;&#61; num_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));<br /><br />    <b>let</b> existing_shares &#61; <a href="table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&amp;<b>mut</b> pool.shares, shareholder);<br />    &#42;existing_shares &#61; &#42;existing_shares &#45; num_shares;<br /><br />    // Remove the shareholder completely <b>if</b> they have no shares left.<br />    <b>let</b> remaining_shares &#61; &#42;existing_shares;<br />    <b>if</b> (remaining_shares &#61;&#61; 0) &#123;<br />        <a href="table.md#0x1_table_remove">table::remove</a>(&amp;<b>mut</b> pool.shares, shareholder);<br />    &#125;;<br /><br />    remaining_shares<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares"></a>

## Function `amount_to_shares`

Return the number of new shares <code>coins_amount</code> can buy in <code>pool</code>.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64): u128 &#123;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(pool, coins_amount, pool.total_coins)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares_with_total_coins"></a>

## Function `amount_to_shares_with_total_coins`

Return the number of new shares <code>coins_amount</code> can buy in <code>pool</code> with a custom total coins number.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64, total_coins: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u128 &#123;<br />    // No shares yet so amount is worth the same number of shares.<br />    <b>if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;<br />        // Multiply by scaling factor <b>to</b> minimize rounding errors during <b>internal</b> calculations for buy ins/redeems.<br />        // This can overflow but scaling factor is expected <b>to</b> be chosen carefully so this would not overflow.<br />        <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(coins_amount) &#42; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(pool.scaling_factor)<br />    &#125; <b>else</b> &#123;<br />        // Shares price &#61; total_coins / total existing shares.<br />        // New number of shares &#61; new_amount / shares_price &#61; new_amount &#42; existing_shares / total_amount.<br />        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.<br />        <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(pool, <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(coins_amount), pool.total_shares, <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(total_coins))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount"></a>

## Function `shares_to_amount`

Return the number of coins <code>shares</code> are worth in <code>pool</code>.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128): u64 &#123;<br />    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(pool, shares, pool.total_coins)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_coins"></a>

## Function `shares_to_amount_with_total_coins`

Return the number of coins <code>shares</code> are worth in <code>pool</code> with a custom total coins number.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128, total_coins: u64): u64 &#123;<br />    // No shares or coins yet so shares are worthless.<br />    <b>if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;<br />        0<br />    &#125; <b>else</b> &#123;<br />        // Shares price &#61; total_coins / total existing shares.<br />        // Shares worth &#61; shares &#42; shares price &#61; shares &#42; total_coins / total existing shares.<br />        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.<br />        (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(pool, shares, <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(total_coins), pool.total_shares) <b>as</b> u64)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_stats"></a>

## Function `shares_to_amount_with_total_stats`

Return the number of coins <code>shares</code> are worth in <code>pool</code> with custom total coins and shares numbers.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_stats">shares_to_amount_with_total_stats</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64, total_shares: u128): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_stats">shares_to_amount_with_total_stats</a>(<br />    pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>,<br />    shares: u128,<br />    total_coins: u64,<br />    total_shares: u128,<br />): u64 &#123;<br />    <b>if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; total_shares &#61;&#61; 0) &#123;<br />        0<br />    &#125; <b>else</b> &#123;<br />        (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(pool, shares, <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(total_coins), total_shares) <b>as</b> u64)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_multiply_then_divide"></a>

## Function `multiply_then_divide`



<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(_pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, x: u128, y: u128, z: u128): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(_pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, x: u128, y: u128, z: u128): u128 &#123;<br />    <b>let</b> result &#61; (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(x) &#42; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(y)) / <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(z);<br />    (result <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_to_u128"></a>

## Function `to_u128`



<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(num: u64): u128<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(num: u64): u128 &#123;<br />    (num <b>as</b> u128)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_pool_u64_unbound_to_u256"></a>

## Function `to_u256`



<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(num: u128): u256<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(num: u128): u256 &#123;<br />    (num <b>as</b> u256)<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Pool"></a>

### Struct `Pool`


<pre><code><b>struct</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> <b>has</b> store<br /></code></pre>



<dl>
<dt>
<code>total_coins: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_shares: u128</code>
</dt>
<dd>

</dd>
<dt>
<code>shares: <a href="table_with_length.md#0x1_table_with_length_TableWithLength">table_with_length::TableWithLength</a>&lt;<b>address</b>, u128&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scaling_factor: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <b>forall</b> addr: <b>address</b>:<br />    <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(shares, addr) &#61;&#61;&gt; (<a href="table.md#0x1_table_spec_get">table::spec_get</a>(shares, addr) &gt; 0);<br /></code></pre>




<a id="0x1_pool_u64_unbound_spec_contains"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): bool &#123;<br />   <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder)<br />&#125;<br /></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder);<br /></code></pre>




<a id="0x1_pool_u64_unbound_spec_shares"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u64 &#123;<br />   <b>if</b> (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder)) &#123;<br />       <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder)<br />   &#125;<br />   <b>else</b> &#123;<br />       0<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_shares"></a>

### Function `shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u128<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder);<br /></code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u64<br /></code></pre>




<pre><code><b>let</b> shares &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder);<br /><b>let</b> total_coins &#61; pool.total_coins;<br /><b>aborts_if</b> pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0 &amp;&amp; (shares &#42; total_coins) / pool.total_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;<br /><b>ensures</b> result &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool, shares, total_coins);<br /></code></pre>



<a id="@Specification_1_buy_in"></a>

### Function `buy_in`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128<br /></code></pre>




<pre><code><b>let</b> new_shares &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(pool, coins_amount, pool.total_coins);<br /><b>aborts_if</b> pool.total_coins &#43; coins_amount &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> pool.total_shares &#43; new_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;<br /><b>include</b> coins_amount &gt; 0 &#61;&#61;&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a> &#123; new_shares: new_shares &#125;;<br /><b>include</b> coins_amount &gt; 0 &#61;&#61;&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a> &#123; new_shares: new_shares &#125;;<br /><b>ensures</b> pool.total_coins &#61;&#61; <b>old</b>(pool.total_coins) &#43; coins_amount;<br /><b>ensures</b> pool.total_shares &#61;&#61; <b>old</b>(pool.total_shares) &#43; new_shares;<br /><b>ensures</b> result &#61;&#61; new_shares;<br /></code></pre>



<a id="@Specification_1_add_shares"></a>

### Function `add_shares`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128<br /></code></pre>




<pre><code><b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a>;<br /><b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a>;<br /><b>let</b> key_exists &#61; <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder);<br /><b>ensures</b> result &#61;&#61; <b>if</b> (key_exists) &#123; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder) &#125;<br /><b>else</b> &#123; new_shares &#125;;<br /></code></pre>




<a id="0x1_pool_u64_unbound_AddSharesAbortsIf"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a> &#123;<br />pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;<br />shareholder: <b>address</b>;<br />new_shares: u64;<br /><b>let</b> key_exists &#61; <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder);<br /><b>let</b> current_shares &#61; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder);<br /><b>aborts_if</b> key_exists &amp;&amp; current_shares &#43; new_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;<br />&#125;<br /></code></pre>




<a id="0x1_pool_u64_unbound_AddSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a> &#123;<br />pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;<br />shareholder: <b>address</b>;<br />new_shares: u64;<br /><b>let</b> key_exists &#61; <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder);<br /><b>let</b> current_shares &#61; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder);<br /><b>ensures</b> key_exists &#61;&#61;&gt;<br />    pool.shares &#61;&#61; <a href="table.md#0x1_table_spec_set">table::spec_set</a>(<b>old</b>(pool.shares), shareholder, current_shares &#43; new_shares);<br /><b>ensures</b> (!key_exists &amp;&amp; new_shares &gt; 0) &#61;&#61;&gt;<br />    pool.shares &#61;&#61; <a href="table.md#0x1_table_spec_set">table::spec_set</a>(<b>old</b>(pool.shares), shareholder, new_shares);<br />&#125;<br /></code></pre>




<a id="0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u128 &#123;<br />   <b>if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;<br />       coins_amount &#42; pool.scaling_factor<br />   &#125;<br />   <b>else</b> &#123;<br />       (coins_amount &#42; pool.total_shares) / total_coins<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_redeem_shares"></a>

### Function `redeem_shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64<br /></code></pre>




<pre><code><b>let</b> redeemed_coins &#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool, shares_to_redeem, pool.total_coins);<br /><b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder);<br /><b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder) &lt; shares_to_redeem;<br /><b>aborts_if</b> pool.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a> &lt; redeemed_coins;<br /><b>aborts_if</b> pool.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a> &lt; shares_to_redeem;<br /><b>ensures</b> pool.total_coins &#61;&#61; <b>old</b>(pool.total_coins) &#45; redeemed_coins;<br /><b>ensures</b> pool.total_shares &#61;&#61; <b>old</b>(pool.total_shares) &#45; shares_to_redeem;<br /><b>include</b> shares_to_redeem &gt; 0 &#61;&#61;&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a> &#123; num_shares: shares_to_redeem &#125;;<br /><b>ensures</b> result &#61;&#61; redeemed_coins;<br /></code></pre>



<a id="@Specification_1_transfer_shares"></a>

### Function `transfer_shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u128)<br /></code></pre>




<pre><code><b>aborts_if</b> (shareholder_1 !&#61; shareholder_2) &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_2) &amp;&amp;<br />    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_2) &#43; shares_to_transfer &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>);<br /><b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_1);<br /><b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_1) &lt; shares_to_transfer;<br /><b>ensures</b> shareholder_1 &#61;&#61; shareholder_2 &#61;&#61;&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(pool), shareholder_1) &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_1);<br /><b>ensures</b> ((shareholder_1 !&#61; shareholder_2) &amp;&amp; (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(pool), shareholder_1) &#61;&#61; shares_to_transfer)) &#61;&#61;&gt;<br />    !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_1);<br /><b>ensures</b> (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0) &#61;&#61;&gt;<br />    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_2));<br /><b>ensures</b> (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(<b>old</b>(pool), shareholder_2)) &#61;&#61;&gt;<br />    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_2) &amp;&amp; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_2) &#61;&#61; shares_to_transfer);<br /><b>ensures</b> (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(<b>old</b>(pool), shareholder_2)) &#61;&#61;&gt;<br />    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_2) &amp;&amp; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_2) &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(pool), shareholder_2) &#43; shares_to_transfer);<br /><b>ensures</b> ((shareholder_1 !&#61; shareholder_2) &amp;&amp; (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(pool), shareholder_1) &gt; shares_to_transfer)) &#61;&#61;&gt;<br />    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder_1) &amp;&amp; (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder_1) &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(pool), shareholder_1) &#45; shares_to_transfer));<br /></code></pre>



<a id="@Specification_1_deduct_shares"></a>

### Function `deduct_shares`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(pool: &amp;<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder);<br /><b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool, shareholder) &lt; num_shares;<br /><b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a>;<br /><b>let</b> remaining_shares &#61; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder) &#45; num_shares;<br /><b>ensures</b> remaining_shares &gt; 0 &#61;&#61;&gt; result &#61;&#61; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder);<br /><b>ensures</b> remaining_shares &#61;&#61; 0 &#61;&#61;&gt; result &#61;&#61; 0;<br /></code></pre>




<a id="0x1_pool_u64_unbound_DeductSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a> &#123;<br />pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;<br />shareholder: <b>address</b>;<br />num_shares: u64;<br /><b>let</b> remaining_shares &#61; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder) &#45; num_shares;<br /><b>ensures</b> remaining_shares &gt; 0 &#61;&#61;&gt; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder) &#61;&#61; remaining_shares;<br /><b>ensures</b> remaining_shares &#61;&#61; 0 &#61;&#61;&gt; !<a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder);<br />&#125;<br /></code></pre>



<a id="@Specification_1_amount_to_shares_with_total_coins"></a>

### Function `amount_to_shares_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64, total_coins: u64): u128<br /></code></pre>




<pre><code><b>aborts_if</b> pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0<br />    &amp;&amp; (coins_amount &#42; pool.total_shares) / total_coins &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;<br /><b>aborts_if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0)<br />    &amp;&amp; coins_amount &#42; pool.scaling_factor &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;<br /><b>aborts_if</b> pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0 &amp;&amp; total_coins &#61;&#61; 0;<br /><b>ensures</b> result &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(pool, coins_amount, total_coins);<br /></code></pre>



<a id="@Specification_1_shares_to_amount_with_total_coins"></a>

### Function `shares_to_amount_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64): u64<br /></code></pre>




<pre><code><b>aborts_if</b> pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0<br />    &amp;&amp; (shares &#42; total_coins) / pool.total_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;<br /><b>ensures</b> result &#61;&#61; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool, shares, total_coins);<br /></code></pre>




<a id="0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128, total_coins: u64): u64 &#123;<br />   <b>if</b> (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;<br />       0<br />   &#125;<br />   <b>else</b> &#123;<br />       (shares &#42; total_coins) / pool.total_shares<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_multiply_then_divide"></a>

### Function `multiply_then_divide`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(_pool: &amp;<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, x: u128, y: u128, z: u128): u128<br /></code></pre>




<pre><code><b>aborts_if</b> z &#61;&#61; 0;<br /><b>aborts_if</b> (x &#42; y) / z &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;<br /><b>ensures</b> result &#61;&#61; (x &#42; y) / z;<br /></code></pre>



<a id="@Specification_1_to_u128"></a>

### Function `to_u128`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u128">to_u128</a>(num: u64): u128<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; num;<br /></code></pre>



<a id="@Specification_1_to_u256"></a>

### Function `to_u256`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_to_u256">to_u256</a>(num: u128): u256<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; num;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
