
<a id="0x1_pool_u64"></a>

# Module `0x1::pool_u64`


Simple module for tracking and calculating shares of a pool of coins. The shares are worth more as the total coins in
the pool increases. New shareholder can buy more shares or redeem their existing shares.

Example flow:
1. Pool start outs empty.
2. Shareholder A buys in with 1000 coins. A will receive 1000 shares in the pool. Pool now has 1000 total coins and
1000 total shares.
3. Pool appreciates in value from rewards and now has 2000 coins. A's 1000 shares are now worth 2000 coins.
4. Shareholder B now buys in with 1000 coins. Since before the buy in, each existing share is worth 2 coins, B will
receive 500 shares in exchange for 1000 coins. Pool now has 1500 shares and 3000 coins.
5. Pool appreciates in value from rewards and now has 6000 coins.
6. A redeems 500 shares. Each share is worth 6000 / 1500 = 4. A receives 2000 coins. Pool has 4000 coins and 1000
shares left.


-  [Struct `Pool`](#0x1_pool_u64_Pool)
-  [Constants](#@Constants_0)
-  [Function `new`](#0x1_pool_u64_new)
-  [Function `create`](#0x1_pool_u64_create)
-  [Function `create_with_scaling_factor`](#0x1_pool_u64_create_with_scaling_factor)
-  [Function `destroy_empty`](#0x1_pool_u64_destroy_empty)
-  [Function `total_coins`](#0x1_pool_u64_total_coins)
-  [Function `total_shares`](#0x1_pool_u64_total_shares)
-  [Function `contains`](#0x1_pool_u64_contains)
-  [Function `shares`](#0x1_pool_u64_shares)
-  [Function `balance`](#0x1_pool_u64_balance)
-  [Function `shareholders`](#0x1_pool_u64_shareholders)
-  [Function `shareholders_count`](#0x1_pool_u64_shareholders_count)
-  [Function `update_total_coins`](#0x1_pool_u64_update_total_coins)
-  [Function `buy_in`](#0x1_pool_u64_buy_in)
-  [Function `add_shares`](#0x1_pool_u64_add_shares)
-  [Function `redeem_shares`](#0x1_pool_u64_redeem_shares)
-  [Function `transfer_shares`](#0x1_pool_u64_transfer_shares)
-  [Function `deduct_shares`](#0x1_pool_u64_deduct_shares)
-  [Function `amount_to_shares`](#0x1_pool_u64_amount_to_shares)
-  [Function `amount_to_shares_with_total_coins`](#0x1_pool_u64_amount_to_shares_with_total_coins)
-  [Function `shares_to_amount`](#0x1_pool_u64_shares_to_amount)
-  [Function `shares_to_amount_with_total_coins`](#0x1_pool_u64_shares_to_amount_with_total_coins)
-  [Function `multiply_then_divide`](#0x1_pool_u64_multiply_then_divide)
-  [Function `to_u128`](#0x1_pool_u64_to_u128)
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


<pre><code><b>use</b> <a href="../../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_pool_u64_Pool"></a>

## Struct `Pool`



<pre><code><b>struct</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>shareholders_limit: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_coins: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_shares: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>shares: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholders: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
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


<a id="0x1_pool_u64_MAX_U64"></a>



<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_pool_u64_EINSUFFICIENT_SHARES"></a>

Cannot redeem more shares than the shareholder has in the pool.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>: u64 = 4;
</code></pre>



<a id="0x1_pool_u64_EPOOL_IS_NOT_EMPTY"></a>

Cannot destroy non-empty pool.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_pool_u64_EPOOL_TOTAL_COINS_OVERFLOW"></a>

Pool's total coins cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>: u64 = 6;
</code></pre>



<a id="0x1_pool_u64_EPOOL_TOTAL_SHARES_OVERFLOW"></a>

Pool's total shares cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_EPOOL_TOTAL_SHARES_OVERFLOW">EPOOL_TOTAL_SHARES_OVERFLOW</a>: u64 = 7;
</code></pre>



<a id="0x1_pool_u64_ESHAREHOLDER_NOT_FOUND"></a>

Shareholder not present in pool.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_pool_u64_ESHAREHOLDER_SHARES_OVERFLOW"></a>

Shareholder cannot have more than u64.max shares.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>: u64 = 5;
</code></pre>



<a id="0x1_pool_u64_ETOO_MANY_SHAREHOLDERS"></a>

There are too many shareholders in the pool.


<pre><code><b>const</b> <a href="pool_u64.md#0x1_pool_u64_ETOO_MANY_SHAREHOLDERS">ETOO_MANY_SHAREHOLDERS</a>: u64 = 2;
</code></pre>



<a id="0x1_pool_u64_new"></a>

## Function `new`

Create a new pool.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_new">new</a>(shareholders_limit: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_new">new</a>(shareholders_limit: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> {
    // Default <b>to</b> a scaling factor of 1 (effectively no scaling).
    <a href="pool_u64.md#0x1_pool_u64_create_with_scaling_factor">create_with_scaling_factor</a>(shareholders_limit, 1)
}
</code></pre>



</details>

<a id="0x1_pool_u64_create"></a>

## Function `create`

Deprecated. Use <code>new</code> instead.
Create a new pool.


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_create">create</a>(shareholders_limit: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_create">create</a>(shareholders_limit: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> {
    <a href="pool_u64.md#0x1_pool_u64_new">new</a>(shareholders_limit)
}
</code></pre>



</details>

<a id="0x1_pool_u64_create_with_scaling_factor"></a>

## Function `create_with_scaling_factor`

Create a new pool with custom <code>scaling_factor</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_create_with_scaling_factor">create_with_scaling_factor</a>(shareholders_limit: u64, scaling_factor: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_create_with_scaling_factor">create_with_scaling_factor</a>(shareholders_limit: u64, scaling_factor: u64): <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> {
    <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> {
        shareholders_limit,
        total_coins: 0,
        total_shares: 0,
        shares: <a href="simple_map.md#0x1_simple_map_create">simple_map::create</a>&lt;<b>address</b>, u64&gt;(),
        shareholders: <a href="../../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;(),
        scaling_factor,
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty pool. This will fail if the pool has any balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_destroy_empty">destroy_empty</a>(self: <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_destroy_empty">destroy_empty</a>(self: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>) {
    <b>assert</b>!(self.total_coins == 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="pool_u64.md#0x1_pool_u64_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>));
    <b>let</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> {
        shareholders_limit: _,
        total_coins: _,
        total_shares: _,
        shares: _,
        shareholders: _,
        scaling_factor: _,
    } = self;
}
</code></pre>



</details>

<a id="0x1_pool_u64_total_coins"></a>

## Function `total_coins`

Return <code>self</code>'s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_total_coins">total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_total_coins">total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>): u64 {
    self.total_coins
}
</code></pre>



</details>

<a id="0x1_pool_u64_total_shares"></a>

## Function `total_shares`

Return the total number of shares across all shareholders in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_total_shares">total_shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_total_shares">total_shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>): u64 {
    self.total_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_contains"></a>

## Function `contains`

Return true if <code>shareholder</code> is in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>): bool {
    <a href="simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&self.shares, &shareholder)
}
</code></pre>



</details>

<a id="0x1_pool_u64_shares"></a>

## Function `shares`

Return the number of shares of <code>stakeholder</code> in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>): u64 {
    <b>if</b> (<a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self, shareholder)) {
        *<a href="simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&self.shares, &shareholder)
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_balance"></a>

## Function `balance`

Return the balance in coins of <code>shareholder</code> in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_balance">balance</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_balance">balance</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>): u64 {
    <b>let</b> num_shares = <a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self, shareholder);
    <a href="pool_u64.md#0x1_pool_u64_shares_to_amount">shares_to_amount</a>(self, num_shares)
}
</code></pre>



</details>

<a id="0x1_pool_u64_shareholders"></a>

## Function `shareholders`

Return the list of shareholders in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shareholders">shareholders</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shareholders">shareholders</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; {
    self.shareholders
}
</code></pre>



</details>

<a id="0x1_pool_u64_shareholders_count"></a>

## Function `shareholders_count`

Return the number of shareholders in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shareholders_count">shareholders_count</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shareholders_count">shareholders_count</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>): u64 {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&self.shareholders)
}
</code></pre>



</details>

<a id="0x1_pool_u64_update_total_coins"></a>

## Function `update_total_coins`

Update <code>self</code>'s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_update_total_coins">update_total_coins</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, new_total_coins: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_update_total_coins">update_total_coins</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, new_total_coins: u64) {
    self.total_coins = new_total_coins;
}
</code></pre>



</details>

<a id="0x1_pool_u64_buy_in"></a>

## Function `buy_in`

Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u64 {
    <b>if</b> (coins_amount == 0) <b>return</b> 0;

    <b>let</b> new_shares = <a href="pool_u64.md#0x1_pool_u64_amount_to_shares">amount_to_shares</a>(self, coins_amount);
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a> - self.total_coins &gt;= coins_amount, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>));
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a> - self.total_shares &gt;= new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>));

    self.total_coins = self.total_coins + coins_amount;
    self.total_shares = self.total_shares + new_shares;
    <a href="pool_u64.md#0x1_pool_u64_add_shares">add_shares</a>(self, shareholder, new_shares);
    new_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_add_shares"></a>

## Function `add_shares`

Add the number of shares directly for <code>shareholder</code> in <code>self</code>.
This would dilute other shareholders if the pool's balance of coins didn't change.


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, new_shares: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>, new_shares: u64): u64 {
    <b>if</b> (<a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self, shareholder)) {
        <b>let</b> existing_shares = <a href="simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> self.shares, &shareholder);
        <b>let</b> current_shares = *existing_shares;
        <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a> - current_shares &gt;= new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>));

        *existing_shares = current_shares + new_shares;
        *existing_shares
    } <b>else</b> <b>if</b> (new_shares &gt; 0) {
        <b>assert</b>!(
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&self.shareholders) &lt; self.shareholders_limit,
            <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="pool_u64.md#0x1_pool_u64_ETOO_MANY_SHAREHOLDERS">ETOO_MANY_SHAREHOLDERS</a>),
        );

        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> self.shareholders, shareholder);
        <a href="simple_map.md#0x1_simple_map_add">simple_map::add</a>(&<b>mut</b> self.shares, shareholder, new_shares);
        new_shares
    } <b>else</b> {
        new_shares
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_redeem_shares"></a>

## Function `redeem_shares`

Allow <code>shareholder</code> to redeem their shares in <code>self</code> for coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u64): u64 {
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self, shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self, shareholder) &gt;= shares_to_redeem, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));

    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;

    <b>let</b> redeemed_coins = <a href="pool_u64.md#0x1_pool_u64_shares_to_amount">shares_to_amount</a>(self, shares_to_redeem);
    self.total_coins = self.total_coins - redeemed_coins;
    self.total_shares = self.total_shares - shares_to_redeem;
    <a href="pool_u64.md#0x1_pool_u64_deduct_shares">deduct_shares</a>(self, shareholder, shares_to_redeem);

    redeemed_coins
}
</code></pre>



</details>

<a id="0x1_pool_u64_transfer_shares"></a>

## Function `transfer_shares`

Transfer shares from <code>shareholder_1</code> to <code>shareholder_2</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_transfer_shares">transfer_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_transfer_shares">transfer_shares</a>(
    self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>,
    shareholder_1: <b>address</b>,
    shareholder_2: <b>address</b>,
    shares_to_transfer: u64,
) {
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self, shareholder_1), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self, shareholder_1) &gt;= shares_to_transfer, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));
    <b>if</b> (shares_to_transfer == 0) <b>return</b>;

    <a href="pool_u64.md#0x1_pool_u64_deduct_shares">deduct_shares</a>(self, shareholder_1, shares_to_transfer);
    <a href="pool_u64.md#0x1_pool_u64_add_shares">add_shares</a>(self, shareholder_2, shares_to_transfer);
}
</code></pre>



</details>

<a id="0x1_pool_u64_deduct_shares"></a>

## Function `deduct_shares`

Directly deduct <code>shareholder</code>'s number of shares in <code>self</code> and return the number of remaining shares.


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, num_shares: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>, num_shares: u64): u64 {
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self, shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(<a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self, shareholder) &gt;= num_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64.md#0x1_pool_u64_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));

    <b>let</b> existing_shares = <a href="simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> self.shares, &shareholder);
    *existing_shares = *existing_shares - num_shares;

    // Remove the shareholder completely <b>if</b> they have no shares left.
    <b>let</b> remaining_shares = *existing_shares;
    <b>if</b> (remaining_shares == 0) {
        <b>let</b> (_, shareholder_index) = <a href="../../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&self.shareholders, &shareholder);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> self.shareholders, shareholder_index);
        <a href="simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> self.shares, &shareholder);
    };

    remaining_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_amount_to_shares"></a>

## Function `amount_to_shares`

Return the number of new shares <code>coins_amount</code> can buy in <code>self</code>.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_amount_to_shares">amount_to_shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, coins_amount: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_amount_to_shares">amount_to_shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, coins_amount: u64): u64 {
    <a href="pool_u64.md#0x1_pool_u64_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self, coins_amount, self.total_coins)
}
</code></pre>



</details>

<a id="0x1_pool_u64_amount_to_shares_with_total_coins"></a>

## Function `amount_to_shares_with_total_coins`

Return the number of new shares <code>coins_amount</code> can buy in <code>self</code> with a custom total coins number.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, coins_amount: u64, total_coins: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u64 {
    // No shares yet so amount is worth the same number of shares.
    <b>if</b> (self.total_coins == 0 || self.total_shares == 0) {
        // Multiply by scaling factor <b>to</b> minimize rounding errors during <b>internal</b> calculations for buy ins/redeems.
        // This can overflow but scaling factor is expected <b>to</b> be chosen carefully so this would not overflow.
        coins_amount * self.scaling_factor
    } <b>else</b> {
        // Shares price = total_coins / total existing shares.
        // New number of shares = new_amount / shares_price = new_amount * existing_shares / total_amount.
        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.
        <a href="pool_u64.md#0x1_pool_u64_multiply_then_divide">multiply_then_divide</a>(self, coins_amount, self.total_shares, total_coins)
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_shares_to_amount"></a>

## Function `shares_to_amount`

Return the number of coins <code>shares</code> are worth in <code>self</code>.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares_to_amount">shares_to_amount</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shares: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares_to_amount">shares_to_amount</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shares: u64): u64 {
    <a href="pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self, shares, self.total_coins)
}
</code></pre>



</details>

<a id="0x1_pool_u64_shares_to_amount_with_total_coins"></a>

## Function `shares_to_amount_with_total_coins`

Return the number of coins <code>shares</code> are worth in <code>self</code> with a custom total coins number.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shares: u64, total_coins: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shares: u64, total_coins: u64): u64 {
    // No shares or coins yet so shares are worthless.
    <b>if</b> (self.total_coins == 0 || self.total_shares == 0) {
        0
    } <b>else</b> {
        // Shares price = total_coins / total existing shares.
        // Shares worth = shares * shares price = shares * total_coins / total existing shares.
        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.
        <a href="pool_u64.md#0x1_pool_u64_multiply_then_divide">multiply_then_divide</a>(self, shares, total_coins, self.total_shares)
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_multiply_then_divide"></a>

## Function `multiply_then_divide`



<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, x: u64, y: u64, z: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, x: u64, y: u64, z: u64): u64 {
    <b>let</b> result = (<a href="pool_u64.md#0x1_pool_u64_to_u128">to_u128</a>(x) * <a href="pool_u64.md#0x1_pool_u64_to_u128">to_u128</a>(y)) / <a href="pool_u64.md#0x1_pool_u64_to_u128">to_u128</a>(z);
    (result <b>as</b> u64)
}
</code></pre>



</details>

<a id="0x1_pool_u64_to_u128"></a>

## Function `to_u128`



<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_to_u128">to_u128</a>(num: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_to_u128">to_u128</a>(num: u64): u128 {
    (num <b>as</b> u128)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_1_Pool"></a>

### Struct `Pool`


<pre><code><b>struct</b> <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a> <b>has</b> store
</code></pre>



<dl>
<dt>
<code>shareholders_limit: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_coins: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_shares: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>shares: <a href="simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>shareholders: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scaling_factor: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>invariant</b> <b>forall</b> addr: <b>address</b>:
    (<a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(shares, addr) == <a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(shareholders, addr));
<b>invariant</b> <b>forall</b> i in 0..len(shareholders), j in 0..len(shareholders):
    shareholders[i] == shareholders[j] ==&gt; i == j;
</code></pre>




<a id="0x1_pool_u64_spec_contains"></a>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_spec_contains">spec_contains</a>(pool: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>): bool {
   <a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(pool.shares, shareholder)
}
</code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_contains">contains</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="pool_u64.md#0x1_pool_u64_spec_contains">spec_contains</a>(self, shareholder);
</code></pre>




<a id="0x1_pool_u64_spec_shares"></a>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(pool: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shareholder: <b>address</b>): u64 {
   <b>if</b> (<a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(pool.shares, shareholder)) {
       <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(pool.shares, shareholder)
   }
   <b>else</b> {
       0
   }
}
</code></pre>



<a id="@Specification_1_shares"></a>

### Function `shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares">shares</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(self, shareholder);
</code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_balance">balance</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>




<pre><code><b>let</b> shares = <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(self, shareholder);
<b>let</b> total_coins = self.total_coins;
<b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0 && (shares * total_coins) / self.total_shares &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>ensures</b> result == <a href="pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares, total_coins);
</code></pre>



<a id="@Specification_1_buy_in"></a>

### Function `buy_in`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u64
</code></pre>




<pre><code><b>let</b> new_shares = <a href="pool_u64.md#0x1_pool_u64_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(self, coins_amount, self.total_coins);
<b>aborts_if</b> self.total_coins + coins_amount &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> self.total_shares + new_shares &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>include</b> coins_amount &gt; 0 ==&gt; <a href="pool_u64.md#0x1_pool_u64_AddSharesAbortsIf">AddSharesAbortsIf</a> { new_shares: new_shares };
<b>include</b> coins_amount &gt; 0 ==&gt; <a href="pool_u64.md#0x1_pool_u64_AddSharesEnsures">AddSharesEnsures</a> { new_shares: new_shares };
<b>ensures</b> self.total_coins == <b>old</b>(self.total_coins) + coins_amount;
<b>ensures</b> self.total_shares == <b>old</b>(self.total_shares) + new_shares;
<b>ensures</b> result == new_shares;
</code></pre>



<a id="@Specification_1_add_shares"></a>

### Function `add_shares`


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, new_shares: u64): u64
</code></pre>




<pre><code><b>include</b> <a href="pool_u64.md#0x1_pool_u64_AddSharesAbortsIf">AddSharesAbortsIf</a>;
<b>include</b> <a href="pool_u64.md#0x1_pool_u64_AddSharesEnsures">AddSharesEnsures</a>;
<b>let</b> key_exists = <a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.shares, shareholder);
<b>ensures</b> result == <b>if</b> (key_exists) { <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder) }
<b>else</b> { new_shares };
</code></pre>




<a id="0x1_pool_u64_AddSharesAbortsIf"></a>


<pre><code><b>schema</b> <a href="pool_u64.md#0x1_pool_u64_AddSharesAbortsIf">AddSharesAbortsIf</a> {
    self: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>;
    shareholder: <b>address</b>;
    new_shares: u64;
    <b>let</b> key_exists = <a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.shares, shareholder);
    <b>let</b> current_shares = <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder);
    <b>aborts_if</b> key_exists && current_shares + new_shares &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
    <b>aborts_if</b> !key_exists && new_shares &gt; 0 && len(self.shareholders) &gt;= self.shareholders_limit;
}
</code></pre>




<a id="0x1_pool_u64_AddSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64.md#0x1_pool_u64_AddSharesEnsures">AddSharesEnsures</a> {
    self: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>;
    shareholder: <b>address</b>;
    new_shares: u64;
    <b>let</b> key_exists = <a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.shares, shareholder);
    <b>let</b> current_shares = <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder);
    <b>ensures</b> key_exists ==&gt;
        self.shares == <a href="simple_map.md#0x1_simple_map_spec_set">simple_map::spec_set</a>(<b>old</b>(self.shares), shareholder, current_shares + new_shares);
    <b>ensures</b> (!key_exists && new_shares &gt; 0) ==&gt;
        self.shares == <a href="simple_map.md#0x1_simple_map_spec_set">simple_map::spec_set</a>(<b>old</b>(self.shares), shareholder, new_shares);
    <b>ensures</b> (!key_exists && new_shares &gt; 0) ==&gt;
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_eq_push_back">vector::eq_push_back</a>(self.shareholders, <b>old</b>(self.shareholders), shareholder);
}
</code></pre>




