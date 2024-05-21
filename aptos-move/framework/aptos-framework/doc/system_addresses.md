
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


<pre><code>use 0x1::error;
use 0x1::signer;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_system_addresses_ENOT_APTOS_FRAMEWORK_ADDRESS"></a>

The address/account did not correspond to the core framework address


<pre><code>const ENOT_APTOS_FRAMEWORK_ADDRESS: u64 &#61; 3;
</code></pre>



<a id="0x1_system_addresses_ENOT_CORE_RESOURCE_ADDRESS"></a>

The address/account did not correspond to the core resource address


<pre><code>const ENOT_CORE_RESOURCE_ADDRESS: u64 &#61; 1;
</code></pre>



<a id="0x1_system_addresses_ENOT_FRAMEWORK_RESERVED_ADDRESS"></a>

The address is not framework reserved address


<pre><code>const ENOT_FRAMEWORK_RESERVED_ADDRESS: u64 &#61; 4;
</code></pre>



<a id="0x1_system_addresses_EVM"></a>

The operation can only be performed by the VM


<pre><code>const EVM: u64 &#61; 2;
</code></pre>



<a id="0x1_system_addresses_assert_core_resource"></a>

## Function `assert_core_resource`



<pre><code>public fun assert_core_resource(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_core_resource(account: &amp;signer) &#123;
    assert_core_resource_address(signer::address_of(account))
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_assert_core_resource_address"></a>

## Function `assert_core_resource_address`



<pre><code>public fun assert_core_resource_address(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_core_resource_address(addr: address) &#123;
    assert!(is_core_resource_address(addr), error::permission_denied(ENOT_CORE_RESOURCE_ADDRESS))
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_core_resource_address"></a>

## Function `is_core_resource_address`



<pre><code>public fun is_core_resource_address(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_core_resource_address(addr: address): bool &#123;
    addr &#61;&#61; @core_resources
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_assert_aptos_framework"></a>

## Function `assert_aptos_framework`



<pre><code>public fun assert_aptos_framework(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_aptos_framework(account: &amp;signer) &#123;
    assert!(
        is_aptos_framework_address(signer::address_of(account)),
        error::permission_denied(ENOT_APTOS_FRAMEWORK_ADDRESS),
    )
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_assert_framework_reserved_address"></a>

## Function `assert_framework_reserved_address`



<pre><code>public fun assert_framework_reserved_address(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_framework_reserved_address(account: &amp;signer) &#123;
    assert_framework_reserved(signer::address_of(account));
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_assert_framework_reserved"></a>

## Function `assert_framework_reserved`



<pre><code>public fun assert_framework_reserved(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_framework_reserved(addr: address) &#123;
    assert!(
        is_framework_reserved_address(addr),
        error::permission_denied(ENOT_FRAMEWORK_RESERVED_ADDRESS),
    )
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_framework_reserved_address"></a>

## Function `is_framework_reserved_address`

Return true if <code>addr</code> is 0x0 or under the on chain governance's control.


<pre><code>public fun is_framework_reserved_address(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_framework_reserved_address(addr: address): bool &#123;
    is_aptos_framework_address(addr) &#124;&#124;
        addr &#61;&#61; @0x2 &#124;&#124;
        addr &#61;&#61; @0x3 &#124;&#124;
        addr &#61;&#61; @0x4 &#124;&#124;
        addr &#61;&#61; @0x5 &#124;&#124;
        addr &#61;&#61; @0x6 &#124;&#124;
        addr &#61;&#61; @0x7 &#124;&#124;
        addr &#61;&#61; @0x8 &#124;&#124;
        addr &#61;&#61; @0x9 &#124;&#124;
        addr &#61;&#61; @0xa
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_aptos_framework_address"></a>

## Function `is_aptos_framework_address`

Return true if <code>addr</code> is 0x1.


<pre><code>public fun is_aptos_framework_address(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_aptos_framework_address(addr: address): bool &#123;
    addr &#61;&#61; @aptos_framework
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_assert_vm"></a>

## Function `assert_vm`

Assert that the signer has the VM reserved address.


<pre><code>public fun assert_vm(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun assert_vm(account: &amp;signer) &#123;
    assert!(is_vm(account), error::permission_denied(EVM))
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_vm"></a>

## Function `is_vm`

Return true if <code>addr</code> is a reserved address for the VM to call system modules.


<pre><code>public fun is_vm(account: &amp;signer): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_vm(account: &amp;signer): bool &#123;
    is_vm_address(signer::address_of(account))
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_vm_address"></a>

## Function `is_vm_address`

Return true if <code>addr</code> is a reserved address for the VM to call system modules.


<pre><code>public fun is_vm_address(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_vm_address(addr: address): bool &#123;
    addr &#61;&#61; @vm_reserved
&#125;
</code></pre>



</details>

<a id="0x1_system_addresses_is_reserved_address"></a>

## Function `is_reserved_address`

Return true if <code>addr</code> is either the VM address or an Aptos Framework address.


<pre><code>public fun is_reserved_address(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_reserved_address(addr: address): bool &#123;
    is_aptos_framework_address(addr) &#124;&#124; is_vm_address(addr)
&#125;
</code></pre>



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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_assert_core_resource"></a>

### Function `assert_core_resource`


<pre><code>public fun assert_core_resource(account: &amp;signer)
</code></pre>




<pre><code>pragma opaque;
include AbortsIfNotCoreResource &#123; addr: signer::address_of(account) &#125;;
</code></pre>



<a id="@Specification_1_assert_core_resource_address"></a>

### Function `assert_core_resource_address`


<pre><code>public fun assert_core_resource_address(addr: address)
</code></pre>




<pre><code>pragma opaque;
include AbortsIfNotCoreResource;
</code></pre>



<a id="@Specification_1_is_core_resource_address"></a>

### Function `is_core_resource_address`


<pre><code>public fun is_core_resource_address(addr: address): bool
</code></pre>




<pre><code>pragma opaque;
aborts_if false;
ensures result &#61;&#61; (addr &#61;&#61; @core_resources);
</code></pre>



<a id="@Specification_1_assert_aptos_framework"></a>

### Function `assert_aptos_framework`


<pre><code>public fun assert_aptos_framework(account: &amp;signer)
</code></pre>




<pre><code>pragma opaque;
include AbortsIfNotAptosFramework;
</code></pre>



<a id="@Specification_1_assert_framework_reserved_address"></a>

### Function `assert_framework_reserved_address`


<pre><code>public fun assert_framework_reserved_address(account: &amp;signer)
</code></pre>




<pre><code>aborts_if !is_framework_reserved_address(signer::address_of(account));
</code></pre>



<a id="@Specification_1_assert_framework_reserved"></a>

### Function `assert_framework_reserved`


<pre><code>public fun assert_framework_reserved(addr: address)
</code></pre>




<pre><code>aborts_if !is_framework_reserved_address(addr);
</code></pre>


Specifies that a function aborts if the account does not have the aptos framework address.


<a id="0x1_system_addresses_AbortsIfNotAptosFramework"></a>


<pre><code>schema AbortsIfNotAptosFramework &#123;
    account: signer;
    // This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
    aborts_if signer::address_of(account) !&#61; @aptos_framework with error::PERMISSION_DENIED;
&#125;
</code></pre>



<a id="@Specification_1_assert_vm"></a>

### Function `assert_vm`


<pre><code>public fun assert_vm(account: &amp;signer)
</code></pre>




<pre><code>pragma opaque;
include AbortsIfNotVM;
</code></pre>


Specifies that a function aborts if the account does not have the VM reserved address.


<a id="0x1_system_addresses_AbortsIfNotVM"></a>


<pre><code>schema AbortsIfNotVM &#123;
    account: signer;
    // This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
    aborts_if signer::address_of(account) !&#61; @vm_reserved with error::PERMISSION_DENIED;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
