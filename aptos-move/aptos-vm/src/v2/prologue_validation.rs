// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implementation of validation logic executed during user transaction prologue.

use crate::{
    aptos_vm::{serialized_signer, SerializedSigners},
    errors::convert_prologue_error,
    gas::check_gas,
    move_vm_ext::AptosMoveResolver,
    system_module_names::{
        ACCOUNT_ABSTRACTION_MODULE, AUTHENTICATE, MULTISIG_ACCOUNT_MODULE,
        VALIDATE_MULTISIG_TRANSACTION,
    },
    transaction_validation::{
        common_prologue_serialized_args, multisig_prologue_args, APTOS_TRANSACTION_VALIDATION,
    },
    v2::session::{AptosSession, Session, UserTransactionSession},
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    function_info::FunctionInfo,
    move_utils::as_move_value::AsMoveValue,
    transaction::{
        authenticator::{AbstractionAuthData, AuthenticationProof},
        ReplayProtector, TransactionPayload,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveTypeLayout, MoveValue},
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{logging::expect_no_verification_errors, Loader};
use move_vm_types::gas::GasMeter;

macro_rules! feature_under_gating {
    ($msg:expr) => {
        VMStatus::error(StatusCode::FEATURE_UNDER_GATING, Some($msg.to_string()))
    };
}

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    /// Validates user transaction by running checks and the prologue. All transactions that fail
    /// validation need to be discarded.
    pub(crate) fn execute_user_transaction_prologue(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<(), VMStatus> {
        self.validate_no_duplicate_signers()?;

        self.validate_keyless_authenticators()?;
        let serialized_signers = self.validate_aa_dispatchable_authentication(gas_meter)?;

        self.validate_transaction_payload()?;
        self.validate_orderless_txn_feature_gating()?;
        self.validate_gas()?;

        // The prologues MUST be run AFTER any validation. Otherwise, you may run prologue and hit
        // SEQUENCE_NUMBER_TOO_NEW if there is more than one transaction from the same sender and
        // end up skipping validation.
        self.run_common_prologue(&serialized_signers)?;
        self.run_multisig_prologue()?;

        // All checks passed.
        assert!(self.serialized_signers.is_none());
        self.serialized_signers = Some(serialized_signers);

        Ok(())
    }
}

// Private interfaces.
impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn validate_no_duplicate_signers(&self) -> Result<(), VMStatus> {
        if self.txn.contains_duplicate_signers() {
            return Err(VMStatus::error(
                StatusCode::SIGNERS_CONTAIN_DUPLICATES,
                None,
            ));
        }
        Ok(())
    }

    fn validate_keyless_authenticators(&self) -> Result<(), VMStatus> {
        // If there are any keyless authenticators, validate them all.
        let keyless_authenticators = aptos_types::keyless::get_authenticators(self.txn)
            .map_err(|_| VMStatus::error(StatusCode::INVALID_SIGNATURE, None))?;
        if !keyless_authenticators.is_empty() && !self.is_simulation {
            // TODO(aptos-vm-v2):
            //   1. Fix keyless validation: it requires module storage, and we have loader. Looks
            //      like it is better to wrap the V1 code in a dispatch.
            //   2. Keyless computes some key, which we can do lazily on use instead of when the VM
            //      is created, though ideally we cache it in environment.
            //   3. Group accesses are not efficient here, provide access to group interfaces
            //      directly.
            // let _pvk = keyless_validation::get_groth16_vk_onchain(
            //     self.session.data_view,
            //     self.session.loader,
            // )
            // .ok()
            // .and_then(|vk| vk.try_into().ok());
            // keyless_validation::validate_authenticators(
            //     &pvk,
            //     &keyless_authenticators,
            //     self.session.features,
            //     self.session.data_view,
            //     self.session.loader,
            // )?;
        }
        Ok(())
    }

    /// Runs AA authentication (if feature is enabled) returning senders and fee payer for the
    /// transaction.
    fn validate_aa_dispatchable_authentication(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<SerializedSigners, VMStatus> {
        let senders = self.txn_metadata.senders();
        let proofs = self.txn_metadata.authentication_proofs();

        let fee_payer_signer = match self.txn_metadata.fee_payer {
            None => None,
            Some(fee_payer) => Some(match &self.txn_metadata.fee_payer_authentication_proof {
                Some(AuthenticationProof::Key(_)) | Some(AuthenticationProof::None) | None => {
                    serialized_signer(&fee_payer)
                },
                Some(AuthenticationProof::Abstraction {
                    function_info,
                    auth_data,
                }) => self.session.dispatchable_authenticate(
                    gas_meter,
                    fee_payer,
                    function_info.clone(),
                    auth_data,
                )?,
            }),
        };

        let sender_signers = itertools::zip_eq(senders, proofs)
            .map(|(sender, proof)| match proof {
                AuthenticationProof::None | AuthenticationProof::Key(_) => {
                    Ok(serialized_signer(&sender))
                },
                AuthenticationProof::Abstraction {
                    function_info,
                    auth_data,
                } => self.session.dispatchable_authenticate(
                    gas_meter,
                    sender,
                    function_info.clone(),
                    auth_data,
                ),
            })
            .collect::<Result<_, _>>()?;

        Ok(SerializedSigners::new(sender_signers, fee_payer_signer))
    }

    /// Checks that transaction's payload and its executable are well-formed. In particular, the
    /// following invariants are preserved:
    ///   - Payload V2 can only be used if the corresponding feature flag is enabled.
    ///   - Non-multisig transactions cannot have empty payload.
    ///   - Multisig transactions cannot have script payload (not supported).
    fn validate_transaction_payload(&self) -> Result<(), VMStatus> {
        if matches!(self.txn.payload(), TransactionPayload::Payload(_))
            && !self.session.features.is_transaction_payload_v2_enabled()
        {
            return Err(feature_under_gating!(
                "Transaction payload V2 cannot be used because the feature is not enabled"
            ));
        }

        if self.executable.is_empty() && !self.txn_extra_config.is_multisig() {
            return Err(VMStatus::error(
                StatusCode::EMPTY_PAYLOAD_PROVIDED,
                Some("Empty provided for a non-multisig transaction".to_string()),
            ));
        }

        if self.executable.is_script() && self.txn_extra_config.is_multisig() {
            return Err(feature_under_gating!(
                "Script payload is not supported for multisig transactions"
            ));
        }

        Ok(())
    }

    /// If orderless transactions are not enabled and the nonce replay protector is used, returns
    /// an error.
    fn validate_orderless_txn_feature_gating(&self) -> Result<(), VMStatus> {
        if !self.session.features.is_orderless_txns_enabled() {
            if let ReplayProtector::Nonce(_) = self.txn.replay_protector() {
                return Err(feature_under_gating!(
                    "Orderless transactions cannot be used because the feature is not enabled"
                ));
            }
        }
        Ok(())
    }

    fn validate_gas(&self) -> Result<(), VMStatus> {
        check_gas(
            self.session.gas_params,
            self.gas_feature_version(),
            self.session.data_view,
            &self.txn_metadata,
            self.features(),
            self.is_approved_gov_script,
            self.session.log_context,
        )
    }

    fn run_common_prologue(
        &mut self,
        serialized_signers: &SerializedSigners,
    ) -> Result<(), VMStatus> {
        let (prologue_function_name, args) = common_prologue_serialized_args(
            &self.txn_metadata,
            self.features(),
            serialized_signers,
            self.is_simulation,
        )?;
        self.session
            .execute_unmetered_system_function(
                &APTOS_TRANSACTION_VALIDATION.module_id(),
                prologue_function_name,
                args,
            )
            .map_err(expect_no_verification_errors)
            .or_else(|err| convert_prologue_error(err, self.session.log_context))?;
        Ok(())
    }

    fn run_multisig_prologue(&mut self) -> Result<(), VMStatus> {
        if let Some(multisig_address) = self.txn_extra_config.multisig_address() {
            if !self.is_simulation
                || self
                    .features()
                    .is_transaction_simulation_enhancement_enabled()
            {
                let args = multisig_prologue_args(
                    &self.txn_metadata,
                    self.features(),
                    multisig_address,
                    &self.executable,
                )?;
                self.session
                    .execute_unmetered_system_function(
                        &MULTISIG_ACCOUNT_MODULE,
                        VALIDATE_MULTISIG_TRANSACTION,
                        serialize_values(&args),
                    )
                    .map_err(expect_no_verification_errors)
                    .or_else(|err| convert_prologue_error(err, self.session.log_context))?;
            }
        }
        Ok(())
    }
}

impl<'a, DataView, CodeLoader> Session<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    /// Executes AA authentication via native dynamic dispatch ([FunctionInfo] provides information
    /// which authentication function needs to be called).
    fn dispatchable_authenticate(
        &mut self,
        gas_meter: &mut impl GasMeter,
        sender: AccountAddress,
        function_info: FunctionInfo,
        auth_data: &AbstractionAuthData,
    ) -> Result<Vec<u8>, VMStatus> {
        let enabled = match auth_data {
            AbstractionAuthData::V1 { .. } => self.features.is_account_abstraction_enabled(),
            AbstractionAuthData::DerivableV1 { .. } => {
                self.features.is_derivable_account_abstraction_enabled()
            },
        };

        if !enabled {
            return Err(feature_under_gating!(
                "AA authentication can not be used because the feature is not enabled"
            ));
        }

        let auth_data = bcs::to_bytes(auth_data).expect("Authentication data is serializable");
        let mut args = serialize_values(&vec![
            MoveValue::Signer(sender),
            function_info.as_move_value(),
        ]);
        args.push(auth_data);

        self.execute_function_bypass_visibility(
            &ACCOUNT_ABSTRACTION_MODULE,
            AUTHENTICATE,
            vec![],
            args,
            gas_meter,
        )
        .map(|mut return_vals| {
            assert!(
                return_vals.mutable_reference_outputs.is_empty()
                    && return_vals.return_values.len() == 1,
                "Abstraction authentication function must only have 1 return value"
            );
            let (signer_data, signer_layout) = return_vals.return_values.pop().expect("Must exist");
            assert_eq!(
                signer_layout,
                MoveTypeLayout::Signer,
                "Abstraction authentication function returned non-signer."
            );
            signer_data
        })
        .map_err(|mut err| {
            if err.major_status() == StatusCode::OUT_OF_GAS {
                err.set_major_status(StatusCode::ACCOUNT_AUTHENTICATION_GAS_LIMIT_EXCEEDED);
            }
            err.into_vm_status()
        })
    }
}
