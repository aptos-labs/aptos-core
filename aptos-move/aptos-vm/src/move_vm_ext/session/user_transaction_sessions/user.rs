// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        convert_modules_into_write_ops,
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::UserSessionChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    verifier, AptosVM,
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{on_chain_config::Features, transaction::ModuleBundle};
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::{compatibility::Compatibility, errors::Location, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, ident_str, value::MoveValue, vm_status::VMStatus,
};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage, StagingModuleStorage};

#[derive(Deref, DerefMut)]
pub struct UserSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    pub session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> UserSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        prologue_change_set: VMChangeSet,
    ) -> Self {
        let session_id = SessionId::txn_meta(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn legacy_inherit_prologue_session(prologue_session: RespawnedSession<'r, 'l>) -> Self {
        Self {
            session: prologue_session,
        }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
        module_storage: &impl ModuleStorage,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let (change_set, module_write_set) =
            session.finish_with_squashed_change_set(change_set_configs, module_storage, false)?;
        UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)
    }

    /// Finishes the session while also processing the publish request, and running
    /// module initialization if necessary. This function is used by the new loader
    /// and code cache implementations.
    pub fn finish_with_module_publishing_and_initialization(
        mut self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        traversal_context: &mut TraversalContext,
        features: &Features,
        change_set_configs: &ChangeSetConfigs,
        destination: AccountAddress,
        bundle: ModuleBundle,
        modules: &[CompiledModule],
        compatability_checks: Compatibility,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        // Stage module bundle on top of module storage. In case modules cannot be added (for
        // example, fail compatibility checks, create cycles, etc.), we return an error here.
        let staging_module_storage = StagingModuleStorage::create_with_compat_config(
            &destination,
            compatability_checks,
            module_storage,
            bundle.into_bytes(),
        )?;

        let init_func_name = ident_str!("init_module");
        for module in modules {
            // Check if module existed previously. If not, we do not run initialization.
            if module_storage.check_module_exists(module.self_addr(), module.self_name())? {
                continue;
            }

            self.session.execute(|session| {
                let module_id = module.self_id();
                let init_function_exists = session
                    .load_function(&staging_module_storage, &module_id, init_func_name, &[])
                    .is_ok();

                if init_function_exists {
                    // We need to check that init_module function we found is well-formed.
                    verifier::module_init::verify_module_init_function(module)
                        .map_err(|e| e.finish(Location::Undefined))?;

                    session.execute_function_bypass_visibility(
                        &module_id,
                        init_func_name,
                        vec![],
                        vec![MoveValue::Signer(destination).simple_serialize().unwrap()],
                        gas_meter,
                        traversal_context,
                        &staging_module_storage,
                    )?;
                }
                Ok::<_, VMStatus>(())
            })?;
        }

        // Get the changes from running module initialization.
        let (change_set, empty_write_set) = self
            .finish(change_set_configs, &staging_module_storage)?
            .unpack();
        empty_write_set
            .is_empty_or_invariant_violation()
            .map_err(|e| {
                e.with_message("init_module cannot publish modules".to_string())
                    .finish(Location::Undefined)
            })?;

        let write_ops = convert_modules_into_write_ops(
            resolver,
            features,
            module_storage,
            staging_module_storage.release_verified_module_bundle(),
        )
        .map_err(|e| e.finish(Location::Undefined))?;
        let module_write_set = ModuleWriteSet::new(false, write_ops);
        UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)
    }
}
