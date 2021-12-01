
<a name="@Overview_of_Diem_Transaction_Scripts_0"></a>

# Overview of Diem Transaction Scripts


-  [Introduction](#@Introduction_1)
    -  [Predefined Statuses](#@Predefined_Statuses_2)
    -  [Move Aborts](#@Move_Aborts_3)
        -  [Move Explain](#@Move_Explain_4)
    -  [Specifications](#@Specifications_5)
-  [Transaction Script Summaries](#@Transaction_Script_Summaries_6)
    -  [Account Creation](#@Account_Creation_7)
        -  [Script create_child_vasp_account](#@Script_create_child_vasp_account_8)
        -  [Script create_validator_operator_account](#@Script_create_validator_operator_account_9)
        -  [Script create_validator_account](#@Script_create_validator_account_10)
        -  [Script create_parent_vasp_account](#@Script_create_parent_vasp_account_11)
        -  [Script create_designated_dealer](#@Script_create_designated_dealer_12)
    -  [Account Administration](#@Account_Administration_13)
        -  [Script add_currency_to_account](#@Script_add_currency_to_account_14)
        -  [Script add_recovery_rotation_capability](#@Script_add_recovery_rotation_capability_15)
        -  [Script publish_shared_ed25519_public_key](#@Script_publish_shared_ed25519_public_key_16)
        -  [Script rotate_authentication_key](#@Script_rotate_authentication_key_17)
        -  [Script rotate_authentication_key_with_nonce](#@Script_rotate_authentication_key_with_nonce_18)
        -  [Script rotate_authentication_key_with_nonce_admin](#@Script_rotate_authentication_key_with_nonce_admin_19)
        -  [Script rotate_authentication_key_with_recovery_address](#@Script_rotate_authentication_key_with_recovery_address_20)
        -  [Script rotate_dual_attestation_info](#@Script_rotate_dual_attestation_info_21)
        -  [Script rotate_shared_ed25519_public_key](#@Script_rotate_shared_ed25519_public_key_22)
    -  [Payments](#@Payments_23)
        -  [Script peer_to_peer_with_metadata](#@Script_peer_to_peer_with_metadata_24)
    -  [Validator and Validator Operator Administration](#@Validator_and_Validator_Operator_Administration_25)
        -  [Script add_validator_and_reconfigure](#@Script_add_validator_and_reconfigure_26)
        -  [Script register_validator_config](#@Script_register_validator_config_27)
        -  [Script remove_validator_and_reconfigure](#@Script_remove_validator_and_reconfigure_28)
        -  [Script set_validator_config_and_reconfigure](#@Script_set_validator_config_and_reconfigure_29)
        -  [Script set_validator_operator](#@Script_set_validator_operator_30)
        -  [Script set_validator_operator_with_nonce_admin](#@Script_set_validator_operator_with_nonce_admin_31)
    -  [Treasury and Compliance Operations](#@Treasury_and_Compliance_Operations_32)
        -  [Script preburn](#@Script_preburn_33)
        -  [Script burn_with_amount](#@Script_burn_with_amount_34)
        -  [Script cancel_burn_with_amount](#@Script_cancel_burn_with_amount_35)
        -  [Script burn_txn_fees](#@Script_burn_txn_fees_36)
        -  [Script tiered_mint](#@Script_tiered_mint_37)
        -  [Script freeze_account](#@Script_freeze_account_38)
        -  [Script unfreeze_account](#@Script_unfreeze_account_39)
        -  [Script update_dual_attestation_limit](#@Script_update_dual_attestation_limit_40)
        -  [Script update_exchange_rate](#@Script_update_exchange_rate_41)
        -  [Script update_minting_ability](#@Script_update_minting_ability_42)
    -  [System Administration](#@System_Administration_43)
        -  [Script update_diem_version](#@Script_update_diem_version_44)
-  [Transaction Scripts](#@Transaction_Scripts_45)
    -  [Account Creation](#@Account_Creation_46)
    -  [Account Administration](#@Account_Administration_47)
    -  [Payments](#@Payments_48)
    -  [Validator and Validator Operator Administration](#@Validator_and_Validator_Operator_Administration_49)
    -  [Treasury and Compliance Operations](#@Treasury_and_Compliance_Operations_50)
    -  [System Administration](#@System_Administration_51)
    -  [Index](#@Index_52)



<a name="@Introduction_1"></a>

## Introduction


On-chain state is updated via the execution of transaction scripts sent from
accounts that exist on-chain. This page documents each allowed transaction
script on Diem, and the common state changes that can be performed to the
blockchain via these transaction scripts along with their arguments and common
error conditions.

The execution of a transaction script can result in a number of different error
conditions and statuses being returned for each transaction that is committed
on-chain. These statuses and errors can be categorized into two buckets:
* [Predefined statuses](#predefined-statuses): are specific statuses that are returned from the VM, e.g., <code>OutOfGas</code>, or <code>Executed</code>; and
* [Move Abort errors](#move-aborts): are errors that are raised from the Move modules and/or scripts published on-chain.

There are also a number of statuses that can be returned at the time of
submission of the transaction to the system through JSON-RPC, these are detailed in the
[JSON-RPC specification](https://github.com/diem/diem/blob/main/json-rpc/docs/method_submit.md#errors).


<a name="@Predefined_Statuses_2"></a>

### Predefined Statuses


The predefined set of runtime statuses that can be returned to the user as a
result of executing any transaction script is given by the following table:

| Name                     | Description                                                                                              |
| ----                     | ---                                                                                                      |
| <code>Executed</code>               | The transaction was executed successfully.                                                               |
| <code>OutOfGas</code>               | The transaction ran out of gas during execution.                                                         |
| <code>MiscellaneousError</code>     | The transaction was malformed, e.g., an argument was not in BCS format. Possible, but unlikely to occur. |
| <code>ExecutionFailure{ ...}</code> | The transaction encountered an uncaught error. Possible, but unlikely to occur.                          |

**This set of statuses is considered stable**, and they should not be expected to
change. Any changes will be publicized and an upgrade process will be outlined
if/when these statuses or their meanings are updated.


<a name="@Move_Aborts_3"></a>

### Move Aborts


Each Move abort error status consists of two pieces of data:
* The Move <code>location</code> where the abort was raised. This can be either from within a <code>Script</code> or from within a specific <code>Module</code>.
* The <code>abort_code</code> that was raised.

The <code>abort_code</code> is a <code>u64</code> that is constructed from two values:
1. The **error category** which is encoded in the lower 8 bits of the code. Error categories are
declared in the <code><a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">Errors</a></code> module and are globally unique across the Diem framework. There is a limited
fixed set of predefined categories, and the framework is guaranteed to use these consistently.
2. The **error reason** which is encoded in the remaining 56 bits of the code. The reason is a unique
number relative to the module which raised the error and can be used to obtain more information about
the error at hand. It should primarily be used for diagnosis purposes. Error reasons may change over time as the
framework evolves.

The most common set of Move abort errors that can be returned depend on the transaction script
and they are therefore detailed in the documentation for each transaction
script. Each abort condition is broken down into its category, reason, and a
description of the error in the context of the particular transaction script
e.g.,

| Error Category           | Error Reason                                | Description                                               |
| ----------------         | --------------                              | -------------                                             |
| <code>Errors::NOT_PUBLISHED</code>  | <code>DiemAccount::EPAYER_DOESNT_HOLD_CURRENCY</code> | <code>payer</code> doesn't hold a balance in <code>Currency</code>.             |
| <code>Errors::LIMIT_EXCEEDED</code> | <code>DiemAccount::EINSUFFICIENT_BALANCE</code>       | <code>amount</code> is greater than <code>payer</code>'s balance in <code>Currency</code>. |

For each of these tables, the **error categories should be considered stable**;
any changes to these categories will be be well-publicized in advance. On the
other hand, the **error reasons should be considered only semi-stable**; changes
to these may occur without notice, but changes are not expected to be common.


<a name="@Move_Explain_4"></a>

#### Move Explain


The abort conditions detailed in each transaction script are not meant to
be complete, but the list of error categories are. Additionally, any abort conditions
raised will have a human readable explanation attached to it (if possible) in the
[response](https://github.com/diem/diem/blob/main/json-rpc/docs/type_transaction.md#type-moveabortexplanation)
from a
[JSON-RPC query for a committed transaction](https://github.com/diem/diem/blob/main/json-rpc/json-rpc-spec.md).
These explanations are based off of the human-understandable explanations provided by the
[Move Explain](https://github.com/diem/diem/tree/main/language/tools/move-explain)
tool which can also be called on the command-line.


<a name="@Specifications_5"></a>

### Specifications


Transaction scripts come together with formal specifications. See [this document](./spec_documentation.md)
for a discussion of specifications and pointers to further documentation.

---

<a name="@Transaction_Script_Summaries_6"></a>

## Transaction Script Summaries

---

The set of transaction scripts that are allowed to be sent to the blockchain
can be categorized into six different buckets:
* [Account Creation](#account-creation)
* [Account Administration](#account-administration)
* [Payments](#payments)
* [Validator and Validator Operator Administration](#validator-and-validator-operator-administration)
* [Treasury and Compliance Operations](#treasury-and-compliance-operations)
* [System Administration](#system-administration)

This section contains a brief summary for each along with a link to the script's
detailed documentation. The entire list of detailed documentation for each
transaction script categorized in the same manner as here can be found in the
[transaction scripts](#transaction-scripts) section in this document.


<a name="@Account_Creation_7"></a>

### Account Creation


---

<a name="@Script_create_child_vasp_account_8"></a>

#### Script create_child_vasp_account


Creates a Child VASP account with its parent being the sending account of the transaction.
The sender of the transaction must be a Parent VASP account.

Script documentation: <code>AccountCreationScripts::create_child_vasp_account</code>

---

<a name="@Script_create_validator_operator_account_9"></a>

#### Script create_validator_operator_account


Creates a Validator Operator account. This transaction can only be sent by the Diem
Root account.

Script documentation: <code>AccountCreationScripts::create_validator_operator_account</code>

---

<a name="@Script_create_validator_account_10"></a>

#### Script create_validator_account


Creates a Validator account. This transaction can only be sent by the Diem
Root account.

Script documentation: <code>AccountCreationScripts::create_validator_account</code>

---

<a name="@Script_create_parent_vasp_account_11"></a>

#### Script create_parent_vasp_account


Creates a Parent VASP account with the specified human name. Must be called by the Treasury Compliance account.

Script documentation: <code>AccountCreationScripts::create_parent_vasp_account</code>


---

<a name="@Script_create_designated_dealer_12"></a>

#### Script create_designated_dealer


Creates a Designated Dealer account with the provided information, and initializes it with
default mint tiers. The transaction can only be sent by the Treasury Compliance account.

Script documentation: <code>AccountCreationScripts::create_designated_dealer</code>



<a name="@Account_Administration_13"></a>

### Account Administration


---

<a name="@Script_add_currency_to_account_14"></a>

#### Script add_currency_to_account


Adds a zero <code>Currency</code> balance to the sending <code>account</code>. This will enable <code>account</code> to
send, receive, and hold <code>Diem::Diem&lt;Currency&gt;</code> coins. This transaction can be
successfully sent by any account that is allowed to hold balances
(e.g., VASP, Designated Dealer).

Script documentation: <code>AccountAdministrationScripts::add_currency_to_account</code>


---

<a name="@Script_add_recovery_rotation_capability_15"></a>

#### Script add_recovery_rotation_capability


Stores the sending accounts ability to rotate its authentication key with a designated recovery
account. Both the sending and recovery accounts need to belong to the same VASP and
both be VASP accounts. After this transaction both the sending account and the
specified recovery account can rotate the sender account's authentication key.

Script documentation: <code>AccountAdministrationScripts::add_recovery_rotation_capability</code>


---

<a name="@Script_publish_shared_ed25519_public_key_16"></a>

#### Script publish_shared_ed25519_public_key


Rotates the authentication key of the sending account to the
newly-specified public key and publishes a new shared authentication key
under the sender's account. Any account can send this transaction.

Script documentation: <code>AccountAdministrationScripts::publish_shared_ed25519_public_key</code>


---

<a name="@Script_rotate_authentication_key_17"></a>

#### Script rotate_authentication_key


Rotates the transaction sender's authentication key to the supplied new authentication key. May
be sent by any account.

Script documentation: <code>AccountAdministrationScripts::rotate_authentication_key</code>


---

<a name="@Script_rotate_authentication_key_with_nonce_18"></a>

#### Script rotate_authentication_key_with_nonce


Rotates the sender's authentication key to the supplied new authentication key. May be sent by
any account that has a sliding nonce resource published under it (usually this is Treasury
Compliance or Diem Root accounts).

Script documentation: <code>AccountAdministrationScripts::rotate_authentication_key_with_nonce</code>


---

<a name="@Script_rotate_authentication_key_with_nonce_admin_19"></a>

#### Script rotate_authentication_key_with_nonce_admin


Rotates the specified account's authentication key to the supplied new authentication key. May
only be sent by the Diem Root account as a write set transaction.


Script documentation: <code>AccountAdministrationScripts::rotate_authentication_key_with_nonce_admin</code>


---

<a name="@Script_rotate_authentication_key_with_recovery_address_20"></a>

#### Script rotate_authentication_key_with_recovery_address


Rotates the authentication key of a specified account that is part of a recovery address to a
new authentication key. Only used for accounts that are part of a recovery address (see
<code>AccountAdministrationScripts::add_recovery_rotation_capability</code> for account restrictions).

Script documentation: <code>AccountAdministrationScripts::rotate_authentication_key_with_recovery_address</code>


---

<a name="@Script_rotate_dual_attestation_info_21"></a>

#### Script rotate_dual_attestation_info


Updates the url used for off-chain communication, and the public key used to verify dual
attestation on-chain. Transaction can be sent by any account that has dual attestation
information published under it. In practice the only such accounts are Designated Dealers and
Parent VASPs.

Script documentation: <code>AccountAdministrationScripts::rotate_dual_attestation_info</code>


---

<a name="@Script_rotate_shared_ed25519_public_key_22"></a>

#### Script rotate_shared_ed25519_public_key


Rotates the authentication key in a <code>SharedEd25519PublicKey</code>. This transaction can be sent by
any account that has previously published a shared ed25519 public key using
<code>AccountAdministrationScripts::publish_shared_ed25519_public_key</code>.

Script documentation: <code>AccountAdministrationScripts::rotate_shared_ed25519_public_key</code>


<a name="@Payments_23"></a>

### Payments


---

<a name="@Script_peer_to_peer_with_metadata_24"></a>

#### Script peer_to_peer_with_metadata


Transfers a given number of coins in a specified currency from one account to another.
Transfers over a specified amount defined on-chain that are between two different VASPs, or
other accounts that have opted-in will be subject to on-chain checks to ensure the receiver has
agreed to receive the coins.  This transaction can be sent by any account that can hold a
balance, and to any account that can hold a balance. Both accounts must hold balances in the
currency being transacted.

Script documentation: <code>PaymentScripts::peer_to_peer_with_metadata</code>



<a name="@Validator_and_Validator_Operator_Administration_25"></a>

### Validator and Validator Operator Administration


---

<a name="@Script_add_validator_and_reconfigure_26"></a>

#### Script add_validator_and_reconfigure


Adds a validator account to the validator set, and triggers a
reconfiguration of the system to admit the account to the validator set for the system. This
transaction can only be successfully called by the Diem Root account.

Script documentation: <code>ValidatorAdministrationScripts::add_validator_and_reconfigure</code>


---

<a name="@Script_register_validator_config_27"></a>

#### Script register_validator_config


Updates a validator's configuration. This does not reconfigure the system and will not update
the configuration in the validator set that is seen by other validators in the network. Can
only be successfully sent by a Validator Operator account that is already registered with a
validator.

Script documentation: <code>ValidatorAdministrationScripts::register_validator_config</code>


---

<a name="@Script_remove_validator_and_reconfigure_28"></a>

#### Script remove_validator_and_reconfigure


This script removes a validator account from the validator set, and triggers a reconfiguration
of the system to remove the validator from the system. This transaction can only be
successfully called by the Diem Root account.

Script documentation: <code>ValidatorAdministrationScripts::remove_validator_and_reconfigure</code>


---

<a name="@Script_set_validator_config_and_reconfigure_29"></a>

#### Script set_validator_config_and_reconfigure


Updates a validator's configuration, and triggers a reconfiguration of the system to update the
validator set with this new validator configuration.  Can only be successfully sent by a
Validator Operator account that is already registered with a validator.

Script documentation: <code>ValidatorAdministrationScripts::set_validator_config_and_reconfigure</code>


---

<a name="@Script_set_validator_operator_30"></a>

#### Script set_validator_operator


Sets the validator operator for a validator in the validator's configuration resource "locally"
and does not reconfigure the system. Changes from this transaction will not picked up by the
system until a reconfiguration of the system is triggered. May only be sent by an account with
Validator role.

Script documentation: <code>ValidatorAdministrationScripts::set_validator_operator</code>


---

<a name="@Script_set_validator_operator_with_nonce_admin_31"></a>

#### Script set_validator_operator_with_nonce_admin


Sets the validator operator for a validator in the validator's configuration resource "locally"
and does not reconfigure the system. Changes from this transaction will not picked up by the
system until a reconfiguration of the system is triggered. May only be sent by the Diem Root
account as a write set transaction.

Script documentation: <code>ValidatorAdministrationScripts::set_validator_operator_with_nonce_admin</code>



<a name="@Treasury_and_Compliance_Operations_32"></a>

### Treasury and Compliance Operations


---

<a name="@Script_preburn_33"></a>

#### Script preburn


Moves a specified number of coins in a given currency from the account's
balance to its preburn area after which the coins may be burned. This
transaction may be sent by any account that holds a balance and preburn area
in the specified currency.

Script documentation: <code>TreasuryComplianceScripts::preburn</code>


---

<a name="@Script_burn_with_amount_34"></a>

#### Script burn_with_amount


Burns the coins held in a preburn resource in the preburn queue at the
specified preburn address, which are equal to the <code>amount</code> specified in the
transaction. Finds the first relevant outstanding preburn request with
matching amount and removes the contained coins from the system. The sending
account must be the Treasury Compliance account.
The account that holds the preburn queue resource will normally be a Designated
Dealer, but there are no enforced requirements that it be one.

Script documentation: <code>TreasuryComplianceScripts::burn</code>


---

<a name="@Script_cancel_burn_with_amount_35"></a>

#### Script cancel_burn_with_amount


Cancels and returns the coins held in the preburn area under
<code>preburn_address</code>, which are equal to the <code>amount</code> specified in the transaction. Finds the first preburn
resource with the matching amount and returns the funds to the <code>preburn_address</code>'s balance.
Can only be successfully sent by an account with Treasury Compliance role.

Script documentation: <code>TreasuryComplianceScripts::cancel_burn</code>


---

<a name="@Script_burn_txn_fees_36"></a>

#### Script burn_txn_fees


Burns the transaction fees collected in the <code>CoinType</code> currency so that the
Diem association may reclaim the backing coins off-chain. May only be sent
by the Treasury Compliance account.

Script documentation: <code>TreasuryComplianceScripts::burn_txn_fees</code>


---

<a name="@Script_tiered_mint_37"></a>

#### Script tiered_mint


Mints a specified number of coins in a currency to a Designated Dealer. The sending account
must be the Treasury Compliance account, and coins can only be minted to a Designated Dealer
account.

Script documentation: <code>TreasuryComplianceScripts::tiered_mint</code>


---

<a name="@Script_freeze_account_38"></a>

#### Script freeze_account


Freezes the account at <code><b>address</b></code>. The sending account of this transaction
must be the Treasury Compliance account. The account being frozen cannot be
the Diem Root or Treasury Compliance account. After the successful
execution of this transaction no transactions may be sent from the frozen
account, and the frozen account may not send or receive coins.

Script documentation: <code>TreasuryComplianceScripts::freeze_account</code>


---

<a name="@Script_unfreeze_account_39"></a>

#### Script unfreeze_account


Unfreezes the account at <code><b>address</b></code>. The sending account of this transaction must be the
Treasury Compliance account. After the successful execution of this transaction transactions
may be sent from the previously frozen account, and coins may be sent and received.

Script documentation: <code>TreasuryComplianceScripts::unfreeze_account</code>


---

<a name="@Script_update_dual_attestation_limit_40"></a>

#### Script update_dual_attestation_limit


Update the dual attestation limit on-chain. Defined in terms of micro-XDX.  The transaction can
only be sent by the Treasury Compliance account.  After this transaction all inter-VASP
payments over this limit must be checked for dual attestation.

Script documentation: <code>TreasuryComplianceScripts::update_dual_attestation_limit</code>


---

<a name="@Script_update_exchange_rate_41"></a>

#### Script update_exchange_rate


Update the rough on-chain exchange rate between a specified currency and XDX (as a conversion
to micro-XDX). The transaction can only be sent by the Treasury Compliance account. After this
transaction the updated exchange rate will be used for normalization of gas prices, and for
dual attestation checking.

Script documentation: <code>TreasuryComplianceScripts::update_exchange_rate</code>


---

<a name="@Script_update_minting_ability_42"></a>

#### Script update_minting_ability


Script to allow or disallow minting of new coins in a specified currency.  This transaction can
only be sent by the Treasury Compliance account.  Turning minting off for a currency will have
no effect on coins already in circulation, and coins may still be removed from the system.

Script documentation: <code>TreasuryComplianceScripts::update_minting_ability</code>



<a name="@System_Administration_43"></a>

### System Administration


---

<a name="@Script_update_diem_version_44"></a>

#### Script update_diem_version


Updates the Diem major version that is stored on-chain and is used by the VM.  This
transaction can only be sent from the Diem Root account.

Script documentation: <code>SystemAdministrationScripts::update_diem_version</code>



---

<a name="@Transaction_Scripts_45"></a>

## Transaction Scripts

---


<a name="@Account_Creation_46"></a>

### Account Creation


> undefined move-include `AccountCreationScripts`


---

<a name="@Account_Administration_47"></a>

### Account Administration


> undefined move-include `AccountAdministrationScripts`


---

<a name="@Payments_48"></a>

### Payments


> undefined move-include `PaymentScripts`


---

<a name="@Validator_and_Validator_Operator_Administration_49"></a>

### Validator and Validator Operator Administration


> undefined move-include `ValidatorAdministrationScripts`


---

<a name="@Treasury_and_Compliance_Operations_50"></a>

### Treasury and Compliance Operations


> undefined move-include `TreasuryComplianceScripts`


---

<a name="@System_Administration_51"></a>

### System Administration


> undefined move-include `SystemAdministrationScripts`



<a name="@Index_52"></a>

### Index


-  [`0x1::Account`](Account.md#0x1_Account)
-  [`0x1::BCS`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/BCS.md#0x1_BCS)
-  [`0x1::Capability`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability)
-  [`0x1::ChainId`](ChainId.md#0x1_ChainId)
-  [`0x1::CoreGenesis`](CoreGenesis.md#0x1_CoreGenesis)
-  [`0x1::DiemTimestamp`](DiemTimestamp.md#0x1_DiemTimestamp)
-  [`0x1::DiemVersion`](DiemVersion.md#0x1_DiemVersion)
-  [`0x1::Errors`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors)
-  [`0x1::Hash`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Hash.md#0x1_Hash)
-  [`0x1::Signer`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer)
-  [`0x1::SystemAddresses`](SystemAddresses.md#0x1_SystemAddresses)
-  [`0x1::Vector`](../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector)


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
