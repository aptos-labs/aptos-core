
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


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_version_Version"></a>

## Resource `Version`



<pre><code><b>struct</b> <a href="version.md#0x1_version_Version">Version</a> <b>has</b> drop, store, key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a> <b>has</b> key<br /></code></pre>



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


<pre><code><b>const</b> <a href="version.md#0x1_version_EINVALID_MAJOR_VERSION_NUMBER">EINVALID_MAJOR_VERSION_NUMBER</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_version_ENOT_AUTHORIZED"></a>

Account is not authorized to make this change.


<pre><code><b>const</b> <a href="version.md#0x1_version_ENOT_AUTHORIZED">ENOT_AUTHORIZED</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_version_initialize"></a>

## Function `initialize`

Only called during genesis.
Publishes the Version config.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_version: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_version: u64) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>move_to</b>(aptos_framework, <a href="version.md#0x1_version_Version">Version</a> &#123; major: initial_version &#125;);<br />    // Give aptos framework <a href="account.md#0x1_account">account</a> capability <b>to</b> call set <a href="version.md#0x1_version">version</a>. This allows on chain governance <b>to</b> do it through<br />    // control of the aptos framework <a href="account.md#0x1_account">account</a>.<br />    <b>move_to</b>(aptos_framework, <a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a> &#123;&#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_version_set_version"></a>

## Function `set_version`

Deprecated by <code><a href="version.md#0x1_version_set_for_next_epoch">set_for_next_epoch</a>()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_version">set_version</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_version">set_version</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64) <b>acquires</b> <a href="version.md#0x1_version_Version">Version</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="version.md#0x1_version_ENOT_AUTHORIZED">ENOT_AUTHORIZED</a>));<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> old_major &#61; <b>borrow_global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework).major;<br />    <b>assert</b>!(old_major &lt; major, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="version.md#0x1_version_EINVALID_MAJOR_VERSION_NUMBER">EINVALID_MAJOR_VERSION_NUMBER</a>));<br /><br />    <b>let</b> config &#61; <b>borrow_global_mut</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework);<br />    config.major &#61; major;<br /><br />    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated <a href="version.md#0x1_version">version</a>.<br />    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_version_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Used in on&#45;chain governances to update the major version for the next epoch.
Example usage:
&#45; <code>aptos_framework::version::set_for_next_epoch(&amp;framework_signer, new_version);</code>
&#45; <code>aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);</code>


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64) <b>acquires</b> <a href="version.md#0x1_version_Version">Version</a> &#123;<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="version.md#0x1_version_ENOT_AUTHORIZED">ENOT_AUTHORIZED</a>));<br />    <b>let</b> old_major &#61; <b>borrow_global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework).major;<br />    <b>assert</b>!(old_major &lt; major, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="version.md#0x1_version_EINVALID_MAJOR_VERSION_NUMBER">EINVALID_MAJOR_VERSION_NUMBER</a>));<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="version.md#0x1_version_Version">Version</a> &#123;major&#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_version_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="version.md#0x1_version_Version">Version</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="version.md#0x1_version_Version">Version</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;()) &#123;<br />        <b>let</b> new_value &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework) &#61; new_value;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_value);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_version_initialize_for_test"></a>

## Function `initialize_for_test`

Only called in tests and testnets. This allows the core resources account, which only exists in tests/testnets,
to update the version.


<pre><code><b>fun</b> <a href="version.md#0x1_version_initialize_for_test">initialize_for_test</a>(core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="version.md#0x1_version_initialize_for_test">initialize_for_test</a>(core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_core_resource">system_addresses::assert_core_resource</a>(core_resources);<br />    <b>move_to</b>(core_resources, <a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a> &#123;&#125;);<br />&#125;<br /></code></pre>



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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, initial_version: u64)<br /></code></pre>


Abort if resource already exists in <code>@aptos_framwork</code> when initializing.


<pre><code>// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework) !&#61; @aptos_framework;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework) &#61;&#61; <a href="version.md#0x1_version_Version">Version</a> &#123; major: initial_version &#125;;<br /><b>ensures</b> <b>global</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(@aptos_framework) &#61;&#61; <a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a> &#123;&#125;;<br /></code></pre>



<a id="@Specification_1_set_version"></a>

### Function `set_version`


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_version">set_version</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 120;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>();<br /><b>requires</b> <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &gt;&#61; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">reconfiguration::last_reconfiguration_time</a>();<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework);<br /><b>let</b> old_major &#61; <b>global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework).major;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> !(old_major &lt; major);<br /><b>ensures</b> <b>global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework).major &#61;&#61; major;<br /></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code><b>public</b> entry <b>fun</b> <a href="version.md#0x1_version_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, major: u64)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="version.md#0x1_version_SetVersionCapability">SetVersionCapability</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>global</b>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;(@aptos_framework).major &gt;&#61; major;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">config_buffer::PendingConfigs</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="version.md#0x1_version_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="version.md#0x1_version_Version">Version</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_initialize_for_test"></a>

### Function `initialize_for_test`


<pre><code><b>fun</b> <a href="version.md#0x1_version_initialize_for_test">initialize_for_test</a>(core_resources: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


This module turns on <code>aborts_if_is_strict</code>, so need to add spec for test function <code>initialize_for_test</code>.


<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
