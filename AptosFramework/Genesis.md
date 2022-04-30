
<a name="0x1_Genesis"></a>

# Module `0x1::Genesis`



-  [Function `initialize`](#0x1_Genesis_initialize)
-  [Function `initialize_internal`](#0x1_Genesis_initialize_internal)
-  [Function `create_initialize_validators`](#0x1_Genesis_create_initialize_validators)


<pre><code><b>use</b> <a href="Account.md#0x1_Account">0x1::Account</a>;
<b>use</b> <a href="Block.md#0x1_Block">0x1::Block</a>;
<b>use</b> <a href="ChainId.md#0x1_ChainId">0x1::ChainId</a>;
<b>use</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig">0x1::ConsensusConfig</a>;
<b>use</b> <a href="../MoveStdlib/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="../MoveStdlib/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="Stake.md#0x1_Stake">0x1::Stake</a>;
<b>use</b> <a href="TestCoin.md#0x1_TestCoin">0x1::TestCoin</a>;
<b>use</b> <a href="Timestamp.md#0x1_Timestamp">0x1::Timestamp</a>;
<b>use</b> <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">0x1::TransactionPublishingOption</a>;
<b>use</b> <a href="VMConfig.md#0x1_VMConfig">0x1::VMConfig</a>;
<b>use</b> <a href="../MoveStdlib/Vector.md#0x1_Vector">0x1::Vector</a>;
<b>use</b> <a href="Version.md#0x1_Version">0x1::Version</a>;
</code></pre>



<a name="0x1_Genesis_initialize"></a>

## Function `initialize`



<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize">initialize</a>(core_resource_account: signer, core_resource_account_auth_key: vector&lt;u8&gt;, initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;, is_open_module: bool, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, min_price_per_gas_unit: u64, epoch_interval: u64, minimum_stake: u64, maximum_stake: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize">initialize</a>(
    core_resource_account: signer,
    core_resource_account_auth_key: vector&lt;u8&gt;,
    initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;,
    is_open_module: bool,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_version: u64,
    consensus_config: vector&lt;u8&gt;,
    min_price_per_gas_unit: u64,
    epoch_interval: u64,
    minimum_stake: u64,
    maximum_stake: u64,
) {
    <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(
        &core_resource_account,
        core_resource_account_auth_key,
        initial_script_allow_list,
        is_open_module,
        instruction_schedule,
        native_schedule,
        chain_id,
        initial_version,
        consensus_config,
        min_price_per_gas_unit,
        epoch_interval,
        minimum_stake,
        maximum_stake,
    )
}
</code></pre>



</details>

<a name="0x1_Genesis_initialize_internal"></a>

## Function `initialize_internal`



<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(core_resource_account: &signer, core_resource_account_auth_key: vector&lt;u8&gt;, initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;, is_open_module: bool, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, chain_id: u8, initial_version: u64, consensus_config: vector&lt;u8&gt;, min_price_per_gas_unit: u64, epoch_interval: u64, minimum_stake: u64, maximum_stake: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(
    core_resource_account: &signer,
    core_resource_account_auth_key: vector&lt;u8&gt;,
    initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;,
    is_open_module: bool,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_version: u64,
    consensus_config: vector&lt;u8&gt;,
    min_price_per_gas_unit: u64,
    epoch_interval: u64,
    minimum_stake: u64,
    maximum_stake: u64,
) {
    // initialize the core resource account
    <a href="Account.md#0x1_Account_initialize">Account::initialize</a>(
        core_resource_account,
        @AptosFramework,
        b"<a href="Account.md#0x1_Account">Account</a>",
        b"script_prologue",
        b"module_prologue",
        b"writeset_prologue",
        b"script_prologue",
        b"epilogue",
        b"writeset_epilogue",
        <b>false</b>,
    );
    <a href="Account.md#0x1_Account_create_account_internal">Account::create_account_internal</a>(<a href="../MoveStdlib/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(core_resource_account));
    <a href="Account.md#0x1_Account_rotate_authentication_key_internal">Account::rotate_authentication_key_internal</a>(core_resource_account, <b>copy</b> core_resource_account_auth_key);
    // initialize the core framework account
    <b>let</b> core_framework_account = <a href="Account.md#0x1_Account_create_core_framework_account">Account::create_core_framework_account</a>();
    <a href="Account.md#0x1_Account_rotate_authentication_key_internal">Account::rotate_authentication_key_internal</a>(&core_framework_account, core_resource_account_auth_key);

    // Consensus config setup
    <a href="ConsensusConfig.md#0x1_ConsensusConfig_initialize">ConsensusConfig::initialize</a>(core_resource_account);
    <a href="Version.md#0x1_Version_initialize">Version::initialize</a>(core_resource_account, initial_version);
    <a href="Stake.md#0x1_Stake_initialize_validator_set">Stake::initialize_validator_set</a>(core_resource_account, minimum_stake, maximum_stake);

    <a href="VMConfig.md#0x1_VMConfig_initialize">VMConfig::initialize</a>(
        core_resource_account,
        instruction_schedule,
        native_schedule,
        min_price_per_gas_unit,
    );

    <a href="ConsensusConfig.md#0x1_ConsensusConfig_set">ConsensusConfig::set</a>(core_resource_account, consensus_config);

    <a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption_initialize">TransactionPublishingOption::initialize</a>(core_resource_account, initial_script_allow_list, is_open_module);

    <a href="TestCoin.md#0x1_TestCoin_initialize">TestCoin::initialize</a>(core_resource_account, 1000000);
    <a href="TestCoin.md#0x1_TestCoin_mint_internal">TestCoin::mint_internal</a>(core_resource_account, <a href="../MoveStdlib/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(core_resource_account), 18446744073709551615);

    // Pad the event counter for the Root account <b>to</b> match DPN. This
    // _MUST_ match the new epoch event counter otherwise all manner of
    // things start <b>to</b> <b>break</b>.
    <a href="../MoveStdlib/docs/Event.md#0x1_Event_destroy_handle">Event::destroy_handle</a>(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;u64&gt;(core_resource_account));
    <a href="../MoveStdlib/docs/Event.md#0x1_Event_destroy_handle">Event::destroy_handle</a>(<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;u64&gt;(core_resource_account));

    // this needs <b>to</b> be called at the very end
    <a href="ChainId.md#0x1_ChainId_initialize">ChainId::initialize</a>(core_resource_account, chain_id);
    <a href="Reconfiguration.md#0x1_Reconfiguration_initialize">Reconfiguration::initialize</a>(core_resource_account);
    <a href="Block.md#0x1_Block_initialize_block_metadata">Block::initialize_block_metadata</a>(core_resource_account, epoch_interval);
    <a href="Timestamp.md#0x1_Timestamp_set_time_has_started">Timestamp::set_time_has_started</a>(core_resource_account);
}
</code></pre>



</details>

<a name="0x1_Genesis_create_initialize_validators"></a>

## Function `create_initialize_validators`

Sets up the initial validator set for the network.
The validator "owner" accounts, and their authentication
keys are encoded in the <code>owners</code> and <code>owner_auth_key</code> vectors.
Each validator signs consensus messages with the private key corresponding to the Ed25519
public key in <code>consensus_pubkeys</code>.
Finally, each validator must specify the network address
(see types/src/network_address/mod.rs) for itself and its full nodes.


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_create_initialize_validators">create_initialize_validators</a>(core_resource_account: signer, owners: vector&lt;<b>address</b>&gt;, owner_auth_keys: vector&lt;vector&lt;u8&gt;&gt;, consensus_pubkeys: vector&lt;vector&lt;u8&gt;&gt;, validator_network_addresses: vector&lt;vector&lt;u8&gt;&gt;, full_node_network_addresses: vector&lt;vector&lt;u8&gt;&gt;, staking_distribution: vector&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_create_initialize_validators">create_initialize_validators</a>(
    core_resource_account: signer,
    owners: vector&lt;<b>address</b>&gt;,
    owner_auth_keys: vector&lt;vector&lt;u8&gt;&gt;,
    consensus_pubkeys: vector&lt;vector&lt;u8&gt;&gt;,
    validator_network_addresses: vector&lt;vector&lt;u8&gt;&gt;,
    full_node_network_addresses: vector&lt;vector&lt;u8&gt;&gt;,
    staking_distribution: vector&lt;u64&gt;,
) {
    <b>let</b> num_owners = <a href="../MoveStdlib/Vector.md#0x1_Vector_length">Vector::length</a>(&owners);
    <b>let</b> num_owner_keys = <a href="../MoveStdlib/Vector.md#0x1_Vector_length">Vector::length</a>(&owner_auth_keys);
    <b>assert</b>!(num_owners == num_owner_keys, 0);
    <b>let</b> num_validator_network_addresses = <a href="../MoveStdlib/Vector.md#0x1_Vector_length">Vector::length</a>(&validator_network_addresses);
    <b>assert</b>!(num_owner_keys == num_validator_network_addresses, 0);
    <b>let</b> num_full_node_network_addresses = <a href="../MoveStdlib/Vector.md#0x1_Vector_length">Vector::length</a>(&full_node_network_addresses);
    <b>assert</b>!(num_validator_network_addresses == num_full_node_network_addresses, 0);
    <b>let</b> num_staking = <a href="../MoveStdlib/Vector.md#0x1_Vector_length">Vector::length</a>(&staking_distribution);
    <b>assert</b>!(num_full_node_network_addresses == num_staking, 0);

    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_owners) {
        <b>let</b> owner = <a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&owners, i);
        // create each validator account and rotate its auth key <b>to</b> the correct value
        <b>let</b> (owner_account, _) = <a href="Account.md#0x1_Account_create_account_internal">Account::create_account_internal</a>(*owner);

        <b>let</b> owner_auth_key = *<a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&owner_auth_keys, i);
        <a href="Account.md#0x1_Account_rotate_authentication_key_internal">Account::rotate_authentication_key_internal</a>(&owner_account, owner_auth_key);

        // <b>use</b> the operator account set up the validator config
        <b>let</b> validator_network_address = *<a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&validator_network_addresses, i);
        <b>let</b> full_node_network_address = *<a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&full_node_network_addresses, i);
        <b>let</b> consensus_pubkey = *<a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&consensus_pubkeys, i);
        <a href="Stake.md#0x1_Stake_register_validator_candidate">Stake::register_validator_candidate</a>(
            &owner_account,
            consensus_pubkey,
            validator_network_address,
            full_node_network_address
        );
        <b>let</b> amount = *<a href="../MoveStdlib/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&staking_distribution, i);
        <a href="Stake.md#0x1_Stake_delegate_stake">Stake::delegate_stake</a>(&core_resource_account, *owner, amount, 100000);
        <a href="Stake.md#0x1_Stake_join_validator_set">Stake::join_validator_set</a>(&owner_account);

        i = i + 1;
    };
    <a href="Stake.md#0x1_Stake_on_new_epoch">Stake::on_new_epoch</a>();
}
</code></pre>



</details>
