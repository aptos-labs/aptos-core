
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


<pre><code>use 0x1::aptos_coin;
use 0x1::coin;
use 0x1::error;
use 0x1::event;
use 0x1::option;
use 0x1::stake;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_transaction_fee_AptosCoinCapabilities"></a>

## Resource `AptosCoinCapabilities`

Stores burn capability to burn the gas fees.


<pre><code>struct AptosCoinCapabilities has key
</code></pre>



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


<pre><code>struct AptosCoinMintCapability has key
</code></pre>



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


<pre><code>struct CollectedFeesPerBlock has key
</code></pre>



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


<pre><code>&#35;[event]
struct FeeStatement has drop, store
</code></pre>



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


<pre><code>const EALREADY_COLLECTING_FEES: u64 &#61; 1;
</code></pre>



<a id="0x1_transaction_fee_EINVALID_BURN_PERCENTAGE"></a>

The burn percentage is out of range [0, 100].


<pre><code>const EINVALID_BURN_PERCENTAGE: u64 &#61; 3;
</code></pre>



<a id="0x1_transaction_fee_ENO_LONGER_SUPPORTED"></a>

No longer supported.


<pre><code>const ENO_LONGER_SUPPORTED: u64 &#61; 4;
</code></pre>



<a id="0x1_transaction_fee_initialize_fee_collection_and_distribution"></a>

## Function `initialize_fee_collection_and_distribution`

