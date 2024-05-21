
<a id="0x1_version"></a>

# Module `0x1::version`

Maintains the version number for the blockchain.


-  [Resource `Version`](#0x1_version_Version)
-  [Resource `SetVersionCapability`](#0x1_version_SetVersionCapability)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_version_initialize)
-  [Function `set_version`](#0x1_version_set_version)
-  [Function `set_for_next_epoch`](#0x1_version_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_version_on_new_epoch)
-  [Function `initialize_for_test`](#0x1_version_initialize_for_test)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set_version`](#@Specification_1_set_version)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `initialize_for_test`](#@Specification_1_initialize_for_test)


<pre><code>use 0x1::chain_status;<br/>use 0x1::config_buffer;<br/>use 0x1::error;<br/>use 0x1::reconfiguration;<br/>use 0x1::signer;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_version_Version"></a>

## Resource `Version`



<pre><code>struct Version has drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>major: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_version_SetVersionCapability"></a>

## Resource `SetVersionCapability`



<pre><code>struct SetVersionCapability has key<br/></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_version_EINVALID_MAJOR_VERSION_NUMBER"></a>

Specified major version number must be greater than current version number.


<pre><code>const EINVALID_MAJOR_VERSION_NUMBER: u64 &#61; 1;<br/></code></pre>



<a id="0x1_version_ENOT_AUTHORIZED"></a>

Account is not authorized to make this change.


<pre><code>const ENOT_AUTHORIZED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_version_initialize"></a>

## Function `initialize`

Only called during genesis.
Publishes the Version config.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, initial_version: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, initial_version: u64) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    move_to(aptos_framework, Version &#123; major: initial_version &#125;);<br/>    // Give aptos framework account capability to call set version. This allows on chain governance to do it through<br/>    // control of the aptos framework account.<br/>    move_to(aptos_framework, SetVersionCapability &#123;&#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_version_set_version"></a>

## Function `set_version`

Deprecated by <code>set_for_next_epoch()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public entry fun set_version(account: &amp;signer, major: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_version(account: &amp;signer, major: u64) acquires Version &#123;<br/>    assert!(exists&lt;SetVersionCapability&gt;(signer::address_of(account)), error::permission_denied(ENOT_AUTHORIZED));<br/>    chain_status::assert_genesis();<br/><br/>    let old_major &#61; borrow_global&lt;Version&gt;(@aptos_framework).major;<br/>    assert!(old_major &lt; major, error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER));<br/><br/>    let config &#61; borrow_global_mut&lt;Version&gt;(@aptos_framework);<br/>    config.major &#61; major;<br/><br/>    // Need to trigger reconfiguration so validator nodes can sync on the updated version.<br/>    reconfiguration::reconfigure();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_version_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Used in on-chain governances to update the major version for the next epoch.
Example usage:
- <code>aptos_framework::version::set_for_next_epoch(&amp;framework_signer, new_version);</code>
- <code>aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);</code>


<pre><code>public entry fun set_for_next_epoch(account: &amp;signer, major: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun set_for_next_epoch(account: &amp;signer, major: u64) acquires Version &#123;<br/>    assert!(exists&lt;SetVersionCapability&gt;(signer::address_of(account)), error::permission_denied(ENOT_AUTHORIZED));<br/>    let old_major &#61; borrow_global&lt;Version&gt;(@aptos_framework).major;<br/>    assert!(old_major &lt; major, error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER));<br/>    config_buffer::upsert(Version &#123;major&#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_version_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>Version</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires Version &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (config_buffer::does_exist&lt;Version&gt;()) &#123;<br/>        let new_value &#61; config_buffer::extract&lt;Version&gt;();<br/>        if (exists&lt;Version&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;Version&gt;(@aptos_framework) &#61; new_value;<br/>        &#125; else &#123;<br/>            move_to(framework, new_value);<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_version_initialize_for_test"></a>

## Function `initialize_for_test`

Only called in tests and testnets. This allows the core resources account, which only exists in tests/testnets,
to update the version.


<pre><code>fun initialize_for_test(core_resources: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize_for_test(core_resources: &amp;signer) &#123;<br/>    system_addresses::assert_core_resource(core_resources);<br/>    move_to(core_resources, SetVersionCapability &#123;&#125;);<br/>&#125;<br/></code></pre>



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
<td>During genesis, the Version resource should be initialized with the initial version and stored along with its capability under the aptos framework account.</td>
<td>Medium</td>
<td>The initialize function ensures that the signer is the aptos framework account and stores the Version and SetVersionCapability resources in it.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The version should be updateable after initialization, but only by the Aptos framework account and with an increasing version number.</td>
<td>Medium</td>
<td>The version number for the blockchain should be updatable whenever necessary. This functionality is provided by the set_version function which ensures that the new version is greater than the previous one.</td>
<td>Formally verified via <a href="#high-level-req-2">set_version</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, initial_version: u64)<br/></code></pre>


Abort if resource already exists in <code>@aptos_framwork</code> when initializing.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
aborts_if signer::address_of(aptos_framework) !&#61; @aptos_framework;<br/>aborts_if exists&lt;Version&gt;(@aptos_framework);<br/>aborts_if exists&lt;SetVersionCapability&gt;(@aptos_framework);<br/>ensures exists&lt;Version&gt;(@aptos_framework);<br/>ensures exists&lt;SetVersionCapability&gt;(@aptos_framework);<br/>ensures global&lt;Version&gt;(@aptos_framework) &#61;&#61; Version &#123; major: initial_version &#125;;<br/>ensures global&lt;SetVersionCapability&gt;(@aptos_framework) &#61;&#61; SetVersionCapability &#123;&#125;;<br/></code></pre>



<a id="@Specification_1_set_version"></a>

### Function `set_version`


<pre><code>public entry fun set_version(account: &amp;signer, major: u64)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 120;<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>include staking_config::StakingRewardsConfigRequirement;<br/>requires chain_status::is_genesis();<br/>requires timestamp::spec_now_microseconds() &gt;&#61; reconfiguration::last_reconfiguration_time();<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>aborts_if !exists&lt;SetVersionCapability&gt;(signer::address_of(account));<br/>aborts_if !exists&lt;Version&gt;(@aptos_framework);<br/>let old_major &#61; global&lt;Version&gt;(@aptos_framework).major;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if !(old_major &lt; major);<br/>ensures global&lt;Version&gt;(@aptos_framework).major &#61;&#61; major;<br/></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code>public entry fun set_for_next_epoch(account: &amp;signer, major: u64)<br/></code></pre>




<pre><code>aborts_if !exists&lt;SetVersionCapability&gt;(signer::address_of(account));<br/>aborts_if !exists&lt;Version&gt;(@aptos_framework);<br/>aborts_if global&lt;Version&gt;(@aptos_framework).major &gt;&#61; major;<br/>aborts_if !exists&lt;config_buffer::PendingConfigs&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);<br/>include config_buffer::OnNewEpochRequirement&lt;Version&gt;;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_initialize_for_test"></a>

### Function `initialize_for_test`


<pre><code>fun initialize_for_test(core_resources: &amp;signer)<br/></code></pre>


This module turns on <code>aborts_if_is_strict</code>, so need to add spec for test function <code>initialize_for_test</code>.


<pre><code>pragma verify &#61; false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
