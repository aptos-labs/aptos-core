
<a id="0x1_account_abstraction"></a>

# Module `0x1::account_abstraction`



-  [Struct `UpdateDispatchableAuthenticator`](#0x1_account_abstraction_UpdateDispatchableAuthenticator)
-  [Struct `RemoveDispatchableAuthenticator`](#0x1_account_abstraction_RemoveDispatchableAuthenticator)
-  [Enum Resource `DispatchableAuthenticator`](#0x1_account_abstraction_DispatchableAuthenticator)
-  [Constants](#@Constants_0)
-  [Function `add_authentication_function`](#0x1_account_abstraction_add_authentication_function)
-  [Function `remove_authentication_function`](#0x1_account_abstraction_remove_authentication_function)
-  [Function `remove_authenticator`](#0x1_account_abstraction_remove_authenticator)
-  [Function `resource_addr`](#0x1_account_abstraction_resource_addr)
-  [Function `update_dispatchable_authenticator_impl`](#0x1_account_abstraction_update_dispatchable_authenticator_impl)
-  [Function `using_dispatchable_authenticator`](#0x1_account_abstraction_using_dispatchable_authenticator)
-  [Function `dispatchable_authenticator`](#0x1_account_abstraction_dispatchable_authenticator)
-  [Function `dispatchable_authenticator_internal`](#0x1_account_abstraction_dispatchable_authenticator_internal)
-  [Function `authenticate`](#0x1_account_abstraction_authenticate)
-  [Function `dispatchable_authenticate`](#0x1_account_abstraction_dispatchable_authenticate)
-  [Function `add_dispatchable_authentication_function`](#0x1_account_abstraction_add_dispatchable_authentication_function)
-  [Function `remove_dispatchable_authentication_function`](#0x1_account_abstraction_remove_dispatchable_authentication_function)
-  [Function `remove_dispatchable_authenticator`](#0x1_account_abstraction_remove_dispatchable_authenticator)
-  [Specification](#@Specification_1)
    -  [Function `dispatchable_authenticate`](#@Specification_1_dispatchable_authenticate)


<pre><code><b>use</b> <a href="auth_data.md#0x1_auth_data">0x1::auth_data</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="function_info.md#0x1_function_info">0x1::function_info</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="ordered_map.md#0x1_ordered_map">0x1::ordered_map</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a id="0x1_account_abstraction_UpdateDispatchableAuthenticator"></a>

## Struct `UpdateDispatchableAuthenticator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="account_abstraction.md#0x1_account_abstraction_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> <b>has</b> drop, store
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
<code><b>update</b>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>auth_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_abstraction_RemoveDispatchableAuthenticator"></a>

## Struct `RemoveDispatchableAuthenticator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="account_abstraction.md#0x1_account_abstraction_RemoveDispatchableAuthenticator">RemoveDispatchableAuthenticator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_abstraction_DispatchableAuthenticator"></a>

## Enum Resource `DispatchableAuthenticator`

The dispatchable authenticator that defines how to authenticates this account in the specified module.
An integral part of Account Abstraction.


<pre><code>#[resource_group_member(#[group = <a href="object.md#0x1_object_ObjectGroup">0x1::object::ObjectGroup</a>])]
enum <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>auth_functions: <a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_account_abstraction_MAX_U64"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_account_abstraction_ENOT_MASTER_SIGNER"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>: u64 = 4;
</code></pre>



<a id="0x1_account_abstraction_EAUTH_FUNCTION_SIGNATURE_MISMATCH"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_EAUTH_FUNCTION_SIGNATURE_MISMATCH">EAUTH_FUNCTION_SIGNATURE_MISMATCH</a>: u64 = 3;
</code></pre>



<a id="0x1_account_abstraction_EDEPRECATED_FUNCTION"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>: u64 = 6;
</code></pre>



<a id="0x1_account_abstraction_EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED">EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED</a>: u64 = 1;
</code></pre>



<a id="0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE">EFUNCTION_INFO_EXISTENCE</a>: u64 = 2;
</code></pre>



<a id="0x1_account_abstraction_EINCONSISTENT_SIGNER_ADDRESS"></a>



<pre><code><b>const</b> <a href="account_abstraction.md#0x1_account_abstraction_EINCONSISTENT_SIGNER_ADDRESS">EINCONSISTENT_SIGNER_ADDRESS</a>: u64 = 5;
</code></pre>



<a id="0x1_account_abstraction_add_authentication_function"></a>

## Function `add_authentication_function`

Add dispatchable authentication function that enables account abstraction via this function.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_add_authentication_function">add_authentication_function</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_add_authentication_function">add_authentication_function</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    module_address: <b>address</b>,
    module_name: String,
    function_name: String,
) <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>assert</b>!(!is_permissioned_signer(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account_abstraction.md#0x1_account_abstraction_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <a href="account_abstraction.md#0x1_account_abstraction_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
        <a href="account.md#0x1_account">account</a>,
        <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(module_address, module_name, function_name),
        <b>true</b>
    );
}
</code></pre>



</details>

<a id="0x1_account_abstraction_remove_authentication_function"></a>

## Function `remove_authentication_function`

Remove dispatchable authentication function that enables account abstraction via this function.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_authentication_function">remove_authentication_function</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, module_address: <b>address</b>, module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_authentication_function">remove_authentication_function</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    module_address: <b>address</b>,
    module_name: String,
    function_name: String,
) <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>assert</b>!(!is_permissioned_signer(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account_abstraction.md#0x1_account_abstraction_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <a href="account_abstraction.md#0x1_account_abstraction_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
        <a href="account.md#0x1_account">account</a>,
        <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(module_address, module_name, function_name),
        <b>false</b>
    );
}
</code></pre>



</details>

<a id="0x1_account_abstraction_remove_authenticator"></a>

## Function `remove_authenticator`

Remove dispatchable authenticator so that all dispatchable authentication functions will be removed as well.
After calling this function, the account is not abstracted at all.
Note: it is a private entry function that can only be called directly from transaction.


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_authenticator">remove_authenticator</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_authenticator">remove_authenticator</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>assert</b>!(!is_permissioned_signer(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account_abstraction.md#0x1_account_abstraction_ENOT_MASTER_SIGNER">ENOT_MASTER_SIGNER</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
        <b>move_from</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr);
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account_abstraction.md#0x1_account_abstraction_RemoveDispatchableAuthenticator">RemoveDispatchableAuthenticator</a> {
            <a href="account.md#0x1_account">account</a>: addr,
        });
    };
}
</code></pre>



</details>

<a id="0x1_account_abstraction_resource_addr"></a>

## Function `resource_addr`



<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(source: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(source: <b>address</b>): <b>address</b> {
    <a href="object.md#0x1_object_create_user_derived_object_address">object::create_user_derived_object_address</a>(source, @aptos_fungible_asset)
}
</code></pre>



</details>

<a id="0x1_account_abstraction_update_dispatchable_authenticator_impl"></a>

## Function `update_dispatchable_authenticator_impl`



<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auth_function: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, is_add: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_update_dispatchable_authenticator_impl">update_dispatchable_authenticator_impl</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    auth_function: FunctionInfo,
    is_add: bool,
) <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> resource_addr = <a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(addr);
    <b>let</b> dispatcher_auth_function_info = <a href="function_info.md#0x1_function_info_new_function_info_from_address">function_info::new_function_info_from_address</a>(
        @aptos_framework,
        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"<a href="account_abstraction.md#0x1_account_abstraction">account_abstraction</a>"),
        <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"dispatchable_authenticate"),
    );
    <b>assert</b>!(
        <a href="function_info.md#0x1_function_info_check_dispatch_type_compatibility">function_info::check_dispatch_type_compatibility</a>(&dispatcher_auth_function_info, &auth_function),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account_abstraction.md#0x1_account_abstraction_EAUTH_FUNCTION_SIGNATURE_MISMATCH">EAUTH_FUNCTION_SIGNATURE_MISMATCH</a>)
    );
    <b>if</b> (is_add && !<b>exists</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
        <b>move_to</b>(
            &<a href="create_signer.md#0x1_create_signer_create_signer">create_signer::create_signer</a>(resource_addr),
            DispatchableAuthenticator::V1 { auth_functions: <a href="ordered_map.md#0x1_ordered_map_new">ordered_map::new</a>() }
        );
    };
    <b>assert</b>!(<b>exists</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_abstraction.md#0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE">EFUNCTION_INFO_EXISTENCE</a>));
    <b>let</b> current_map = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr).auth_functions;
    <b>if</b> (is_add) {
        <b>assert</b>!(
            !<a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(current_map, &auth_function),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account_abstraction.md#0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE">EFUNCTION_INFO_EXISTENCE</a>)
        );
        <a href="ordered_map.md#0x1_ordered_map_add">ordered_map::add</a>(current_map, auth_function, <b>true</b>);
    } <b>else</b> {
        <b>assert</b>!(
            <a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(current_map, &auth_function),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_abstraction.md#0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE">EFUNCTION_INFO_EXISTENCE</a>)
        );
        <a href="ordered_map.md#0x1_ordered_map_remove">ordered_map::remove</a>(current_map, &auth_function);
    };
    <a href="event.md#0x1_event_emit">event::emit</a>(
        <a href="account_abstraction.md#0x1_account_abstraction_UpdateDispatchableAuthenticator">UpdateDispatchableAuthenticator</a> {
            <a href="account.md#0x1_account">account</a>: addr,
            <b>update</b>: <b>if</b> (is_add) { b"add" } <b>else</b> { b"remove" },
            auth_function,
        }
    );
    <b>if</b> (<a href="ordered_map.md#0x1_ordered_map_length">ordered_map::length</a>(current_map) == 0) {
            <a href="account_abstraction.md#0x1_account_abstraction_remove_authenticator">remove_authenticator</a>(<a href="account.md#0x1_account">account</a>);
    }
}
</code></pre>



</details>

<a id="0x1_account_abstraction_using_dispatchable_authenticator"></a>

## Function `using_dispatchable_authenticator`

Return <code><b>true</b></code> if the account is an abstracted account that can be authenticated with dispatchable move authenticator.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_using_dispatchable_authenticator">using_dispatchable_authenticator</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_using_dispatchable_authenticator">using_dispatchable_authenticator</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(<a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(addr))
}
</code></pre>



</details>

<a id="0x1_account_abstraction_dispatchable_authenticator"></a>

## Function `dispatchable_authenticator`

Return the current dispatchable authenticator move function info. <code>None</code> means this authentication scheme is disabled.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticator">dispatchable_authenticator</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticator">dispatchable_authenticator</a>(addr: <b>address</b>): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;FunctionInfo&gt;&gt; <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>let</b> resource_addr = <a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(addr);
    <b>if</b> (<b>exists</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(
            <a href="ordered_map.md#0x1_ordered_map_keys">ordered_map::keys</a>(&<b>borrow_global</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(resource_addr).auth_functions)
        )
    } <b>else</b> { <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() }
}
</code></pre>



</details>

<a id="0x1_account_abstraction_dispatchable_authenticator_internal"></a>

## Function `dispatchable_authenticator_internal`



<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticator_internal">dispatchable_authenticator_internal</a>(addr: <b>address</b>): &<a href="ordered_map.md#0x1_ordered_map_OrderedMap">ordered_map::OrderedMap</a>&lt;<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, bool&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticator_internal">dispatchable_authenticator_internal</a>(addr: <b>address</b>): &OrderedMap&lt;FunctionInfo, bool&gt; {
    <b>assert</b>!(<a href="account_abstraction.md#0x1_account_abstraction_using_dispatchable_authenticator">using_dispatchable_authenticator</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_abstraction.md#0x1_account_abstraction_EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED">EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED</a>));
    &<b>borrow_global</b>&lt;<a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a>&gt;(<a href="account_abstraction.md#0x1_account_abstraction_resource_addr">resource_addr</a>(addr)).auth_functions
}
</code></pre>



</details>

<a id="0x1_account_abstraction_authenticate"></a>

## Function `authenticate`



<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_authenticate">authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, func_info: <a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>, signing_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_authenticate">authenticate</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    func_info: FunctionInfo,
    signing_data: AbstractionAuthData,
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="account_abstraction.md#0x1_account_abstraction_DispatchableAuthenticator">DispatchableAuthenticator</a> {
    <b>let</b> master_signer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&<a href="account.md#0x1_account">account</a>);
    <b>let</b> func_infos = <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticator_internal">dispatchable_authenticator_internal</a>(master_signer_addr);
    <b>assert</b>!(<a href="ordered_map.md#0x1_ordered_map_contains">ordered_map::contains</a>(func_infos, &func_info), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account_abstraction.md#0x1_account_abstraction_EFUNCTION_INFO_EXISTENCE">EFUNCTION_INFO_EXISTENCE</a>));
    <a href="function_info.md#0x1_function_info_load_module_from_function">function_info::load_module_from_function</a>(&func_info);
    <b>let</b> returned_signer = <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticate">dispatchable_authenticate</a>(<a href="account.md#0x1_account">account</a>, signing_data, &func_info);
    // Returned <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> MUST represent the same <a href="account.md#0x1_account">account</a> <b>address</b>. Otherwise, it may <b>break</b> the <b>invariant</b> of Aptos blockchain!
    <b>assert</b>!(
        master_signer_addr == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(&returned_signer),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="account_abstraction.md#0x1_account_abstraction_EINCONSISTENT_SIGNER_ADDRESS">EINCONSISTENT_SIGNER_ADDRESS</a>)
    );
    returned_signer
}
</code></pre>



</details>

<a id="0x1_account_abstraction_dispatchable_authenticate"></a>

## Function `dispatchable_authenticate`

The native function to dispatch customized move authentication function.


<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticate">dispatchable_authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signing_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticate">dispatchable_authenticate</a>(
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    signing_data: AbstractionAuthData,
    function: &FunctionInfo
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



</details>

<a id="0x1_account_abstraction_add_dispatchable_authentication_function"></a>

## Function `add_dispatchable_authentication_function`



<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_add_dispatchable_authentication_function">add_dispatchable_authentication_function</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _module_address: <b>address</b>, _module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_add_dispatchable_authentication_function">add_dispatchable_authentication_function</a>(
    _account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _module_address: <b>address</b>,
    _module_name: String,
    _function_name: String,
) {
    <b>abort</b> std::error::unavailable(<a href="account_abstraction.md#0x1_account_abstraction_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>)
}
</code></pre>



</details>

<a id="0x1_account_abstraction_remove_dispatchable_authentication_function"></a>

## Function `remove_dispatchable_authentication_function`



<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_dispatchable_authentication_function">remove_dispatchable_authentication_function</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _module_address: <b>address</b>, _module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, _function_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_dispatchable_authentication_function">remove_dispatchable_authentication_function</a>(
    _account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    _module_address: <b>address</b>,
    _module_name: String,
    _function_name: String,
) {
    <b>abort</b> std::error::unavailable(<a href="account_abstraction.md#0x1_account_abstraction_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>)
}
</code></pre>



</details>

<a id="0x1_account_abstraction_remove_dispatchable_authenticator"></a>

## Function `remove_dispatchable_authenticator`



<pre><code>#[deprecated]
<b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_dispatchable_authenticator">remove_dispatchable_authenticator</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_remove_dispatchable_authenticator">remove_dispatchable_authenticator</a>(
    _account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) {
    <b>abort</b> std::error::unavailable(<a href="account_abstraction.md#0x1_account_abstraction_EDEPRECATED_FUNCTION">EDEPRECATED_FUNCTION</a>)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>




<a id="0x1_account_abstraction_spec_dispatchable_authenticate"></a>


<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_spec_dispatchable_authenticate">spec_dispatchable_authenticate</a>(
   <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
   signing_data: AbstractionAuthData,
   function: &FunctionInfo
): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
</code></pre>



<a id="@Specification_1_dispatchable_authenticate"></a>

### Function `dispatchable_authenticate`


<pre><code><b>fun</b> <a href="account_abstraction.md#0x1_account_abstraction_dispatchable_authenticate">dispatchable_authenticate</a>(<a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signing_data: <a href="auth_data.md#0x1_auth_data_AbstractionAuthData">auth_data::AbstractionAuthData</a>, function: &<a href="function_info.md#0x1_function_info_FunctionInfo">function_info::FunctionInfo</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>ensures</b> [abstract] result == <a href="account_abstraction.md#0x1_account_abstraction_spec_dispatchable_authenticate">spec_dispatchable_authenticate</a>(<a href="account.md#0x1_account">account</a>, signing_data, function);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
