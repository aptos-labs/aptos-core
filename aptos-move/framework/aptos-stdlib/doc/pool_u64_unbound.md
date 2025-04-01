
<a id="0x1_pool_u64_unbound"></a>

# Module `0x1::pool_u64_unbound`


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
<b>use</b> <a href="table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
</code></pre>



<a id="0x1_pool_u64_unbound_Pool"></a>

## Struct `Pool`



<pre><code><b>struct</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> <b>has</b> store
</code></pre>



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



<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_pool_u64_unbound_MAX_U128"></a>



<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>: u128 = 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_pool_u64_unbound_EINSUFFICIENT_SHARES"></a>

Cannot redeem more shares than the shareholder has in the pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>: u64 = 4;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY"></a>

Cannot destroy non-empty pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>: u64 = 3;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW"></a>

Pool's total coins cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>: u64 = 6;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW"></a>

Pool's total shares cannot exceed u64.max.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW">EPOOL_TOTAL_SHARES_OVERFLOW</a>: u64 = 7;
</code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND"></a>

Shareholder not present in pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW"></a>

Shareholder cannot have more than u64.max shares.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>: u64 = 5;
</code></pre>



<a id="0x1_pool_u64_unbound_ETOO_MANY_SHAREHOLDERS"></a>

There are too many shareholders in the pool.


<pre><code><b>const</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ETOO_MANY_SHAREHOLDERS">ETOO_MANY_SHAREHOLDERS</a>: u64 = 2;
</code></pre>



<a id="0x1_pool_u64_unbound_new"></a>

## Function `new`

