
<a name="0x1_SystemAdministrationScripts"></a>

# Module `0x1::SystemAdministrationScripts`

This module contains Diem Framework script functions to administer the
network outside of validators and validator operators.


-  [Function `update_diem_version`](#0x1_SystemAdministrationScripts_update_diem_version)
    -  [Summary](#@Summary_0)
    -  [Technical Description](#@Technical_Description_1)
    -  [Parameters](#@Parameters_2)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_3)
-  [Function `set_gas_constants`](#0x1_SystemAdministrationScripts_set_gas_constants)
    -  [Summary](#@Summary_4)
    -  [Technical Description](#@Technical_Description_5)
    -  [Parameters](#@Parameters_6)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_7)
-  [Function `initialize_diem_consensus_config`](#0x1_SystemAdministrationScripts_initialize_diem_consensus_config)
    -  [Summary](#@Summary_8)
    -  [Technical Description](#@Technical_Description_9)
    -  [Parameters](#@Parameters_10)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_11)
-  [Function `update_diem_consensus_config`](#0x1_SystemAdministrationScripts_update_diem_consensus_config)
    -  [Summary](#@Summary_12)
    -  [Technical Description](#@Technical_Description_13)
    -  [Parameters](#@Parameters_14)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_15)


<pre><code><b>use</b> <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">0x1::DiemConsensusConfig</a>;
<b>use</b> <a href="DiemVMConfig.md#0x1_DiemVMConfig">0x1::DiemVMConfig</a>;
<b>use</b> <a href="ExperimentalVersion.md#0x1_ExperimentalVersion">0x1::ExperimentalVersion</a>;
</code></pre>



<a name="0x1_SystemAdministrationScripts_update_diem_version"></a>

## Function `update_diem_version`


<a name="@Summary_0"></a>

### Summary

Updates the Diem major version that is stored on-chain and is used by the VM.  This
transaction can only be sent from the Diem Root account.


<a name="@Technical_Description_1"></a>

### Technical Description

Updates the <code><a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemVersion.md#0x1_DiemVersion">DiemVersion</a></code> on-chain config and emits a <code><a href="DiemConfig.md#0x1_DiemConfig_NewEpochEvent">DiemConfig::NewEpochEvent</a></code> to trigger
a reconfiguration of the system. The <code>major</code> version that is passed in must be strictly greater
than the current major version held on-chain. The VM reads this information and can use it to
preserve backwards compatibility with previous major versions of the VM.


<a name="@Parameters_2"></a>

### Parameters

| Name            | Type     | Description                                                                |
| ------          | ------   | -------------                                                              |
| <code>account</code>       | <code>signer</code> | Signer of the sending account. Must be the Diem Root account.              |
| <code>_sliding_nonce</code> | <code>u64</code>    | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction. |
| <code>major</code>         | <code>u64</code>    | The <code>major</code> version of the VM to be used from this transaction on.         |


<a name="@Common_Abort_Conditions_3"></a>

### Common Abort Conditions

| Error Category             | Error Reason                                  | Description                                                                                |
| ----------------           | --------------                                | -------------                                                                              |
| <code>Errors::REQUIRES_ADDRESS</code> | <code>CoreAddresses::EDIEM_ROOT</code>                   | <code>account</code> is not the Diem Root account.                                                    |
| <code>Errors::INVALID_ARGUMENT</code> | <code>DiemVersion::EINVALID_MAJOR_VERSION_NUMBER</code>  | <code>major</code> is less-than or equal to the current major version stored on-chain.                |


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_update_diem_version">update_diem_version</a>(account: signer, _sliding_nonce: u64, major: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_update_diem_version">update_diem_version</a>(account: signer, _sliding_nonce: u64, major: u64) {
    <a href="ExperimentalVersion.md#0x1_ExperimentalVersion_set">ExperimentalVersion::set</a>(&account, major)
}
</code></pre>



</details>

<a name="0x1_SystemAdministrationScripts_set_gas_constants"></a>

## Function `set_gas_constants`


<a name="@Summary_4"></a>

### Summary

Updates the gas constants stored on chain and used by the VM for gas
metering. This transaction can only be sent from the Diem Root account.


<a name="@Technical_Description_5"></a>

### Technical Description

Updates the on-chain config holding the <code><a href="DiemVMConfig.md#0x1_DiemVMConfig">DiemVMConfig</a></code> and emits a
<code><a href="DiemConfig.md#0x1_DiemConfig_NewEpochEvent">DiemConfig::NewEpochEvent</a></code> to trigger a reconfiguration of the system.


<a name="@Parameters_6"></a>

### Parameters

| Name                                | Type     | Description                                                                                            |
| ------                              | ------   | -------------                                                                                          |
| <code>account</code>                           | <code>signer</code> | Signer of the sending account. Must be the Diem Root account.                                          |
| <code>_sliding_nonce</code>                     | <code>u64</code>    | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction.                             |
| <code>global_memory_per_byte_cost</code>       | <code>u64</code>    | The new cost to read global memory per-byte to be used for gas metering.                               |
| <code>global_memory_per_byte_write_cost</code> | <code>u64</code>    | The new cost to write global memory per-byte to be used for gas metering.                              |
| <code>min_transaction_gas_units</code>         | <code>u64</code>    | The new flat minimum amount of gas required for any transaction.                                       |
| <code>large_transaction_cutoff</code>          | <code>u64</code>    | The new size over which an additional charge will be assessed for each additional byte.                |
| <code>intrinsic_gas_per_byte</code>            | <code>u64</code>    | The new number of units of gas that to be charged per-byte over the new <code>large_transaction_cutoff</code>.    |
| <code>maximum_number_of_gas_units</code>       | <code>u64</code>    | The new maximum number of gas units that can be set in a transaction.                                  |
| <code>min_price_per_gas_unit</code>            | <code>u64</code>    | The new minimum gas price that can be set for a transaction.                                           |
| <code>max_price_per_gas_unit</code>            | <code>u64</code>    | The new maximum gas price that can be set for a transaction.                                           |
| <code>max_transaction_size_in_bytes</code>     | <code>u64</code>    | The new maximum size of a transaction that can be processed.                                           |
| <code>gas_unit_scaling_factor</code>           | <code>u64</code>    | The new scaling factor to use when scaling between external and internal gas units.                    |
| <code>default_account_size</code>              | <code>u64</code>    | The new default account size to use when assessing final costs for reads and writes to global storage. |


<a name="@Common_Abort_Conditions_7"></a>

### Common Abort Conditions

| Error Category             | Error Reason                                | Description                                                                                |
| ----------------           | --------------                              | -------------                                                                              |
| <code>Errors::INVALID_ARGUMENT</code> | <code><a href="DiemVMConfig.md#0x1_DiemVMConfig_EGAS_CONSTANT_INCONSISTENCY">DiemVMConfig::EGAS_CONSTANT_INCONSISTENCY</a></code> | The provided gas constants are inconsistent.                                               |
| <code>Errors::REQUIRES_ADDRESS</code> | <code>CoreAddresses::EDIEM_ROOT</code>                 | <code>account</code> is not the Diem Root account.                                                    |


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_set_gas_constants">set_gas_constants</a>(dr_account: signer, _sliding_nonce: u64, global_memory_per_byte_cost: u64, global_memory_per_byte_write_cost: u64, min_transaction_gas_units: u64, large_transaction_cutoff: u64, intrinsic_gas_per_byte: u64, maximum_number_of_gas_units: u64, min_price_per_gas_unit: u64, max_price_per_gas_unit: u64, max_transaction_size_in_bytes: u64, gas_unit_scaling_factor: u64, default_account_size: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_set_gas_constants">set_gas_constants</a>(
    dr_account: signer,
    _sliding_nonce: u64,
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
    <a href="DiemVMConfig.md#0x1_DiemVMConfig_set_gas_constants">DiemVMConfig::set_gas_constants</a>(
            &dr_account,
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
    )
}
</code></pre>



</details>

<a name="0x1_SystemAdministrationScripts_initialize_diem_consensus_config"></a>

## Function `initialize_diem_consensus_config`


<a name="@Summary_8"></a>

### Summary

Initializes the Diem consensus config that is stored on-chain.  This
transaction can only be sent from the Diem Root account.


<a name="@Technical_Description_9"></a>

### Technical Description

Initializes the <code><a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">DiemConsensusConfig</a></code> on-chain config to empty and allows future updates from DiemRoot via
<code>update_diem_consensus_config</code>. This doesn't emit a <code><a href="DiemConfig.md#0x1_DiemConfig_NewEpochEvent">DiemConfig::NewEpochEvent</a></code>.


<a name="@Parameters_10"></a>

### Parameters

| Name            | Type      | Description                                                                |
| ------          | ------    | -------------                                                              |
| <code>account</code>       | <code>signer</code> | Signer of the sending account. Must be the Diem Root account.               |
| <code>_sliding_nonce</code> | <code>u64</code>     | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction. |


<a name="@Common_Abort_Conditions_11"></a>

### Common Abort Conditions

| Error Category             | Error Reason                                  | Description                                                                                |
| ----------------           | --------------                                | -------------                                                                              |
| <code>Errors::REQUIRES_ADDRESS</code> | <code>CoreAddresses::EDIEM_ROOT</code>                   | <code>account</code> is not the Diem Root account.                                                    |


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_initialize_diem_consensus_config">initialize_diem_consensus_config</a>(account: signer, _sliding_nonce: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_initialize_diem_consensus_config">initialize_diem_consensus_config</a>(account: signer, _sliding_nonce: u64) {
    <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_initialize">DiemConsensusConfig::initialize</a>(&account);
}
</code></pre>



</details>

<a name="0x1_SystemAdministrationScripts_update_diem_consensus_config"></a>

## Function `update_diem_consensus_config`


<a name="@Summary_12"></a>

### Summary

Updates the Diem consensus config that is stored on-chain and is used by the Consensus.  This
transaction can only be sent from the Diem Root account.


<a name="@Technical_Description_13"></a>

### Technical Description

Updates the <code><a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig">DiemConsensusConfig</a></code> on-chain config and emits a <code><a href="DiemConfig.md#0x1_DiemConfig_NewEpochEvent">DiemConfig::NewEpochEvent</a></code> to trigger
a reconfiguration of the system.


<a name="@Parameters_14"></a>

### Parameters

| Name            | Type          | Description                                                                |
| ------          | ------        | -------------                                                              |
| <code>account</code>       | <code>signer</code>      | Signer of the sending account. Must be the Diem Root account.              |
| <code>_sliding_nonce</code> | <code>u64</code>         | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction. |
| <code>config</code>        | <code>vector&lt;u8&gt;</code>  | The serialized bytes of consensus config.                                  |


<a name="@Common_Abort_Conditions_15"></a>

### Common Abort Conditions

| Error Category             | Error Reason                                  | Description                                                                                |
| ----------------           | --------------                                | -------------                                                                              |
| <code>Errors::REQUIRES_ADDRESS</code> | <code>CoreAddresses::EDIEM_ROOT</code>                   | <code>account</code> is not the Diem Root account.                                                    |


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_update_diem_consensus_config">update_diem_consensus_config</a>(account: signer, _sliding_nonce: u64, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="SystemAdministrationScripts.md#0x1_SystemAdministrationScripts_update_diem_consensus_config">update_diem_consensus_config</a>(account: signer, _sliding_nonce: u64, config: vector&lt;u8&gt;) {
    <a href="DiemConsensusConfig.md#0x1_DiemConsensusConfig_set">DiemConsensusConfig::set</a>(&account, config)
}
</code></pre>



</details>
