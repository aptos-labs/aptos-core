
<a id="0x1_reconfiguration"></a>

# Module `0x1::reconfiguration`

Publishes configuration information for validators, and issues reconfiguration events<br/> to synchronize configuration changes for the validators.


-  [Struct `NewEpochEvent`](#0x1_reconfiguration_NewEpochEvent)
-  [Struct `NewEpoch`](#0x1_reconfiguration_NewEpoch)
-  [Resource `Configuration`](#0x1_reconfiguration_Configuration)
-  [Resource `DisableReconfiguration`](#0x1_reconfiguration_DisableReconfiguration)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_reconfiguration_initialize)
-  [Function `disable_reconfiguration`](#0x1_reconfiguration_disable_reconfiguration)
-  [Function `enable_reconfiguration`](#0x1_reconfiguration_enable_reconfiguration)
-  [Function `reconfiguration_enabled`](#0x1_reconfiguration_reconfiguration_enabled)
-  [Function `reconfigure`](#0x1_reconfiguration_reconfigure)
-  [Function `last_reconfiguration_time`](#0x1_reconfiguration_last_reconfiguration_time)
-  [Function `current_epoch`](#0x1_reconfiguration_current_epoch)
-  [Function `emit_genesis_reconfiguration_event`](#0x1_reconfiguration_emit_genesis_reconfiguration_event)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `disable_reconfiguration`](#@Specification_1_disable_reconfiguration)
    -  [Function `enable_reconfiguration`](#@Specification_1_enable_reconfiguration)
    -  [Function `reconfiguration_enabled`](#@Specification_1_reconfiguration_enabled)
    -  [Function `reconfigure`](#@Specification_1_reconfigure)
    -  [Function `last_reconfiguration_time`](#@Specification_1_last_reconfiguration_time)
    -  [Function `current_epoch`](#@Specification_1_current_epoch)
    -  [Function `emit_genesis_reconfiguration_event`](#@Specification_1_emit_genesis_reconfiguration_event)


<pre><code>use 0x1::account;<br/>use 0x1::chain_status;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::reconfiguration_state;<br/>use 0x1::signer;<br/>use 0x1::stake;<br/>use 0x1::storage_gas;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/>use 0x1::transaction_fee;<br/></code></pre>



<a id="0x1_reconfiguration_NewEpochEvent"></a>

## Struct `NewEpochEvent`

Event that signals consensus to start a new epoch,<br/> with new configuration information. This is also called a<br/> &quot;reconfiguration event&quot;


<pre><code>&#35;[event]<br/>struct NewEpochEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_reconfiguration_NewEpoch"></a>

## Struct `NewEpoch`

Event that signals consensus to start a new epoch,<br/> with new configuration information. This is also called a<br/> &quot;reconfiguration event&quot;


<pre><code>&#35;[event]<br/>struct NewEpoch has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_reconfiguration_Configuration"></a>

## Resource `Configuration`

Holds information about state of reconfiguration


<pre><code>struct Configuration has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>
 Epoch number
</dd>
<dt>
<code>last_reconfiguration_time: u64</code>
</dt>
<dd>
 Time of last reconfiguration. Only changes on reconfiguration events.
</dd>
<dt>
<code>events: event::EventHandle&lt;reconfiguration::NewEpochEvent&gt;</code>
</dt>
<dd>
 Event handle for reconfiguration events
</dd>
</dl>


</details>

<a id="0x1_reconfiguration_DisableReconfiguration"></a>

## Resource `DisableReconfiguration`

Reconfiguration will be disabled if this resource is published under the<br/> aptos_framework system address


<pre><code>struct DisableReconfiguration has key<br/></code></pre>



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


<a id="0x1_reconfiguration_ECONFIG"></a>

A <code>Reconfiguration</code> resource is in an invalid state


<pre><code>const ECONFIG: u64 &#61; 2;<br/></code></pre>



<a id="0x1_reconfiguration_ECONFIGURATION"></a>

The <code>Configuration</code> resource is in an invalid state


<pre><code>const ECONFIGURATION: u64 &#61; 1;<br/></code></pre>



<a id="0x1_reconfiguration_EINVALID_BLOCK_TIME"></a>

An invalid block time was encountered.


<pre><code>const EINVALID_BLOCK_TIME: u64 &#61; 4;<br/></code></pre>



<a id="0x1_reconfiguration_EINVALID_GUID_FOR_EVENT"></a>

An invalid block time was encountered.


<pre><code>const EINVALID_GUID_FOR_EVENT: u64 &#61; 5;<br/></code></pre>



<a id="0x1_reconfiguration_EMODIFY_CAPABILITY"></a>

A <code>ModifyConfigCapability</code> is in a different state than was expected


<pre><code>const EMODIFY_CAPABILITY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_reconfiguration_initialize"></a>

## Function `initialize`

Only called during genesis.<br/> Publishes <code>Configuration</code> resource. Can only be invoked by aptos framework account, and only a single time in Genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    // assert it matches `new_epoch_event_key()`, otherwise the event can&apos;t be recognized<br/>    assert!(account::get_guid_next_creation_num(signer::address_of(aptos_framework)) &#61;&#61; 2, error::invalid_state(EINVALID_GUID_FOR_EVENT));<br/>    move_to&lt;Configuration&gt;(<br/>        aptos_framework,<br/>        Configuration &#123;<br/>            epoch: 0,<br/>            last_reconfiguration_time: 0,<br/>            events: account::new_event_handle&lt;NewEpochEvent&gt;(aptos_framework),<br/>        &#125;<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_disable_reconfiguration"></a>

## Function `disable_reconfiguration`

Private function to temporarily halt reconfiguration.<br/> This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.


<pre><code>fun disable_reconfiguration(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun disable_reconfiguration(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(reconfiguration_enabled(), error::invalid_state(ECONFIGURATION));<br/>    move_to(aptos_framework, DisableReconfiguration &#123;&#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_enable_reconfiguration"></a>

## Function `enable_reconfiguration`

Private function to resume reconfiguration.<br/> This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.


<pre><code>fun enable_reconfiguration(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun enable_reconfiguration(aptos_framework: &amp;signer) acquires DisableReconfiguration &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/><br/>    assert!(!reconfiguration_enabled(), error::invalid_state(ECONFIGURATION));<br/>    DisableReconfiguration &#123;&#125; &#61; move_from&lt;DisableReconfiguration&gt;(signer::address_of(aptos_framework));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_reconfiguration_enabled"></a>

## Function `reconfiguration_enabled`



<pre><code>fun reconfiguration_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun reconfiguration_enabled(): bool &#123;<br/>    !exists&lt;DisableReconfiguration&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_reconfigure"></a>

## Function `reconfigure`

Signal validators to start using new configuration. Must be called from friend config modules.


<pre><code>public(friend) fun reconfigure()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun reconfigure() acquires Configuration &#123;<br/>    // Do not do anything if genesis has not finished.<br/>    if (chain_status::is_genesis() &#124;&#124; timestamp::now_microseconds() &#61;&#61; 0 &#124;&#124; !reconfiguration_enabled()) &#123;<br/>        return<br/>    &#125;;<br/><br/>    let config_ref &#61; borrow_global_mut&lt;Configuration&gt;(@aptos_framework);<br/>    let current_time &#61; timestamp::now_microseconds();<br/><br/>    // Do not do anything if a reconfiguration event is already emitted within this transaction.<br/>    //<br/>    // This is OK because:<br/>    // &#45; The time changes in every non&#45;empty block<br/>    // &#45; A block automatically ends after a transaction that emits a reconfiguration event, which is guaranteed by<br/>    //   VM spec that all transactions comming after a reconfiguration transaction will be returned as Retry<br/>    //   status.<br/>    // &#45; Each transaction must emit at most one reconfiguration event<br/>    //<br/>    // Thus, this check ensures that a transaction that does multiple &quot;reconfiguration required&quot; actions emits only<br/>    // one reconfiguration event.<br/>    //<br/>    if (current_time &#61;&#61; config_ref.last_reconfiguration_time) &#123;<br/>        return<br/>    &#125;;<br/><br/>    reconfiguration_state::on_reconfig_start();<br/><br/>    // Reconfiguration &quot;forces the block&quot; to end, as mentioned above. Therefore, we must process the collected fees<br/>    // explicitly so that staking can distribute them.<br/>    //<br/>    // This also handles the case when a validator is removed due to the governance proposal. In particular, removing<br/>    // the validator causes a reconfiguration. We explicitly process fees, i.e. we drain aggregatable coin and populate<br/>    // the fees table, prior to calling `on_new_epoch()`. That call, in turn, distributes transaction fees for all active<br/>    // and pending_inactive validators, which include any validator that is to be removed.<br/>    if (features::collect_and_distribute_gas_fees()) &#123;<br/>        // All transactions after reconfiguration are Retry. Therefore, when the next<br/>        // block starts and tries to assign/burn collected fees it will be just 0 and<br/>        // nothing will be assigned.<br/>        transaction_fee::process_collected_fees();<br/>    &#125;;<br/><br/>    // Call stake to compute the new validator set and distribute rewards and transaction fees.<br/>    stake::on_new_epoch();<br/>    storage_gas::on_reconfig();<br/><br/>    assert!(current_time &gt; config_ref.last_reconfiguration_time, error::invalid_state(EINVALID_BLOCK_TIME));<br/>    config_ref.last_reconfiguration_time &#61; current_time;<br/>    spec &#123;<br/>        assume config_ref.epoch &#43; 1 &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    config_ref.epoch &#61; config_ref.epoch &#43; 1;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            NewEpoch &#123;<br/>                epoch: config_ref.epoch,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;NewEpochEvent&gt;(<br/>        &amp;mut config_ref.events,<br/>        NewEpochEvent &#123;<br/>            epoch: config_ref.epoch,<br/>        &#125;,<br/>    );<br/><br/>    reconfiguration_state::on_reconfig_finish();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_last_reconfiguration_time"></a>

## Function `last_reconfiguration_time`



<pre><code>public fun last_reconfiguration_time(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun last_reconfiguration_time(): u64 acquires Configuration &#123;<br/>    borrow_global&lt;Configuration&gt;(@aptos_framework).last_reconfiguration_time<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_current_epoch"></a>

## Function `current_epoch`



<pre><code>public fun current_epoch(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun current_epoch(): u64 acquires Configuration &#123;<br/>    borrow_global&lt;Configuration&gt;(@aptos_framework).epoch<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_emit_genesis_reconfiguration_event"></a>

## Function `emit_genesis_reconfiguration_event`

Emit a <code>NewEpochEvent</code> event. This function will be invoked by genesis directly to generate the very first<br/> reconfiguration event.


<pre><code>fun emit_genesis_reconfiguration_event()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_genesis_reconfiguration_event() acquires Configuration &#123;<br/>    let config_ref &#61; borrow_global_mut&lt;Configuration&gt;(@aptos_framework);<br/>    assert!(config_ref.epoch &#61;&#61; 0 &amp;&amp; config_ref.last_reconfiguration_time &#61;&#61; 0, error::invalid_state(ECONFIGURATION));<br/>    config_ref.epoch &#61; 1;<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(<br/>            NewEpoch &#123;<br/>                epoch: config_ref.epoch,<br/>            &#125;,<br/>        );<br/>    &#125;;<br/>    event::emit_event&lt;NewEpochEvent&gt;(<br/>        &amp;mut config_ref.events,<br/>        NewEpochEvent &#123;<br/>            epoch: config_ref.epoch,<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The Configuration resource is stored under the Aptos framework account with initial values upon module&apos;s initialization.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The Configuration resource may only be initialized with specific values and published under the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;1&quot;&gt;initialize&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;The reconfiguration status may be determined at any time without causing an abort, indicating whether or not the system allows reconfiguration.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The reconfiguration_enabled function will never abort and always returns a boolean value that accurately represents whether the system allows reconfiguration.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;reconfiguration_enabled&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;For each reconfiguration, the epoch value (config_ref.epoch) increases by 1, and one &apos;NewEpochEvent&apos; is emitted.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;After reconfiguration, the reconfigure() function increases the epoch value of the configuration by one and increments the counter of the NewEpochEvent&apos;s EventHandle by one.&lt;/td&gt;<br/>&lt;td&gt;Audited that these two values remain in sync.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;Reconfiguration is possible only if genesis has started and reconfiguration is enabled. Also, the last reconfiguration must not be the current time, returning early without further actions otherwise.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The reconfigure() function may only execute to perform successful reconfiguration when genesis has started and when reconfiguration is enabled. Without satisfying both conditions, the function returns early without executing any further actions.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;reconfigure&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;Consecutive reconfigurations without the passage of time are not permitted.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The reconfigure() function enforces the restriction that reconfiguration may only be performed when the current time is not equal to the last_reconfiguration_time.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5&quot;&gt;reconfigure&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;Configuration&gt;(@aptos_framework);<br/>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt;<br/>    (timestamp::spec_now_microseconds() &gt;&#61; last_reconfiguration_time());<br/></code></pre>


Make sure the signer address is @aptos_framework.


<a id="0x1_reconfiguration_AbortsIfNotAptosFramework"></a>


<pre><code>schema AbortsIfNotAptosFramework &#123;<br/>aptos_framework: &amp;signer;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(addr);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>


Address @aptos_framework must exist resource Account and Configuration.<br/> Already exists in framework account.<br/> Guid_creation_num should be 2 according to logic.


<pre><code>include AbortsIfNotAptosFramework;<br/>let addr &#61; signer::address_of(aptos_framework);<br/>let post config &#61; global&lt;Configuration&gt;(@aptos_framework);<br/>requires exists&lt;Account&gt;(addr);<br/>aborts_if !(global&lt;Account&gt;(addr).guid_creation_num &#61;&#61; 2);<br/>aborts_if exists&lt;Configuration&gt;(@aptos_framework);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 1&lt;/a&gt;:
ensures exists&lt;Configuration&gt;(@aptos_framework);<br/>ensures config.epoch &#61;&#61; 0 &amp;&amp; config.last_reconfiguration_time &#61;&#61; 0;<br/>ensures config.events &#61;&#61; event::EventHandle&lt;NewEpochEvent&gt; &#123;<br/>    counter: 0,<br/>    guid: guid::GUID &#123;<br/>        id: guid::ID &#123;<br/>            creation_num: 2,<br/>            addr: @aptos_framework<br/>        &#125;<br/>    &#125;<br/>&#125;;<br/></code></pre>



<a id="@Specification_1_disable_reconfiguration"></a>

### Function `disable_reconfiguration`


<pre><code>fun disable_reconfiguration(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>include AbortsIfNotAptosFramework;<br/>aborts_if exists&lt;DisableReconfiguration&gt;(@aptos_framework);<br/>ensures exists&lt;DisableReconfiguration&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_enable_reconfiguration"></a>

### Function `enable_reconfiguration`


<pre><code>fun enable_reconfiguration(aptos_framework: &amp;signer)<br/></code></pre>


Make sure the caller is admin and check the resource DisableReconfiguration.


<pre><code>include AbortsIfNotAptosFramework;<br/>aborts_if !exists&lt;DisableReconfiguration&gt;(@aptos_framework);<br/>ensures !exists&lt;DisableReconfiguration&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_reconfiguration_enabled"></a>

### Function `reconfiguration_enabled`


<pre><code>fun reconfiguration_enabled(): bool<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
aborts_if false;<br/>ensures result &#61;&#61; !exists&lt;DisableReconfiguration&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_reconfigure"></a>

### Function `reconfigure`


<pre><code>public(friend) fun reconfigure()<br/></code></pre>




<pre><code>pragma verify &#61; true;<br/>pragma verify_duration_estimate &#61; 600;<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>let success &#61; !(chain_status::is_genesis() &#124;&#124; timestamp::spec_now_microseconds() &#61;&#61; 0 &#124;&#124; !reconfiguration_enabled())<br/>    &amp;&amp; timestamp::spec_now_microseconds() !&#61; global&lt;Configuration&gt;(@aptos_framework).last_reconfiguration_time;<br/>include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;<br/>include success &#61;&#61;&gt; aptos_coin::ExistsAptosCoin;<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>aborts_if false;<br/>ensures success &#61;&#61;&gt; global&lt;Configuration&gt;(@aptos_framework).epoch &#61;&#61; old(global&lt;Configuration&gt;(@aptos_framework).epoch) &#43; 1;<br/>ensures success &#61;&#61;&gt; global&lt;Configuration&gt;(@aptos_framework).last_reconfiguration_time &#61;&#61; timestamp::spec_now_microseconds();<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt; and &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
ensures !success &#61;&#61;&gt; global&lt;Configuration&gt;(@aptos_framework).epoch &#61;&#61; old(global&lt;Configuration&gt;(@aptos_framework).epoch);<br/></code></pre>



<a id="@Specification_1_last_reconfiguration_time"></a>

### Function `last_reconfiguration_time`


<pre><code>public fun last_reconfiguration_time(): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;Configuration&gt;(@aptos_framework);<br/>ensures result &#61;&#61; global&lt;Configuration&gt;(@aptos_framework).last_reconfiguration_time;<br/></code></pre>



<a id="@Specification_1_current_epoch"></a>

### Function `current_epoch`


<pre><code>public fun current_epoch(): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;Configuration&gt;(@aptos_framework);<br/>ensures result &#61;&#61; global&lt;Configuration&gt;(@aptos_framework).epoch;<br/></code></pre>



<a id="@Specification_1_emit_genesis_reconfiguration_event"></a>

### Function `emit_genesis_reconfiguration_event`


<pre><code>fun emit_genesis_reconfiguration_event()<br/></code></pre>


When genesis_event emit the epoch and the <code>last_reconfiguration_time</code> .<br/> Should equal to 0


<pre><code>aborts_if !exists&lt;Configuration&gt;(@aptos_framework);<br/>let config_ref &#61; global&lt;Configuration&gt;(@aptos_framework);<br/>aborts_if !(config_ref.epoch &#61;&#61; 0 &amp;&amp; config_ref.last_reconfiguration_time &#61;&#61; 0);<br/>ensures global&lt;Configuration&gt;(@aptos_framework).epoch &#61;&#61; 1;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
