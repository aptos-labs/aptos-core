
<a name="0x1_Genesis"></a>

# Module `0x1::Genesis`



-  [Function `initialize`](#0x1_Genesis_initialize)
-  [Function `initialize_internal`](#0x1_Genesis_initialize_internal)
-  [Function `create_initialize_owners_operators`](#0x1_Genesis_create_initialize_owners_operators)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/CoreGenesis.md#0x1_CoreGenesis">0x1::CoreGenesis</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">0x1::Event</a>;
<b>use</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount">0x1::ExperimentalAccount</a>;
<b>use</b> <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig">0x1::ExperimentalConsensusConfig</a>;
<b>use</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">0x1::ExperimentalParallelExecutionConfig</a>;
<b>use</b> <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig">0x1::ExperimentalVMConfig</a>;
<b>use</b> <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig">0x1::ExperimentalValidatorConfig</a>;
<b>use</b> <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig">0x1::ExperimentalValidatorOperatorConfig</a>;
<b>use</b> <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet">0x1::ExperimentalValidatorSet</a>;
<b>use</b> <a href="ExperimentalVersion.md#0x1_ExperimentalVersion">0x1::ExperimentalVersion</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig">0x1::ValidatorConfig</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">0x1::ValidatorOperatorConfig</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_Genesis_initialize"></a>

## Function `initialize`



<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize">initialize</a>(dr_account: signer, _tc_account: signer, dr_auth_key: vector&lt;u8&gt;, _tc_auth_key: vector&lt;u8&gt;, _initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;, _is_open_module: bool, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, chain_id: u8, initial_diem_version: u64, consensus_config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize">initialize</a>(
    dr_account: signer,
    _tc_account: signer,
    dr_auth_key: vector&lt;u8&gt;,
    _tc_auth_key: vector&lt;u8&gt;,
    _initial_script_allow_list: vector&lt;vector&lt;u8&gt;&gt;,
    _is_open_module: bool,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_diem_version: u64,
    consensus_config: vector&lt;u8&gt;,
) {
    <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(
        &dr_account,
        dr_auth_key,
        instruction_schedule,
        native_schedule,
        chain_id,
        initial_diem_version,
        consensus_config,
    )
}
</code></pre>



</details>

<a name="0x1_Genesis_initialize_internal"></a>

## Function `initialize_internal`



<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(dr_account: &signer, dr_auth_key: vector&lt;u8&gt;, instruction_schedule: vector&lt;u8&gt;, native_schedule: vector&lt;u8&gt;, chain_id: u8, initial_diem_version: u64, consensus_config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_initialize_internal">initialize_internal</a>(
    dr_account: &signer,
    dr_auth_key: vector&lt;u8&gt;,
    instruction_schedule: vector&lt;u8&gt;,
    native_schedule: vector&lt;u8&gt;,
    chain_id: u8,
    initial_diem_version: u64,
    consensus_config: vector&lt;u8&gt;,
) {
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_initialize">ExperimentalAccount::initialize</a>(dr_account, x"00000000000000000000000000000000");

    // Pad the event counter for the Diem Root account <b>to</b> match DPN. This
    // _MUST_ match the new epoch event counter otherwise all manner of
    // things start <b>to</b> <b>break</b>.
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_destroy_handle">Event::destroy_handle</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;u64&gt;(dr_account));
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_destroy_handle">Event::destroy_handle</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;u64&gt;(dr_account));
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_destroy_handle">Event::destroy_handle</a>(<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event_new_event_handle">Event::new_event_handle</a>&lt;u64&gt;(dr_account));

    // Consensus config setup
    <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_initialize">ExperimentalConsensusConfig::initialize</a>(dr_account);
    // Parallel execution config setup
    <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_initialize_parallel_execution">ExperimentalParallelExecutionConfig::initialize_parallel_execution</a>(dr_account);
    <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_initialize_validator_set">ExperimentalValidatorSet::initialize_validator_set</a>(dr_account);
    <a href="ExperimentalVersion.md#0x1_ExperimentalVersion_initialize">ExperimentalVersion::initialize</a>(dr_account, initial_diem_version);

    // Rotate auth keys for DiemRoot account <b>to</b> the given
    // values
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_rotate_authentication_key">ExperimentalAccount::rotate_authentication_key</a>(dr_account, dr_auth_key);
    <a href="ExperimentalVMConfig.md#0x1_ExperimentalVMConfig_initialize">ExperimentalVMConfig::initialize</a>(
        dr_account,
        instruction_schedule,
        native_schedule,
    );

    <a href="ExperimentalConsensusConfig.md#0x1_ExperimentalConsensusConfig_set">ExperimentalConsensusConfig::set</a>(dr_account, consensus_config);

    <a href="ExperimentalValidatorConfig.md#0x1_ExperimentalValidatorConfig_initialize">ExperimentalValidatorConfig::initialize</a>(dr_account);
    <a href="ExperimentalValidatorOperatorConfig.md#0x1_ExperimentalValidatorOperatorConfig_initialize">ExperimentalValidatorOperatorConfig::initialize</a>(dr_account);

    // this needs <b>to</b> be called at the very end
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/CoreGenesis.md#0x1_CoreGenesis_init">CoreGenesis::init</a>(dr_account, chain_id);
}
</code></pre>



</details>

<a name="0x1_Genesis_create_initialize_owners_operators"></a>

## Function `create_initialize_owners_operators`

Sets up the initial validator set for the Diem network.
The validator "owner" accounts, their UTF-8 names, and their authentication
keys are encoded in the <code>owners</code>, <code>owner_names</code>, and <code>owner_auth_key</code> vectors.
Each validator signs consensus messages with the private key corresponding to the Ed25519
public key in <code>consensus_pubkeys</code>.
Each validator owner has its operation delegated to an "operator" (which may be
the owner). The operators, their names, and their authentication keys are encoded
in the <code>operators</code>, <code>operator_names</code>, and <code>operator_auth_keys</code> vectors.
Finally, each validator must specify the network address
(see diem/types/src/network_address/mod.rs) for itself and its full nodes.


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_create_initialize_owners_operators">create_initialize_owners_operators</a>(dr_account: signer, owners: vector&lt;signer&gt;, owner_names: vector&lt;vector&lt;u8&gt;&gt;, owner_auth_keys: vector&lt;vector&lt;u8&gt;&gt;, consensus_pubkeys: vector&lt;vector&lt;u8&gt;&gt;, operators: vector&lt;signer&gt;, operator_names: vector&lt;vector&lt;u8&gt;&gt;, operator_auth_keys: vector&lt;vector&lt;u8&gt;&gt;, validator_network_addresses: vector&lt;vector&lt;u8&gt;&gt;, full_node_network_addresses: vector&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="Genesis.md#0x1_Genesis_create_initialize_owners_operators">create_initialize_owners_operators</a>(
    dr_account: signer,
    owners: vector&lt;signer&gt;,
    owner_names: vector&lt;vector&lt;u8&gt;&gt;,
    owner_auth_keys: vector&lt;vector&lt;u8&gt;&gt;,
    consensus_pubkeys: vector&lt;vector&lt;u8&gt;&gt;,
    operators: vector&lt;signer&gt;,
    operator_names: vector&lt;vector&lt;u8&gt;&gt;,
    operator_auth_keys: vector&lt;vector&lt;u8&gt;&gt;,
    validator_network_addresses: vector&lt;vector&lt;u8&gt;&gt;,
    full_node_network_addresses: vector&lt;vector&lt;u8&gt;&gt;,
) {
    <b>let</b> num_owners = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&owners);
    <b>let</b> num_owner_names = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&owner_names);
    <b>assert</b>!(num_owners == num_owner_names, 0);
    <b>let</b> num_owner_keys = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&owner_auth_keys);
    <b>assert</b>!(num_owner_names == num_owner_keys, 0);
    <b>let</b> num_operators = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&operators);
    <b>assert</b>!(num_owner_keys == num_operators, 0);
    <b>let</b> num_operator_names = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&operator_names);
    <b>assert</b>!(num_operators == num_operator_names, 0);
    <b>let</b> num_operator_keys = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&operator_auth_keys);
    <b>assert</b>!(num_operator_names == num_operator_keys, 0);
    <b>let</b> num_validator_network_addresses = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&validator_network_addresses);
    <b>assert</b>!(num_operator_keys == num_validator_network_addresses, 0);
    <b>let</b> num_full_node_network_addresses = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_length">Vector::length</a>(&full_node_network_addresses);
    <b>assert</b>!(num_validator_network_addresses == num_full_node_network_addresses, 0);

    <b>let</b> i = 0;
    <b>let</b> dummy_auth_key_prefix = x"00000000000000000000000000000000";
    <b>while</b> (i &lt; num_owners) {
        <b>let</b> owner = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&owners, i);
        <b>let</b> owner_address = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(owner);
        <b>let</b> owner_name = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&owner_names, i);
        // create each validator account and rotate its auth key <b>to</b> the correct value
        <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_account">ExperimentalAccount::create_validator_account</a>(
            &dr_account, owner_address, <b>copy</b> dummy_auth_key_prefix, owner_name
        );

        <b>let</b> owner_auth_key = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&owner_auth_keys, i);
        <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_rotate_authentication_key">ExperimentalAccount::rotate_authentication_key</a>(owner, owner_auth_key);

        <b>let</b> operator = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&operators, i);
        <b>let</b> operator_address = <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(operator);
        <b>let</b> operator_name = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&operator_names, i);
        // create the operator account + rotate its auth key <b>if</b> it does not already exist
        <b>if</b> (!<a href="ExperimentalAccount.md#0x1_ExperimentalAccount_exists_at">ExperimentalAccount::exists_at</a>(operator_address)) {
            <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_operator_account">ExperimentalAccount::create_validator_operator_account</a>(
                &dr_account, operator_address, <b>copy</b> dummy_auth_key_prefix, <b>copy</b> operator_name
            );
            <b>let</b> operator_auth_key = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&operator_auth_keys, i);
            <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_rotate_authentication_key">ExperimentalAccount::rotate_authentication_key</a>(operator, operator_auth_key);
        };
        // assign the operator <b>to</b> its validator
        <b>assert</b>!(<a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_get_human_name">ValidatorOperatorConfig::get_human_name</a>(operator_address) == operator_name, 0);
        <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig_set_operator">ValidatorConfig::set_operator</a>(owner, operator_address);

        // <b>use</b> the operator account set up the validator config
        <b>let</b> validator_network_address = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&validator_network_addresses, i);
        <b>let</b> full_node_network_address = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&full_node_network_addresses, i);
        <b>let</b> consensus_pubkey = *<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_borrow">Vector::borrow</a>(&consensus_pubkeys, i);
        <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/ValidatorConfig.md#0x1_ValidatorConfig_set_config">ValidatorConfig::set_config</a>(
            operator,
            owner_address,
            consensus_pubkey,
            validator_network_address,
            full_node_network_address
        );

        // finally, add this validator <b>to</b> the validator set
        <a href="ExperimentalValidatorSet.md#0x1_ExperimentalValidatorSet_add_validator">ExperimentalValidatorSet::add_validator</a>(&dr_account, owner_address);

        i = i + 1;
    }
}
</code></pre>



</details>