<a id="0x1_pool_u64_spec_amount_to_shares_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(pool: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u64 {
   <b>if</b> (pool.total_coins == 0 || pool.total_shares == 0) {
       coins_amount * pool.scaling_factor
   }
   <b>else</b> {
       (coins_amount * pool.total_shares) / total_coins
   }
}
</code></pre>



<a id="@Specification_1_redeem_shares"></a>

### Function `redeem_shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u64): u64
</code></pre>




<pre><code><b>let</b> redeemed_coins = <a href="pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares_to_redeem, self.total_coins);
<b>aborts_if</b> !<a href="pool_u64.md#0x1_pool_u64_spec_contains">spec_contains</a>(self, shareholder);
<b>aborts_if</b> <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(self, shareholder) &lt; shares_to_redeem;
<b>aborts_if</b> self.<a href="pool_u64.md#0x1_pool_u64_total_coins">total_coins</a> &lt; redeemed_coins;
<b>aborts_if</b> self.<a href="pool_u64.md#0x1_pool_u64_total_shares">total_shares</a> &lt; shares_to_redeem;
<b>ensures</b> self.total_coins == <b>old</b>(self.total_coins) - redeemed_coins;
<b>ensures</b> self.total_shares == <b>old</b>(self.total_shares) - shares_to_redeem;
<b>include</b> shares_to_redeem &gt; 0 ==&gt; <a href="pool_u64.md#0x1_pool_u64_DeductSharesEnsures">DeductSharesEnsures</a> {
    num_shares: shares_to_redeem
};
<b>ensures</b> result == redeemed_coins;
</code></pre>



