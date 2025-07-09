// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::AptosMoveResolver,
    v2::{data_cache::TransactionDataCacheAdapter, session::UserTransactionSession},
    verifier,
};
use aptos_framework::natives::code::{NativeCodeContext, PublishRequest};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    on_chain_config::FeatureFlag, state_store::state_value::StateValueMetadata, write_set::WriteOp,
};
use aptos_vm_types::{
    module_and_script_storage::{
        code_storage::AptosCodeStorage, module_storage::AptosModuleStorage,
    },
    module_write_set::ModuleWrite,
};
use bytes::Bytes;
use derive_more::Deref;
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    deserializer::DeserializerConfig,
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumBytes,
    ident_str,
    identifier::IdentStr,
    language_storage::ModuleId,
    value::MoveValue,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{
    check_dependencies_and_charge_gas, move_vm::MoveVM, AsUnsyncModuleStorage, LoadedFunction,
    LoadedFunctionOwner, Module, ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_vm_types::{code::ModuleBytesStorage, module_linker_error};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Deref)]
struct NewModule {
    #[deref]
    module: CompiledModule,
    module_bytes: Vec<u8>,
}

#[derive(Deref)]
struct OldModule {
    #[deref]
    module: Arc<Module>,
    #[allow(dead_code)]
    module_size: u64,
    #[allow(dead_code)]
    state_value_metadata: StateValueMetadata,
}

enum PublishingUnit {
    New(NewModule),
    Republished(NewModule, #[allow(dead_code)] OldModule),
}

impl PublishingUnit {
    fn new_module(&self) -> &NewModule {
        match self {
            PublishingUnit::New(new_module) => new_module,
            PublishingUnit::Republished(new_module, _) => new_module,
        }
    }

    fn self_id(&self) -> ModuleId {
        match self {
            PublishingUnit::New(m) => m.self_id(),
            PublishingUnit::Republished(m, _) => m.self_id(),
        }
    }

    #[allow(dead_code)]
    fn into_module_write(self) -> ModuleWrite<WriteOp> {
        match self {
            PublishingUnit::New(_new_module) => {
                // TODO
                unimplemented!()
            },
            PublishingUnit::Republished(_new_module, _) => {
                // TODO
                unimplemented!()
            },
        }
    }
}

struct VerifiedModuleBundle {
    #[allow(dead_code)]
    destination: AccountAddress,
    verified_modules: BTreeMap<ModuleId, PublishingUnit>,
}

struct StagingArea<'a, CodeView> {
    bundle: &'a VerifiedModuleBundle,
    code_view: &'a CodeView,
}

impl<CodeView> ModuleBytesStorage for StagingArea<'_, CodeView>
where
    CodeView: ModuleStorage,
{
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        let id = ModuleId::new(*address, module_name.to_owned());
        if let Some(unit) = self.bundle.verified_modules.get(&id) {
            return Ok(Some(unit.new_module().module_bytes.clone().into()));
        }
        self.code_view.fetch_module_bytes(address, module_name)
    }
}

impl<CodeView> WithRuntimeEnvironment for StagingArea<'_, CodeView>
where
    CodeView: WithRuntimeEnvironment,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_view.runtime_environment()
    }
}

impl<'a, DataView, CodeView> UserTransactionSession<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosCodeStorage,
{
    pub(crate) fn resolve_published_modules(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
    ) -> Result<(), VMStatus> {
        let publish_request = match self
            .session
            .extensions
            .get_mut::<NativeCodeContext>()
            .extract_publish_request()
        {
            Some(publish_request) => publish_request,
            None => {
                // No publish request, there is nothing to do.
                return Ok(());
            },
        };

        let verified_bundle = self.verify_module_bundle(gas_meter, publish_request)?;
        self.check_module_linking_and_run_initialization(gas_meter, &verified_bundle)?;

        unimplemented!()
    }
}