Create a new pool.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> {
    // Default <b>to</b> a scaling factor of 1 (effectively no scaling).
    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(1)
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_create"></a>

## Function `create`

Deprecated. Use <code>new</code> instead.
Create a new pool.


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create">create</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create">create</a>(): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> {
    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_new">new</a>()
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_create_with_scaling_factor"></a>

## Function `create_with_scaling_factor`

Create a new pool with custom <code>scaling_factor</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(scaling_factor: u64): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_create_with_scaling_factor">create_with_scaling_factor</a>(scaling_factor: u64): <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> {
    <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> {
        total_coins: 0,
        total_shares: 0,
        shares: <a href="table.md#0x1_table_new">table::new</a>&lt;<b>address</b>, u128&gt;(),
        scaling_factor,
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty pool. This will fail if the pool has any balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_destroy_empty">destroy_empty</a>(self: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_destroy_empty">destroy_empty</a>(self: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>) {
    <b>assert</b>!(self.total_coins == 0, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY">EPOOL_IS_NOT_EMPTY</a>));
    <b>let</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> {
        total_coins: _,
        total_shares: _,
        shares,
        scaling_factor: _,
    } = self;
    shares.destroy_empty::&lt;<b>address</b>, u128&gt;();
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_coins"></a>

## Function `total_coins`

Return <code>self</code>'s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u64 {
    self.total_coins
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_shares"></a>

## Function `total_shares`

Return the total number of shares across all shareholders in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u128 {
    self.total_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_contains"></a>

## Function `contains`

Return true if <code>shareholder</code> is in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): bool {
    self.shares.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder)
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares"></a>

## Function `shares`

Return the number of shares of <code>stakeholder</code> in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u128 {
    <b>if</b> (self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder)) {
        *self.shares.borrow(shareholder)
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_balance"></a>

## Function `balance`

Return the balance in coins of <code>shareholder</code> in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u64 {
    <b>let</b> num_shares = self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(shareholder);
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(num_shares)
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shareholders_count"></a>

## Function `shareholders_count`

Return the number of shareholders in <code>self</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shareholders_count">shareholders_count</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shareholders_count">shareholders_count</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>): u64 {
    self.shares.length()
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_update_total_coins"></a>

## Function `update_total_coins`

Update <code>self</code>'s total balance of coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_update_total_coins">update_total_coins</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, new_total_coins: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_update_total_coins">update_total_coins</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, new_total_coins: u64) {
    self.total_coins = new_total_coins;
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_buy_in"></a>

## Function `buy_in`

Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128 {
    <b>if</b> (coins_amount == 0) <b>return</b> 0;

    <b>let</b> new_shares = self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(coins_amount);
    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a> - self.total_coins &gt;= coins_amount, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW">EPOOL_TOTAL_COINS_OVERFLOW</a>));
    <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a> - self.total_shares &gt;= new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW">EPOOL_TOTAL_SHARES_OVERFLOW</a>));

    self.total_coins += coins_amount;
    self.total_shares += new_shares;
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(shareholder, new_shares);
    new_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_add_shares"></a>

## Function `add_shares`

Add the number of shares directly for <code>shareholder</code> in <code>self</code>.
This would dilute other shareholders if the pool's balance of coins didn't change.


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128 {
    <b>if</b> (self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder)) {
        <b>let</b> existing_shares = self.shares.borrow_mut(shareholder);
        <b>let</b> current_shares = *existing_shares;
        <b>assert</b>!(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a> - current_shares &gt;= new_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW">ESHAREHOLDER_SHARES_OVERFLOW</a>));

        *existing_shares = current_shares + new_shares;
        *existing_shares
    } <b>else</b> <b>if</b> (new_shares &gt; 0) {
        self.shares.add(shareholder, new_shares);
        new_shares
    } <b>else</b> {
        new_shares
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_redeem_shares"></a>

## Function `redeem_shares`

Allow <code>shareholder</code> to redeem their shares in <code>self</code> for coins.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64 {
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(shareholder) &gt;= shares_to_redeem, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));

    <b>if</b> (shares_to_redeem == 0) <b>return</b> 0;

    <b>let</b> redeemed_coins = self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(shares_to_redeem);
    self.total_coins -= redeemed_coins;
    self.total_shares -= shares_to_redeem;
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(shareholder, shares_to_redeem);

    redeemed_coins
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_transfer_shares"></a>

## Function `transfer_shares`

Transfer shares from <code>shareholder_1</code> to <code>shareholder_2</code>.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(
    self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>,
    shareholder_1: <b>address</b>,
    shareholder_2: <b>address</b>,
    shares_to_transfer: u128,
) {
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder_1), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(shareholder_1) &gt;= shares_to_transfer, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));
    <b>if</b> (shares_to_transfer == 0) <b>return</b>;

    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(shareholder_1, shares_to_transfer);
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(shareholder_2, shares_to_transfer);
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_deduct_shares"></a>

## Function `deduct_shares`

Directly deduct <code>shareholder</code>'s number of shares in <code>self</code> and return the number of remaining shares.


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128 {
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(shareholder), <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND">ESHAREHOLDER_NOT_FOUND</a>));
    <b>assert</b>!(self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(shareholder) &gt;= num_shares, <a href="../../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_EINSUFFICIENT_SHARES">EINSUFFICIENT_SHARES</a>));

    <b>let</b> existing_shares = self.shares.borrow_mut(shareholder);
    *existing_shares -= num_shares;

    // Remove the shareholder completely <b>if</b> they have no shares left.
    <b>let</b> remaining_shares = *existing_shares;
    <b>if</b> (remaining_shares == 0) {
        self.shares.remove(shareholder);
    };

    remaining_shares
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares"></a>

## Function `amount_to_shares`

Return the number of new shares <code>coins_amount</code> can buy in <code>self</code>.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares">amount_to_shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64): u128 {
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(coins_amount, self.total_coins)
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares_with_total_coins"></a>

## Function `amount_to_shares_with_total_coins`

Return the number of new shares <code>coins_amount</code> can buy in <code>self</code> with a custom total coins number.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64, total_coins: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u128 {
    // No shares yet so amount is worth the same number of shares.
    <b>if</b> (self.total_coins == 0 || self.total_shares == 0) {
        // Multiply by scaling factor <b>to</b> minimize rounding errors during <b>internal</b> calculations for buy ins/redeems.
        // This can overflow but scaling factor is expected <b>to</b> be chosen carefully so this would not overflow.
        (coins_amount <b>as</b> u128) * (self.scaling_factor <b>as</b> u128)
    } <b>else</b> {
        // Shares price = total_coins / total existing shares.
        // New number of shares = new_amount / shares_price = new_amount * existing_shares / total_amount.
        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.
        self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(coins_amount <b>as</b> u128, self.total_shares, total_coins <b>as</b> u128)
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount"></a>

## Function `shares_to_amount`

Return the number of coins <code>shares</code> are worth in <code>self</code>.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount">shares_to_amount</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128): u64 {
    self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(shares, self.total_coins)
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_coins"></a>

## Function `shares_to_amount_with_total_coins`

Return the number of coins <code>shares</code> are worth in <code>self</code> with a custom total coins number.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128, total_coins: u64): u64 {
    // No shares or coins yet so shares are worthless.
    <b>if</b> (self.total_coins == 0 || self.total_shares == 0) {
        0
    } <b>else</b> {
        // Shares price = total_coins / total existing shares.
        // Shares worth = shares * shares price = shares * total_coins / total existing shares.
        // We rearrange the calc and do multiplication first <b>to</b> avoid rounding errors.
        (self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(shares, total_coins <b>as</b> u128, self.total_shares) <b>as</b> u64)
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_stats"></a>

## Function `shares_to_amount_with_total_stats`

Return the number of coins <code>shares</code> are worth in <code>pool</code> with custom total coins and shares numbers.


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_stats">shares_to_amount_with_total_stats</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64, total_shares: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_stats">shares_to_amount_with_total_stats</a>(
    self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>,
    shares: u128,
    total_coins: u64,
    total_shares: u128,
): u64 {
    <b>if</b> (self.total_coins == 0 || total_shares == 0) {
        0
    } <b>else</b> {
        (self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(shares, total_coins <b>as</b> u128, total_shares) <b>as</b> u64)
    }
}
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_multiply_then_divide"></a>

## Function `multiply_then_divide`



<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, x: u128, y: u128, z: u128): u128 {
    <a href="math128.md#0x1_math128_mul_div">math128::mul_div</a>(x, y, z)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Pool"></a>

### Struct `Pool`


<pre><code><b>struct</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a> <b>has</b> store
</code></pre>



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



<pre><code><b>invariant</b> <b>forall</b> addr: <b>address</b>:
    <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(shares, addr) ==&gt; (<a href="table.md#0x1_table_spec_get">table::spec_get</a>(shares, addr) &gt; 0);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_contains"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): bool {
   <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(pool.shares, shareholder)
}
</code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_contains">contains</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_shares"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shareholder: <b>address</b>): u64 {
   <b>if</b> (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(pool, shareholder)) {
       <a href="table.md#0x1_table_spec_get">table::spec_get</a>(pool.shares, shareholder)
   }
   <b>else</b> {
       0
   }
}
</code></pre>



<a id="@Specification_1_shares"></a>

### Function `shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares">shares</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u128
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder);
</code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_balance">balance</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>): u64
</code></pre>




<pre><code><b>let</b> shares = <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder);
<b>let</b> total_coins = self.total_coins;
<b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0 && (shares * total_coins) / self.total_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;
<b>ensures</b> result == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares, total_coins);
</code></pre>



<a id="@Specification_1_buy_in"></a>

### Function `buy_in`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_buy_in">buy_in</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, coins_amount: u64): u128
</code></pre>




<pre><code><b>let</b> new_shares = <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(self, coins_amount, self.total_coins);
<b>aborts_if</b> self.total_coins + coins_amount &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;
<b>aborts_if</b> self.total_shares + new_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;
<b>include</b> coins_amount &gt; 0 ==&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a> { new_shares };
<b>include</b> coins_amount &gt; 0 ==&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a> { new_shares };
<b>ensures</b> self.total_coins == <b>old</b>(self.total_coins) + coins_amount;
<b>ensures</b> self.total_shares == <b>old</b>(self.total_shares) + new_shares;
<b>ensures</b> result == new_shares;
</code></pre>



<a id="@Specification_1_add_shares"></a>

### Function `add_shares`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_add_shares">add_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, new_shares: u128): u128
</code></pre>




<pre><code><b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a>;
<b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a>;
<b>let</b> key_exists = <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(self.shares, shareholder);
<b>ensures</b> result == <b>if</b> (key_exists) { <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder) }
<b>else</b> { new_shares };
</code></pre>




<a id="0x1_pool_u64_unbound_AddSharesAbortsIf"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesAbortsIf">AddSharesAbortsIf</a> {
    self: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;
    shareholder: <b>address</b>;
    new_shares: u64;
    <b>let</b> key_exists = <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(self.shares, shareholder);
    <b>let</b> current_shares = <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder);
    <b>aborts_if</b> key_exists && current_shares + new_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;
}
</code></pre>




<a id="0x1_pool_u64_unbound_AddSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_AddSharesEnsures">AddSharesEnsures</a> {
    self: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;
    shareholder: <b>address</b>;
    new_shares: u64;
    <b>let</b> key_exists = <a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(self.shares, shareholder);
    <b>let</b> current_shares = <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder);
    <b>ensures</b> key_exists ==&gt;
        self.shares == <a href="table.md#0x1_table_spec_set">table::spec_set</a>(<b>old</b>(self.shares), shareholder, current_shares + new_shares);
    <b>ensures</b> (!key_exists && new_shares &gt; 0) ==&gt;
        self.shares == <a href="table.md#0x1_table_spec_set">table::spec_set</a>(<b>old</b>(self.shares), shareholder, new_shares);
}
</code></pre>




<a id="0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, coins_amount: u64, total_coins: u64): u128 {
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


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_redeem_shares">redeem_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, shares_to_redeem: u128): u64
</code></pre>




<pre><code><b>let</b> redeemed_coins = <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares_to_redeem, self.total_coins);
<b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder);
<b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder) &lt; shares_to_redeem;
<b>aborts_if</b> self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_coins">total_coins</a> &lt; redeemed_coins;
<b>aborts_if</b> self.<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_total_shares">total_shares</a> &lt; shares_to_redeem;
<b>ensures</b> self.total_coins == <b>old</b>(self.total_coins) - redeemed_coins;
<b>ensures</b> self.total_shares == <b>old</b>(self.total_shares) - shares_to_redeem;
<b>include</b> shares_to_redeem &gt; 0 ==&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a> {
    num_shares: shares_to_redeem
};
<b>ensures</b> result == redeemed_coins;
</code></pre>