<a id="@Specification_1_transfer_shares"></a>

### Function `transfer_shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_transfer_shares">transfer_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u64)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>aborts_if</b> !<a href="pool_u64.md#0x1_pool_u64_spec_contains">spec_contains</a>(self, shareholder_1);
<b>aborts_if</b> <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(self, shareholder_1) &lt; shares_to_transfer;
</code></pre>



<a id="@Specification_1_deduct_shares"></a>

### Function `deduct_shares`


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shareholder: <b>address</b>, num_shares: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> !<a href="pool_u64.md#0x1_pool_u64_spec_contains">spec_contains</a>(self, shareholder);
<b>aborts_if</b> <a href="pool_u64.md#0x1_pool_u64_spec_shares">spec_shares</a>(self, shareholder) &lt; num_shares;
<b>include</b> <a href="pool_u64.md#0x1_pool_u64_DeductSharesEnsures">DeductSharesEnsures</a>;
<b>let</b> remaining_shares = <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder) - num_shares;
<b>ensures</b> remaining_shares &gt; 0 ==&gt; result == <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder);
<b>ensures</b> remaining_shares == 0 ==&gt; result == 0;
</code></pre>




<a id="0x1_pool_u64_DeductSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64.md#0x1_pool_u64_DeductSharesEnsures">DeductSharesEnsures</a> {
    self: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>;
    shareholder: <b>address</b>;
    num_shares: u64;
    <b>let</b> remaining_shares = <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder) - num_shares;
    <b>ensures</b> remaining_shares &gt; 0 ==&gt; <a href="simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(self.shares, shareholder) == remaining_shares;
    <b>ensures</b> remaining_shares == 0 ==&gt; !<a href="simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(self.shares, shareholder);
    <b>ensures</b> remaining_shares == 0 ==&gt; !<a href="../../move-stdlib/doc/vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(self.shareholders, shareholder);
}
</code></pre>



