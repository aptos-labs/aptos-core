
<a name="0x1_TransactionFee"></a>

# Module `0x1::TransactionFee`



-  [Function `burn_fee`](#0x1_TransactionFee_burn_fee)


<pre><code><b>use</b> <a href="TestCoin.md#0x1_TestCoin">0x1::TestCoin</a>;
</code></pre>



<a name="0x1_TransactionFee_burn_fee"></a>

## Function `burn_fee`

Burn transaction fees in epilogue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="TransactionFee.md#0x1_TransactionFee_burn_fee">burn_fee</a>(fee: <a href="TestCoin.md#0x1_TestCoin_Coin">TestCoin::Coin</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="TransactionFee.md#0x1_TransactionFee_burn_fee">burn_fee</a>(fee: Coin) {
    <a href="TestCoin.md#0x1_TestCoin_burn_gas">TestCoin::burn_gas</a>(fee);
}
</code></pre>



</details>
