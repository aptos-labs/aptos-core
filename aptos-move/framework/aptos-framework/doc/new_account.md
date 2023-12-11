
<a id="0x1_new_account"></a>

# Module `0x1::new_account`



-  [Struct `NewAccountResourceGroup`](#0x1_new_account_NewAccountResourceGroup)
-  [Resource `Account`](#0x1_new_account_Account)
-  [Resource `NativeAuthenticator`](#0x1_new_account_NativeAuthenticator)
-  [Resource `CustomizedAuthenticator`](#0x1_new_account_CustomizedAuthenticator)
-  [Constants](#@Constants_0)
-  [Function `gen_native_authenticator`](#0x1_new_account_gen_native_authenticator)
-  [Function `gen_customized_authenticator`](#0x1_new_account_gen_customized_authenticator)
-  [Function `update_native_authenticator`](#0x1_new_account_update_native_authenticator)
-  [Function `update_native_authenticator_impl`](#0x1_new_account_update_native_authenticator_impl)
-  [Function `update_customized_authenticator_impl`](#0x1_new_account_update_customized_authenticator_impl)
-  [Function `create_account`](#0x1_new_account_create_account)
-  [Function `create_account_unchecked`](#0x1_new_account_create_account_unchecked)
-  [Function `exists_at`](#0x1_new_account_exists_at)
-  [Function `using_native_authenticator`](#0x1_new_account_using_native_authenticator)
-  [Function `using_customized_authenticator`](#0x1_new_account_using_customized_authenticator)
-  [Function `get_sequence_number`](#0x1_new_account_get_sequence_number)
-  [Function `get_native_authentication_key`](#0x1_new_account_get_native_authentication_key)
-  [Function `increment_sequence_number`](#0x1_new_account_increment_sequence_number)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_new_account_NewAccountResourceGroup"></a>

## Struct `NewAccountResourceGroup`

A shared resource group for storing new account resources together in storage.


<pre><code>#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="new_account.md#0x1_new_account_NewAccountResourceGroup">NewAccountResourceGroup</a>
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_new_account_Account"></a>

## Resource `Account`

Resource representing an account object.


<pre><code>#[resource_group_member(#[group = <a href="new_account.md#0x1_new_account_NewAccountResourceGroup">0x1::new_account::NewAccountResourceGroup</a>])]
<b>struct</b> <a href="new_account.md#0x1_new_account_Account">Account</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_new_account_NativeAuthenticator"></a>

## Resource `NativeAuthenticator`

The native authenticator where the key is used for authenticator verification in native code.


<pre><code>#[resource_group_member(#[group = <a href="new_account.md#0x1_new_account_NewAccountResourceGroup">0x1::new_account::NewAccountResourceGroup</a>])]
<b>struct</b> <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_new_account_CustomizedAuthenticator"></a>

## Resource `CustomizedAuthenticator`

The customized authenticator that defines how to authenticates this account in the specified module.
An integral part of Account Abstraction.
UNIMPLEMENTED.


<pre><code>#[resource_group_member(#[group = <a href="new_account.md#0x1_new_account_NewAccountResourceGroup">0x1::new_account::NewAccountResourceGroup</a>])]
<b>struct</b> <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_new_account_MAX_U64"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_new_account_ECANNOT_RESERVED_ADDRESS"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>: u64 = 2;
</code></pre>



<a id="0x1_new_account_EACCOUNT_EXISTENCE"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>: u64 = 1;
</code></pre>



<a id="0x1_new_account_ENATIVE_AUTHENTICATOR_EXISTENCE"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_ENATIVE_AUTHENTICATOR_EXISTENCE">ENATIVE_AUTHENTICATOR_EXISTENCE</a>: u64 = 5;
</code></pre>



<a id="0x1_new_account_ENOT_OWNER"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_ENOT_OWNER">ENOT_OWNER</a>: u64 = 4;
</code></pre>



<a id="0x1_new_account_ESEQUENCE_NUMBER_OVERFLOW"></a>



<pre><code><b>const</b> <a href="new_account.md#0x1_new_account_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>: u64 = 3;
</code></pre>



<a id="0x1_new_account_gen_native_authenticator"></a>

## Function `gen_native_authenticator`

The function to generate native authenticator resource.


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_gen_native_authenticator">gen_native_authenticator</a>(key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="new_account.md#0x1_new_account_NativeAuthenticator">new_account::NativeAuthenticator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_gen_native_authenticator">gen_native_authenticator</a>(key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
        key,
    }
}
</code></pre>



</details>

<a id="0x1_new_account_gen_customized_authenticator"></a>

## Function `gen_customized_authenticator`

The function to generate customized authenticator resource.


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_gen_customized_authenticator">gen_customized_authenticator</a>(account_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">new_account::CustomizedAuthenticator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_gen_customized_authenticator">gen_customized_authenticator</a>(
    account_address: <b>address</b>,
    module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a> {
    <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a> { account_address, module_name }
}
</code></pre>



</details>

<a id="0x1_new_account_update_native_authenticator"></a>

## Function `update_native_authenticator`

Update native authenticator, FKA account rotation.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="new_account.md#0x1_new_account_update_native_authenticator">update_native_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="new_account.md#0x1_new_account_update_native_authenticator">update_native_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="new_account.md#0x1_new_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>, <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
        key,
    });
}
</code></pre>



</details>

<a id="0x1_new_account_update_native_authenticator_impl"></a>

## Function `update_native_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, authenticator: <a href="new_account.md#0x1_new_account_NativeAuthenticator">new_account::NativeAuthenticator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    authenticator: <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>
) <b>acquires</b> <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="new_account.md#0x1_new_account_exists_at">exists_at</a>(account_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="new_account.md#0x1_new_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address)) {
        <b>move_from</b>&lt;<a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address);
    };
    <b>if</b> (<b>exists</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address)) {
        <b>let</b> current = <b>borrow_global_mut</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address);
        <b>if</b> (*current != authenticator) {
            *current = authenticator;
        }
    } <b>else</b> {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, authenticator)
    }
}
</code></pre>



</details>

<a id="0x1_new_account_update_customized_authenticator_impl"></a>

## Function `update_customized_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_update_customized_authenticator_impl">update_customized_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, authenticator: <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">new_account::CustomizedAuthenticator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_update_customized_authenticator_impl">update_customized_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    authenticator: <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>
) <b>acquires</b> <a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="new_account.md#0x1_new_account_exists_at">exists_at</a>(account_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="new_account.md#0x1_new_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address)) {
        <b>move_from</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address);
    };
    <b>if</b> (<b>exists</b>&lt;<a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address)) {
        <b>let</b> current = <b>borrow_global_mut</b>&lt;<a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address);
        <b>if</b> (*current != authenticator) {
            *current = authenticator;
        }
    } <b>else</b> {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, authenticator)
    }
}
</code></pre>



</details>

<a id="0x1_new_account_create_account"></a>

## Function `create_account`

Publishes a new <code><a href="new_account.md#0x1_new_account_Account">Account</a></code> resource under <code>new_address</code>. A ConstructorRef representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    // there cannot be an <a href="new_account.md#0x1_new_account_Account">Account</a> resource under new_addr already.
    <b>assert</b>!(!<a href="new_account.md#0x1_new_account_exists_at">exists_at</a>(new_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="new_account.md#0x1_new_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));

    // NOTE: @core_resources gets created via a `create_account` call, so we do not <b>include</b> it below.
    <b>assert</b>!(
        new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="new_account.md#0x1_new_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)
    );
    <a href="new_account.md#0x1_new_account_create_account_unchecked">create_account_unchecked</a>(new_address)
}
</code></pre>



</details>

<a id="0x1_new_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code><b>fun</b> <a href="new_account.md#0x1_new_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="new_account.md#0x1_new_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> <a href="new_account.md#0x1_new_account">new_account</a> = <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(new_address);
    <b>move_to</b>(
        &<a href="new_account.md#0x1_new_account">new_account</a>,
        <a href="new_account.md#0x1_new_account_Account">Account</a> {
            sequence_number: 0,
        }
    );
    <b>move_to</b>(&<a href="new_account.md#0x1_new_account">new_account</a>,
        <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
            key: <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&new_address)
        }
    );
    <a href="new_account.md#0x1_new_account">new_account</a>
}
</code></pre>



