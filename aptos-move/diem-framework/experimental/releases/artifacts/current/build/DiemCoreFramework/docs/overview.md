
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
* <code><a href="DiemBlock.md#0x1_DiemBlock">DiemBlock</a></code>
* <code><a href="DiemConfig.md#0x1_DiemConfig">DiemConfig</a></code>
* <code><a href="DiemTimestamp.md#0x1_DiemTimestamp">DiemTimestamp</a></code>
* <code>DiemTransactionPublishingOption</code>
* <code><a href="DiemVersion.md#0x1_DiemVersion">DiemVersion</a></code>
* <code><a href="DiemVMConfig.md#0x1_DiemVMConfig">DiemVMConfig</a></code>
* <code>TransactionFee</code>
* <code><a href="DiemSystem.md#0x1_DiemSystem">DiemSystem</a></code>
* <code><a href="ValidatorConfig.md#0x1_ValidatorConfig">ValidatorConfig</a></code>
* <code><a href="ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig">ValidatorOperatorConfig</a></code>
* <code>Genesis</code> (Note: not published on-chain)


<a name="@Module_Utility_Libraries_5"></a>

### Module Utility Libraries

* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">Errors</a></code>
* <code>CoreAddresses</code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event">Event</a></code>
* <code>FixedPoint32</code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Hash.md#0x1_Hash">Hash</a></code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS">BCS</a></code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">Option</a></code>
* <code>SlidingNonce</code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">Vector</a></code>
* <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">Signer</a></code>


<a name="@Index_6"></a>

## Index


-  [`0x1::Account`](Account.md#0x1_Account)
-  [`0x1::BCS`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS)
-  [`0x1::Capability`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability)
-  [`0x1::ChainId`](ChainId.md#0x1_ChainId)
-  [`0x1::CoreGenesis`](CoreGenesis.md#0x1_CoreGenesis)
-  [`0x1::DiemBlock`](DiemBlock.md#0x1_DiemBlock)
-  [`0x1::DiemConfig`](DiemConfig.md#0x1_DiemConfig)
-  [`0x1::DiemConsensusConfig`](DiemConsensusConfig.md#0x1_DiemConsensusConfig)
-  [`0x1::DiemSystem`](DiemSystem.md#0x1_DiemSystem)
-  [`0x1::DiemTimestamp`](DiemTimestamp.md#0x1_DiemTimestamp)
-  [`0x1::DiemVMConfig`](DiemVMConfig.md#0x1_DiemVMConfig)
-  [`0x1::DiemVersion`](DiemVersion.md#0x1_DiemVersion)
-  [`0x1::Errors`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors)
-  [`0x1::Event`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Event.md#0x1_Event)
-  [`0x1::GUID`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID)
-  [`0x1::Hash`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Hash.md#0x1_Hash)
-  [`0x1::Option`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option)
-  [`0x1::ParallelExecutionConfig`](ParallelExecutionConfig.md#0x1_ParallelExecutionConfig)
-  [`0x1::Signature`](Signature.md#0x1_Signature)
-  [`0x1::Signer`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer)
-  [`0x1::SystemAddresses`](SystemAddresses.md#0x1_SystemAddresses)
-  [`0x1::ValidatorConfig`](ValidatorConfig.md#0x1_ValidatorConfig)
-  [`0x1::ValidatorOperatorConfig`](ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig)
-  [`0x1::Vector`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector)


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
