
<a name="@Diem_Framework_Modules_0"></a>

# Diem Framework Modules


This is the root document for the Diem framework module documentation. The Diem framework provides a set of Move
modules which define the resources and functions available for the Diem blockchain. Each module is individually
documented here, together with its implementation and
[formal specification](../../script_documentation/spec_documentation.md).

Move modules are not directly called by clients, but instead are used to implement *transaction scripts*.
For documentation of transaction scripts which constitute the client API, see
[../../script_documentation/script_documentation.md](../../script_documentation/script_documentation.md).

The Move modules in the Diem Framework can be bucketed in to a couple categories:


<a name="@Treasury_and_Compliance_1"></a>

### Treasury and Compliance

* <code>AccountFreezing</code>
* <code>AccountLimits</code>
* <code>DesignatedDealer</code>
* <code>DualAttestation</code>

* <code>XUS</code>
* <code>XDX</code>
* <code>Diem</code>
* <code>RegisteredCurrencies</code>


<a name="@Authentication_2"></a>

### Authentication

* <code>Authenticator</code>
* <code>RecoveryAddress</code>
* <code>SharedEd25519PublicKey</code>
* <code><a href="Signature.md#0x1_Signature">Signature</a></code>


<a name="@Accounts_and_Access_Control_3"></a>

### Accounts and Access Control

* <code>DiemAccount</code>
* <code>Roles</code>
* <code>VASP</code>


<a name="@System_Management_4"></a>

### System Management

* <code><a href="ChainId.md#0x1_ChainId">ChainId</a></code>
* <code><a href="Block.md#0x1_Block">Block</a></code>
* <code><a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a></code>
* <code><a href="Timestamp.md#0x1_Timestamp">Timestamp</a></code>
* <code><a href="TransactionPublishingOption.md#0x1_TransactionPublishingOption">TransactionPublishingOption</a></code>
* <code><a href="Version.md#0x1_Version">Version</a></code>
* <code><a href="VMConfig.md#0x1_VMConfig">VMConfig</a></code>
* <code>TransactionFee</code>
* <code><a href="ValidatorSystem.md#0x1_ValidatorSystem">ValidatorSystem</a></code>
* <code><a href="ValidatorConfig.md#0x1_ValidatorConfig">ValidatorConfig</a></code>
* <code><a href="ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">ValidatorOperatorConfig</a></code>
* <code>Genesis</code> (Note: not published on-chain)


<a name="@Module_Utility_Libraries_5"></a>

### Module Utility Libraries

* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">Errors</a></code>
* <code>CoreAddresses</code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">Event</a></code>
* <code>FixedPoint32</code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Hash.md#0x1_Hash">Hash</a></code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS">BCS</a></code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a></code>
* <code>SlidingNonce</code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">Vector</a></code>
* <code><a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">Signer</a></code>


<a name="@Index_6"></a>

## Index


-  [`0x1::Account`](Account.md#0x1_Account)
-  [`0x1::BCS`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS)
-  [`0x1::Block`](Block.md#0x1_Block)
-  [`0x1::Capability`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability)
-  [`0x1::ChainId`](ChainId.md#0x1_ChainId)
-  [`0x1::ConsensusConfig`](ConsensusConfig.md#0x1_ConsensusConfig)
-  [`0x1::CoreGenesis`](CoreGenesis.md#0x1_CoreGenesis)
-  [`0x1::Errors`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors)
-  [`0x1::Event`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event)
-  [`0x1::GUID`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID)
-  [`0x1::Hash`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Hash.md#0x1_Hash)
-  [`0x1::Option`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option)
-  [`0x1::ParallelExecutionConfig`](ParallelExecutionConfig.md#0x1_ParallelExecutionConfig)
-  [`0x1::Reconfiguration`](Reconfiguration.md#0x1_Reconfiguration)
-  [`0x1::Signature`](Signature.md#0x1_Signature)
-  [`0x1::Signer`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer)
-  [`0x1::SystemAddresses`](SystemAddresses.md#0x1_SystemAddresses)
-  [`0x1::Timestamp`](Timestamp.md#0x1_Timestamp)
-  [`0x1::TransactionPublishingOption`](TransactionPublishingOption.md#0x1_TransactionPublishingOption)
-  [`0x1::VMConfig`](VMConfig.md#0x1_VMConfig)
-  [`0x1::ValidatorConfig`](ValidatorConfig.md#0x1_ValidatorConfig)
-  [`0x1::ValidatorOperatorConfig`](ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig)
-  [`0x1::ValidatorSystem`](ValidatorSystem.md#0x1_ValidatorSystem)
-  [`0x1::Vector`](../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector)
-  [`0x1::Version`](Version.md#0x1_Version)


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
