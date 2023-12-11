
<a id="0x1_account_v2"></a>

# Module `0x1::account_v2`



-  [Resource `Account`](#0x1_account_v2_Account)
-  [Resource `NativeAuthenticator`](#0x1_account_v2_NativeAuthenticator)
-  [Resource `CustomizedAuthenticator`](#0x1_account_v2_CustomizedAuthenticator)
-  [Constants](#@Constants_0)
-  [Function `gen_native_authenticator`](#0x1_account_v2_gen_native_authenticator)
-  [Function `gen_customized_authenticator`](#0x1_account_v2_gen_customized_authenticator)
-  [Function `update_native_authenticator`](#0x1_account_v2_update_native_authenticator)
-  [Function `update_native_authenticator_impl`](#0x1_account_v2_update_native_authenticator_impl)
-  [Function `update_customized_authenticator_impl`](#0x1_account_v2_update_customized_authenticator_impl)
-  [Function `generate_signer_from_owner`](#0x1_account_v2_generate_signer_from_owner)
-  [Function `transfer`](#0x1_account_v2_transfer)
-  [Function `make_self_owned`](#0x1_account_v2_make_self_owned)
-  [Function `create_resource_account`](#0x1_account_v2_create_resource_account)
-  [Function `create_account`](#0x1_account_v2_create_account)
-  [Function `create_account_unchecked`](#0x1_account_v2_create_account_unchecked)
-  [Function `exists_at`](#0x1_account_v2_exists_at)
-  [Function `using_native_authenticator`](#0x1_account_v2_using_native_authenticator)
-  [Function `using_customized_authenticator`](#0x1_account_v2_using_customized_authenticator)
-  [Function `get_sequence_number`](#0x1_account_v2_get_sequence_number)
-  [Function `get_native_authentication_key`](#0x1_account_v2_get_native_authentication_key)
-  [Function `increment_sequence_number`](#0x1_account_v2_increment_sequence_number)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
</code></pre>



<a id="0x1_account_v2_Account"></a>

## Resource `Account`

Resource representing an account object.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transfer_ref: <a href="object.md#0x1_object_TransferRef">object::TransferRef</a></code>
</dt>
<dd>

</dd>
<dt>
<code>extend_ref: <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_v2_NativeAuthenticator"></a>

## Resource `NativeAuthenticator`

The native authenticator where the key is used for authenticator verification in native code.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> <b>has</b> <b>copy</b>, drop, store, key
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

<a id="0x1_account_v2_CustomizedAuthenticator"></a>

## Resource `CustomizedAuthenticator`

The customized authenticator that defines how to authenticates this account in the specified module.
An integral part of Account Abstraction.
UNIMPLEMENTED.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a> <b>has</b> <b>copy</b>, drop, store, key
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


<a id="0x1_account_v2_MAX_U64"></a>



<pre><code><b>const</b> <a href="account_v2.md#0x1_account_v2_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_account_v2_ECANNOT_RESERVED_ADDRESS"></a>



<pre><code><b>const</b> <a href="account_v2.md#0x1_account_v2_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>: u64 = 2;
</code></pre>



<a id="0x1_account_v2_EACCOUNT_EXISTENCE"></a>



<pre><code><b>const</b> <a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>: u64 = 1;
</code></pre>



<a id="0x1_account_v2_ENOT_OWNER"></a>



<pre><code><b>const</b> <a href="account_v2.md#0x1_account_v2_ENOT_OWNER">ENOT_OWNER</a>: u64 = 4;
</code></pre>



<a id="0x1_account_v2_ESEQUENCE_NUMBER_OVERFLOW"></a>



<pre><code><b>const</b> <a href="account_v2.md#0x1_account_v2_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>: u64 = 3;
</code></pre>



<a id="0x1_account_v2_gen_native_authenticator"></a>

## Function `gen_native_authenticator`

The function to generate native authenticator resource.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_gen_native_authenticator">gen_native_authenticator</a>(key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">account_v2::NativeAuthenticator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_gen_native_authenticator">gen_native_authenticator</a>(key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
        key,
    }
}
</code></pre>



</details>

<a id="0x1_account_v2_gen_customized_authenticator"></a>

## Function `gen_customized_authenticator`

The function to generate customized authenticator resource.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_gen_customized_authenticator">gen_customized_authenticator</a>(account_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">account_v2::CustomizedAuthenticator</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_gen_customized_authenticator">gen_customized_authenticator</a>(
    account_address: <b>address</b>,
    module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a> {
    <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a> { account_address, module_name }
}
</code></pre>



</details>

<a id="0x1_account_v2_update_native_authenticator"></a>

## Function `update_native_authenticator`

Update native authenticator, FKA account rotation.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_native_authenticator">update_native_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_native_authenticator">update_native_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="account_v2.md#0x1_account_v2_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>, <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
        key,
    });
}
</code></pre>



</details>

<a id="0x1_account_v2_update_native_authenticator_impl"></a>

## Function `update_native_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, authenticator: <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">account_v2::NativeAuthenticator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_native_authenticator_impl">update_native_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    authenticator: <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>
) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(account_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address)) {
        <b>move_from</b>&lt;<a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address);
    };
    <b>if</b> (<b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address)) {
        <b>let</b> current = <b>borrow_global_mut</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address);
        <b>if</b> (*current != authenticator) {
            *current = authenticator;
        }
    } <b>else</b> {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, authenticator)
    }
}
</code></pre>



</details>

<a id="0x1_account_v2_update_customized_authenticator_impl"></a>

## Function `update_customized_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_customized_authenticator_impl">update_customized_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, authenticator: <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">account_v2::CustomizedAuthenticator</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_update_customized_authenticator_impl">update_customized_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    authenticator: <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>
) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>, <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(account_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>if</b> (<b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address)) {
        <b>move_from</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(account_address);
    };
    <b>if</b> (<b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address)) {
        <b>let</b> current = <b>borrow_global_mut</b>&lt;<a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(account_address);
        <b>if</b> (*current != authenticator) {
            *current = authenticator;
        }
    } <b>else</b> {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, authenticator)
    }
}
</code></pre>



</details>

<a id="0x1_account_v2_generate_signer_from_owner"></a>

## Function `generate_signer_from_owner`

In account v2, there is no signer_capability anymore. The ownership of an account could offer the signer the
owned account to the owner.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_generate_signer_from_owner">generate_signer_from_owner</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="account_v2.md#0x1_account_v2_Account">account_v2::Account</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_generate_signer_from_owner">generate_signer_from_owner</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, account_object: Object&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    <b>let</b> <a href="account.md#0x1_account">account</a> = <a href="object.md#0x1_object_object_address">object::object_address</a>(&account_object);
    <b>assert</b>!(<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>assert</b>!(<a href="object.md#0x1_object_is_owner">object::is_owner</a>(account_object, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account_v2.md#0x1_account_v2_ENOT_OWNER">ENOT_OWNER</a>));
    <a href="object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&<b>borrow_global</b>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>).extend_ref)
}
</code></pre>



</details>

<a id="0x1_account_v2_transfer"></a>

## Function `transfer`

Transfer ownership of the account.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_transfer">transfer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_transfer">transfer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, owner: <b>address</b>) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    <b>let</b> account_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(account_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>let</b> current_owner = <a href="object.md#0x1_object_owner">object::owner</a>(<a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(account_address));
    <b>if</b> (current_owner == owner) {
        <b>return</b>
    };
    <b>let</b> linear_transfer_ref = <a href="object.md#0x1_object_generate_linear_transfer_ref">object::generate_linear_transfer_ref</a>(
        &<b>borrow_global</b>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(account_address).transfer_ref
    );
    <a href="object.md#0x1_object_transfer_with_ref">object::transfer_with_ref</a>(linear_transfer_ref, owner);
}
</code></pre>



</details>

<a id="0x1_account_v2_make_self_owned"></a>

## Function `make_self_owned`

A utility function to claim back the ownership of the account to itself.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_make_self_owned">make_self_owned</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> inline <b>fun</b> <a href="account_v2.md#0x1_account_v2_make_self_owned">make_self_owned</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    aptos_framework::account_v2::transfer(<a href="account.md#0x1_account">account</a>, std::signer::address_of(<a href="account.md#0x1_account">account</a>));
}
</code></pre>



</details>

<a id="0x1_account_v2_create_resource_account"></a>

## Function `create_resource_account`

Create resource account v2.


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_create_resource_account">create_resource_account</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_create_resource_account">create_resource_account</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    <b>let</b> resource_addr = <a href="object.md#0x1_object_create_object_address">object::create_object_address</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source), seed);
    <b>assert</b>!(!<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(resource_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <b>let</b> self = <a href="account_v2.md#0x1_account_v2_create_account_unchecked">create_account_unchecked</a>(resource_addr);
    <a href="account_v2.md#0x1_account_v2_transfer">transfer</a>(&self, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source));
    self
}
</code></pre>



</details>

<a id="0x1_account_v2_create_account"></a>

## Function `create_account`

Publishes a new <code><a href="account_v2.md#0x1_account_v2_Account">Account</a></code> resource under <code>new_address</code>. A ConstructorRef representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    // there cannot be an <a href="account_v2.md#0x1_account_v2_Account">Account</a> resource under new_addr already.
    <b>assert</b>!(!<a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(new_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account_v2.md#0x1_account_v2_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));

    // NOTE: @core_resources gets created via a `create_account` call, so we do not <b>include</b> it below.
    <b>assert</b>!(
        new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account_v2.md#0x1_account_v2_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)
    );
    <a href="account_v2.md#0x1_account_v2_create_account_unchecked">create_account_unchecked</a>(new_address)
}
</code></pre>



</details>

<a id="0x1_account_v2_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code><b>fun</b> <a href="account_v2.md#0x1_account_v2_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account_v2.md#0x1_account_v2_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> new_account_cref = &<a href="object.md#0x1_object_create_object_at_address">object::create_object_at_address</a>(new_address, <b>true</b>);
    <b>let</b> new_account = &<a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(new_account_cref);
    <b>move_to</b>(
        new_account,
        <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
            sequence_number: 0,
            transfer_ref: <a href="object.md#0x1_object_generate_transfer_ref">object::generate_transfer_ref</a>(new_account_cref),
            extend_ref: <a href="object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(new_account_cref)
        }
    );
    <b>move_to</b>(new_account,
        <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
            key: <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&new_address)
        }
    );
    <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(new_account_cref)
}
</code></pre>



