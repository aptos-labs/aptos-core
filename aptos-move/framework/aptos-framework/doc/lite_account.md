
<a id="0x1_lite_account"></a>

# Module `0x1::lite_account`



-  [Struct `UpdateNativeAuthenticator`](#0x1_lite_account_UpdateNativeAuthenticator)
-  [Struct `UpdateDispatchableAuthenticator`](#0x1_lite_account_UpdateDispatchableAuthenticator)
-  [Struct `LiteAccountGroup`](#0x1_lite_account_LiteAccountGroup)
-  [Resource `Account`](#0x1_lite_account_Account)
-  [Resource `NativeAuthenticator`](#0x1_lite_account_NativeAuthenticator)
-  [Resource `DispatchableAuthenticator`](#0x1_lite_account_DispatchableAuthenticator)
-  [Resource `LegacyGUIDCreactionNumber`](#0x1_lite_account_LegacyGUIDCreactionNumber)
-  [Resource `LegacyRotationCapabilityOffer`](#0x1_lite_account_LegacyRotationCapabilityOffer)
-  [Resource `LegacySignerCapabilityOffer`](#0x1_lite_account_LegacySignerCapabilityOffer)
-  [Constants](#@Constants_0)
-  [Function `update_native_authenticator`](#0x1_lite_account_update_native_authenticator)
-  [Function `remove_native_authenticator`](#0x1_lite_account_remove_native_authenticator)
-  [Function `update_dispatchable_authenticator`](#0x1_lite_account_update_dispatchable_authenticator)
-  [Function `remove_dispatchable_authenticator`](#0x1_lite_account_remove_dispatchable_authenticator)
-  [Function `resource_addr`](#0x1_lite_account_resource_addr)
-  [Function `create_user_derived_object_address_impl`](#0x1_lite_account_create_user_derived_object_address_impl)
-  [Function `update_native_authenticator_impl`](#0x1_lite_account_update_native_authenticator_impl)
-  [Function `update_dispatchable_authenticator_impl`](#0x1_lite_account_update_dispatchable_authenticator_impl)
-  [Function `create_account`](#0x1_lite_account_create_account)
-  [Function `create_account_unchecked`](#0x1_lite_account_create_account_unchecked)
-  [Function `create_account_with_resource`](#0x1_lite_account_create_account_with_resource)
-  [Function `account_resource_exists_at`](#0x1_lite_account_account_resource_exists_at)
-  [Function `using_native_authenticator`](#0x1_lite_account_using_native_authenticator)
-  [Function `using_dispatchable_authenticator`](#0x1_lite_account_using_dispatchable_authenticator)
-  [Function `get_sequence_number`](#0x1_lite_account_get_sequence_number)
-  [Function `native_authenticator`](#0x1_lite_account_native_authenticator)
-  [Function `dispatchable_authenticator`](#0x1_lite_account_dispatchable_authenticator)
-  [Function `increment_sequence_number`](#0x1_lite_account_increment_sequence_number)
-  [Function `dispatchable_authenticate`](#0x1_lite_account_dispatchable_authenticate)
-  [Function `guid_creation_number`](#0x1_lite_account_guid_creation_number)
-  [Function `set_sequence_number`](#0x1_lite_account_set_sequence_number)
-  [Function `set_guid_creation_number`](#0x1_lite_account_set_guid_creation_number)
-  [Function `create_guid`](#0x1_lite_account_create_guid)
-  [Function `rotation_capability_offer`](#0x1_lite_account_rotation_capability_offer)
-  [Function `signer_capability_offer`](#0x1_lite_account_signer_capability_offer)
-  [Function `set_rotation_capability_offer`](#0x1_lite_account_set_rotation_capability_offer)
-  [Function `set_signer_capability_offer`](#0x1_lite_account_set_signer_capability_offer)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_lite_account_UpdateNativeAuthenticator"></a>

## Struct `UpdateNativeAuthenticator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_UpdateDispatchableAuthenticator"></a>

## Struct `UpdateDispatchableAuthenticator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_LiteAccountGroup"></a>

## Struct `LiteAccountGroup`

A shared resource group for storing new account resources together in storage.


<pre><code>#[resource_group(#[scope = <b>address</b>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_LiteAccountGroup">LiteAccountGroup</a>
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

<a id="0x1_lite_account_Account"></a>

## Resource `Account`

Resource representing an account object.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_Account">Account</a> <b>has</b> key
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

<a id="0x1_lite_account_NativeAuthenticator"></a>

## Resource `NativeAuthenticator`

The native authenticator where the key is used for authenticator verification in native code.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_DispatchableAuthenticator"></a>

## Resource `DispatchableAuthenticator`

The dispatchable authenticator that defines how to authenticates this account in the specified module.
An integral part of Account Abstraction.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>auth_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_LegacyGUIDCreactionNumber"></a>

## Resource `LegacyGUIDCreactionNumber`

Legacy field from deprecated Account module.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>creation_number: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_LegacyRotationCapabilityOffer"></a>

## Resource `LegacyRotationCapabilityOffer`

Legacy field from deprecated Account module.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>for: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_lite_account_LegacySignerCapabilityOffer"></a>

## Resource `LegacySignerCapabilityOffer`

Legacy field from deprecated Account module.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
<b>struct</b> <a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>for: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_lite_account_MAX_U64"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_lite_account_EACCOUNT_EXISTENCE"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>: u64 = 1;
</code></pre>



<a id="0x1_lite_account_EAUTH_FUNCTION_SIGNATURE_MISMATCH"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_EAUTH_FUNCTION_SIGNATURE_MISMATCH">EAUTH_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 7;
</code></pre>



<a id="0x1_lite_account_ECANNOT_RESERVED_ADDRESS"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>: u64 = 2;
</code></pre>



<a id="0x1_lite_account_ECUSTOMIZED_AUTHENTICATOR_IS_NOT_USED"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_ECUSTOMIZED_AUTHENTICATOR_IS_NOT_USED">ECUSTOMIZED_AUTHENTICATOR_IS_NOT_USED</a>: u64 = 6;
</code></pre>



<a id="0x1_lite_account_EMALFORMED_AUTHENTICATION_KEY"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>: u64 = 4;
</code></pre>



<a id="0x1_lite_account_ENATIVE_AUTHENTICATOR_IS_NOT_USED"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_ENATIVE_AUTHENTICATOR_IS_NOT_USED">ENATIVE_AUTHENTICATOR_IS_NOT_USED</a>: u64 = 5;
</code></pre>



<a id="0x1_lite_account_ENOT_OWNER"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_ENOT_OWNER">ENOT_OWNER</a>: u64 = 8;
</code></pre>



<a id="0x1_lite_account_ESEQUENCE_NUMBER_OVERFLOW"></a>



<pre><code><b>const</b> <a href="lite_account.md#0x1_lite_account_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>: u64 = 3;
</code></pre>



<a id="0x1_lite_account_update_native_authenticator"></a>

## Function `update_native_authenticator`

Update native authenticator, FKA account rotation.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_native_authenticator">update_native_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_native_authenticator">update_native_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="lite_account.md#0x1_lite_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(key));
}
</code></pre>



</details>

<a id="0x1_lite_account_remove_native_authenticator"></a>

## Function `remove_native_authenticator`

Remove native authenticator so that this account could not be authenticated via native authenticator.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_remove_native_authenticator">remove_native_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_remove_native_authenticator">remove_native_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> {
    <a href="lite_account.md#0x1_lite_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>())
}
</code></pre>



</details>

<a id="0x1_lite_account_update_dispatchable_authenticator"></a>

## Function `update_dispatchable_authenticator`

Update dispatchable authenticator that enables account abstraction.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator">update_dispatchable_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator">update_dispatchable_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    module_address: <b>address</b>,
    module_name: String,
    function_name: String,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
        <a href="account.md#0x1_account">account</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(module_address, module_name, function_name))
    );
}
</code></pre>



</details>

<a id="0x1_lite_account_remove_dispatchable_authenticator"></a>

## Function `remove_dispatchable_authenticator`

Update dispatchable authenticator that disables account abstraction.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_remove_dispatchable_authenticator">remove_dispatchable_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="lite_account.md#0x1_lite_account_remove_dispatchable_authenticator">remove_dispatchable_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
        <a href="account.md#0x1_account">account</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    );
}
</code></pre>



</details>

<a id="0x1_lite_account_resource_addr"></a>

## Function `resource_addr`



<pre><code><b>fun</b> <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(source: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(source: <b>address</b>): <b>address</b> {
    <a href="lite_account.md#0x1_lite_account_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source, @aptos_fungible_asset)
}
</code></pre>



</details>

<a id="0x1_lite_account_create_user_derived_object_address_impl"></a>

## Function `create_user_derived_object_address_impl`



<pre><code><b>fun</b> <a href="lite_account.md#0x1_lite_account_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_user_derived_object_address_impl">create_user_derived_object_address_impl</a>(source: <b>address</b>, derive_from: <b>address</b>): <b>address</b>;
</code></pre>



</details>

<a id="0x1_lite_account_update_native_authenticator_impl"></a>

## Function `update_native_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key_option: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_native_authenticator_impl">update_native_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    new_auth_key_option: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&new_auth_key_option)) {
        <b>let</b> new_auth_key = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&new_auth_key_option);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_auth_key) == 32,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="lite_account.md#0x1_lite_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>)
        );
        <b>let</b> native_auth_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&addr);
        <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr)) {
            <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(native_auth_key) == new_auth_key_option) {
                <b>let</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> { auth_key } = <b>move_from</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr);
                <a href="event.md#0x1_event_emit">event::emit</a>(
                    <a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> {
                        <a href="account.md#0x1_account">account</a>: addr,
                        old_auth_key: auth_key, new_auth_key: new_auth_key_option
                    }
                );
            } <b>else</b> {
                <b>let</b> current = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr).auth_key;
                <b>if</b> (*current != new_auth_key_option) {
                    <a href="event.md#0x1_event_emit">event::emit</a>(
                        <a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> {
                            <a href="account.md#0x1_account">account</a>: addr,
                            old_auth_key: *current, new_auth_key: new_auth_key_option
                        }
                    );
                    *current = new_auth_key_option;
                };
            }
        } <b>else</b> <b>if</b> (new_auth_key != &native_auth_key) {
            <b>move_to</b>(
                &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr),
                <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> { auth_key: new_auth_key_option }
            );
            <a href="event.md#0x1_event_emit">event::emit</a>(
                <a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> {
                    <a href="account.md#0x1_account">account</a>: addr,
                    old_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
                        native_auth_key
                    ), new_auth_key: new_auth_key_option
                }
            )
        };
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr)) {
        <b>let</b> authenticator = <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&authenticator.auth_key)) {
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> {
                <a href="account.md#0x1_account">account</a>: addr,
                old_auth_key: authenticator.auth_key,
                new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
            });
            authenticator.auth_key = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
        };
    } <b>else</b> {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="lite_account.md#0x1_lite_account_UpdateNativeAuthenticator">UpdateNativeAuthenticator</a> {
            <a href="account.md#0x1_account">account</a>: addr,
            old_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&resource_addr)),
            new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        });
        <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> { auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() });
    };
}
</code></pre>



</details>

<a id="0x1_lite_account_update_dispatchable_authenticator_impl"></a>

## Function `update_dispatchable_authenticator_impl`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auth_function_option: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    auth_function_option: Option&lt;FunctionInfo&gt;,
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&auth_function_option)) {
        <b>let</b> auth_function = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(auth_function_option);
        <b>let</b> dispatcher_auth_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
            @aptos_framework,
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="lite_account.md#0x1_lite_account">lite_account</a>"),
            <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_authenticate"),
        );
        <b>assert</b>!(
            <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(&dispatcher_auth_function_info, &auth_function),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="lite_account.md#0x1_lite_account_EAUTH_FUNCTION_SIGNATURE_MISMATCH">EAUTH_FUNCTION_SIGNATURE_MISMATCH</a>)
        );
        <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
            <b>let</b> current = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr).auth_function;
            <b>if</b> (*current != auth_function) {
                <a href="event.md#0x1_event_emit">event::emit</a>(
                    <a href="lite_account.md#0x1_lite_account_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> {
                        <a href="account.md#0x1_account">account</a>: addr,
                        old_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*current),
                        new_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(auth_function)
                    }
                );
                *current = auth_function;
            }
        } <b>else</b> {
            <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> { auth_function });
            <a href="event.md#0x1_event_emit">event::emit</a>(
                <a href="lite_account.md#0x1_lite_account_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> {
                    <a href="account.md#0x1_account">account</a>: addr,
                    old_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
                    new_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(auth_function)
                }
            );
        }
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
        <b>let</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> { auth_function } = <b>move_from</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="lite_account.md#0x1_lite_account_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> {
            <a href="account.md#0x1_account">account</a>: addr,
            old_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(auth_function),
            new_auth_function: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
        });
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_create_account"></a>

