
<a name="0x1_BasicScripts"></a>

# Module `0x1::BasicScripts`



-  [Function `create_account`](#0x1_BasicScripts_create_account)
-  [Function `transfer`](#0x1_BasicScripts_transfer)
-  [Function `rotate_authentication_key`](#0x1_BasicScripts_rotate_authentication_key)


<pre><code><b>use</b> <a href="AptosAccount.md#0x1_AptosAccount">0x1::AptosAccount</a>;
<b>use</b> <a href="TestCoin.md#0x1_TestCoin">0x1::TestCoin</a>;
</code></pre>



<a name="0x1_BasicScripts_create_account"></a>

## Function `create_account`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_create_account">create_account</a>(new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_create_account">create_account</a>(
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
) {
    <b>let</b> signer = <a href="AptosAccount.md#0x1_AptosAccount_create_account">AptosAccount::create_account</a>(new_account_address, auth_key_prefix);
    <a href="TestCoin.md#0x1_TestCoin_register">TestCoin::register</a>(&signer);
}
</code></pre>



</details>

<a name="0x1_BasicScripts_transfer"></a>

## Function `transfer`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_transfer">transfer</a>(from: signer, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_transfer">transfer</a>(from: signer, <b>to</b>: <b>address</b>, amount: u64){
    <a href="TestCoin.md#0x1_TestCoin_transfer">TestCoin::transfer</a>(&from, <b>to</b>, amount)
}
</code></pre>



</details>

<a name="0x1_BasicScripts_rotate_authentication_key"></a>

## Function `rotate_authentication_key`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_rotate_authentication_key">rotate_authentication_key</a>(account: signer, new_authentication_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="BasicScripts.md#0x1_BasicScripts_rotate_authentication_key">rotate_authentication_key</a>(
    account: signer,
    new_authentication_key: vector&lt;u8&gt;,
) {
    <a href="AptosAccount.md#0x1_AptosAccount_rotate_authentication_key">AptosAccount::rotate_authentication_key</a>(&account, new_authentication_key)
}
</code></pre>



</details>
