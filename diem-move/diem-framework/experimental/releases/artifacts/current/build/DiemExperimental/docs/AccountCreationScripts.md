
<a name="0x1_AccountCreationScripts"></a>

# Module `0x1::AccountCreationScripts`



-  [Function `create_validator_operator_account`](#0x1_AccountCreationScripts_create_validator_operator_account)
    -  [Summary](#@Summary_0)
    -  [Technical Description](#@Technical_Description_1)
    -  [Events](#@Events_2)
    -  [Parameters](#@Parameters_3)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_4)
    -  [Related Scripts](#@Related_Scripts_5)
-  [Function `create_validator_account`](#0x1_AccountCreationScripts_create_validator_account)
    -  [Summary](#@Summary_6)
    -  [Technical Description](#@Technical_Description_7)
    -  [Events](#@Events_8)
    -  [Parameters](#@Parameters_9)
    -  [Common Abort Conditions](#@Common_Abort_Conditions_10)
    -  [Related Scripts](#@Related_Scripts_11)
-  [Function `create_account`](#0x1_AccountCreationScripts_create_account)


<pre><code><b>use</b> <a href="ExperimentalAccount.md#0x1_ExperimentalAccount">0x1::ExperimentalAccount</a>;
</code></pre>



<a name="0x1_AccountCreationScripts_create_validator_operator_account"></a>

## Function `create_validator_operator_account`


<a name="@Summary_0"></a>

### Summary

Creates a Validator Operator account. This transaction can only be sent by the Diem
Root account.


<a name="@Technical_Description_1"></a>

### Technical Description

Creates an account with a Validator Operator role at <code>new_account_address</code>, with authentication key
<code>auth_key_prefix</code> | <code>new_account_address</code>. It publishes a
<code><a href="ValidatorOperatorConfig.md#0x1_ValidatorOperatorConfig_ValidatorOperatorConfig">ValidatorOperatorConfig::ValidatorOperatorConfig</a></code> resource with the specified <code>human_name</code>.
This script does not assign the validator operator to any validator accounts but only creates the account.
Authentication key prefixes, and how to construct them from an ed25519 public key are described
[here](https://developers.diem.com/docs/core/accounts/#addresses-authentication-keys-and-cryptographic-keys).


<a name="@Events_2"></a>

### Events

Successful execution will emit:
* A <code>ExperimentalAccount::CreateAccountEvent</code> with the <code>created</code> field being <code>new_account_address</code>,
and the <code>rold_id</code> field being <code><a href="Roles.md#0x1_Roles_VALIDATOR_OPERATOR_ROLE_ID">Roles::VALIDATOR_OPERATOR_ROLE_ID</a></code>. This is emitted on the
<code>ExperimentalAccount::AccountOperationsCapability</code> <code>creation_events</code> handle.


<a name="@Parameters_3"></a>

### Parameters

| Name                  | Type         | Description                                                                              |
| ------                | ------       | -------------                                                                            |
| <code>dr_account</code>          | <code>signer</code>     | The signer of the sending account of this transaction. Must be the Diem Root signer.     |
| <code>sliding_nonce</code>       | <code>u64</code>        | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction.               |
| <code>new_account_address</code> | <code><b>address</b></code>    | Address of the to-be-created Validator account.                                          |
| <code>auth_key_prefix</code>     | <code>vector&lt;u8&gt;</code> | The authentication key prefix that will be used initially for the newly created account. |
| <code>human_name</code>          | <code>vector&lt;u8&gt;</code> | ASCII-encoded human name for the validator.                                              |


<a name="@Common_Abort_Conditions_4"></a>

### Common Abort Conditions

| Error Category              | Error Reason                            | Description                                                                                |
| ----------------            | --------------                          | -------------                                                                              |
| <code>Errors::NOT_PUBLISHED</code>     | <code>SlidingNonce::ESLIDING_NONCE</code>          | A <code>SlidingNonce</code> resource is not published under <code>dr_account</code>.                             |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_TOO_OLD</code>          | The <code>sliding_nonce</code> is too old and it's impossible to determine if it's duplicated or not. |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_TOO_NEW</code>          | The <code>sliding_nonce</code> is too far in the future.                                              |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_ALREADY_RECORDED</code> | The <code>sliding_nonce</code> has been previously recorded.                                          |
| <code>Errors::REQUIRES_ADDRESS</code>  | <code>CoreAddresses::EDIEM_ROOT</code>            | The sending account is not the Diem Root account.                                         |
| <code>Errors::REQUIRES_ROLE</code>     | <code><a href="Roles.md#0x1_Roles_EDIEM_ROOT">Roles::EDIEM_ROOT</a></code>                    | The sending account is not the Diem Root account.                                         |
| <code>Errors::ALREADY_PUBLISHED</code> | <code><a href="Roles.md#0x1_Roles_EROLE_ID">Roles::EROLE_ID</a></code>                       | The <code>new_account_address</code> address is already taken.                                        |


<a name="@Related_Scripts_5"></a>

### Related Scripts

* <code><a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_account">AccountCreationScripts::create_validator_account</a></code>
* <code>ValidatorAdministrationScripts::add_validator_and_reconfigure</code>
* <code>ValidatorAdministrationScripts::register_validator_config</code>
* <code>ValidatorAdministrationScripts::remove_validator_and_reconfigure</code>
* <code>ValidatorAdministrationScripts::set_validator_operator</code>
* <code>ValidatorAdministrationScripts::set_validator_operator_with_nonce_admin</code>
* <code>ValidatorAdministrationScripts::set_validator_config_and_reconfigure</code>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_operator_account">create_validator_operator_account</a>(dr_account: signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_operator_account">create_validator_operator_account</a>(
    dr_account: signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;
) {
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_operator_account">ExperimentalAccount::create_validator_operator_account</a>(
        &dr_account,
        new_account_address,
        auth_key_prefix,
        human_name,
    );
}
</code></pre>



</details>

<a name="0x1_AccountCreationScripts_create_validator_account"></a>

## Function `create_validator_account`


<a name="@Summary_6"></a>

### Summary

Creates a Validator account. This transaction can only be sent by the Diem
Root account.


<a name="@Technical_Description_7"></a>

### Technical Description

Creates an account with a Validator role at <code>new_account_address</code>, with authentication key
<code>auth_key_prefix</code> | <code>new_account_address</code>. It publishes a
<code><a href="ValidatorConfig.md#0x1_ValidatorConfig_ValidatorConfig">ValidatorConfig::ValidatorConfig</a></code> resource with empty <code>config</code>, and
<code>operator_account</code> fields. The <code>human_name</code> field of the
<code><a href="ValidatorConfig.md#0x1_ValidatorConfig_ValidatorConfig">ValidatorConfig::ValidatorConfig</a></code> is set to the passed in <code>human_name</code>.
This script does not add the validator to the validator set or the system,
but only creates the account.
Authentication keys, prefixes, and how to construct them from an ed25519 public key are described
[here](https://developers.diem.com/docs/core/accounts/#addresses-authentication-keys-and-cryptographic-keys).


<a name="@Events_8"></a>

### Events

Successful execution will emit:
* A <code>ExperimentalAccount::CreateAccountEvent</code> with the <code>created</code> field being <code>new_account_address</code>,
and the <code>rold_id</code> field being <code><a href="Roles.md#0x1_Roles_VALIDATOR_ROLE_ID">Roles::VALIDATOR_ROLE_ID</a></code>. This is emitted on the
<code>ExperimentalAccount::AccountOperationsCapability</code> <code>creation_events</code> handle.


<a name="@Parameters_9"></a>

### Parameters

| Name                  | Type         | Description                                                                              |
| ------                | ------       | -------------                                                                            |
| <code>dr_account</code>          | <code>signer</code>     | The signer of the sending account of this transaction. Must be the Diem Root signer.     |
| <code>sliding_nonce</code>       | <code>u64</code>        | The <code>sliding_nonce</code> (see: <code>SlidingNonce</code>) to be used for this transaction.               |
| <code>new_account_address</code> | <code><b>address</b></code>    | Address of the to-be-created Validator account.                                          |
| <code>auth_key_prefix</code>     | <code>vector&lt;u8&gt;</code> | The authentication key prefix that will be used initially for the newly created account. |
| <code>human_name</code>          | <code>vector&lt;u8&gt;</code> | ASCII-encoded human name for the validator.                                              |


<a name="@Common_Abort_Conditions_10"></a>

### Common Abort Conditions

| Error Category              | Error Reason                            | Description                                                                                |
| ----------------            | --------------                          | -------------                                                                              |
| <code>Errors::NOT_PUBLISHED</code>     | <code>SlidingNonce::ESLIDING_NONCE</code>          | A <code>SlidingNonce</code> resource is not published under <code>dr_account</code>.                             |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_TOO_OLD</code>          | The <code>sliding_nonce</code> is too old and it's impossible to determine if it's duplicated or not. |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_TOO_NEW</code>          | The <code>sliding_nonce</code> is too far in the future.                                              |
| <code>Errors::INVALID_ARGUMENT</code>  | <code>SlidingNonce::ENONCE_ALREADY_RECORDED</code> | The <code>sliding_nonce</code> has been previously recorded.                                          |
| <code>Errors::REQUIRES_ADDRESS</code>  | <code>CoreAddresses::EDIEM_ROOT</code>            | The sending account is not the Diem Root account.                                         |
| <code>Errors::REQUIRES_ROLE</code>     | <code><a href="Roles.md#0x1_Roles_EDIEM_ROOT">Roles::EDIEM_ROOT</a></code>                    | The sending account is not the Diem Root account.                                         |
| <code>Errors::ALREADY_PUBLISHED</code> | <code><a href="Roles.md#0x1_Roles_EROLE_ID">Roles::EROLE_ID</a></code>                       | The <code>new_account_address</code> address is already taken.                                        |


<a name="@Related_Scripts_11"></a>

### Related Scripts

* <code><a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_operator_account">AccountCreationScripts::create_validator_operator_account</a></code>
* <code>ValidatorAdministrationScripts::add_validator_and_reconfigure</code>
* <code>ValidatorAdministrationScripts::register_validator_config</code>
* <code>ValidatorAdministrationScripts::remove_validator_and_reconfigure</code>
* <code>ValidatorAdministrationScripts::set_validator_operator</code>
* <code>ValidatorAdministrationScripts::set_validator_operator_with_nonce_admin</code>
* <code>ValidatorAdministrationScripts::set_validator_config_and_reconfigure</code>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_account">create_validator_account</a>(dr_account: signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;, human_name: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_validator_account">create_validator_account</a>(
    dr_account: signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
    human_name: vector&lt;u8&gt;,
) {
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_validator_account">ExperimentalAccount::create_validator_account</a>(
        &dr_account,
        new_account_address,
        auth_key_prefix,
        human_name,
    );
  }
</code></pre>



</details>

<a name="0x1_AccountCreationScripts_create_account"></a>

## Function `create_account`



<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_account">create_account</a>(_account: signer, new_account_address: <b>address</b>, auth_key_prefix: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AccountCreationScripts.md#0x1_AccountCreationScripts_create_account">create_account</a>(
    _account: signer,
    new_account_address: <b>address</b>,
    auth_key_prefix: vector&lt;u8&gt;,
) {
    <a href="ExperimentalAccount.md#0x1_ExperimentalAccount_create_account">ExperimentalAccount::create_account</a>(
        new_account_address,
        auth_key_prefix,
    );
}
</code></pre>



</details>