## Function `create_account`

Publishes a lite <code><a href="lite_account.md#0x1_lite_account_Account">Account</a></code> resource under <code>new_address</code>. A ConstructorRef representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    // there cannot be an <a href="lite_account.md#0x1_lite_account_Account">Account</a> resource under new_address already.
    <b>assert</b>!(!<a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(new_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="lite_account.md#0x1_lite_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));

    // NOTE: @core_resources gets created via a `create_account` call, so we do not <b>include</b> it below.
    <b>assert</b>!(
        new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="lite_account.md#0x1_lite_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)
    );
    <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(new_address)
}
</code></pre>



</details>

<a id="0x1_lite_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account_unchecked">create_account_unchecked</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account_unchecked">create_account_unchecked</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    // there cannot be an <a href="lite_account.md#0x1_lite_account_Account">Account</a> resource under new_addr already.
    <b>assert</b>!(!<a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="lite_account.md#0x1_lite_account_EACCOUNT_EXISTENCE">EACCOUNT_EXISTENCE</a>));
    <a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(addr)
}
</code></pre>



</details>

<a id="0x1_lite_account_create_account_with_resource"></a>

## Function `create_account_with_resource`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account_with_resource">create_account_with_resource</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_account_with_resource">create_account_with_resource</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> new_account = <a href="lite_account.md#0x1_lite_account_create_account">create_account</a>(new_address);
    <b>move_to</b>(
        &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(new_address)),
        <a href="lite_account.md#0x1_lite_account_Account">Account</a> {
            sequence_number: 0,
        }
    );
    new_account
}
</code></pre>



