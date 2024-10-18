
<a id="0x1_intent"></a>

# Module `0x1::intent`



-  [Resource `TradeIntent`](#0x1_intent_TradeIntent)
-  [Struct `TradeSession`](#0x1_intent_TradeSession)
-  [Constants](#@Constants_0)
-  [Function `create_intent`](#0x1_intent_create_intent)
-  [Function `start_intent_session`](#0x1_intent_start_intent_session)
-  [Function `get_argument`](#0x1_intent_get_argument)
-  [Function `finish_intent_session`](#0x1_intent_finish_intent_session)
-  [Function `revoke_intent`](#0x1_intent_revoke_intent)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a id="0x1_intent_TradeIntent"></a>

## Resource `TradeIntent`



<pre><code><b>struct</b> <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>offered_resource: Source</code>
</dt>
<dd>

</dd>
<dt>
<code>argument: Args</code>
</dt>
<dd>

</dd>
<dt>
<code>self_delete_ref: <a href="object.md#0x1_object_DeleteRef">object::DeleteRef</a></code>
</dt>
<dd>

</dd>
<dt>
<code>expiry_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>witness_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_intent_TradeSession"></a>

## Struct `TradeSession`



<pre><code><b>struct</b> <a href="intent.md#0x1_intent_TradeSession">TradeSession</a>&lt;Args&gt;
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>argument: Args</code>
</dt>
<dd>

</dd>
<dt>
<code>witness_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_intent_ECONSUMPTION_FUNCTION_TYPE_MISMATCH"></a>

The registered hook function for consuming resource doesn't match the type requirement.


<pre><code><b>const</b> <a href="intent.md#0x1_intent_ECONSUMPTION_FUNCTION_TYPE_MISMATCH">ECONSUMPTION_FUNCTION_TYPE_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x1_intent_EINTENT_EXPIRED"></a>

The offered intent has expired


<pre><code><b>const</b> <a href="intent.md#0x1_intent_EINTENT_EXPIRED">EINTENT_EXPIRED</a>: u64 = 0;
</code></pre>



<a id="0x1_intent_EINVALID_WITNESS"></a>

Provided wrong witness to complete intent.


<pre><code><b>const</b> <a href="intent.md#0x1_intent_EINVALID_WITNESS">EINVALID_WITNESS</a>: u64 = 3;
</code></pre>



<a id="0x1_intent_ENOT_OWNER"></a>

Only owner can revoke an intent.


<pre><code><b>const</b> <a href="intent.md#0x1_intent_ENOT_OWNER">ENOT_OWNER</a>: u64 = 2;
</code></pre>



<a id="0x1_intent_create_intent"></a>

## Function `create_intent`



<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_create_intent">create_intent</a>&lt;Source: store, Args: drop, store, Witness: drop&gt;(offered_resource: Source, argument: Args, expiry_time: u64, issuer: <b>address</b>): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;Source, Args&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_create_intent">create_intent</a>&lt;Source: store, Args: store + drop, Witness: drop&gt;(
    offered_resource: Source,
    argument: Args,
    expiry_time: u64,
    issuer: <b>address</b>,
): Object&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt; {
    <b>let</b> constructor_ref = <a href="object.md#0x1_object_create_object">object::create_object</a>(issuer);
    <b>let</b> object_signer = <a href="object.md#0x1_object_generate_signer">object::generate_signer</a>(&constructor_ref);
    <b>let</b> self_delete_ref = <a href="object.md#0x1_object_generate_delete_ref">object::generate_delete_ref</a>(&constructor_ref);

    <b>move_to</b>&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;(
        &object_signer,
        <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a> {
            offered_resource,
            argument,
            expiry_time,
            self_delete_ref,
            witness_type: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;Witness&gt;(),
        }
    );
    <a href="object.md#0x1_object_object_from_constructor_ref">object::object_from_constructor_ref</a>(&constructor_ref)
}
</code></pre>



</details>

<a id="0x1_intent_start_intent_session"></a>

## Function `start_intent_session`



<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_start_intent_session">start_intent_session</a>&lt;Source: store, Args: drop, store&gt;(<a href="intent.md#0x1_intent">intent</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;Source, Args&gt;&gt;): (Source, <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;Args&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_start_intent_session">start_intent_session</a>&lt;Source: store, Args: store + drop&gt;(
    <a href="intent.md#0x1_intent">intent</a>: Object&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;,
): (Source, <a href="intent.md#0x1_intent_TradeSession">TradeSession</a>&lt;Args&gt;) <b>acquires</b> <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a> {
    <b>let</b> intent_ref = <b>borrow_global</b>&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="intent.md#0x1_intent">intent</a>));
    <b>assert</b>!(<a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() &lt;= intent_ref.expiry_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="intent.md#0x1_intent_EINTENT_EXPIRED">EINTENT_EXPIRED</a>));

    <b>let</b> <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a> {
        offered_resource,
        argument,
        expiry_time: _,
        self_delete_ref,
        witness_type,
    } = <b>move_from</b>&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="intent.md#0x1_intent">intent</a>));

    <a href="object.md#0x1_object_delete">object::delete</a>(self_delete_ref);

    <b>return</b> (offered_resource, <a href="intent.md#0x1_intent_TradeSession">TradeSession</a> {
        argument,
        witness_type,
    })
}
</code></pre>



</details>

<a id="0x1_intent_get_argument"></a>

## Function `get_argument`



<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_get_argument">get_argument</a>&lt;Args&gt;(session: &<a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;Args&gt;): &Args
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_get_argument">get_argument</a>&lt;Args&gt;(session: &<a href="intent.md#0x1_intent_TradeSession">TradeSession</a>&lt;Args&gt;): &Args {
    &session.argument
}
</code></pre>



</details>

<a id="0x1_intent_finish_intent_session"></a>

## Function `finish_intent_session`



<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_finish_intent_session">finish_intent_session</a>&lt;Witness: drop, Args: drop, store&gt;(session: <a href="intent.md#0x1_intent_TradeSession">intent::TradeSession</a>&lt;Args&gt;, _witness: Witness)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_finish_intent_session">finish_intent_session</a>&lt;Witness: drop, Args: store + drop&gt;(
    session: <a href="intent.md#0x1_intent_TradeSession">TradeSession</a>&lt;Args&gt;,
    _witness: Witness,
) {
    <b>let</b> <a href="intent.md#0x1_intent_TradeSession">TradeSession</a> {
        argument:_ ,
        witness_type,
    } = session;

    <b>assert</b>!(<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;Witness&gt;() == witness_type, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="intent.md#0x1_intent_EINVALID_WITNESS">EINVALID_WITNESS</a>));
}
</code></pre>