<a id="@Specification_1_transfer_shares"></a>

### Function `transfer_shares`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_transfer_shares">transfer_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder_1: <b>address</b>, shareholder_2: <b>address</b>, shares_to_transfer: u128)
</code></pre>




<pre><code><b>aborts_if</b> (shareholder_1 != shareholder_2) && shares_to_transfer &gt; 0 && <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_2) &&
    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder_2) + shares_to_transfer &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>);
<b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_1);
<b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder_1) &lt; shares_to_transfer;
<b>ensures</b> shareholder_1 == shareholder_2 ==&gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(self), shareholder_1) == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(
    self, shareholder_1);
<b>ensures</b> ((shareholder_1 != shareholder_2) && (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(self), shareholder_1) == shares_to_transfer)) ==&gt;
    !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_1);
<b>ensures</b> (shareholder_1 != shareholder_2 && shares_to_transfer &gt; 0) ==&gt;
    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_2));
<b>ensures</b> (shareholder_1 != shareholder_2 && shares_to_transfer &gt; 0 && !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(<b>old</b>(self), shareholder_2)) ==&gt;
    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_2) && <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder_2) == shares_to_transfer);
<b>ensures</b> (shareholder_1 != shareholder_2 && shares_to_transfer &gt; 0 && <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(<b>old</b>(self), shareholder_2)) ==&gt;
    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_2) && <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder_2) == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(self), shareholder_2) + shares_to_transfer);
