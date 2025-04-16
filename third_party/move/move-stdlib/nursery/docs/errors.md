
<a id="0x1_errors"></a>

# Module `0x1::errors`

Module defining error codes used in Move aborts throughout the framework.

A <code>u64</code> error code is constructed from two values:

1. The *error category* which is encoded in the lower 8 bits of the code. Error categories are
declared in this module and are globally unique across the Diem framework. There is a limited
fixed set of predefined categories, and the framework is guaranteed to use those consistently.

2. The *error reason* which is encoded in the remaining 56 bits of the code. The reason is a unique
number relative to the module which raised the error and can be used to obtain more information about
the error at hand. It is mostly used for diagnosis purposes. Error reasons may change over time as the
framework evolves.

>TODO: determine what kind of stability guarantees we give about reasons/associated module.


-  [Constants](#@Constants_0)
-  [Function `make`](#0x1_errors_make)
-  [Function `invalid_state`](#0x1_errors_invalid_state)
-  [Function `requires_address`](#0x1_errors_requires_address)
-  [Function `requires_role`](#0x1_errors_requires_role)
-  [Function `requires_capability`](#0x1_errors_requires_capability)
-  [Function `not_published`](#0x1_errors_not_published)
-  [Function `already_published`](#0x1_errors_already_published)
-  [Function `invalid_argument`](#0x1_errors_invalid_argument)
-  [Function `limit_exceeded`](#0x1_errors_limit_exceeded)
-  [Function `internal`](#0x1_errors_internal)
-  [Function `custom`](#0x1_errors_custom)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_errors_ALREADY_PUBLISHED"></a>

Attempting to publish a resource that is already published. Example: calling an initialization function
twice.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_ALREADY_PUBLISHED">ALREADY_PUBLISHED</a>: u8 = 6;
</code></pre>



<a id="0x1_errors_CUSTOM"></a>

A custom error category for extension points.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_CUSTOM">CUSTOM</a>: u8 = 255;
</code></pre>



<a id="0x1_errors_INTERNAL"></a>

An internal error (bug) has occurred.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_INTERNAL">INTERNAL</a>: u8 = 10;
</code></pre>



<a id="0x1_errors_INVALID_ARGUMENT"></a>

An argument provided to an operation is invalid. Example: a signing key has the wrong format.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_INVALID_ARGUMENT">INVALID_ARGUMENT</a>: u8 = 7;
</code></pre>



<a id="0x1_errors_INVALID_STATE"></a>

The system is in a state where the performed operation is not allowed. Example: call to a function only allowed
in genesis.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_INVALID_STATE">INVALID_STATE</a>: u8 = 1;
</code></pre>



<a id="0x1_errors_LIMIT_EXCEEDED"></a>

A limit on an amount, e.g. a currency, is exceeded. Example: withdrawal of money after account limits window
is exhausted.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_LIMIT_EXCEEDED">LIMIT_EXCEEDED</a>: u8 = 8;
</code></pre>



<a id="0x1_errors_NOT_PUBLISHED"></a>

A resource is required but not published. Example: access to non-existing AccountLimits resource.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_NOT_PUBLISHED">NOT_PUBLISHED</a>: u8 = 5;
</code></pre>



<a id="0x1_errors_REQUIRES_ADDRESS"></a>

The signer of a transaction does not have the expected address for this operation. Example: a call to a function
which publishes a resource under a particular address.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_REQUIRES_ADDRESS">REQUIRES_ADDRESS</a>: u8 = 2;
</code></pre>



<a id="0x1_errors_REQUIRES_CAPABILITY"></a>

The signer of a transaction does not have a required capability.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_REQUIRES_CAPABILITY">REQUIRES_CAPABILITY</a>: u8 = 4;
</code></pre>



<a id="0x1_errors_REQUIRES_ROLE"></a>

The signer of a transaction does not have the expected  role for this operation. Example: a call to a function
which requires the signer to have the role of treasury compliance.


<pre><code><b>const</b> <a href="errors.md#0x1_errors_REQUIRES_ROLE">REQUIRES_ROLE</a>: u8 = 3;
</code></pre>



<a id="0x1_errors_make"></a>

## Function `make`

A function to create an error from a category and a reason.


<pre><code><b>fun</b> <a href="errors.md#0x1_errors_make">make</a>(category: u8, reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="errors.md#0x1_errors_make">make</a>(category: u8, reason: u64): u64 {
    (category <b>as</b> u64) + (reason &lt;&lt; 8)
}
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>ensures</b> [concrete] result == category + (reason &lt;&lt; 8) % (1 &lt;&lt; 64);
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == category;
</code></pre>



</details>

<a id="0x1_errors_invalid_state"></a>

## Function `invalid_state`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_invalid_state">invalid_state</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_invalid_state">invalid_state</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_INVALID_STATE">INVALID_STATE</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_INVALID_STATE">INVALID_STATE</a>;
</code></pre>



</details>

<a id="0x1_errors_requires_address"></a>

## Function `requires_address`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_address">requires_address</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_address">requires_address</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_REQUIRES_ADDRESS">REQUIRES_ADDRESS</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_REQUIRES_ADDRESS">REQUIRES_ADDRESS</a>;
</code></pre>



</details>

<a id="0x1_errors_requires_role"></a>

## Function `requires_role`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_role">requires_role</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_role">requires_role</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_REQUIRES_ROLE">REQUIRES_ROLE</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_REQUIRES_ROLE">REQUIRES_ROLE</a>;
</code></pre>



</details>

<a id="0x1_errors_requires_capability"></a>

## Function `requires_capability`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_capability">requires_capability</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_requires_capability">requires_capability</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_REQUIRES_CAPABILITY">REQUIRES_CAPABILITY</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_REQUIRES_CAPABILITY">REQUIRES_CAPABILITY</a>;
</code></pre>



</details>

<a id="0x1_errors_not_published"></a>

## Function `not_published`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_not_published">not_published</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_not_published">not_published</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_NOT_PUBLISHED">NOT_PUBLISHED</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_NOT_PUBLISHED">NOT_PUBLISHED</a>;
</code></pre>



</details>

<a id="0x1_errors_already_published"></a>

## Function `already_published`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_already_published">already_published</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_already_published">already_published</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_ALREADY_PUBLISHED">ALREADY_PUBLISHED</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_ALREADY_PUBLISHED">ALREADY_PUBLISHED</a>;
</code></pre>



</details>

<a id="0x1_errors_invalid_argument"></a>

## Function `invalid_argument`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_invalid_argument">invalid_argument</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_invalid_argument">invalid_argument</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_INVALID_ARGUMENT">INVALID_ARGUMENT</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_INVALID_ARGUMENT">INVALID_ARGUMENT</a>;
</code></pre>



</details>

<a id="0x1_errors_limit_exceeded"></a>

## Function `limit_exceeded`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_limit_exceeded">limit_exceeded</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_limit_exceeded">limit_exceeded</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_LIMIT_EXCEEDED">LIMIT_EXCEEDED</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_LIMIT_EXCEEDED">LIMIT_EXCEEDED</a>;
</code></pre>



</details>

<a id="0x1_errors_internal"></a>

## Function `internal`



<pre><code><b>public</b> <b>fun</b> <b>internal</b>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <b>internal</b>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_INTERNAL">INTERNAL</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_INTERNAL">INTERNAL</a>;
</code></pre>



</details>

<a id="0x1_errors_custom"></a>

## Function `custom`



<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_custom">custom</a>(reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="errors.md#0x1_errors_custom">custom</a>(reason: u64): u64 { <a href="errors.md#0x1_errors_make">make</a>(<a href="errors.md#0x1_errors_CUSTOM">CUSTOM</a>, reason) }
</code></pre>



</details>

<details>
<summary>Specification</summary>



<pre><code><b>pragma</b> opaque = <b>true</b>;
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="errors.md#0x1_errors_CUSTOM">CUSTOM</a>;
</code></pre>



</details>