</details>

<a id="0x1_lite_account_account_resource_exists_at"></a>

## Function `account_resource_exists_at`

Return <code><b>true</b></code> if Account resource exists at this address.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_Account">Account</a>&gt;(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr))
}
</code></pre>



</details>

<a id="0x1_lite_account_using_native_authenticator"></a>

## Function `using_native_authenticator`

Return <code><b>true</b></code> if the account could be authenticated with native authenticator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_using_native_authenticator">using_native_authenticator</a>(addr: <b>address</b>): bool <b>acquires</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    !<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(
        &<b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr).auth_key
    )
}
</code></pre>



</details>

<a id="0x1_lite_account_using_dispatchable_authenticator"></a>

## Function `using_dispatchable_authenticator`

Return <code><b>true</b></code> if the account is an abstracted account that can be authenticated with dispatchable move authenticator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_using_dispatchable_authenticator">using_dispatchable_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_using_dispatchable_authenticator">using_dispatchable_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr))
}
</code></pre>



</details>

<a id="0x1_lite_account_get_sequence_number"></a>

## Function `get_sequence_number`

Return the current sequence number.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="lite_account.md#0x1_lite_account_Account">Account</a> {
    <b>if</b> (<a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr)) {
        <b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_Account">Account</a>&gt;(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr)).sequence_number
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_native_authenticator"></a>

## Function `native_authenticator`

Return the current native authenticator. <code>None</code> means this authentication scheme is disabled.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_native_authenticator">native_authenticator</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_native_authenticator">native_authenticator</a>(addr: <b>address</b>): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr)) {
        <b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_NativeAuthenticator">NativeAuthenticator</a>&gt;(resource_addr).auth_key
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&addr))
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_dispatchable_authenticator"></a>