<b>ensures</b> ((shareholder_1 != shareholder_2) && (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(self), shareholder_1) &gt; shares_to_transfer)) ==&gt;
    (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder_1) && (<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder_1) == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(<b>old</b>(self), shareholder_1) - shares_to_transfer));
</code></pre>



<a id="@Specification_1_deduct_shares"></a>

### Function `deduct_shares`


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_deduct_shares">deduct_shares</a>(self: &<b>mut</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shareholder: <b>address</b>, num_shares: u128): u128
</code></pre>




<pre><code><b>aborts_if</b> !<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_contains">spec_contains</a>(self, shareholder);
<b>aborts_if</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares">spec_shares</a>(self, shareholder) &lt; num_shares;
<b>include</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a>;
<b>let</b> remaining_shares = <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder) - num_shares;
<b>ensures</b> remaining_shares &gt; 0 ==&gt; result == <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder);
<b>ensures</b> remaining_shares == 0 ==&gt; result == 0;
</code></pre>




<a id="0x1_pool_u64_unbound_DeductSharesEnsures"></a>


<pre><code><b>schema</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_DeductSharesEnsures">DeductSharesEnsures</a> {
    self: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>;
    shareholder: <b>address</b>;
    num_shares: u64;
    <b>let</b> remaining_shares = <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder) - num_shares;
    <b>ensures</b> remaining_shares &gt; 0 ==&gt; <a href="table.md#0x1_table_spec_get">table::spec_get</a>(self.shares, shareholder) == remaining_shares;
    <b>ensures</b> remaining_shares == 0 ==&gt; !<a href="table.md#0x1_table_spec_contains">table::spec_contains</a>(self.shares, shareholder);
}
</code></pre>



