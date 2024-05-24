
<a id="0x1_system_addresses"></a>

# Module `0x1::system_addresses`



-  [Constants](#@Constants_0)
-  [Function `assert_core_resource`](#0x1_system_addresses_assert_core_resource)
-  [Function `assert_core_resource_address`](#0x1_system_addresses_assert_core_resource_address)
-  [Function `is_core_resource_address`](#0x1_system_addresses_is_core_resource_address)
-  [Function `assert_aptos_framework`](#0x1_system_addresses_assert_aptos_framework)
-  [Function `assert_framework_reserved_address`](#0x1_system_addresses_assert_framework_reserved_address)
-  [Function `assert_framework_reserved`](#0x1_system_addresses_assert_framework_reserved)
-  [Function `is_framework_reserved_address`](#0x1_system_addresses_is_framework_reserved_address)
-  [Function `is_aptos_framework_address`](#0x1_system_addresses_is_aptos_framework_address)
-  [Function `assert_vm`](#0x1_system_addresses_assert_vm)
-  [Function `is_vm`](#0x1_system_addresses_is_vm)
-  [Function `is_vm_address`](#0x1_system_addresses_is_vm_address)
-  [Function `is_reserved_address`](#0x1_system_addresses_is_reserved_address)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `assert_core_resource`](#@Specification_1_assert_core_resource)
    -  [Function `assert_core_resource_address`](#@Specification_1_assert_core_resource_address)
    -  [Function `is_core_resource_address`](#@Specification_1_is_core_resource_address)
    -  [Function `assert_aptos_framework`](#@Specification_1_assert_aptos_framework)
    -  [Function `assert_framework_reserved_address`](#@Specification_1_assert_framework_reserved_address)
    -  [Function `assert_framework_reserved`](#@Specification_1_assert_framework_reserved)
    -  [Function `assert_vm`](#@Specification_1_assert_vm)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /></code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_system_addresses_ENOT_APTOS_FRAMEWORK_ADDRESS"></a>

The address/account did not correspond to the core framework address


<pre><code><b>const</b> <a href="system_addresses.md#0x1_system_addresses_ENOT_APTOS_FRAMEWORK_ADDRESS">ENOT_APTOS_FRAMEWORK_ADDRESS</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_system_addresses_ENOT_CORE_RESOURCE_ADDRESS"></a>

The address/account did not correspond to the core resource address


<pre><code><b>const</b> <a href="system_addresses.md#0x1_system_addresses_ENOT_CORE_RESOURCE_ADDRESS">ENOT_CORE_RESOURCE_ADDRESS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_system_addresses_ENOT_FRAMEWORK_RESERVED_ADDRESS"></a>

The address is not framework reserved address


<pre><code><b>const</b> <a href="system_addresses.md#0x1_system_addresses_ENOT_FRAMEWORK_RESERVED_ADDRESS">ENOT_FRAMEWORK_RESERVED_ADDRESS</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_system_addresses_EVM"></a>

The operation can only be performed by the VM


<pre><code><b>const</b> <a href="system_addresses.md#0x1_system_addresses_EVM">EVM</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_system_addresses_assert_core_resource"></a>

## Function `assert_core_resource`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">assert_core_resource</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">assert_core_resource</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_core_resource_address">assert_core_resource_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_assert_core_resource_address"></a>

## Function `assert_core_resource_address`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource_address">assert_core_resource_address</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource_address">assert_core_resource_address</a>(addr: <b>address</b>) &#123;<br />    <b>assert</b>!(<a href="system_addresses.md#0x1_system_addresses_is_core_resource_address">is_core_resource_address</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="system_addresses.md#0x1_system_addresses_ENOT_CORE_RESOURCE_ADDRESS">ENOT_CORE_RESOURCE_ADDRESS</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_core_resource_address"></a>

## Function `is_core_resource_address`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_core_resource_address">is_core_resource_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_core_resource_address">is_core_resource_address</a>(addr: <b>address</b>): bool &#123;<br />    addr &#61;&#61; @core_resources<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_assert_aptos_framework"></a>

## Function `assert_aptos_framework`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>assert</b>!(<br />        <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">is_aptos_framework_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>)),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="system_addresses.md#0x1_system_addresses_ENOT_APTOS_FRAMEWORK_ADDRESS">ENOT_APTOS_FRAMEWORK_ADDRESS</a>),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_assert_framework_reserved_address"></a>

## Function `assert_framework_reserved_address`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved_address">assert_framework_reserved_address</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved_address">assert_framework_reserved_address</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">assert_framework_reserved</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_assert_framework_reserved"></a>

## Function `assert_framework_reserved`



<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">assert_framework_reserved</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">assert_framework_reserved</a>(addr: <b>address</b>) &#123;<br />    <b>assert</b>!(<br />        <a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">is_framework_reserved_address</a>(addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="system_addresses.md#0x1_system_addresses_ENOT_FRAMEWORK_RESERVED_ADDRESS">ENOT_FRAMEWORK_RESERVED_ADDRESS</a>),<br />    )<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_framework_reserved_address"></a>

## Function `is_framework_reserved_address`

Return true if <code>addr</code> is 0x0 or under the on chain governance&apos;s control.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">is_framework_reserved_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">is_framework_reserved_address</a>(addr: <b>address</b>): bool &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">is_aptos_framework_address</a>(addr) &#124;&#124;<br />        addr &#61;&#61; @0x2 &#124;&#124;<br />        addr &#61;&#61; @0x3 &#124;&#124;<br />        addr &#61;&#61; @0x4 &#124;&#124;<br />        addr &#61;&#61; @0x5 &#124;&#124;<br />        addr &#61;&#61; @0x6 &#124;&#124;<br />        addr &#61;&#61; @0x7 &#124;&#124;<br />        addr &#61;&#61; @0x8 &#124;&#124;<br />        addr &#61;&#61; @0x9 &#124;&#124;<br />        addr &#61;&#61; @0xa<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_aptos_framework_address"></a>

## Function `is_aptos_framework_address`

Return true if <code>addr</code> is 0x1.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">is_aptos_framework_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">is_aptos_framework_address</a>(addr: <b>address</b>): bool &#123;<br />    addr &#61;&#61; @aptos_framework<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_assert_vm"></a>

## Function `assert_vm`

Assert that the signer has the VM reserved address.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_vm">assert_vm</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_vm">assert_vm</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <b>assert</b>!(<a href="system_addresses.md#0x1_system_addresses_is_vm">is_vm</a>(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="system_addresses.md#0x1_system_addresses_EVM">EVM</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_vm"></a>

## Function `is_vm`

Return true if <code>addr</code> is a reserved address for the VM to call system modules.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">is_vm</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_vm">is_vm</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): bool &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_is_vm_address">is_vm_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_vm_address"></a>

## Function `is_vm_address`

Return true if <code>addr</code> is a reserved address for the VM to call system modules.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_vm_address">is_vm_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_vm_address">is_vm_address</a>(addr: <b>address</b>): bool &#123;<br />    addr &#61;&#61; @vm_reserved<br />&#125;<br /></code></pre>



</details>

<a id="0x1_system_addresses_is_reserved_address"></a>

## Function `is_reserved_address`

Return true if <code>addr</code> is either the VM address or an Aptos Framework address.


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_reserved_address">is_reserved_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_reserved_address">is_reserved_address</a>(addr: <b>address</b>): bool &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">is_aptos_framework_address</a>(addr) &#124;&#124; <a href="system_addresses.md#0x1_system_addresses_is_vm_address">is_vm_address</a>(addr)<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Asserting that a provided address corresponds to the Core Resources address should always yield a true result when matched.</td>
<td>Low</td>
<td>The assert_core_resource and assert_core_resource_address functions ensure that the provided signer or address belong to the @core_resources account.</td>
<td>Formally verified via <a href="#high-level-req-1">AbortsIfNotCoreResource</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Asserting that a provided address corresponds to the Aptos Framework Resources address should always yield a true result when matched.</td>
<td>High</td>
<td>The assert_aptos_framework function ensures that the provided signer belongs to the @aptos_framework account.</td>
<td>Formally verified via <a href="#high-level-req-2">AbortsIfNotAptosFramework</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Asserting that a provided address corresponds to the VM address should always yield a true result when matched.</td>
<td>High</td>
<td>The assert_vm function ensure that the provided signer belongs to the @vm_reserved account.</td>
<td>Formally verified via <a href="#high-level-req-3">AbortsIfNotVM</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_assert_core_resource"></a>

### Function `assert_core_resource`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">assert_core_resource</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotCoreResource">AbortsIfNotCoreResource</a> &#123; addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) &#125;;<br /></code></pre>



<a id="@Specification_1_assert_core_resource_address"></a>

### Function `assert_core_resource_address`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_core_resource_address">assert_core_resource_address</a>(addr: <b>address</b>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotCoreResource">AbortsIfNotCoreResource</a>;<br /></code></pre>



<a id="@Specification_1_is_core_resource_address"></a>

### Function `is_core_resource_address`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_is_core_resource_address">is_core_resource_address</a>(addr: <b>address</b>): bool<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; (addr &#61;&#61; @core_resources);<br /></code></pre>



<a id="@Specification_1_assert_aptos_framework"></a>

### Function `assert_aptos_framework`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a>;<br /></code></pre>



<a id="@Specification_1_assert_framework_reserved_address"></a>

### Function `assert_framework_reserved_address`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved_address">assert_framework_reserved_address</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">is_framework_reserved_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /></code></pre>



<a id="@Specification_1_assert_framework_reserved"></a>

### Function `assert_framework_reserved`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_framework_reserved">assert_framework_reserved</a>(addr: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_framework_reserved_address">is_framework_reserved_address</a>(addr);<br /></code></pre>


Specifies that a function aborts if the account does not have the aptos framework address.


<a id="0x1_system_addresses_AbortsIfNotAptosFramework"></a>


<pre><code><b>schema</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a> &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) !&#61; @aptos_framework <b>with</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_PERMISSION_DENIED">error::PERMISSION_DENIED</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_vm"></a>

### Function `assert_vm`


<pre><code><b>public</b> <b>fun</b> <a href="system_addresses.md#0x1_system_addresses_assert_vm">assert_vm</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotVM">AbortsIfNotVM</a>;<br /></code></pre>


Specifies that a function aborts if the account does not have the VM reserved address.


<a id="0x1_system_addresses_AbortsIfNotVM"></a>


<pre><code><b>schema</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotVM">AbortsIfNotVM</a> &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br />// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
    <b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>) !&#61; @vm_reserved <b>with</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_PERMISSION_DENIED">error::PERMISSION_DENIED</a>;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