<a id="@Specification_1_amount_to_shares_with_total_coins"></a>

### Function `amount_to_shares_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, coins_amount: u64, total_coins: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0
    && (coins_amount * self.total_shares) / total_coins &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> (self.total_coins == 0 || self.total_shares == 0)
    && coins_amount * self.scaling_factor &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0 && total_coins == 0;
<b>ensures</b> result == <a href="pool_u64.md#0x1_pool_u64_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(self, coins_amount, total_coins);
</code></pre>



<a id="@Specification_1_shares_to_amount_with_total_coins"></a>

### Function `shares_to_amount_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, shares: u64, total_coins: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0
    && (shares * total_coins) / self.total_shares &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>ensures</b> result == <a href="pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares, total_coins);
</code></pre>




<a id="0x1_pool_u64_spec_shares_to_amount_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64.md#0x1_pool_u64_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool: <a href="pool_u64.md#0x1_pool_u64_Pool">Pool</a>, shares: u64, total_coins: u64): u64 {
   <b>if</b> (pool.total_coins == 0 || pool.total_shares == 0) {
       0
   }
   <b>else</b> {
       (shares * total_coins) / pool.total_shares
   }
}
</code></pre>



<a id="@Specification_1_multiply_then_divide"></a>

### Function `multiply_then_divide`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64.md#0x1_pool_u64_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64.md#0x1_pool_u64_Pool">pool_u64::Pool</a>, x: u64, y: u64, z: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> z == 0;
<b>aborts_if</b> (x * y) / z &gt; <a href="pool_u64.md#0x1_pool_u64_MAX_U64">MAX_U64</a>;
<b>ensures</b> result == (x * y) / z;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
