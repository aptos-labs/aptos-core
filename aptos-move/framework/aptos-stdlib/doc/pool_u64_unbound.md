
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


<pre><code>use 0x1::error;
use 0x1::table_with_length;
</code></pre>



<a id="0x1_pool_u64_unbound_Pool"></a>

## Struct `Pool`



<pre><code>struct Pool has store
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
<code>shares: table_with_length::TableWithLength&lt;address, u128&gt;</code>
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



<pre><code>const MAX_U64: u64 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_pool_u64_unbound_MAX_U128"></a>



<pre><code>const MAX_U128: u128 &#61; 340282366920938463463374607431768211455;
</code></pre>



<a id="0x1_pool_u64_unbound_EINSUFFICIENT_SHARES"></a>

Cannot redeem more shares than the shareholder has in the pool.


<pre><code>const EINSUFFICIENT_SHARES: u64 &#61; 4;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_IS_NOT_EMPTY"></a>

Cannot destroy non-empty pool.


<pre><code>const EPOOL_IS_NOT_EMPTY: u64 &#61; 3;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_COINS_OVERFLOW"></a>

Pool's total coins cannot exceed u64.max.


<pre><code>const EPOOL_TOTAL_COINS_OVERFLOW: u64 &#61; 6;
</code></pre>



<a id="0x1_pool_u64_unbound_EPOOL_TOTAL_SHARES_OVERFLOW"></a>

Pool's total shares cannot exceed u64.max.


<pre><code>const EPOOL_TOTAL_SHARES_OVERFLOW: u64 &#61; 7;
</code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_NOT_FOUND"></a>

Shareholder not present in pool.


<pre><code>const ESHAREHOLDER_NOT_FOUND: u64 &#61; 1;
</code></pre>



<a id="0x1_pool_u64_unbound_ESHAREHOLDER_SHARES_OVERFLOW"></a>

Shareholder cannot have more than u64.max shares.


<pre><code>const ESHAREHOLDER_SHARES_OVERFLOW: u64 &#61; 5;
</code></pre>



<a id="0x1_pool_u64_unbound_ETOO_MANY_SHAREHOLDERS"></a>

There are too many shareholders in the pool.


<pre><code>const ETOO_MANY_SHAREHOLDERS: u64 &#61; 2;
</code></pre>



<a id="0x1_pool_u64_unbound_new"></a>

## Function `new`

Create a new pool.


<pre><code>public fun new(): pool_u64_unbound::Pool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new(): Pool &#123;
    // Default to a scaling factor of 1 (effectively no scaling).
    create_with_scaling_factor(1)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_create"></a>

## Function `create`

Deprecated. Use <code>new</code> instead.
Create a new pool.


<pre><code>&#35;[deprecated]
public fun create(): pool_u64_unbound::Pool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create(): Pool &#123;
    new()
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_create_with_scaling_factor"></a>

## Function `create_with_scaling_factor`

Create a new pool with custom <code>scaling_factor</code>.


<pre><code>public fun create_with_scaling_factor(scaling_factor: u64): pool_u64_unbound::Pool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_with_scaling_factor(scaling_factor: u64): Pool &#123;
    Pool &#123;
        total_coins: 0,
        total_shares: 0,
        shares: table::new&lt;address, u128&gt;(),
        scaling_factor,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_destroy_empty"></a>

## Function `destroy_empty`

Destroy an empty pool. This will fail if the pool has any balance of coins.