<a id="@Specification_1_amount_to_shares_with_total_coins"></a>

### Function `amount_to_shares_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_amount_to_shares_with_total_coins">amount_to_shares_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, coins_amount: u64, total_coins: u64): u128
</code></pre>




<pre><code><b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0
    && (coins_amount * self.total_shares) / total_coins &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;
<b>aborts_if</b> (self.total_coins == 0 || self.total_shares == 0)
    && coins_amount * self.scaling_factor &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;
<b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0 && total_coins == 0;
<b>ensures</b> result == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins">spec_amount_to_shares_with_total_coins</a>(self, coins_amount, total_coins);
</code></pre>



<a id="@Specification_1_shares_to_amount_with_total_coins"></a>

### Function `shares_to_amount_with_total_coins`


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_shares_to_amount_with_total_coins">shares_to_amount_with_total_coins</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, shares: u128, total_coins: u64): u64
</code></pre>




<pre><code><b>aborts_if</b> self.total_coins &gt; 0 && self.total_shares &gt; 0
    && (shares * total_coins) / self.total_shares &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U64">MAX_U64</a>;
<b>ensures</b> result == <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(self, shares, total_coins);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins"></a>


<pre><code><b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins">spec_shares_to_amount_with_total_coins</a>(pool: <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">Pool</a>, shares: u128, total_coins: u64): u64 {
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


<pre><code><b>public</b> <b>fun</b> <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_multiply_then_divide">multiply_then_divide</a>(self: &<a href="pool_u64_unbound.md#0x1_pool_u64_unbound_Pool">pool_u64_unbound::Pool</a>, x: u128, y: u128, z: u128): u128
</code></pre>




<pre><code><b>aborts_if</b> z == 0;
<b>aborts_if</b> (x * y) / z &gt; <a href="pool_u64_unbound.md#0x1_pool_u64_unbound_MAX_U128">MAX_U128</a>;
<b>ensures</b> result == (x * y) / z;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
