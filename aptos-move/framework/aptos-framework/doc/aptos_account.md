
<a name="0x1_aptos_account"></a>

# Module `0x1::aptos_account`



-  [Constants](#@Constants_0)
-  [Function `create_account`](#0x1_aptos_account_create_account)
-  [Function `transfer`](#0x1_aptos_account_transfer)
-  [Function `assert_account_exists`](#0x1_aptos_account_assert_account_exists)
-  [Function `assert_account_is_registered_for_apt`](#0x1_aptos_account_assert_account_is_registered_for_apt)
-  [Specification](#@Specification_1)
    -  [Function `create_account`](#@Specification_1_create_account)
    -  [Function `transfer`](#@Specification_1_transfer)
    -  [Function `assert_account_exists`](#@Specification_1_assert_account_exists)
    -  [Function `assert_account_is_registered_for_apt`](#@Specification_1_assert_account_is_registered_for_apt)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="aptos_coin.md#0x1_aptos_coin">0x1::aptos_coin</a>;
<b>use</b> <a href="coin.md#0x1_coin">0x1::coin</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x1_aptos_account_EACCOUNT_NOT_FOUND"></a>

Account does not exist.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a name="0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT"></a>

Account is not registered to receive APT.


<pre><code><b>const</b> <a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>: u64 = 2;
</code></pre>



<a name="0x1_aptos_account_create_account"></a>

## Function `create_account`

Basic account creation methods.


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>) {
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="account.md#0x1_account_create_account">account::create_account</a>(auth_key);
    <a href="coin.md#0x1_coin_register">coin::register</a>&lt;AptosCoin&gt;(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>);
}
</code></pre>



</details>

<a name="0x1_aptos_account_transfer"></a>

## Function `transfer`



<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64) {
    <b>if</b> (!<a href="account.md#0x1_account_exists_at">account::exists_at</a>(<b>to</b>)) {
        <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(<b>to</b>)
    };
    <a href="coin.md#0x1_coin_transfer">coin::transfer</a>&lt;AptosCoin&gt;(source, <b>to</b>, amount)
}
</code></pre>



</details>

<a name="0x1_aptos_account_assert_account_exists"></a>

## Function `assert_account_exists`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>) {
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_FOUND">EACCOUNT_NOT_FOUND</a>));
}
</code></pre>



</details>

<a name="0x1_aptos_account_assert_account_is_registered_for_apt"></a>

## Function `assert_account_is_registered_for_apt`



<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>) {
    <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr);
    <b>assert</b>!(<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="aptos_account.md#0x1_aptos_account_EACCOUNT_NOT_REGISTERED_FOR_APT">EACCOUNT_NOT_REGISTERED_FOR_APT</a>));
}
</code></pre>



</details>

<a name="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a name="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_create_account">create_account</a>(auth_key: <b>address</b>)
</code></pre>


Check if the bytes of the auth_key is 32.
The Account does not exist under the auth_key before creating the account.
Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code><b>include</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccount">CreateAccount</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);
<b>ensures</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(auth_key);
</code></pre>




<a name="0x1_aptos_account_CreateAccount"></a>


<pre><code><b>schema</b> <a href="aptos_account.md#0x1_aptos_account_CreateAccount">CreateAccount</a> {
    auth_key: <b>address</b>;
    <b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">account::Account</a>&gt;(auth_key);
    <b>aborts_if</b> <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(auth_key);
    <b>aborts_if</b> auth_key == @vm_reserved || auth_key == @aptos_framework || auth_key == @aptos_token;
    <b>aborts_if</b> <b>exists</b>&lt;<a href="coin.md#0x1_coin_CoinStore">coin::CoinStore</a>&lt;AptosCoin&gt;&gt;(auth_key);
}
</code></pre>




<a name="0x1_aptos_account_length_judgment"></a>


<pre><code><b>fun</b> <a href="aptos_account.md#0x1_aptos_account_length_judgment">length_judgment</a>(auth_key: <b>address</b>): bool {
   <b>use</b> std::bcs;

   <b>let</b> authentication_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(auth_key);
   len(authentication_key) != 32
}
</code></pre>



<a name="@Specification_1_transfer"></a>

### Function `transfer`


<pre><code><b>public</b> entry <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_transfer">transfer</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>




<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a name="@Specification_1_assert_account_exists"></a>

### Function `assert_account_exists`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_exists">assert_account_exists</a>(addr: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);
</code></pre>



<a name="@Specification_1_assert_account_is_registered_for_apt"></a>

### Function `assert_account_is_registered_for_apt`


<pre><code><b>public</b> <b>fun</b> <a href="aptos_account.md#0x1_aptos_account_assert_account_is_registered_for_apt">assert_account_is_registered_for_apt</a>(addr: <b>address</b>)
</code></pre>


Check if the address existed.
Check if the AptosCoin under the address existed.


<pre><code><b>aborts_if</b> !<a href="account.md#0x1_account_exists_at">account::exists_at</a>(addr);
<b>aborts_if</b> !<a href="coin.md#0x1_coin_is_account_registered">coin::is_account_registered</a>&lt;AptosCoin&gt;(addr);
</code></pre>


[move-book]: https://move-language.github.io/move/introduction.html