</details>

<a id="0x1_account_v2_exists_at"></a>

## Function `exists_at`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    is_object(addr) && <b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_account_v2_using_native_authenticator"></a>

## Function `using_native_authenticator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_account_v2_using_customized_authenticator"></a>

## Function `using_customized_authenticator`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_using_customized_authenticator">using_customized_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_using_customized_authenticator">using_customized_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="account_v2.md#0x1_account_v2_CustomizedAuthenticator">CustomizedAuthenticator</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_account_v2_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    <b>borrow_global</b>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(addr).sequence_number
}
</code></pre>



</details>

<a id="0x1_account_v2_get_native_authentication_key"></a>

## Function `get_native_authentication_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_get_native_authentication_key">get_native_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_v2.md#0x1_account_v2_get_native_authentication_key">get_native_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a> {
    <b>assert</b>!(<a href="account_v2.md#0x1_account_v2_using_native_authenticator">using_native_authenticator</a>(addr), 0);
    <b>borrow_global</b>&lt;<a href="account_v2.md#0x1_account_v2_NativeAuthenticator">NativeAuthenticator</a>&gt;(addr).key
}
</code></pre>



</details>

<a id="0x1_account_v2_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account_v2.md#0x1_account_v2_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>) <b>acquires</b> <a href="account_v2.md#0x1_account_v2_Account">Account</a> {
    <b>let</b> sequence_number = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="account_v2.md#0x1_account_v2_Account">Account</a>&gt;(addr).sequence_number;

    <b>assert</b>!(
        (*sequence_number <b>as</b> u128) &lt; <a href="account_v2.md#0x1_account_v2_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="account_v2.md#0x1_account_v2_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>)
    );
    *sequence_number = *sequence_number + 1;
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