</details>

<a id="0x1_new_account_exists_at"></a>

## Function `exists_at`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="new_account.md#0x1_new_account_Account">Account</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_new_account_using_native_authenticator"></a>

## Function `using_native_authenticator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_new_account_using_customized_authenticator"></a>

## Function `using_customized_authenticator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_using_customized_authenticator">using_customized_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_using_customized_authenticator">using_customized_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="new_account.md#0x1_new_account_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_new_account_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="new_account.md#0x1_new_account_Account">Account</a> {
    <b>borrow_global</b>&lt;<a href="new_account.md#0x1_new_account_Account">Account</a>&gt;(addr).sequence_number
}
</code></pre>



</details>

<a id="0x1_new_account_get_native_authentication_key"></a>

## Function `get_native_authentication_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_get_native_authentication_key">get_native_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="new_account.md#0x1_new_account_get_native_authentication_key">get_native_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>assert</b>!(<a href="new_account.md#0x1_new_account_using_native_authenticator">using_native_authenticator</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="new_account.md#0x1_new_account_ENATIVE_AUTHENTICATOR_EXISTENCE">ENATIVE_AUTHENTICATOR_EXISTENCE</a>));
    <b>borrow_global</b>&lt;<a href="new_account.md#0x1_new_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(addr).key
}
</code></pre>



</details>

<a id="0x1_new_account_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="new_account.md#0x1_new_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>) <b>acquires</b> <a href="new_account.md#0x1_new_account_Account">Account</a> {
    <b>let</b> sequence_number = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="new_account.md#0x1_new_account_Account">Account</a>&gt;(addr).sequence_number;

    <b>assert</b>!(
        (*sequence_number <b>as</b> u128) &lt; <a href="new_account.md#0x1_new_account_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="new_account.md#0x1_new_account_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>)
    );
    *sequence_number = *sequence_number + 1;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