<pre><code>public fun destroy_empty(pool: pool_u64_unbound::Pool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun destroy_empty(pool: Pool) &#123;
    assert!(pool.total_coins &#61;&#61; 0, error::invalid_state(EPOOL_IS_NOT_EMPTY));
    let Pool &#123;
        total_coins: _,
        total_shares: _,
        shares,
        scaling_factor: _,
    &#125; &#61; pool;
    table::destroy_empty&lt;address, u128&gt;(shares);
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_coins"></a>

## Function `total_coins`

Return <code>pool</code>'s total balance of coins.


<pre><code>public fun total_coins(pool: &amp;pool_u64_unbound::Pool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun total_coins(pool: &amp;Pool): u64 &#123;
    pool.total_coins
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_total_shares"></a>

## Function `total_shares`

Return the total number of shares across all shareholders in <code>pool</code>.


<pre><code>public fun total_shares(pool: &amp;pool_u64_unbound::Pool): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun total_shares(pool: &amp;Pool): u128 &#123;
    pool.total_shares
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_contains"></a>

## Function `contains`

Return true if <code>shareholder</code> is in <code>pool</code>.


<pre><code>public fun contains(pool: &amp;pool_u64_unbound::Pool, shareholder: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun contains(pool: &amp;Pool, shareholder: address): bool &#123;
    table::contains(&amp;pool.shares, shareholder)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares"></a>

## Function `shares`

Return the number of shares of <code>stakeholder</code> in <code>pool</code>.


<pre><code>public fun shares(pool: &amp;pool_u64_unbound::Pool, shareholder: address): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shares(pool: &amp;Pool, shareholder: address): u128 &#123;
    if (contains(pool, shareholder)) &#123;
        &#42;table::borrow(&amp;pool.shares, shareholder)
    &#125; else &#123;
        0
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_balance"></a>

## Function `balance`

Return the balance in coins of <code>shareholder</code> in <code>pool.</code>


<pre><code>public fun balance(pool: &amp;pool_u64_unbound::Pool, shareholder: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun balance(pool: &amp;Pool, shareholder: address): u64 &#123;
    let num_shares &#61; shares(pool, shareholder);
    shares_to_amount(pool, num_shares)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shareholders_count"></a>

## Function `shareholders_count`

Return the number of shareholders in <code>pool</code>.


<pre><code>public fun shareholders_count(pool: &amp;pool_u64_unbound::Pool): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shareholders_count(pool: &amp;Pool): u64 &#123;
    table::length(&amp;pool.shares)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_update_total_coins"></a>

## Function `update_total_coins`

Update <code>pool</code>'s total balance of coins.


<pre><code>public fun update_total_coins(pool: &amp;mut pool_u64_unbound::Pool, new_total_coins: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_total_coins(pool: &amp;mut Pool, new_total_coins: u64) &#123;
    pool.total_coins &#61; new_total_coins;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_buy_in"></a>

## Function `buy_in`

Allow an existing or new shareholder to add their coins to the pool in exchange for new shares.


<pre><code>public fun buy_in(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun buy_in(pool: &amp;mut Pool, shareholder: address, coins_amount: u64): u128 &#123;
    if (coins_amount &#61;&#61; 0) return 0;

    let new_shares &#61; amount_to_shares(pool, coins_amount);
    assert!(MAX_U64 &#45; pool.total_coins &gt;&#61; coins_amount, error::invalid_argument(EPOOL_TOTAL_COINS_OVERFLOW));
    assert!(MAX_U128 &#45; pool.total_shares &gt;&#61; new_shares, error::invalid_argument(EPOOL_TOTAL_SHARES_OVERFLOW));

    pool.total_coins &#61; pool.total_coins &#43; coins_amount;
    pool.total_shares &#61; pool.total_shares &#43; new_shares;
    add_shares(pool, shareholder, new_shares);
    new_shares
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_add_shares"></a>

## Function `add_shares`

Add the number of shares directly for <code>shareholder</code> in <code>pool</code>.
This would dilute other shareholders if the pool's balance of coins didn't change.


<pre><code>fun add_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, new_shares: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun add_shares(pool: &amp;mut Pool, shareholder: address, new_shares: u128): u128 &#123;
    if (contains(pool, shareholder)) &#123;
        let existing_shares &#61; table::borrow_mut(&amp;mut pool.shares, shareholder);
        let current_shares &#61; &#42;existing_shares;
        assert!(MAX_U128 &#45; current_shares &gt;&#61; new_shares, error::invalid_argument(ESHAREHOLDER_SHARES_OVERFLOW));

        &#42;existing_shares &#61; current_shares &#43; new_shares;
        &#42;existing_shares
    &#125; else if (new_shares &gt; 0) &#123;
        table::add(&amp;mut pool.shares, shareholder, new_shares);
        new_shares
    &#125; else &#123;
        new_shares
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_redeem_shares"></a>

## Function `redeem_shares`

Allow <code>shareholder</code> to redeem their shares in <code>pool</code> for coins.


<pre><code>public fun redeem_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, shares_to_redeem: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun redeem_shares(pool: &amp;mut Pool, shareholder: address, shares_to_redeem: u128): u64 &#123;
    assert!(contains(pool, shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
    assert!(shares(pool, shareholder) &gt;&#61; shares_to_redeem, error::invalid_argument(EINSUFFICIENT_SHARES));

    if (shares_to_redeem &#61;&#61; 0) return 0;

    let redeemed_coins &#61; shares_to_amount(pool, shares_to_redeem);
    pool.total_coins &#61; pool.total_coins &#45; redeemed_coins;
    pool.total_shares &#61; pool.total_shares &#45; shares_to_redeem;
    deduct_shares(pool, shareholder, shares_to_redeem);

    redeemed_coins
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_transfer_shares"></a>

## Function `transfer_shares`

Transfer shares from <code>shareholder_1</code> to <code>shareholder_2</code>.


<pre><code>public fun transfer_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder_1: address, shareholder_2: address, shares_to_transfer: u128)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transfer_shares(
    pool: &amp;mut Pool,
    shareholder_1: address,
    shareholder_2: address,
    shares_to_transfer: u128,
) &#123;
    assert!(contains(pool, shareholder_1), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
    assert!(shares(pool, shareholder_1) &gt;&#61; shares_to_transfer, error::invalid_argument(EINSUFFICIENT_SHARES));
    if (shares_to_transfer &#61;&#61; 0) return;

    deduct_shares(pool, shareholder_1, shares_to_transfer);
    add_shares(pool, shareholder_2, shares_to_transfer);
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_deduct_shares"></a>

## Function `deduct_shares`

Directly deduct <code>shareholder</code>'s number of shares in <code>pool</code> and return the number of remaining shares.


<pre><code>fun deduct_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, num_shares: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun deduct_shares(pool: &amp;mut Pool, shareholder: address, num_shares: u128): u128 &#123;
    assert!(contains(pool, shareholder), error::invalid_argument(ESHAREHOLDER_NOT_FOUND));
    assert!(shares(pool, shareholder) &gt;&#61; num_shares, error::invalid_argument(EINSUFFICIENT_SHARES));

    let existing_shares &#61; table::borrow_mut(&amp;mut pool.shares, shareholder);
    &#42;existing_shares &#61; &#42;existing_shares &#45; num_shares;

    // Remove the shareholder completely if they have no shares left.
    let remaining_shares &#61; &#42;existing_shares;
    if (remaining_shares &#61;&#61; 0) &#123;
        table::remove(&amp;mut pool.shares, shareholder);
    &#125;;

    remaining_shares
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares"></a>

## Function `amount_to_shares`

Return the number of new shares <code>coins_amount</code> can buy in <code>pool</code>.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code>public fun amount_to_shares(pool: &amp;pool_u64_unbound::Pool, coins_amount: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun amount_to_shares(pool: &amp;Pool, coins_amount: u64): u128 &#123;
    amount_to_shares_with_total_coins(pool, coins_amount, pool.total_coins)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_amount_to_shares_with_total_coins"></a>

## Function `amount_to_shares_with_total_coins`

Return the number of new shares <code>coins_amount</code> can buy in <code>pool</code> with a custom total coins number.
<code>amount</code> needs to big enough to avoid rounding number.


<pre><code>public fun amount_to_shares_with_total_coins(pool: &amp;pool_u64_unbound::Pool, coins_amount: u64, total_coins: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun amount_to_shares_with_total_coins(pool: &amp;Pool, coins_amount: u64, total_coins: u64): u128 &#123;
    // No shares yet so amount is worth the same number of shares.
    if (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;
        // Multiply by scaling factor to minimize rounding errors during internal calculations for buy ins/redeems.
        // This can overflow but scaling factor is expected to be chosen carefully so this would not overflow.
        to_u128(coins_amount) &#42; to_u128(pool.scaling_factor)
    &#125; else &#123;
        // Shares price &#61; total_coins / total existing shares.
        // New number of shares &#61; new_amount / shares_price &#61; new_amount &#42; existing_shares / total_amount.
        // We rearrange the calc and do multiplication first to avoid rounding errors.
        multiply_then_divide(pool, to_u128(coins_amount), pool.total_shares, to_u128(total_coins))
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount"></a>

## Function `shares_to_amount`

Return the number of coins <code>shares</code> are worth in <code>pool</code>.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code>public fun shares_to_amount(pool: &amp;pool_u64_unbound::Pool, shares: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shares_to_amount(pool: &amp;Pool, shares: u128): u64 &#123;
    shares_to_amount_with_total_coins(pool, shares, pool.total_coins)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_coins"></a>

## Function `shares_to_amount_with_total_coins`

Return the number of coins <code>shares</code> are worth in <code>pool</code> with a custom total coins number.
<code>shares</code> needs to big enough to avoid rounding number.


<pre><code>public fun shares_to_amount_with_total_coins(pool: &amp;pool_u64_unbound::Pool, shares: u128, total_coins: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shares_to_amount_with_total_coins(pool: &amp;Pool, shares: u128, total_coins: u64): u64 &#123;
    // No shares or coins yet so shares are worthless.
    if (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;
        0
    &#125; else &#123;
        // Shares price &#61; total_coins / total existing shares.
        // Shares worth &#61; shares &#42; shares price &#61; shares &#42; total_coins / total existing shares.
        // We rearrange the calc and do multiplication first to avoid rounding errors.
        (multiply_then_divide(pool, shares, to_u128(total_coins), pool.total_shares) as u64)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_shares_to_amount_with_total_stats"></a>

## Function `shares_to_amount_with_total_stats`

Return the number of coins <code>shares</code> are worth in <code>pool</code> with custom total coins and shares numbers.


<pre><code>public fun shares_to_amount_with_total_stats(pool: &amp;pool_u64_unbound::Pool, shares: u128, total_coins: u64, total_shares: u128): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun shares_to_amount_with_total_stats(
    pool: &amp;Pool,
    shares: u128,
    total_coins: u64,
    total_shares: u128,
): u64 &#123;
    if (pool.total_coins &#61;&#61; 0 &#124;&#124; total_shares &#61;&#61; 0) &#123;
        0
    &#125; else &#123;
        (multiply_then_divide(pool, shares, to_u128(total_coins), total_shares) as u64)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_multiply_then_divide"></a>

## Function `multiply_then_divide`



<pre><code>public fun multiply_then_divide(_pool: &amp;pool_u64_unbound::Pool, x: u128, y: u128, z: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multiply_then_divide(_pool: &amp;Pool, x: u128, y: u128, z: u128): u128 &#123;
    let result &#61; (to_u256(x) &#42; to_u256(y)) / to_u256(z);
    (result as u128)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_to_u128"></a>

## Function `to_u128`



<pre><code>fun to_u128(num: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun to_u128(num: u64): u128 &#123;
    (num as u128)
&#125;
</code></pre>



</details>

<a id="0x1_pool_u64_unbound_to_u256"></a>

## Function `to_u256`



<pre><code>fun to_u256(num: u128): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun to_u256(num: u128): u256 &#123;
    (num as u256)
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Pool"></a>

### Struct `Pool`


<pre><code>struct Pool has store
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
<code>shares: table_with_length::TableWithLength&lt;address, u128&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>scaling_factor: u64</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>invariant forall addr: address:
    table::spec_contains(shares, addr) &#61;&#61;&gt; (table::spec_get(shares, addr) &gt; 0);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_contains"></a>


<pre><code>fun spec_contains(pool: Pool, shareholder: address): bool &#123;
   table::spec_contains(pool.shares, shareholder)
&#125;
</code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>public fun contains(pool: &amp;pool_u64_unbound::Pool, shareholder: address): bool
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_contains(pool, shareholder);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_shares"></a>


<pre><code>fun spec_shares(pool: Pool, shareholder: address): u64 &#123;
   if (spec_contains(pool, shareholder)) &#123;
       table::spec_get(pool.shares, shareholder)
   &#125;
   else &#123;
       0
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_shares"></a>

### Function `shares`


<pre><code>public fun shares(pool: &amp;pool_u64_unbound::Pool, shareholder: address): u128
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; spec_shares(pool, shareholder);
</code></pre>



<a id="@Specification_1_balance"></a>

### Function `balance`


<pre><code>public fun balance(pool: &amp;pool_u64_unbound::Pool, shareholder: address): u64
</code></pre>




<pre><code>let shares &#61; spec_shares(pool, shareholder);
let total_coins &#61; pool.total_coins;
aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0 &amp;&amp; (shares &#42; total_coins) / pool.total_shares &gt; MAX_U64;
ensures result &#61;&#61; spec_shares_to_amount_with_total_coins(pool, shares, total_coins);
</code></pre>



<a id="@Specification_1_buy_in"></a>

### Function `buy_in`


<pre><code>public fun buy_in(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, coins_amount: u64): u128
</code></pre>




<pre><code>let new_shares &#61; spec_amount_to_shares_with_total_coins(pool, coins_amount, pool.total_coins);
aborts_if pool.total_coins &#43; coins_amount &gt; MAX_U64;
aborts_if pool.total_shares &#43; new_shares &gt; MAX_U128;
include coins_amount &gt; 0 &#61;&#61;&gt; AddSharesAbortsIf &#123; new_shares: new_shares &#125;;
include coins_amount &gt; 0 &#61;&#61;&gt; AddSharesEnsures &#123; new_shares: new_shares &#125;;
ensures pool.total_coins &#61;&#61; old(pool.total_coins) &#43; coins_amount;
ensures pool.total_shares &#61;&#61; old(pool.total_shares) &#43; new_shares;
ensures result &#61;&#61; new_shares;
</code></pre>



<a id="@Specification_1_add_shares"></a>

### Function `add_shares`


<pre><code>fun add_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, new_shares: u128): u128
</code></pre>




<pre><code>include AddSharesAbortsIf;
include AddSharesEnsures;
let key_exists &#61; table::spec_contains(pool.shares, shareholder);
ensures result &#61;&#61; if (key_exists) &#123; table::spec_get(pool.shares, shareholder) &#125;
else &#123; new_shares &#125;;
</code></pre>




<a id="0x1_pool_u64_unbound_AddSharesAbortsIf"></a>


<pre><code>schema AddSharesAbortsIf &#123;
    pool: Pool;
    shareholder: address;
    new_shares: u64;
    let key_exists &#61; table::spec_contains(pool.shares, shareholder);
    let current_shares &#61; table::spec_get(pool.shares, shareholder);
    aborts_if key_exists &amp;&amp; current_shares &#43; new_shares &gt; MAX_U128;
&#125;
</code></pre>




<a id="0x1_pool_u64_unbound_AddSharesEnsures"></a>


<pre><code>schema AddSharesEnsures &#123;
    pool: Pool;
    shareholder: address;
    new_shares: u64;
    let key_exists &#61; table::spec_contains(pool.shares, shareholder);
    let current_shares &#61; table::spec_get(pool.shares, shareholder);
    ensures key_exists &#61;&#61;&gt;
        pool.shares &#61;&#61; table::spec_set(old(pool.shares), shareholder, current_shares &#43; new_shares);
    ensures (!key_exists &amp;&amp; new_shares &gt; 0) &#61;&#61;&gt;
        pool.shares &#61;&#61; table::spec_set(old(pool.shares), shareholder, new_shares);
&#125;
</code></pre>




<a id="0x1_pool_u64_unbound_spec_amount_to_shares_with_total_coins"></a>


<pre><code>fun spec_amount_to_shares_with_total_coins(pool: Pool, coins_amount: u64, total_coins: u64): u128 &#123;
   if (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;
       coins_amount &#42; pool.scaling_factor
   &#125;
   else &#123;
       (coins_amount &#42; pool.total_shares) / total_coins
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_redeem_shares"></a>

### Function `redeem_shares`


<pre><code>public fun redeem_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, shares_to_redeem: u128): u64
</code></pre>




<pre><code>let redeemed_coins &#61; spec_shares_to_amount_with_total_coins(pool, shares_to_redeem, pool.total_coins);
aborts_if !spec_contains(pool, shareholder);
aborts_if spec_shares(pool, shareholder) &lt; shares_to_redeem;
aborts_if pool.total_coins &lt; redeemed_coins;
aborts_if pool.total_shares &lt; shares_to_redeem;
ensures pool.total_coins &#61;&#61; old(pool.total_coins) &#45; redeemed_coins;
ensures pool.total_shares &#61;&#61; old(pool.total_shares) &#45; shares_to_redeem;
include shares_to_redeem &gt; 0 &#61;&#61;&gt; DeductSharesEnsures &#123; num_shares: shares_to_redeem &#125;;
ensures result &#61;&#61; redeemed_coins;
</code></pre>



<a id="@Specification_1_transfer_shares"></a>

### Function `transfer_shares`


<pre><code>public fun transfer_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder_1: address, shareholder_2: address, shares_to_transfer: u128)
</code></pre>




<pre><code>aborts_if (shareholder_1 !&#61; shareholder_2) &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; spec_contains(pool, shareholder_2) &amp;&amp;
    (spec_shares(pool, shareholder_2) &#43; shares_to_transfer &gt; MAX_U128);
aborts_if !spec_contains(pool, shareholder_1);
aborts_if spec_shares(pool, shareholder_1) &lt; shares_to_transfer;
ensures shareholder_1 &#61;&#61; shareholder_2 &#61;&#61;&gt; spec_shares(old(pool), shareholder_1) &#61;&#61; spec_shares(pool, shareholder_1);
ensures ((shareholder_1 !&#61; shareholder_2) &amp;&amp; (spec_shares(old(pool), shareholder_1) &#61;&#61; shares_to_transfer)) &#61;&#61;&gt;
    !spec_contains(pool, shareholder_1);
ensures (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0) &#61;&#61;&gt;
    (spec_contains(pool, shareholder_2));
ensures (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; !spec_contains(old(pool), shareholder_2)) &#61;&#61;&gt;
    (spec_contains(pool, shareholder_2) &amp;&amp; spec_shares(pool, shareholder_2) &#61;&#61; shares_to_transfer);
ensures (shareholder_1 !&#61; shareholder_2 &amp;&amp; shares_to_transfer &gt; 0 &amp;&amp; spec_contains(old(pool), shareholder_2)) &#61;&#61;&gt;
    (spec_contains(pool, shareholder_2) &amp;&amp; spec_shares(pool, shareholder_2) &#61;&#61; spec_shares(old(pool), shareholder_2) &#43; shares_to_transfer);
ensures ((shareholder_1 !&#61; shareholder_2) &amp;&amp; (spec_shares(old(pool), shareholder_1) &gt; shares_to_transfer)) &#61;&#61;&gt;
    (spec_contains(pool, shareholder_1) &amp;&amp; (spec_shares(pool, shareholder_1) &#61;&#61; spec_shares(old(pool), shareholder_1) &#45; shares_to_transfer));
</code></pre>



<a id="@Specification_1_deduct_shares"></a>

### Function `deduct_shares`


<pre><code>fun deduct_shares(pool: &amp;mut pool_u64_unbound::Pool, shareholder: address, num_shares: u128): u128
</code></pre>




<pre><code>aborts_if !spec_contains(pool, shareholder);
aborts_if spec_shares(pool, shareholder) &lt; num_shares;
include DeductSharesEnsures;
let remaining_shares &#61; table::spec_get(pool.shares, shareholder) &#45; num_shares;
ensures remaining_shares &gt; 0 &#61;&#61;&gt; result &#61;&#61; table::spec_get(pool.shares, shareholder);
ensures remaining_shares &#61;&#61; 0 &#61;&#61;&gt; result &#61;&#61; 0;
</code></pre>




<a id="0x1_pool_u64_unbound_DeductSharesEnsures"></a>


<pre><code>schema DeductSharesEnsures &#123;
    pool: Pool;
    shareholder: address;
    num_shares: u64;
    let remaining_shares &#61; table::spec_get(pool.shares, shareholder) &#45; num_shares;
    ensures remaining_shares &gt; 0 &#61;&#61;&gt; table::spec_get(pool.shares, shareholder) &#61;&#61; remaining_shares;
    ensures remaining_shares &#61;&#61; 0 &#61;&#61;&gt; !table::spec_contains(pool.shares, shareholder);
&#125;
</code></pre>



<a id="@Specification_1_amount_to_shares_with_total_coins"></a>

### Function `amount_to_shares_with_total_coins`


<pre><code>public fun amount_to_shares_with_total_coins(pool: &amp;pool_u64_unbound::Pool, coins_amount: u64, total_coins: u64): u128
</code></pre>




<pre><code>aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0
    &amp;&amp; (coins_amount &#42; pool.total_shares) / total_coins &gt; MAX_U128;
aborts_if (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0)
    &amp;&amp; coins_amount &#42; pool.scaling_factor &gt; MAX_U128;
aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0 &amp;&amp; total_coins &#61;&#61; 0;
ensures result &#61;&#61; spec_amount_to_shares_with_total_coins(pool, coins_amount, total_coins);
</code></pre>



<a id="@Specification_1_shares_to_amount_with_total_coins"></a>

### Function `shares_to_amount_with_total_coins`


<pre><code>public fun shares_to_amount_with_total_coins(pool: &amp;pool_u64_unbound::Pool, shares: u128, total_coins: u64): u64
</code></pre>




<pre><code>aborts_if pool.total_coins &gt; 0 &amp;&amp; pool.total_shares &gt; 0
    &amp;&amp; (shares &#42; total_coins) / pool.total_shares &gt; MAX_U64;
ensures result &#61;&#61; spec_shares_to_amount_with_total_coins(pool, shares, total_coins);
</code></pre>




<a id="0x1_pool_u64_unbound_spec_shares_to_amount_with_total_coins"></a>


<pre><code>fun spec_shares_to_amount_with_total_coins(pool: Pool, shares: u128, total_coins: u64): u64 &#123;
   if (pool.total_coins &#61;&#61; 0 &#124;&#124; pool.total_shares &#61;&#61; 0) &#123;
       0
   &#125;
   else &#123;
       (shares &#42; total_coins) / pool.total_shares
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_multiply_then_divide"></a>

### Function `multiply_then_divide`


<pre><code>public fun multiply_then_divide(_pool: &amp;pool_u64_unbound::Pool, x: u128, y: u128, z: u128): u128
</code></pre>




<pre><code>aborts_if z &#61;&#61; 0;
aborts_if (x &#42; y) / z &gt; MAX_U128;
ensures result &#61;&#61; (x &#42; y) / z;
</code></pre>



<a id="@Specification_1_to_u128"></a>

### Function `to_u128`


<pre><code>fun to_u128(num: u64): u128
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; num;
</code></pre>



<a id="@Specification_1_to_u256"></a>

### Function `to_u256`


<pre><code>fun to_u256(num: u128): u256
</code></pre>




<pre><code>aborts_if false;
ensures result &#61;&#61; num;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
