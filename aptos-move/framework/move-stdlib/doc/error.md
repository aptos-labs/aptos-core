
<a id="0x1_error"></a>

# Module `0x1::error`

This module defines a set of canonical error codes which are optional to use by applications for the
<code>abort</code> and <code>assert!</code> features.

Canonical error codes use the 3 lowest bytes of the u64 abort code range (the upper 5 bytes are free for other use).
Of those, the highest byte represents the *error category* and the lower two bytes the *error reason*.
Given an error category <code>0x1</code> and a reason <code>0x3</code>, a canonical abort code looks as <code>0x10003</code>.

A module can use a canonical code with a constant declaration of the following form:

```
///  An invalid ASCII character was encountered when creating a string.
const EINVALID_CHARACTER: u64 = 0x010003;
```

This code is both valid in the worlds with and without canonical errors. It can be used as a plain module local
error reason understand by the existing error map tooling, or as a canonical code.

The actual canonical categories have been adopted from Google's canonical error codes, which in turn are derived
from Unix error codes [see here](https://cloud.google.com/apis/design/errors#handling_errors). Each code has an
associated HTTP error code which can be used in REST apis. The mapping from error code to http code is not 1:1;
error codes here are a bit richer than HTTP codes.


-  [Constants](#@Constants_0)
-  [Function `canonical`](#0x1_error_canonical)
-  [Function `invalid_argument`](#0x1_error_invalid_argument)
-  [Function `out_of_range`](#0x1_error_out_of_range)
-  [Function `invalid_state`](#0x1_error_invalid_state)
-  [Function `unauthenticated`](#0x1_error_unauthenticated)
-  [Function `permission_denied`](#0x1_error_permission_denied)
-  [Function `not_found`](#0x1_error_not_found)
-  [Function `aborted`](#0x1_error_aborted)
-  [Function `already_exists`](#0x1_error_already_exists)
-  [Function `resource_exhausted`](#0x1_error_resource_exhausted)
-  [Function `internal`](#0x1_error_internal)
-  [Function `not_implemented`](#0x1_error_not_implemented)
-  [Function `unavailable`](#0x1_error_unavailable)
-  [Specification](#@Specification_1)
    -  [Function `canonical`](#@Specification_1_canonical)


<pre><code></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_error_ABORTED"></a>

Concurrency conflict, such as read-modify-write conflict (http: 409)


<pre><code>const ABORTED: u64 &#61; 7;
</code></pre>



<a id="0x1_error_ALREADY_EXISTS"></a>

The resource that a client tried to create already exists (http: 409)


<pre><code>const ALREADY_EXISTS: u64 &#61; 8;
</code></pre>



<a id="0x1_error_CANCELLED"></a>

Request cancelled by the client (http: 499)


<pre><code>const CANCELLED: u64 &#61; 10;
</code></pre>



<a id="0x1_error_INTERNAL"></a>

Internal error (http: 500)


<pre><code>const INTERNAL: u64 &#61; 11;
</code></pre>



<a id="0x1_error_INVALID_ARGUMENT"></a>

Caller specified an invalid argument (http: 400)


<pre><code>const INVALID_ARGUMENT: u64 &#61; 1;
</code></pre>



<a id="0x1_error_INVALID_STATE"></a>

The system is not in a state where the operation can be performed (http: 400)


<pre><code>const INVALID_STATE: u64 &#61; 3;
</code></pre>



<a id="0x1_error_NOT_FOUND"></a>

A specified resource is not found (http: 404)


<pre><code>const NOT_FOUND: u64 &#61; 6;
</code></pre>



<a id="0x1_error_NOT_IMPLEMENTED"></a>

Feature not implemented (http: 501)


<pre><code>const NOT_IMPLEMENTED: u64 &#61; 12;
</code></pre>



<a id="0x1_error_OUT_OF_RANGE"></a>

An input or result of a computation is out of range (http: 400)


<pre><code>const OUT_OF_RANGE: u64 &#61; 2;
</code></pre>



<a id="0x1_error_PERMISSION_DENIED"></a>

client does not have sufficient permission (http: 403)


<pre><code>const PERMISSION_DENIED: u64 &#61; 5;
</code></pre>



<a id="0x1_error_RESOURCE_EXHAUSTED"></a>

Out of gas or other forms of quota (http: 429)


<pre><code>const RESOURCE_EXHAUSTED: u64 &#61; 9;
</code></pre>



<a id="0x1_error_UNAUTHENTICATED"></a>

Request not authenticated due to missing, invalid, or expired auth token (http: 401)


<pre><code>const UNAUTHENTICATED: u64 &#61; 4;
</code></pre>



<a id="0x1_error_UNAVAILABLE"></a>

The service is currently unavailable. Indicates that a retry could solve the issue (http: 503)


<pre><code>const UNAVAILABLE: u64 &#61; 13;
</code></pre>



<a id="0x1_error_canonical"></a>

## Function `canonical`

Construct a canonical error code from a category and a reason.


<pre><code>public fun canonical(category: u64, reason: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun canonical(category: u64, reason: u64): u64 &#123;
  (category &lt;&lt; 16) &#43; reason
&#125;
</code></pre>



</details>

<a id="0x1_error_invalid_argument"></a>

## Function `invalid_argument`

Functions to construct a canonical error code of the given category.


<pre><code>public fun invalid_argument(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun invalid_argument(r: u64): u64 &#123;  canonical(INVALID_ARGUMENT, r) &#125;
</code></pre>



</details>

<a id="0x1_error_out_of_range"></a>

## Function `out_of_range`



<pre><code>public fun out_of_range(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun out_of_range(r: u64): u64 &#123;  canonical(OUT_OF_RANGE, r) &#125;
</code></pre>



</details>

<a id="0x1_error_invalid_state"></a>

## Function `invalid_state`



<pre><code>public fun invalid_state(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun invalid_state(r: u64): u64 &#123;  canonical(INVALID_STATE, r) &#125;
</code></pre>



</details>

<a id="0x1_error_unauthenticated"></a>

## Function `unauthenticated`



<pre><code>public fun unauthenticated(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unauthenticated(r: u64): u64 &#123; canonical(UNAUTHENTICATED, r) &#125;
</code></pre>



</details>

<a id="0x1_error_permission_denied"></a>

## Function `permission_denied`



<pre><code>public fun permission_denied(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun permission_denied(r: u64): u64 &#123; canonical(PERMISSION_DENIED, r) &#125;
</code></pre>



</details>

<a id="0x1_error_not_found"></a>

## Function `not_found`



<pre><code>public fun not_found(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun not_found(r: u64): u64 &#123; canonical(NOT_FOUND, r) &#125;
</code></pre>



</details>

<a id="0x1_error_aborted"></a>

## Function `aborted`



<pre><code>public fun aborted(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun aborted(r: u64): u64 &#123; canonical(ABORTED, r) &#125;
</code></pre>



</details>

<a id="0x1_error_already_exists"></a>

## Function `already_exists`



<pre><code>public fun already_exists(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun already_exists(r: u64): u64 &#123; canonical(ALREADY_EXISTS, r) &#125;
</code></pre>



</details>

<a id="0x1_error_resource_exhausted"></a>

## Function `resource_exhausted`



<pre><code>public fun resource_exhausted(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resource_exhausted(r: u64): u64 &#123;  canonical(RESOURCE_EXHAUSTED, r) &#125;
</code></pre>



</details>

<a id="0x1_error_internal"></a>

## Function `internal`



<pre><code>public fun internal(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun internal(r: u64): u64 &#123;  canonical(INTERNAL, r) &#125;
</code></pre>



</details>

<a id="0x1_error_not_implemented"></a>

## Function `not_implemented`



<pre><code>public fun not_implemented(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun not_implemented(r: u64): u64 &#123;  canonical(NOT_IMPLEMENTED, r) &#125;
</code></pre>



</details>

<a id="0x1_error_unavailable"></a>

## Function `unavailable`



<pre><code>public fun unavailable(r: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun unavailable(r: u64): u64 &#123; canonical(UNAVAILABLE, r) &#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_canonical"></a>

### Function `canonical`


<pre><code>public fun canonical(category: u64, reason: u64): u64
</code></pre>




<pre><code>pragma opaque &#61; true;
let shl_res &#61; category &lt;&lt; 16;
ensures [concrete] result &#61;&#61; shl_res &#43; reason;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; category;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