// Private interfaces.
impl<'a, DataView, CodeView> UserTransactionSession<'a, DataView, CodeView>
where
    DataView: AptosMoveResolver,
    CodeView: AptosCodeStorage,
{
    fn build_new_module(
        &self,
        gas_meter: &mut impl AptosGasMeter,
        module_bytes: Vec<u8>,
    ) -> Result<NewModule, VMStatus> {
        let config = self.deserializer_config();
        let module =
            CompiledModule::deserialize_with_config(&module_bytes, config).map_err(|err| {
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(format!("Failed to deserialize published module: {err}"))
                    .finish(Location::Undefined)
            })?;

        gas_meter
            .charge_dependency(
                true,
                module.address(),
                module.name(),
                NumBytes::new(module_bytes.len() as u64),
            )
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(NewModule {
            module,
            module_bytes,
        })
    }

    fn build_old_module(
        &self,
        gas_meter: &mut impl AptosGasMeter,
        new_module: &NewModule,
    ) -> Result<Option<OldModule>, VMStatus> {
        let addr = new_module.address();
        let name = new_module.name();

        let module_size = match self
            .session
            .code_view
            .unmetered_get_module_size(addr, name)?
        {
            Some(size) => size as u64,
            None => {
                return Ok(None);
            },
        };

        gas_meter
            .charge_dependency(false, addr, name, NumBytes::new(module_size))
            .map_err(|err| err.finish(Location::Undefined))?;

        let invariant_violation = || {
            PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                .with_message(format!("Module {addr}::{name} must exist"))
                .finish(Location::Undefined)
        };

        let module = self
            .session
            .code_view
            .fetch_verified_module(addr, name)?
            .ok_or_else(invariant_violation)?;
        let state_value_metadata = self
            .session
            .code_view
            .get_module_state_value_metadata(addr, name)
            .map_err(|err| err.finish(Location::Undefined))?
            .ok_or_else(invariant_violation)?;

        Ok(Some(OldModule {
            module,
            module_size,
            state_value_metadata,
        }))
    }

    fn check_module_linking_and_run_initialization(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        bundle: &VerifiedModuleBundle,
    ) -> Result<(), VMStatus> {
        check_dependencies_and_charge_gas(
            self.session.code_view,
            gas_meter,
            self.session.traversal_context,
            [],
            // TODO: fix lifetimes.
            // staging_area
            //     .bundle
            //     .verified_modules
            //     .iter()
            //     .flat_map(|(_, unit)| {
            //         unit.new_module()
            //             .immediate_dependencies_iter()
            //             .chain(unit.new_module().immediate_friends_iter())
            //     })
            //     .filter(|(addr, name)| {
            //         let id = ModuleId::new(**addr, (*name).to_owned());
            //         !staging_area.bundle.verified_modules.contains_key(&id)
            //     }),
        )?;

        let staging_area = StagingArea {
            bundle,
            code_view: self.session.code_view,
        };
        let tmp_code_view = staging_area.as_unsync_module_storage();
        let mut requires_initialization = vec![];

        for module_id in staging_area.bundle.verified_modules.keys() {
            // Check linking to immediate dependencies.
            let module = tmp_code_view
                .fetch_verified_module(module_id.address(), module_id.name())?
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "Staged module {}::{} must always exist",
                            module_id.address(),
                            module_id.name()
                        ))
                        .finish(Location::Undefined)
                        .into_vm_status()
                })?;

            // Also verify that all friends exist.
            for (friend_addr, friend_name) in module.immediate_friends_iter() {
                if !tmp_code_view.check_module_exists(friend_addr, friend_name)? {
                    return Err(module_linker_error!(friend_addr, friend_name).into_vm_status());
                }
            }

            requires_initialization.push(module);
        }

        self.run_init_module(
            gas_meter,
            requires_initialization,
            staging_area.bundle.destination,
            &tmp_code_view,
        )?;

        Ok(())
    }

    fn run_init_module(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        modules: Vec<Arc<Module>>,
        destination: AccountAddress,
        tmp_code_view: &impl AptosModuleStorage,
    ) -> Result<(), VMStatus> {
        let init_func_name = ident_str!("init_module");
        for module in modules {
            if let Ok(function) = module.get_function(init_func_name) {
                verifier::module_init::verify_init_module_function(&function)?;

                let loaded_function = LoadedFunction {
                    owner: LoadedFunctionOwner::Module(module.clone()),
                    ty_args: vec![],
                    function,
                };

                MoveVM::execute_loaded_function(
                    loaded_function,
                    vec![MoveValue::Signer(destination)
                        .simple_serialize()
                        .expect("Signer is always serializable")],
                    &mut TransactionDataCacheAdapter::new(
                        &mut self.session.data_cache,
                        self.session.data_view,
                        tmp_code_view,
                    ),
                    gas_meter,
                    self.session.traversal_context,
                    &mut self.session.extensions,
                    // Note: here we pass a new code view with newly published modules.
                    tmp_code_view,
                )?;
            }
        }
        Ok(())
    }

    fn verify_module_bundle(
        &self,
        gas_meter: &mut impl AptosGasMeter,
        mut publish_request: PublishRequest,
    ) -> Result<VerifiedModuleBundle, VMStatus> {
        let mut verified_modules = BTreeMap::new();

        for module_bytes in publish_request.bundle.into_inner() {
            let new_module = self.build_new_module(gas_meter, module_bytes)?;

            self.remove_from_expected_modules_and_check_allowed_deps(
                &new_module,
                &mut publish_request.expected_modules,
                &publish_request.allowed_deps,
            )?;

            self.validate_module_address(&new_module, &publish_request.destination)?;
            // TODO: reject unstable bytecode.
            self.check_module_complexity(&new_module)?;
            self.verify_module(&new_module)?;
            // TODO: check native structs do not exist.
            // TODO: validate native functions
            // TODO: validate module metadata for publishing
            // TODO: validate events
            // TODO: validate resource groups

            let module_id = new_module.self_id();
            let unit = match self.build_old_module(gas_meter, &new_module)? {
                None => PublishingUnit::New(new_module),
                Some(old_module) => {
                    self.check_compatibility(&new_module, &old_module)?;
                    // TODO: validate events compat.
                    // TODO: validate resource groups compat.
                    PublishingUnit::Republished(new_module, old_module)
                },
            };

            if let Some(unit) = verified_modules.insert(module_id, unit) {
                let msg = format!(
                    "Module {}::{} occurs more than once in published bundle",
                    unit.self_id().address(),
                    unit.self_id().name()
                );
                return Err(PartialVMError::new(StatusCode::DUPLICATE_MODULE_NAME)
                    .with_message(msg)
                    .finish(Location::Undefined)
                    .into_vm_status());
            }
        }

        if !publish_request.expected_modules.is_empty() {
            // TODO: error
        }

        Ok(VerifiedModuleBundle {
            destination: publish_request.destination,
            verified_modules,
        })
    }

    fn remove_from_expected_modules_and_check_allowed_deps(
        &self,
        module: &NewModule,
        expected_modules: &mut BTreeSet<String>,
        allowed_deps: &Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
    ) -> VMResult<()> {
        if !expected_modules.remove(module.self_name().as_str()) {
            // TODO: error
        }
        if let Some(allowed_deps) = allowed_deps.as_ref() {
            for (dep_addr, dep_name) in module.immediate_dependencies_iter() {
                if !allowed_deps.get(dep_addr).map_or(false, |dep_names| {
                    dep_names.contains("") || dep_names.contains(dep_name.as_str())
                }) {
                    // TODO: error.
                }
            }
        }
        Ok(())
    }

    fn validate_module_address(
        &self,
        module: &NewModule,
        destination: &AccountAddress,
    ) -> VMResult<()> {
        if module.address() != destination {
            let msg = format!(
                "Address of compiled module {}::{} does not match the sender {}",
                module.address(),
                module.name(),
                destination
            );
            return Err(
                PartialVMError::new(StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER)
                    .with_message(msg)
                    .finish(Location::Module(module.self_id())),
            );
        }
        Ok(())
    }

    fn check_module_complexity(&self, module: &NewModule) -> VMResult<()> {
        // TODO(Gas): Make budget configurable.
        let budget = 2048 + module.module_bytes.len() as u64 * 20;
        move_binary_format::check_complexity::check_module_complexity(module, budget)
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(())
    }

    fn verify_module(&self, module: &NewModule) -> VMResult<()> {
        move_bytecode_verifier::verify_module_with_config(
            &self.session.vm_config.verifier_config,
            module,
        )
    }

    fn check_compatibility(&self, new_module: &NewModule, old_module: &OldModule) -> VMResult<()> {
        let compatibility = self.compatibility();
        if compatibility.need_check_compat() {
            compatibility
                .check(old_module, new_module)
                .map_err(|err| err.finish(Location::Undefined))?;
        }
        Ok(())
    }

    fn compatibility(&self) -> Compatibility {
        let check_struct_layout = true;
        let check_friend_linking = !self
            .features()
            .is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE);
        let treat_entry_as_public = true;
        Compatibility::new(
            check_struct_layout,
            check_friend_linking,
            treat_entry_as_public,
        )
    }

    fn deserializer_config(&self) -> &DeserializerConfig {
        &self.session.vm_config.deserializer_config
    }
}