## Function `dispatchable_authenticator`

Return the current dispatchable authenticator move function info. <code>None</code> means this authentication scheme is disabled.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_dispatchable_authenticator">dispatchable_authenticator</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_dispatchable_authenticator">dispatchable_authenticator</a>(addr: <b>address</b>): Option&lt;FunctionInfo&gt; <b>acquires</b> <a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
            <b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr).auth_function
        )
    } <b>else</b> { <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
}
</code></pre>



</details>

<a id="0x1_lite_account_increment_sequence_number"></a>

## Function `increment_sequence_number`

Bump sequence number, which is only called by transaction_validation.move in apilogue for sequential transactions.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_Account">Account</a> {
    <b>if</b> (!<a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr)) {
        <a href="lite_account.md#0x1_lite_account_create_account_with_resource">create_account_with_resource</a>(addr);
    };
    <b>let</b> sequence_number = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_Account">Account</a>&gt;(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr)).sequence_number;

    <b>assert</b>!(
        (*sequence_number <b>as</b> u128) &lt; <a href="lite_account.md#0x1_lite_account_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="lite_account.md#0x1_lite_account_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>)
    );
    *sequence_number = *sequence_number + 1;
}
</code></pre>



</details>

<a id="0x1_lite_account_dispatchable_authenticate"></a>

## Function `dispatchable_authenticate`

The native function to dispatch customized move authentication function.


<pre><code><b>fun</b> <a href="lite_account.md#0x1_lite_account_dispatchable_authenticate">dispatchable_authenticate</a>(account_address: <b>address</b>, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="lite_account.md#0x1_lite_account_dispatchable_authenticate">dispatchable_authenticate</a>(
    account_address: <b>address</b>,
    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    function: &FunctionInfo
);
</code></pre>



</details>

<a id="0x1_lite_account_guid_creation_number"></a>

## Function `guid_creation_number`

Methods only for compatibility with account module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_guid_creation_number">guid_creation_number</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_guid_creation_number">guid_creation_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr)) {
        <b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr).creation_number
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_set_sequence_number"></a>

