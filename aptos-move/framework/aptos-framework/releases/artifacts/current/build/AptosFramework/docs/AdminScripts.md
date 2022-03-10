
<a name="0x1_AdminScripts"></a>

# Module `0x1::AdminScripts`



-  [Function `delegate_mint_capability`](#0x1_AdminScripts_delegate_mint_capability)
-  [Function `claim_mint_capability`](#0x1_AdminScripts_claim_mint_capability)
-  [Function `mint`](#0x1_AdminScripts_mint)
-  [Function `set_gas_constants`](#0x1_AdminScripts_set_gas_constants)


<pre><code><b>use</b> <a href="AptosVMConfig.md#0x1_AptosVMConfig">0x1::AptosVMConfig</a>;
<b>use</b> <a href="TestCoin.md#0x1_TestCoin">0x1::TestCoin</a>;
</code></pre>



<a name="0x1_AdminScripts_delegate_mint_capability"></a>

## Function `delegate_mint_capability`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_delegate_mint_capability">delegate_mint_capability</a>(core_resource_account: signer, addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_delegate_mint_capability">delegate_mint_capability</a>(core_resource_account: signer, addr: <b>address</b>) {
    <a href="TestCoin.md#0x1_TestCoin_delegte_mint_capability">TestCoin::delegte_mint_capability</a>(&core_resource_account, addr);
}
</code></pre>



</details>

<a name="0x1_AdminScripts_claim_mint_capability"></a>

## Function `claim_mint_capability`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_claim_mint_capability">claim_mint_capability</a>(sender: signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_claim_mint_capability">claim_mint_capability</a>(sender: signer) {
    <a href="TestCoin.md#0x1_TestCoin_claim_mint_capability">TestCoin::claim_mint_capability</a>(&sender);
}
</code></pre>



</details>

<a name="0x1_AdminScripts_mint"></a>

## Function `mint`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_mint">mint</a>(sender: signer, addr: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_mint">mint</a>(sender: signer, addr: <b>address</b>, amount: u64) {
    <a href="TestCoin.md#0x1_TestCoin_mint">TestCoin::mint</a>(&sender, addr, amount);
}
</code></pre>



</details>

<a name="0x1_AdminScripts_set_gas_constants"></a>

## Function `set_gas_constants`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_set_gas_constants">set_gas_constants</a>(sender: signer, global_memory_per_byte_cost: u64, global_memory_per_byte_write_cost: u64, min_transaction_gas_units: u64, large_transaction_cutoff: u64, intrinsic_gas_per_byte: u64, maximum_number_of_gas_units: u64, min_price_per_gas_unit: u64, max_price_per_gas_unit: u64, max_transaction_size_in_bytes: u64, gas_unit_scaling_factor: u64, default_account_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AdminScripts.md#0x1_AdminScripts_set_gas_constants">set_gas_constants</a>(
    sender: signer,
    global_memory_per_byte_cost: u64,
    global_memory_per_byte_write_cost: u64,
    min_transaction_gas_units: u64,
    large_transaction_cutoff: u64,
    intrinsic_gas_per_byte: u64,
    maximum_number_of_gas_units: u64,
    min_price_per_gas_unit: u64,
    max_price_per_gas_unit: u64,
    max_transaction_size_in_bytes: u64,
    gas_unit_scaling_factor: u64,
    default_account_size: u64,
) {
    <a href="AptosVMConfig.md#0x1_AptosVMConfig_set_gas_constants">AptosVMConfig::set_gas_constants</a>(
        &sender,
        global_memory_per_byte_cost,
        global_memory_per_byte_write_cost,
        min_transaction_gas_units,
        large_transaction_cutoff,
        intrinsic_gas_per_byte,
        maximum_number_of_gas_units,
        min_price_per_gas_unit,
        max_price_per_gas_unit,
        max_transaction_size_in_bytes,
        gas_unit_scaling_factor,
        default_account_size,
    );
}
</code></pre>



</details>
