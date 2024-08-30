// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::UserSessionChangeSet,
            view_with_change_set::ExecutorViewWithChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    verifier, AptosVM,
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::transaction::ModuleBundle;
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::{compatibility::Compatibility, errors::Location, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, ident_str, value::MoveValue, vm_status::VMStatus,
};
use move_vm_runtime::{module_traversal::TraversalContext, TemporaryModuleStorage};

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
        module_storage: &impl AptosModuleStorage,
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
        self,
        vm: &'l AptosVM,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        gas_meter: &mut impl AptosGasMeter,
        transaction_metadata: &TransactionMetadata,
        traversal_context: &mut TraversalContext,
        change_set_configs: &ChangeSetConfigs,
        destination: AccountAddress,
        bundle: ModuleBundle,
        modules: &[CompiledModule],
        compatability_checks: Compatibility,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        // First, finish the current session to extract any user changes.
        // Note that thee should be no published modules here.
        let (mut change_set, empty_module_write_set) =
            self.finish(change_set_configs, module_storage)?.unpack();
        empty_module_write_set
            .is_empty_or_invariant_violation()
            .map_err(|err| {
                err.with_message("User session cannot publish modules on its own".to_string())
                    .finish(Location::Undefined)
                    .into_vm_status()
            })?;

        // Stash user changes on top of existing resolver so that resources
        // can be queried correctly.
        let executor_view = ExecutorViewWithChangeSet::new(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            change_set.clone(),
        );
        let tmp_resolver = vm.as_move_resolver_with_group_view(&executor_view);

        // Create a temporary VM in such a way so that its caches are not shared globally.
        // In particular, any information we cache for this VM will not be visible by other
        // threads. This prevents cases like:
        //    1. VM caches struct names & types when performing linking checks on not yet
        //       published module bundle, or when running init_module.
        //    2. Modules are not published because of out of gas error in epilogue, but the
        //       cached information is still around and is available to other threads.
        // Hence, we also need to create a new session using this new VM.
        let tmp_vm = vm.move_vm_for_module_publishing_and_init_module();
        let mut tmp_session = tmp_vm.new_session(
            &tmp_resolver,
            SessionId::txn_meta(transaction_metadata),
            Some(transaction_metadata.as_user_transaction_context()),
        );

        // Create a temporary module storage with added modules. In case modules cannot
        // be added (fail compatibility checks, create cycles), we return an error here.
        let tmp_module_storage = TemporaryModuleStorage::new_with_compat_config(
            &destination,
            tmp_vm.runtime_environment(),
            compatability_checks,
            module_storage,
            bundle.into_bytes(),
        )?;

        // Modules can be published, and we finally can run module initialization. Since
        // we use a temporary session, temporary VM, and a temporary module storage, no
        // information is observed by other threads, and we cannot end up in an inconsistent
        // state.

        let init_func_name = ident_str!("init_module");
        for module in modules {
            // Check if module existed previously. If not, we do not run initialization.
            if module_storage.check_module_exists(module.self_addr(), module.self_name())? {
                continue;
            }

            let module_id = module.self_id();
            let init_function_exists = tmp_session
                .load_function(&tmp_module_storage, &module_id, init_func_name, &[])
                .is_ok();

            if init_function_exists {
                // We need to check that init_module function we found is well-formed.
                verifier::module_init::verify_module_init_function(module)
                    .map_err(|e| e.finish(Location::Undefined))?;

                tmp_session.execute_function_bypass_visibility(
                    &module_id,
                    init_func_name,
                    vec![],
                    vec![MoveValue::Signer(destination).simple_serialize().unwrap()],
                    gas_meter,
                    traversal_context,
                    &tmp_module_storage,
                )?;
            }
        }

        // Create module write set for the modules to be published. We do not care
        // here about writes to special addresses because there is no flushing.
        let write_ops = tmp_session
            .convert_modules_into_write_ops(
                module_storage,
                tmp_module_storage.release_verified_module_bundle(),
            )
            .map_err(|e| e.finish(Location::Undefined))?;
        let module_write_set = ModuleWriteSet::new(false, write_ops);

        // Also, process init_module changes squashing them with user session changes.
        let (init_module_change_set, empty_write_set) =
            tmp_session.finish(change_set_configs, module_storage)?;
        empty_write_set
            .is_empty_or_invariant_violation()
            .map_err(|e| {
                e.with_message("init_module_cannot publish modules".to_string())
                    .finish(Location::Undefined)
            })?;
        change_set
            .squash_additional_change_set(init_module_change_set)
            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;

        UserSessionChangeSet::new(change_set, module_write_set, change_set_configs)
    }
}
