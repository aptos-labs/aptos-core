// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{data_cache_v2::Session, verifier};
use aptos_gas_meter::AptosGasMeter;
use aptos_table_natives::TableResolver;
use aptos_types::{on_chain_config::Features, transaction::ModuleBundle};
use aptos_vm_types::{
    module_and_script_storage::code_storage::AptosCodeStorage, module_write_set::ModuleWriteSet,
    resolver::ExecutorView,
};
use move_binary_format::{compatibility::Compatibility, errors::Location, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, ident_str, value::MoveValue, vm_status::VMStatus,
};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage, StagingModuleStorage};
use std::collections::BTreeMap;

/// Finishes the session while also processing the publish request, and running module
/// initialization if necessary.
pub(crate) fn finish_with_module_publishing_and_initialization<
    C: AptosCodeStorage,
    T: ExecutorView + TableResolver,
>(
    session: &mut Session<'_, C, T>,
    gas_meter: &mut impl AptosGasMeter,
    traversal_context: &mut TraversalContext,
    _features: &Features,
    destination: AccountAddress,
    bundle: ModuleBundle,
    modules: &[CompiledModule],
    compatability_checks: Compatibility,
) -> Result<ModuleWriteSet, VMStatus> {
    // Stage module bundle on top of module storage. In case modules cannot be added (for
    // example, fail compatibility checks, create cycles, etc.), we return an error here.
    let module_storage = session.code_storage();
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

        let module_id = module.self_id();
        let init_function_exists = staging_module_storage
            .load_function(&module_id, init_func_name, &[])
            .is_ok();

        if init_function_exists {
            // We need to check that init_module function we found is well-formed.
            verifier::module_init::verify_module_init_function(module)
                .map_err(|e| e.finish(Location::Undefined))?;

            session.execute_init_hack(
                &module_id,
                init_func_name,
                vec![],
                vec![MoveValue::Signer(destination).simple_serialize().unwrap()],
                gas_meter,
                traversal_context,
                &staging_module_storage,
            )?;
        }
    }

    // TODO: figure out how to get metadata for init module changes?

    // Get the changes from running module initialization. Note that here we use the staged
    // module storage to ensure resource group metadata from new modules is visible.
    // let Self { session } = self;
    // let change_set = session.finish_with_squashed_change_set(
    //     change_set_configs,
    //     &staging_module_storage,
    //     false,
    // )?;

    // let write_ops = convert_modules_into_write_ops(
    //     session.executor_view(),
    //     features,
    //     module_storage,
    //     staging_module_storage.release_verified_module_bundle(),
    // )
    // .map_err(|e| e.finish(Location::Undefined))?;
    let write_ops = BTreeMap::new();
    Ok(ModuleWriteSet::new(write_ops))
}
