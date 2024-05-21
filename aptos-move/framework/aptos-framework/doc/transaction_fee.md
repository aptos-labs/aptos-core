
<a id="0x1_transaction_fee"></a>

# Module `0x1::transaction_fee`

This module provides an interface to burn or collect and redistribute transaction fees.


-  [Resource `AptosCoinCapabilities`](#0x1_transaction_fee_AptosCoinCapabilities)
-  [Resource `AptosCoinMintCapability`](#0x1_transaction_fee_AptosCoinMintCapability)
-  [Resource `CollectedFeesPerBlock`](#0x1_transaction_fee_CollectedFeesPerBlock)
-  [Struct `FeeStatement`](#0x1_transaction_fee_FeeStatement)
-  [Constants](#@Constants_0)
-  [Function `initialize_fee_collection_and_distribution`](#0x1_transaction_fee_initialize_fee_collection_and_distribution)
-  [Function `is_fees_collection_enabled`](#0x1_transaction_fee_is_fees_collection_enabled)
-  [Function `upgrade_burn_percentage`](#0x1_transaction_fee_upgrade_burn_percentage)
-  [Function `register_proposer_for_fee_collection`](#0x1_transaction_fee_register_proposer_for_fee_collection)
-  [Function `burn_coin_fraction`](#0x1_transaction_fee_burn_coin_fraction)
-  [Function `process_collected_fees`](#0x1_transaction_fee_process_collected_fees)
-  [Function `burn_fee`](#0x1_transaction_fee_burn_fee)
-  [Function `mint_and_refund`](#0x1_transaction_fee_mint_and_refund)
-  [Function `collect_fee`](#0x1_transaction_fee_collect_fee)
-  [Function `store_aptos_coin_burn_cap`](#0x1_transaction_fee_store_aptos_coin_burn_cap)
-  [Function `store_aptos_coin_mint_cap`](#0x1_transaction_fee_store_aptos_coin_mint_cap)
-  [Function `initialize_storage_refund`](#0x1_transaction_fee_initialize_storage_refund)
-  [Function `emit_fee_statement`](#0x1_transaction_fee_emit_fee_statement)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Resource `CollectedFeesPerBlock`](#@Specification_1_CollectedFeesPerBlock)
    -  [Function `initialize_fee_collection_and_distribution`](#@Specification_1_initialize_fee_collection_and_distribution)
    -  [Function `upgrade_burn_percentage`](#@Specification_1_upgrade_burn_percentage)
    -  [Function `register_proposer_for_fee_collection`](#@Specification_1_register_proposer_for_fee_collection)
    -  [Function `burn_coin_fraction`](#@Specification_1_burn_coin_fraction)
    -  [Function `process_collected_fees`](#@Specification_1_process_collected_fees)
    -  [Function `burn_fee`](#@Specification_1_burn_fee)
    -  [Function `mint_and_refund`](#@Specification_1_mint_and_refund)
    -  [Function `collect_fee`](#@Specification_1_collect_fee)
    -  [Function `store_aptos_coin_burn_cap`](#@Specification_1_store_aptos_coin_burn_cap)
    -  [Function `store_aptos_coin_mint_cap`](#@Specification_1_store_aptos_coin_mint_cap)
    -  [Function `initialize_storage_refund`](#@Specification_1_initialize_storage_refund)
    -  [Function `emit_fee_statement`](#@Specification_1_emit_fee_statement)


<pre><code>use 0x1::aptos_coin;<br/>use 0x1::coin;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::option;<br/>use 0x1::stake;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_transaction_fee_AptosCoinCapabilities"></a>

## Resource `AptosCoinCapabilities`

Stores burn capability to burn the gas fees.


<pre><code>struct AptosCoinCapabilities has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>burn_cap: coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_AptosCoinMintCapability"></a>

## Resource `AptosCoinMintCapability`

Stores mint capability to mint the refunds.


<pre><code>struct AptosCoinMintCapability has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_CollectedFeesPerBlock"></a>

## Resource `CollectedFeesPerBlock`

Stores information about the block proposer and the amount of fees
collected when executing the block.


<pre><code>struct CollectedFeesPerBlock has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>amount: coin::AggregatableCoin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: option::Option&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_percentage: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_transaction_fee_FeeStatement"></a>

## Struct `FeeStatement`

Breakdown of fee charge and refund for a transaction.
The structure is:

- Net charge or refund (not in the statement)
- total charge: total_charge_gas_units, matches <code>gas_used</code> in the on-chain <code>TransactionInfo</code>.
This is the sum of the sub-items below. Notice that there's potential precision loss when
the conversion between internal and external gas units and between native token and gas
units, so it's possible that the numbers don't add up exactly. -- This number is the final
charge, while the break down is merely informational.
- gas charge for execution (CPU time): <code>execution_gas_units</code>
- gas charge for IO (storage random access): <code>io_gas_units</code>
- storage fee charge (storage space): <code>storage_fee_octas</code>, to be included in
<code>total_charge_gas_unit</code>, this number is converted to gas units according to the user
specified <code>gas_unit_price</code> on the transaction.
- storage deletion refund: <code>storage_fee_refund_octas</code>, this is not included in <code>gas_used</code> or
<code>total_charge_gas_units</code>, the net charge / refund is calculated by
<code>total_charge_gas_units</code> * <code>gas_unit_price</code> - <code>storage_fee_refund_octas</code>.

This is meant to emitted as a module event.


<pre><code>&#35;[event]<br/>struct FeeStatement has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>total_charge_gas_units: u64</code>
</dt>
<dd>
 Total gas charge.
</dd>
<dt>
<code>execution_gas_units: u64</code>
</dt>
<dd>
 Execution gas charge.
</dd>
<dt>
<code>io_gas_units: u64</code>
</dt>
<dd>
 IO gas charge.
</dd>
<dt>
<code>storage_fee_octas: u64</code>
</dt>
<dd>
 Storage fee charge.
</dd>
<dt>
<code>storage_fee_refund_octas: u64</code>
</dt>
<dd>
 Storage fee refund.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_transaction_fee_EALREADY_COLLECTING_FEES"></a>

Gas fees are already being collected and the struct holding
information about collected amounts is already published.


<pre><code>const EALREADY_COLLECTING_FEES: u64 &#61; 1;<br/></code></pre>



<a id="0x1_transaction_fee_EINVALID_BURN_PERCENTAGE"></a>

The burn percentage is out of range [0, 100].


<pre><code>const EINVALID_BURN_PERCENTAGE: u64 &#61; 3;<br/></code></pre>



<a id="0x1_transaction_fee_ENO_LONGER_SUPPORTED"></a>

No longer supported.


<pre><code>const ENO_LONGER_SUPPORTED: u64 &#61; 4;<br/></code></pre>



<a id="0x1_transaction_fee_initialize_fee_collection_and_distribution"></a>

## Function `initialize_fee_collection_and_distribution`

Initializes the resource storing information about gas fees collection and
distribution. Should be called by on-chain governance.


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(<br/>        !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework),<br/>        error::already_exists(EALREADY_COLLECTING_FEES)<br/>    );<br/>    assert!(burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));<br/><br/>    // Make sure stakng module is aware of transaction fees collection.<br/>    stake::initialize_validator_fees(aptos_framework);<br/><br/>    // Initially, no fees are collected and the block proposer is not set.<br/>    let collected_fees &#61; CollectedFeesPerBlock &#123;<br/>        amount: coin::initialize_aggregatable_coin(aptos_framework),<br/>        proposer: option::none(),<br/>        burn_percentage,<br/>    &#125;;<br/>    move_to(aptos_framework, collected_fees);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_is_fees_collection_enabled"></a>

## Function `is_fees_collection_enabled`



<pre><code>fun is_fees_collection_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_fees_collection_enabled(): bool &#123;<br/>    exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_upgrade_burn_percentage"></a>

## Function `upgrade_burn_percentage`

Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.


<pre><code>public fun upgrade_burn_percentage(aptos_framework: &amp;signer, new_burn_percentage: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_burn_percentage(<br/>    aptos_framework: &amp;signer,<br/>    new_burn_percentage: u8<br/>) acquires AptosCoinCapabilities, CollectedFeesPerBlock &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(new_burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));<br/><br/>    // Prior to upgrading the burn percentage, make sure to process collected<br/>    // fees. Otherwise we would use the new (incorrect) burn_percentage when<br/>    // processing fees later!<br/>    process_collected_fees();<br/><br/>    if (is_fees_collection_enabled()) &#123;<br/>        // Upgrade has no effect unless fees are being collected.<br/>        let burn_percentage &#61; &amp;mut borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework).burn_percentage;<br/>        &#42;burn_percentage &#61; new_burn_percentage<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_register_proposer_for_fee_collection"></a>

## Function `register_proposer_for_fee_collection`

Registers the proposer of the block for gas fees collection. This function
can only be called at the beginning of the block.


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address) acquires CollectedFeesPerBlock &#123;<br/>    if (is_fees_collection_enabled()) &#123;<br/>        let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>        let _ &#61; option::swap_or_fill(&amp;mut collected_fees.proposer, proposer_addr);<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_burn_coin_fraction"></a>

## Function `burn_coin_fraction`

Burns a specified fraction of the coin.


<pre><code>fun burn_coin_fraction(coin: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, burn_percentage: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun burn_coin_fraction(coin: &amp;mut Coin&lt;AptosCoin&gt;, burn_percentage: u8) acquires AptosCoinCapabilities &#123;<br/>    assert!(burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));<br/><br/>    let collected_amount &#61; coin::value(coin);<br/>    spec &#123;<br/>        // We assume that `burn_percentage &#42; collected_amount` does not overflow.<br/>        assume burn_percentage &#42; collected_amount &lt;&#61; MAX_U64;<br/>    &#125;;<br/>    let amount_to_burn &#61; (burn_percentage as u64) &#42; collected_amount / 100;<br/>    if (amount_to_burn &gt; 0) &#123;<br/>        let coin_to_burn &#61; coin::extract(coin, amount_to_burn);<br/>        coin::burn(<br/>            coin_to_burn,<br/>            &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).burn_cap,<br/>        );<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_process_collected_fees"></a>

## Function `process_collected_fees`

Calculates the fee which should be distributed to the block proposer at the
end of an epoch, and records it in the system. This function can only be called
at the beginning of the block or during reconfiguration.


<pre><code>public(friend) fun process_collected_fees()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun process_collected_fees() acquires AptosCoinCapabilities, CollectedFeesPerBlock &#123;<br/>    if (!is_fees_collection_enabled()) &#123;<br/>        return<br/>    &#125;;<br/>    let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/><br/>    // If there are no collected fees, only unset the proposer. See the rationale for<br/>    // setting proposer to option::none() below.<br/>    if (coin::is_aggregatable_coin_zero(&amp;collected_fees.amount)) &#123;<br/>        if (option::is_some(&amp;collected_fees.proposer)) &#123;<br/>            let _ &#61; option::extract(&amp;mut collected_fees.proposer);<br/>        &#125;;<br/>        return<br/>    &#125;;<br/><br/>    // Otherwise get the collected fee, and check if it can distributed later.<br/>    let coin &#61; coin::drain_aggregatable_coin(&amp;mut collected_fees.amount);<br/>    if (option::is_some(&amp;collected_fees.proposer)) &#123;<br/>        // Extract the address of proposer here and reset it to option::none(). This<br/>        // is particularly useful to avoid any undesired side&#45;effects where coins are<br/>        // collected but never distributed or distributed to the wrong account.<br/>        // With this design, processing collected fees enforces that all fees will be burnt<br/>        // unless the proposer is specified in the block prologue. When we have a governance<br/>        // proposal that triggers reconfiguration, we distribute pending fees and burn the<br/>        // fee for the proposal. Otherwise, that fee would be leaked to the next block.<br/>        let proposer &#61; option::extract(&amp;mut collected_fees.proposer);<br/><br/>        // Since the block can be produced by the VM itself, we have to make sure we catch<br/>        // this case.<br/>        if (proposer &#61;&#61; @vm_reserved) &#123;<br/>            burn_coin_fraction(&amp;mut coin, 100);<br/>            coin::destroy_zero(coin);<br/>            return<br/>        &#125;;<br/><br/>        burn_coin_fraction(&amp;mut coin, collected_fees.burn_percentage);<br/>        stake::add_transaction_fee(proposer, coin);<br/>        return<br/>    &#125;;<br/><br/>    // If checks did not pass, simply burn all collected coins and return none.<br/>    burn_coin_fraction(&amp;mut coin, 100);<br/>    coin::destroy_zero(coin)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_burn_fee"></a>

## Function `burn_fee`

Burn transaction fees in epilogue.


<pre><code>public(friend) fun burn_fee(account: address, fee: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun burn_fee(account: address, fee: u64) acquires AptosCoinCapabilities &#123;<br/>    coin::burn_from&lt;AptosCoin&gt;(<br/>        account,<br/>        fee,<br/>        &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).burn_cap,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_mint_and_refund"></a>

## Function `mint_and_refund`

Mint refund in epilogue.


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64) acquires AptosCoinMintCapability &#123;<br/>    let mint_cap &#61; &amp;borrow_global&lt;AptosCoinMintCapability&gt;(@aptos_framework).mint_cap;<br/>    let refund_coin &#61; coin::mint(refund, mint_cap);<br/>    coin::force_deposit(account, refund_coin);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_collect_fee"></a>

## Function `collect_fee`

Collect transaction fees in epilogue.


<pre><code>public(friend) fun collect_fee(account: address, fee: u64)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun collect_fee(account: address, fee: u64) acquires CollectedFeesPerBlock &#123;<br/>    let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/><br/>    // Here, we are always optimistic and always collect fees. If the proposer is not set,<br/>    // or we cannot redistribute fees later for some reason (e.g. account cannot receive AptoCoin)<br/>    // we burn them all at once. This way we avoid having a check for every transaction epilogue.<br/>    let collected_amount &#61; &amp;mut collected_fees.amount;<br/>    coin::collect_into_aggregatable_coin&lt;AptosCoin&gt;(account, fee, collected_amount);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Only called during genesis.


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: BurnCapability&lt;AptosCoin&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, AptosCoinCapabilities &#123; burn_cap &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Only called during genesis.


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: MintCapability&lt;AptosCoin&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, AptosCoinMintCapability &#123; mint_cap &#125;)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_initialize_storage_refund"></a>

## Function `initialize_storage_refund`



<pre><code>&#35;[deprecated]<br/>public fun initialize_storage_refund(_: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_storage_refund(_: &amp;signer) &#123;<br/>    abort error::not_implemented(ENO_LONGER_SUPPORTED)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_transaction_fee_emit_fee_statement"></a>

## Function `emit_fee_statement`



<pre><code>fun emit_fee_statement(fee_statement: transaction_fee::FeeStatement)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_fee_statement(fee_statement: FeeStatement) &#123;<br/>    event::emit(fee_statement)<br/>&#125;<br/></code></pre>



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
<td>Given the blockchain is in an operating state, it guarantees that the Aptos framework signer may burn Aptos coins.</td>
<td>Critical</td>
<td>The AptosCoinCapabilities structure is defined in this module and it stores burn capability to burn the gas fees.</td>
<td>Formally Verified via <a href="#high-level-req-1">module</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The initialization function may only be called once.</td>
<td>Medium</td>
<td>The initialize_fee_collection_and_distribution function ensures CollectedFeesPerBlock does not already exist.</td>
<td>Formally verified via <a href="#high-level-req-2">initialize_fee_collection_and_distribution</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only the admin address is authorized to call the initialization function.</td>
<td>Critical</td>
<td>The initialize_fee_collection_and_distribution function ensures only the Aptos framework address calls it.</td>
<td>Formally verified via <a href="#high-level-req-3">initialize_fee_collection_and_distribution</a>.</td>
</tr>

<tr>
<td>4</td>
<td>The percentage of the burnt collected fee is always a value from 0 to 100.</td>
<td>Medium</td>
<td>During the initialization of CollectedFeesPerBlock in Initialize_fee_collection_and_distribution, and while upgrading burn percentage, it asserts that burn_percentage is within the specified limits.</td>
<td>Formally verified via <a href="#high-level-req-4">CollectedFeesPerBlock</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Prior to upgrading the burn percentage, it must process all the fees collected up to that point.</td>
<td>Critical</td>
<td>The upgrade_burn_percentage function ensures process_collected_fees function is called before updating the burn percentage.</td>
<td>Formally verified in <a href="#high-level-req-5">ProcessCollectedFeesRequiresAndEnsures</a>.</td>
</tr>

<tr>
<td>6</td>
<td>The presence of the resource, indicating collected fees per block under the Aptos framework account, is a prerequisite for the successful execution of the following functionalities: Upgrading burn percentage. Registering a block proposer. Processing collected fees.</td>
<td>Low</td>
<td>The functions: upgrade_burn_percentage, register_proposer_for_fee_collection, and process_collected_fees all ensure that the CollectedFeesPerBlock resource exists under aptos_framework by calling the is_fees_collection_enabled method, which returns a boolean value confirming if the resource exists or not.</td>
<td>Formally verified via <a href="#high-level-req-6.1">register_proposer_for_fee_collection</a>, <a href="#high-level-req-6.2">process_collected_fees</a>, and <a href="#high-level-req-6.3">upgrade_burn_percentage</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_CollectedFeesPerBlock"></a>

### Resource `CollectedFeesPerBlock`


<pre><code>struct CollectedFeesPerBlock has key<br/></code></pre>



<dl>
<dt>
<code>amount: coin::AggregatableCoin&lt;aptos_coin::AptosCoin&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>proposer: option::Option&lt;address&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>burn_percentage: u8</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
invariant burn_percentage &lt;&#61; 100;<br/></code></pre>



<a id="@Specification_1_initialize_fee_collection_and_distribution"></a>

### Function `initialize_fee_collection_and_distribution`


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8)<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>aborts_if burn_percentage &gt; 100;<br/>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if exists&lt;ValidatorFees&gt;(aptos_addr);<br/>include system_addresses::AbortsIfNotAptosFramework &#123; account: aptos_framework &#125;;<br/>include aggregator_factory::CreateAggregatorInternalAbortsIf;<br/>aborts_if exists&lt;CollectedFeesPerBlock&gt;(aptos_addr);<br/>ensures exists&lt;ValidatorFees&gt;(aptos_addr);<br/>ensures exists&lt;CollectedFeesPerBlock&gt;(aptos_addr);<br/></code></pre>



<a id="@Specification_1_upgrade_burn_percentage"></a>

### Function `upgrade_burn_percentage`


<pre><code>public fun upgrade_burn_percentage(aptos_framework: &amp;signer, new_burn_percentage: u8)<br/></code></pre>




<pre><code>aborts_if new_burn_percentage &gt; 100;<br/>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a> and <a id="high-level-req-6.3" href="#high-level-req">high-level requirement 6</a>:
include ProcessCollectedFeesRequiresAndEnsures;<br/>ensures exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework) &#61;&#61;&gt;<br/>    global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).burn_percentage &#61;&#61; new_burn_percentage;<br/></code></pre>



<a id="@Specification_1_register_proposer_for_fee_collection"></a>

### Function `register_proposer_for_fee_collection`


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address)<br/></code></pre>




<pre><code>aborts_if false;<br/>// This enforces <a id="high-level-req-6.1" href="#high-level-req">high-level requirement 6</a>:
ensures is_fees_collection_enabled() &#61;&#61;&gt;<br/>    option::spec_borrow(global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).proposer) &#61;&#61; proposer_addr;<br/></code></pre>



<a id="@Specification_1_burn_coin_fraction"></a>

### Function `burn_coin_fraction`


<pre><code>fun burn_coin_fraction(coin: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, burn_percentage: u8)<br/></code></pre>




<pre><code>requires burn_percentage &lt;&#61; 100;<br/>requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>let amount_to_burn &#61; (burn_percentage &#42; coin::value(coin)) / 100;<br/>include amount_to_burn &gt; 0 &#61;&#61;&gt; coin::CoinSubAbortsIf&lt;AptosCoin&gt; &#123; amount: amount_to_burn &#125;;<br/>ensures coin.value &#61;&#61; old(coin).value &#45; amount_to_burn;<br/></code></pre>




<a id="0x1_transaction_fee_collectedFeesAggregator"></a>


<pre><code>fun collectedFeesAggregator(): AggregatableCoin&lt;AptosCoin&gt; &#123;<br/>   global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount<br/>&#125;<br/></code></pre>




<a id="0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply"></a>


<pre><code>schema RequiresCollectedFeesPerValueLeqBlockAptosSupply &#123;<br/>let maybe_supply &#61; coin::get_coin_supply_opt&lt;AptosCoin&gt;();<br/>requires
        (is_fees_collection_enabled() &amp;&amp; option::is_some(maybe_supply)) &#61;&#61;&gt;<br/>        (aggregator::spec_aggregator_get_val(global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount.value) &lt;&#61;<br/>            optional_aggregator::optional_aggregator_value(<br/>                option::spec_borrow(coin::get_coin_supply_opt&lt;AptosCoin&gt;())<br/>            ));<br/>&#125;<br/></code></pre>




<a id="0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures"></a>


<pre><code>schema ProcessCollectedFeesRequiresAndEnsures &#123;<br/>requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>include RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>aborts_if false;<br/>let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>let post post_collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>let pre_amount &#61; aggregator::spec_aggregator_get_val(collected_fees.amount.value);<br/>let post post_amount &#61; aggregator::spec_aggregator_get_val(post_collected_fees.amount.value);<br/>let fees_table &#61; global&lt;stake::ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let post post_fees_table &#61; global&lt;stake::ValidatorFees&gt;(@aptos_framework).fees_table;<br/>let proposer &#61; option::spec_borrow(collected_fees.proposer);<br/>let fee_to_add &#61; pre_amount &#45; pre_amount &#42; collected_fees.burn_percentage / 100;<br/>ensures is_fees_collection_enabled() &#61;&#61;&gt; option::spec_is_none(post_collected_fees.proposer) &amp;&amp; post_amount &#61;&#61; 0;<br/>ensures is_fees_collection_enabled() &amp;&amp; aggregator::spec_read(collected_fees.amount.value) &gt; 0 &amp;&amp;<br/>    option::spec_is_some(collected_fees.proposer) &#61;&#61;&gt;<br/>    if (proposer !&#61; @vm_reserved) &#123;<br/>        if (table::spec_contains(fees_table, proposer)) &#123;<br/>            table::spec_get(post_fees_table, proposer).value &#61;&#61; table::spec_get(<br/>                fees_table,<br/>                proposer<br/>            ).value &#43; fee_to_add<br/>        &#125; else &#123;<br/>            table::spec_get(post_fees_table, proposer).value &#61;&#61; fee_to_add<br/>        &#125;<br/>    &#125; else &#123;<br/>        option::spec_is_none(post_collected_fees.proposer) &amp;&amp; post_amount &#61;&#61; 0<br/>    &#125;;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_process_collected_fees"></a>

### Function `process_collected_fees`


<pre><code>public(friend) fun process_collected_fees()<br/></code></pre>




<pre><code>// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
include ProcessCollectedFeesRequiresAndEnsures;<br/></code></pre>



<a id="@Specification_1_burn_fee"></a>

### Function `burn_fee`


<pre><code>public(friend) fun burn_fee(account: address, fee: u64)<br/></code></pre>


<code>AptosCoinCapabilities</code> should be exists.


<pre><code>pragma verify &#61; false;<br/>aborts_if !exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);<br/>let account_addr &#61; account;<br/>let amount &#61; fee;<br/>let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>let coin_store &#61; global&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr);<br/>let post post_coin_store &#61; global&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr);<br/>aborts_if amount !&#61; 0 &amp;&amp; !(exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr)<br/>    &amp;&amp; exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr));<br/>aborts_if coin_store.coin.value &lt; amount;<br/>let maybe_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr).supply;<br/>let supply_aggr &#61; option::spec_borrow(maybe_supply);<br/>let value &#61; optional_aggregator::optional_aggregator_value(supply_aggr);<br/>let post post_maybe_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr).supply;<br/>let post post_supply &#61; option::spec_borrow(post_maybe_supply);<br/>let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);<br/>aborts_if option::spec_is_some(maybe_supply) &amp;&amp; value &lt; amount;<br/>ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; amount;<br/>ensures if (option::spec_is_some(maybe_supply)) &#123;<br/>    post_value &#61;&#61; value &#45; amount<br/>&#125; else &#123;<br/>    option::spec_is_none(post_maybe_supply)<br/>&#125;;<br/>ensures coin::supply&lt;AptosCoin&gt; &#61;&#61; old(coin::supply&lt;AptosCoin&gt;) &#45; amount;<br/></code></pre>



<a id="@Specification_1_mint_and_refund"></a>

### Function `mint_and_refund`


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;<br/>aborts_if (refund !&#61; 0) &amp;&amp; !exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr);<br/>include coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: refund &#125;;<br/>aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(account);<br/>aborts_if !exists&lt;AptosCoinMintCapability&gt;(@aptos_framework);<br/>let supply &#61; coin::supply&lt;AptosCoin&gt;;<br/>let post post_supply &#61; coin::supply&lt;AptosCoin&gt;;<br/>aborts_if [abstract] supply &#43; refund &gt; MAX_U128;<br/>ensures post_supply &#61;&#61; supply &#43; refund;<br/></code></pre>



<a id="@Specification_1_collect_fee"></a>

### Function `collect_fee`


<pre><code>public(friend) fun collect_fee(account: address, fee: u64)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/>let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;<br/>let aggr &#61; collected_fees.value;<br/>let coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);<br/>aborts_if !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);<br/>aborts_if fee &gt; 0 &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);<br/>aborts_if fee &gt; 0 &amp;&amp; coin_store.coin.value &lt; fee;<br/>aborts_if fee &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; fee &gt; aggregator::spec_get_limit(aggr);<br/>aborts_if fee &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)<br/>    &#43; fee &gt; MAX_U128;<br/>let post post_coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);<br/>let post post_collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;<br/>ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; fee;<br/>ensures aggregator::spec_aggregator_get_val(post_collected_fees.value) &#61;&#61; aggregator::spec_aggregator_get_val(<br/>    aggr<br/>) &#43; fee;<br/></code></pre>



<a id="@Specification_1_store_aptos_coin_burn_cap"></a>

### Function `store_aptos_coin_burn_cap`


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>


Ensure caller is admin.
Aborts if <code>AptosCoinCapabilities</code> already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(addr);<br/>aborts_if exists&lt;AptosCoinCapabilities&gt;(addr);<br/>ensures exists&lt;AptosCoinCapabilities&gt;(addr);<br/></code></pre>



<a id="@Specification_1_store_aptos_coin_mint_cap"></a>

### Function `store_aptos_coin_mint_cap`


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)<br/></code></pre>


Ensure caller is admin.
Aborts if <code>AptosCoinMintCapability</code> already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(addr);<br/>aborts_if exists&lt;AptosCoinMintCapability&gt;(addr);<br/>ensures exists&lt;AptosCoinMintCapability&gt;(addr);<br/></code></pre>



<a id="@Specification_1_initialize_storage_refund"></a>

### Function `initialize_storage_refund`


<pre><code>&#35;[deprecated]<br/>public fun initialize_storage_refund(_: &amp;signer)<br/></code></pre>


Historical. Aborts.


<pre><code>aborts_if true;<br/></code></pre>



<a id="@Specification_1_emit_fee_statement"></a>

### Function `emit_fee_statement`


<pre><code>fun emit_fee_statement(fee_statement: transaction_fee::FeeStatement)<br/></code></pre>


Aborts if module event feature is not enabled.


[move-book]: https://aptos.dev/move/book/SUMMARY