Initializes the resource storing information about gas fees collection and
distribution. Should be called by on-chain governance.


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(
        !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework),
        error::already_exists(EALREADY_COLLECTING_FEES)
    );
    assert!(burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

    // Make sure stakng module is aware of transaction fees collection.
    stake::initialize_validator_fees(aptos_framework);

    // Initially, no fees are collected and the block proposer is not set.
    let collected_fees &#61; CollectedFeesPerBlock &#123;
        amount: coin::initialize_aggregatable_coin(aptos_framework),
        proposer: option::none(),
        burn_percentage,
    &#125;;
    move_to(aptos_framework, collected_fees);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_is_fees_collection_enabled"></a>

## Function `is_fees_collection_enabled`



<pre><code>fun is_fees_collection_enabled(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_fees_collection_enabled(): bool &#123;
    exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_upgrade_burn_percentage"></a>

## Function `upgrade_burn_percentage`

Sets the burn percentage for collected fees to a new value. Should be called by on-chain governance.


<pre><code>public fun upgrade_burn_percentage(aptos_framework: &amp;signer, new_burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_burn_percentage(
    aptos_framework: &amp;signer,
    new_burn_percentage: u8
) acquires AptosCoinCapabilities, CollectedFeesPerBlock &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(new_burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

    // Prior to upgrading the burn percentage, make sure to process collected
    // fees. Otherwise we would use the new (incorrect) burn_percentage when
    // processing fees later!
    process_collected_fees();

    if (is_fees_collection_enabled()) &#123;
        // Upgrade has no effect unless fees are being collected.
        let burn_percentage &#61; &amp;mut borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework).burn_percentage;
        &#42;burn_percentage &#61; new_burn_percentage
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_register_proposer_for_fee_collection"></a>

## Function `register_proposer_for_fee_collection`

Registers the proposer of the block for gas fees collection. This function
can only be called at the beginning of the block.


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address) acquires CollectedFeesPerBlock &#123;
    if (is_fees_collection_enabled()) &#123;
        let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
        let _ &#61; option::swap_or_fill(&amp;mut collected_fees.proposer, proposer_addr);
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_burn_coin_fraction"></a>

## Function `burn_coin_fraction`

Burns a specified fraction of the coin.


<pre><code>fun burn_coin_fraction(coin: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, burn_percentage: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun burn_coin_fraction(coin: &amp;mut Coin&lt;AptosCoin&gt;, burn_percentage: u8) acquires AptosCoinCapabilities &#123;
    assert!(burn_percentage &lt;&#61; 100, error::out_of_range(EINVALID_BURN_PERCENTAGE));

    let collected_amount &#61; coin::value(coin);
    spec &#123;
        // We assume that `burn_percentage &#42; collected_amount` does not overflow.
        assume burn_percentage &#42; collected_amount &lt;&#61; MAX_U64;
    &#125;;
    let amount_to_burn &#61; (burn_percentage as u64) &#42; collected_amount / 100;
    if (amount_to_burn &gt; 0) &#123;
        let coin_to_burn &#61; coin::extract(coin, amount_to_burn);
        coin::burn(
            coin_to_burn,
            &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).burn_cap,
        );
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_process_collected_fees"></a>

## Function `process_collected_fees`

Calculates the fee which should be distributed to the block proposer at the
end of an epoch, and records it in the system. This function can only be called
at the beginning of the block or during reconfiguration.


<pre><code>public(friend) fun process_collected_fees()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun process_collected_fees() acquires AptosCoinCapabilities, CollectedFeesPerBlock &#123;
    if (!is_fees_collection_enabled()) &#123;
        return
    &#125;;
    let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);

    // If there are no collected fees, only unset the proposer. See the rationale for
    // setting proposer to option::none() below.
    if (coin::is_aggregatable_coin_zero(&amp;collected_fees.amount)) &#123;
        if (option::is_some(&amp;collected_fees.proposer)) &#123;
            let _ &#61; option::extract(&amp;mut collected_fees.proposer);
        &#125;;
        return
    &#125;;

    // Otherwise get the collected fee, and check if it can distributed later.
    let coin &#61; coin::drain_aggregatable_coin(&amp;mut collected_fees.amount);
    if (option::is_some(&amp;collected_fees.proposer)) &#123;
        // Extract the address of proposer here and reset it to option::none(). This
        // is particularly useful to avoid any undesired side&#45;effects where coins are
        // collected but never distributed or distributed to the wrong account.
        // With this design, processing collected fees enforces that all fees will be burnt
        // unless the proposer is specified in the block prologue. When we have a governance
        // proposal that triggers reconfiguration, we distribute pending fees and burn the
        // fee for the proposal. Otherwise, that fee would be leaked to the next block.
        let proposer &#61; option::extract(&amp;mut collected_fees.proposer);

        // Since the block can be produced by the VM itself, we have to make sure we catch
        // this case.
        if (proposer &#61;&#61; @vm_reserved) &#123;
            burn_coin_fraction(&amp;mut coin, 100);
            coin::destroy_zero(coin);
            return
        &#125;;

        burn_coin_fraction(&amp;mut coin, collected_fees.burn_percentage);
        stake::add_transaction_fee(proposer, coin);
        return
    &#125;;

    // If checks did not pass, simply burn all collected coins and return none.
    burn_coin_fraction(&amp;mut coin, 100);
    coin::destroy_zero(coin)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_burn_fee"></a>

## Function `burn_fee`

Burn transaction fees in epilogue.


<pre><code>public(friend) fun burn_fee(account: address, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun burn_fee(account: address, fee: u64) acquires AptosCoinCapabilities &#123;
    coin::burn_from&lt;AptosCoin&gt;(
        account,
        fee,
        &amp;borrow_global&lt;AptosCoinCapabilities&gt;(@aptos_framework).burn_cap,
    );
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_mint_and_refund"></a>

## Function `mint_and_refund`

Mint refund in epilogue.


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64) acquires AptosCoinMintCapability &#123;
    let mint_cap &#61; &amp;borrow_global&lt;AptosCoinMintCapability&gt;(@aptos_framework).mint_cap;
    let refund_coin &#61; coin::mint(refund, mint_cap);
    coin::force_deposit(account, refund_coin);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_collect_fee"></a>

## Function `collect_fee`

Collect transaction fees in epilogue.


<pre><code>public(friend) fun collect_fee(account: address, fee: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun collect_fee(account: address, fee: u64) acquires CollectedFeesPerBlock &#123;
    let collected_fees &#61; borrow_global_mut&lt;CollectedFeesPerBlock&gt;(@aptos_framework);

    // Here, we are always optimistic and always collect fees. If the proposer is not set,
    // or we cannot redistribute fees later for some reason (e.g. account cannot receive AptoCoin)
    // we burn them all at once. This way we avoid having a check for every transaction epilogue.
    let collected_amount &#61; &amp;mut collected_fees.amount;
    coin::collect_into_aggregatable_coin&lt;AptosCoin&gt;(account, fee, collected_amount);
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_store_aptos_coin_burn_cap"></a>

## Function `store_aptos_coin_burn_cap`

Only called during genesis.


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: BurnCapability&lt;AptosCoin&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    move_to(aptos_framework, AptosCoinCapabilities &#123; burn_cap &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_store_aptos_coin_mint_cap"></a>

## Function `store_aptos_coin_mint_cap`

Only called during genesis.


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: MintCapability&lt;AptosCoin&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    move_to(aptos_framework, AptosCoinMintCapability &#123; mint_cap &#125;)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_initialize_storage_refund"></a>

## Function `initialize_storage_refund`



<pre><code>&#35;[deprecated]
public fun initialize_storage_refund(_: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_storage_refund(_: &amp;signer) &#123;
    abort error::not_implemented(ENO_LONGER_SUPPORTED)
&#125;
</code></pre>



</details>

<a id="0x1_transaction_fee_emit_fee_statement"></a>

## Function `emit_fee_statement`



<pre><code>fun emit_fee_statement(fee_statement: transaction_fee::FeeStatement)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun emit_fee_statement(fee_statement: FeeStatement) &#123;
    event::emit(fee_statement)
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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_CollectedFeesPerBlock"></a>

### Resource `CollectedFeesPerBlock`


<pre><code>struct CollectedFeesPerBlock has key
</code></pre>



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
invariant burn_percentage &lt;&#61; 100;
</code></pre>



<a id="@Specification_1_initialize_fee_collection_and_distribution"></a>

### Function `initialize_fee_collection_and_distribution`


<pre><code>public fun initialize_fee_collection_and_distribution(aptos_framework: &amp;signer, burn_percentage: u8)
</code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
aborts_if burn_percentage &gt; 100;
let aptos_addr &#61; signer::address_of(aptos_framework);
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if exists&lt;ValidatorFees&gt;(aptos_addr);
include system_addresses::AbortsIfNotAptosFramework &#123; account: aptos_framework &#125;;
include aggregator_factory::CreateAggregatorInternalAbortsIf;
aborts_if exists&lt;CollectedFeesPerBlock&gt;(aptos_addr);
ensures exists&lt;ValidatorFees&gt;(aptos_addr);
ensures exists&lt;CollectedFeesPerBlock&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_upgrade_burn_percentage"></a>

### Function `upgrade_burn_percentage`


<pre><code>public fun upgrade_burn_percentage(aptos_framework: &amp;signer, new_burn_percentage: u8)
</code></pre>




<pre><code>aborts_if new_burn_percentage &gt; 100;
let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
// This enforces <a id="high-level-req-5" href="#high-level-req">high-level requirement 5</a> and <a id="high-level-req-6.3" href="#high-level-req">high-level requirement 6</a>:
include ProcessCollectedFeesRequiresAndEnsures;
ensures exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework) &#61;&#61;&gt;
    global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).burn_percentage &#61;&#61; new_burn_percentage;
</code></pre>



<a id="@Specification_1_register_proposer_for_fee_collection"></a>

### Function `register_proposer_for_fee_collection`


<pre><code>public(friend) fun register_proposer_for_fee_collection(proposer_addr: address)
</code></pre>




<pre><code>aborts_if false;
// This enforces <a id="high-level-req-6.1" href="#high-level-req">high-level requirement 6</a>:
ensures is_fees_collection_enabled() &#61;&#61;&gt;
    option::spec_borrow(global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).proposer) &#61;&#61; proposer_addr;
</code></pre>



<a id="@Specification_1_burn_coin_fraction"></a>

### Function `burn_coin_fraction`


<pre><code>fun burn_coin_fraction(coin: &amp;mut coin::Coin&lt;aptos_coin::AptosCoin&gt;, burn_percentage: u8)
</code></pre>




<pre><code>requires burn_percentage &lt;&#61; 100;
requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
let amount_to_burn &#61; (burn_percentage &#42; coin::value(coin)) / 100;
include amount_to_burn &gt; 0 &#61;&#61;&gt; coin::CoinSubAbortsIf&lt;AptosCoin&gt; &#123; amount: amount_to_burn &#125;;
ensures coin.value &#61;&#61; old(coin).value &#45; amount_to_burn;
</code></pre>




<a id="0x1_transaction_fee_collectedFeesAggregator"></a>


<pre><code>fun collectedFeesAggregator(): AggregatableCoin&lt;AptosCoin&gt; &#123;
   global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount
&#125;
</code></pre>




<a id="0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply"></a>


<pre><code>schema RequiresCollectedFeesPerValueLeqBlockAptosSupply &#123;
    let maybe_supply &#61; coin::get_coin_supply_opt&lt;AptosCoin&gt;();
    requires
        (is_fees_collection_enabled() &amp;&amp; option::is_some(maybe_supply)) &#61;&#61;&gt;
            (aggregator::spec_aggregator_get_val(global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount.value) &lt;&#61;
                optional_aggregator::optional_aggregator_value(
                    option::spec_borrow(coin::get_coin_supply_opt&lt;AptosCoin&gt;())
                ));
&#125;
</code></pre>




<a id="0x1_transaction_fee_ProcessCollectedFeesRequiresAndEnsures"></a>


<pre><code>schema ProcessCollectedFeesRequiresAndEnsures &#123;
    requires exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
    requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
    requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
    include RequiresCollectedFeesPerValueLeqBlockAptosSupply;
    aborts_if false;
    let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
    let post post_collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
    let pre_amount &#61; aggregator::spec_aggregator_get_val(collected_fees.amount.value);
    let post post_amount &#61; aggregator::spec_aggregator_get_val(post_collected_fees.amount.value);
    let fees_table &#61; global&lt;stake::ValidatorFees&gt;(@aptos_framework).fees_table;
    let post post_fees_table &#61; global&lt;stake::ValidatorFees&gt;(@aptos_framework).fees_table;
    let proposer &#61; option::spec_borrow(collected_fees.proposer);
    let fee_to_add &#61; pre_amount &#45; pre_amount &#42; collected_fees.burn_percentage / 100;
    ensures is_fees_collection_enabled() &#61;&#61;&gt; option::spec_is_none(post_collected_fees.proposer) &amp;&amp; post_amount &#61;&#61; 0;
    ensures is_fees_collection_enabled() &amp;&amp; aggregator::spec_read(collected_fees.amount.value) &gt; 0 &amp;&amp;
        option::spec_is_some(collected_fees.proposer) &#61;&#61;&gt;
        if (proposer !&#61; @vm_reserved) &#123;
            if (table::spec_contains(fees_table, proposer)) &#123;
                table::spec_get(post_fees_table, proposer).value &#61;&#61; table::spec_get(
                    fees_table,
                    proposer
                ).value &#43; fee_to_add
            &#125; else &#123;
                table::spec_get(post_fees_table, proposer).value &#61;&#61; fee_to_add
            &#125;
        &#125; else &#123;
            option::spec_is_none(post_collected_fees.proposer) &amp;&amp; post_amount &#61;&#61; 0
        &#125;;
&#125;
</code></pre>



<a id="@Specification_1_process_collected_fees"></a>

### Function `process_collected_fees`


<pre><code>public(friend) fun process_collected_fees()
</code></pre>




<pre><code>// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
include ProcessCollectedFeesRequiresAndEnsures;
</code></pre>



<a id="@Specification_1_burn_fee"></a>

### Function `burn_fee`


<pre><code>public(friend) fun burn_fee(account: address, fee: u64)
</code></pre>


<code>AptosCoinCapabilities</code> should be exists.


<pre><code>pragma verify &#61; false;
aborts_if !exists&lt;AptosCoinCapabilities&gt;(@aptos_framework);
let account_addr &#61; account;
let amount &#61; fee;
let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
let coin_store &#61; global&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr);
let post post_coin_store &#61; global&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr);
aborts_if amount !&#61; 0 &amp;&amp; !(exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr)
    &amp;&amp; exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(account_addr));
aborts_if coin_store.coin.value &lt; amount;
let maybe_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr).supply;
let supply_aggr &#61; option::spec_borrow(maybe_supply);
let value &#61; optional_aggregator::optional_aggregator_value(supply_aggr);
let post post_maybe_supply &#61; global&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr).supply;
let post post_supply &#61; option::spec_borrow(post_maybe_supply);
let post post_value &#61; optional_aggregator::optional_aggregator_value(post_supply);
aborts_if option::spec_is_some(maybe_supply) &amp;&amp; value &lt; amount;
ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; amount;
ensures if (option::spec_is_some(maybe_supply)) &#123;
    post_value &#61;&#61; value &#45; amount
&#125; else &#123;
    option::spec_is_none(post_maybe_supply)
&#125;;
ensures coin::supply&lt;AptosCoin&gt; &#61;&#61; old(coin::supply&lt;AptosCoin&gt;) &#45; amount;
</code></pre>



<a id="@Specification_1_mint_and_refund"></a>

### Function `mint_and_refund`


<pre><code>public(friend) fun mint_and_refund(account: address, refund: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let aptos_addr &#61; type_info::type_of&lt;AptosCoin&gt;().account_address;
aborts_if (refund !&#61; 0) &amp;&amp; !exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(aptos_addr);
include coin::CoinAddAbortsIf&lt;AptosCoin&gt; &#123; amount: refund &#125;;
aborts_if !exists&lt;CoinStore&lt;AptosCoin&gt;&gt;(account);
aborts_if !exists&lt;AptosCoinMintCapability&gt;(@aptos_framework);
let supply &#61; coin::supply&lt;AptosCoin&gt;;
let post post_supply &#61; coin::supply&lt;AptosCoin&gt;;
aborts_if [abstract] supply &#43; refund &gt; MAX_U128;
ensures post_supply &#61;&#61; supply &#43; refund;
</code></pre>



<a id="@Specification_1_collect_fee"></a>

### Function `collect_fee`


<pre><code>public(friend) fun collect_fee(account: address, fee: u64)
</code></pre>




<pre><code>pragma verify &#61; false;
let collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;
let aggr &#61; collected_fees.value;
let coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);
aborts_if !exists&lt;CollectedFeesPerBlock&gt;(@aptos_framework);
aborts_if fee &gt; 0 &amp;&amp; !exists&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);
aborts_if fee &gt; 0 &amp;&amp; coin_store.coin.value &lt; fee;
aborts_if fee &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)
    &#43; fee &gt; aggregator::spec_get_limit(aggr);
aborts_if fee &gt; 0 &amp;&amp; aggregator::spec_aggregator_get_val(aggr)
    &#43; fee &gt; MAX_U128;
let post post_coin_store &#61; global&lt;coin::CoinStore&lt;AptosCoin&gt;&gt;(account);
let post post_collected_fees &#61; global&lt;CollectedFeesPerBlock&gt;(@aptos_framework).amount;
ensures post_coin_store.coin.value &#61;&#61; coin_store.coin.value &#45; fee;
ensures aggregator::spec_aggregator_get_val(post_collected_fees.value) &#61;&#61; aggregator::spec_aggregator_get_val(
    aggr
) &#43; fee;
</code></pre>



<a id="@Specification_1_store_aptos_coin_burn_cap"></a>

### Function `store_aptos_coin_burn_cap`


<pre><code>public(friend) fun store_aptos_coin_burn_cap(aptos_framework: &amp;signer, burn_cap: coin::BurnCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>


Ensure caller is admin.
Aborts if <code>AptosCoinCapabilities</code> already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(addr);
aborts_if exists&lt;AptosCoinCapabilities&gt;(addr);
ensures exists&lt;AptosCoinCapabilities&gt;(addr);
</code></pre>



<a id="@Specification_1_store_aptos_coin_mint_cap"></a>

### Function `store_aptos_coin_mint_cap`


<pre><code>public(friend) fun store_aptos_coin_mint_cap(aptos_framework: &amp;signer, mint_cap: coin::MintCapability&lt;aptos_coin::AptosCoin&gt;)
</code></pre>


Ensure caller is admin.
Aborts if <code>AptosCoinMintCapability</code> already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(addr);
aborts_if exists&lt;AptosCoinMintCapability&gt;(addr);
ensures exists&lt;AptosCoinMintCapability&gt;(addr);
</code></pre>



<a id="@Specification_1_initialize_storage_refund"></a>

### Function `initialize_storage_refund`


<pre><code>&#35;[deprecated]
public fun initialize_storage_refund(_: &amp;signer)
</code></pre>


Historical. Aborts.


<pre><code>aborts_if true;
</code></pre>



<a id="@Specification_1_emit_fee_statement"></a>

### Function `emit_fee_statement`


<pre><code>fun emit_fee_statement(fee_statement: transaction_fee::FeeStatement)
</code></pre>


Aborts if module event feature is not enabled.


[move-book]: https://aptos.dev/move/book/SUMMARY