</details>

<a id="0x1_intent_revoke_intent"></a>

## Function `revoke_intent`



<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_revoke_intent">revoke_intent</a>&lt;Source: store, Args: drop, store&gt;(issuer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="intent.md#0x1_intent">intent</a>: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="intent.md#0x1_intent_TradeIntent">intent::TradeIntent</a>&lt;Source, Args&gt;&gt;): Source
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="intent.md#0x1_intent_revoke_intent">revoke_intent</a>&lt;Source: store, Args: store + drop&gt;(
    issuer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    <a href="intent.md#0x1_intent">intent</a>: Object&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;,
): Source <b>acquires</b> <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a> {
    <b>assert</b>!(<a href="object.md#0x1_object_owner">object::owner</a>(<a href="intent.md#0x1_intent">intent</a>) == <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(issuer), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="intent.md#0x1_intent_ENOT_OWNER">ENOT_OWNER</a>));
    <b>let</b> <a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a> {
        offered_resource,
        argument: _,
        expiry_time: _,
        self_delete_ref,
        witness_type: _,
    } = <b>move_from</b>&lt;<a href="intent.md#0x1_intent_TradeIntent">TradeIntent</a>&lt;Source, Args&gt;&gt;(<a href="object.md#0x1_object_object_address">object::object_address</a>(&<a href="intent.md#0x1_intent">intent</a>));

    <a href="object.md#0x1_object_delete">object::delete</a>(self_delete_ref);
    offered_resource
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