## Function `set_sequence_number`

Only used by account.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_sequence_number">set_sequence_number</a>(addr: <b>address</b>, new_sequence_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_sequence_number">set_sequence_number</a>(addr: <b>address</b>, new_sequence_number: u64) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_Account">Account</a> {
    <b>if</b> (!<a href="lite_account.md#0x1_lite_account_account_resource_exists_at">account_resource_exists_at</a>(addr)) {
        <a href="lite_account.md#0x1_lite_account_create_account_with_resource">create_account_with_resource</a>(addr);
    };
    <b>let</b> sequence_number = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_Account">Account</a>&gt;(<a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr)).sequence_number;
    <b>assert</b>!(
        (new_sequence_number <b>as</b> u128) &lt; <a href="lite_account.md#0x1_lite_account_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="lite_account.md#0x1_lite_account_ESEQUENCE_NUMBER_OVERFLOW">ESEQUENCE_NUMBER_OVERFLOW</a>)
    );
    *sequence_number = new_sequence_number;
}
</code></pre>



</details>

<a id="0x1_lite_account_set_guid_creation_number"></a>

## Function `set_guid_creation_number`

Only used by account.move for migration.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_guid_creation_number">set_guid_creation_number</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, creation_number: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_guid_creation_number">set_guid_creation_number</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    creation_number: u64
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));
    <b>if</b> (!<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr)) {
        <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> {
            creation_number: 0
        });
    };
    <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr).creation_number = creation_number;
}
</code></pre>



</details>

<a id="0x1_lite_account_create_guid"></a>

## Function `create_guid`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_guid">create_guid</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_create_guid">create_guid</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): GUID <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (!<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr)) {
        <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a> {
            creation_number: 0
        });
    };
    <b>let</b> number = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyGUIDCreactionNumber">LegacyGUIDCreactionNumber</a>&gt;(resource_addr).creation_number;
    <a href="guid.md#0x1_guid_create">guid::create</a>(addr, number)
}
</code></pre>



</details>

<a id="0x1_lite_account_rotation_capability_offer"></a>

## Function `rotation_capability_offer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_rotation_capability_offer">rotation_capability_offer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_rotation_capability_offer">rotation_capability_offer</a>(
    addr: <b>address</b>,
): Option&lt;<b>address</b>&gt; <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr).for)
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_signer_capability_offer"></a>

## Function `signer_capability_offer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_signer_capability_offer">signer_capability_offer</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_signer_capability_offer">signer_capability_offer</a>(
    addr: <b>address</b>,
): Option&lt;<b>address</b>&gt; <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a> {
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<b>borrow_global</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr).for)
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_set_rotation_capability_offer"></a>

## Function `set_rotation_capability_offer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_rotation_capability_offer">set_rotation_capability_offer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offeree: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_rotation_capability_offer">set_rotation_capability_offer</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    offeree: Option&lt;<b>address</b>&gt;
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&offeree)) {
        <b>let</b> offeree = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(offeree);
        <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr)) {
            <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr).for = offeree;
        } <b>else</b> {
            <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a> { for: offeree })
        }
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr)) {
        <b>move_from</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacyRotationCapabilityOffer">LegacyRotationCapabilityOffer</a>&gt;(resource_addr);
    }
}
</code></pre>



</details>

<a id="0x1_lite_account_set_signer_capability_offer"></a>

## Function `set_signer_capability_offer`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_signer_capability_offer">set_signer_capability_offer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offeree: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="lite_account.md#0x1_lite_account_set_signer_capability_offer">set_signer_capability_offer</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    offeree: Option&lt;<b>address</b>&gt;
) <b>acquires</b> <a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="lite_account.md#0x1_lite_account_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&offeree)) {
        <b>let</b> offeree = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(offeree);
        <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr)) {
            <b>borrow_global_mut</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr).for = offeree;
        } <b>else</b> {
            <b>move_to</b>(&<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr), <a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a> { for: offeree })
        }
    } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr)) {
        <b>move_from</b>&lt;<a href="lite_account.md#0x1_lite_account_LegacySignerCapabilityOffer">LegacySignerCapabilityOffer</a>&gt;(resource_addr);
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_lite_account_spec_native_authenticator"></a>


<pre><code><b>fun</b> <a href="lite_account.md#0x1_lite_account_spec_native_authenticator">spec_native_authenticator</a>(addr: <b>address</b>): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
