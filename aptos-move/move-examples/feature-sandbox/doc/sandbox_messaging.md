
<a name="0x22_sandbox_messaging"></a>

# Module `0x22::sandbox_messaging`

An example module similar to the hello blockchain demo

This module is to show off all features that can be used in Move in a simple way.

Here is an example of a doc comment.  The doc comments will be used when running the doc generator,
and at this location it will document for the module.


-  [Resource `MessageHolder`](#0x22_sandbox_messaging_MessageHolder)
-  [Struct `MessageChangeEvent`](#0x22_sandbox_messaging_MessageChangeEvent)
-  [Constants](#@Constants_0)
-  [Function `get_message`](#0x22_sandbox_messaging_get_message)
-  [Function `get_message_and_revision`](#0x22_sandbox_messaging_get_message_and_revision)
-  [Function `set_message_admin`](#0x22_sandbox_messaging_set_message_admin)
-  [Function `set_message`](#0x22_sandbox_messaging_set_message)


<pre><code><b>use</b> <a href="../../../framework/aptos-framework/doc/account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../../framework/aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
</code></pre>



<a name="0x22_sandbox_messaging_MessageHolder"></a>

## Resource `MessageHolder`

A message holder resource for this example.  This resource contains
an individual string as a message, and an event handle for Aptos events.

You can see the generic phantom type for the MessageHolder.  This allows us to declar
a type that might not be used directly in the struct, but instead as an inner
generic type.  Drop and store allow the type to be dropped or stored in a holding type.

Additionally, the MessageHolder has key, which allows it to be stored directly in global storage.

These doc comments also work on structs as seen here


<pre><code><b>struct</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_MessageHolder">MessageHolder</a>&lt;T: drop, store&gt; <b>has</b> key
</code></pre>



<a name="0x22_sandbox_messaging_MessageChangeEvent"></a>

## Struct `MessageChangeEvent`

An event for when the holding message changes

This allows us to define a message that will be returned through the events API on the Aptos Full node.
These messages are not accessible within move, and can only have one way communication out to the events
API.


<pre><code><b>struct</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_MessageChangeEvent">MessageChangeEvent</a> <b>has</b> drop, store
</code></pre>



<a name="@Constants_0"></a>

## Constants


<a name="0x22_sandbox_messaging_ENOT_ADMIN"></a>

The action is an admin action, and the caller is not the admin


<pre><code><b>const</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_ENOT_ADMIN">ENOT_ADMIN</a>: u64 = 2;
</code></pre>



<a name="0x22_sandbox_messaging_ENO_MESSAGE_HOLDER"></a>

There is no message holder present at address


<pre><code><b>const</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_ENO_MESSAGE_HOLDER">ENO_MESSAGE_HOLDER</a>: u64 = 1;
</code></pre>



<a name="0x22_sandbox_messaging_get_message"></a>

## Function `get_message`

Retrieves the message from the struct

View functions allow you to return arbitrary data, so we can easily see the internal value of the MessageHolder
without doing extra parsing.

This is also a public function.  This can be called by any other module or in any Move script.  Functions can
return any type.

Functions can have doc comments, which will show up in documentation.


<pre><code><b>public</b> <b>fun</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_get_message">get_message</a>(<b>address</b>: <b>address</b>): <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<a name="0x22_sandbox_messaging_get_message_and_revision"></a>

## Function `get_message_and_revision`

Functions can be friend functions.  These are able to be called from other modules that are declared friends.

Additionally, here's an example of returning a tuple.  Multiple values can be returned at once.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_get_message_and_revision">get_message_and_revision</a>(<b>address</b>: <b>address</b>): (u64, <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<a name="0x22_sandbox_messaging_set_message_admin"></a>

## Function `set_message_admin`

An admin private entry function for setting messages of accounts that have already created message holders

Private entry functions allow you to create externally callable functions in a transaction that cannot
be called from another module.  Entry functions cannot take structs as arguments, and cannot return values.

This allows only the admin (the deployer) to override messages in the account


<pre><code>entry <b>fun</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_set_message_admin">set_message_admin</a>(<a href="../../../framework/aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, message_address: <b>address</b>, message: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<a name="0x22_sandbox_messaging_set_message"></a>

## Function `set_message`

Sets a message for the signer's account

Public entry functiosn allow you to create externally callable functions that can be called from other modules.
Entry functions cannot take structs as arguments, and cannot return values.

Only the owner of the account holding the resource can update the message.


<pre><code><b>public</b> entry <b>fun</b> <a href="sandbox_messaging.md#0x22_sandbox_messaging_set_message">set_message</a>(<a href="../../../framework/aptos-framework/doc/account.md#0x1_account">account</a>: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, message: <a href="../../../framework/aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>
