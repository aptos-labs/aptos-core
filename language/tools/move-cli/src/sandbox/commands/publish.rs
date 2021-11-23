// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sandbox::utils::{
        explain_publish_changeset, explain_publish_error, get_gas_status, module,
        on_disk_state_view::OnDiskStateView,
    },
    NativeFunctionRecord,
};
use anyhow::{bail, Result};
use move_package::compilation::compiled_package::CompiledPackage;
use move_vm_runtime::move_vm::MoveVM;
use std::collections::BTreeMap;

pub fn publish(
    natives: impl IntoIterator<Item = NativeFunctionRecord>,
    state: &OnDiskStateView,
    package: &CompiledPackage,
    no_republish: bool,
    ignore_breaking_changes: bool,
    override_ordering: Option<&[String]>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!(
            "Found {} modules",
            package.modules().collect::<Vec<_>>().len()
        );
    }

    if no_republish {
        let republished = package
            .modules()
            .filter_map(|unit| {
                let id = module(&unit.unit).ok()?.self_id();
                if state.has_module(&id) {
                    Some(format!("{}", id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !republished.is_empty() {
            eprintln!("Failed to republish modules since the --no-republish flag is set. Tried to republish the following modules: {}",
                republished.join(", "));
            return Ok(());
        }
    }

    // use the the publish_module API from the VM if we do not allow breaking changes
    if !ignore_breaking_changes {
        let vm = MoveVM::new(natives).unwrap();
        let mut gas_status = get_gas_status(None)?;
        let mut session = vm.new_session(state);

        let mut has_error = false;
        match override_ordering {
            None => {
                for unit in package.modules() {
                    let module_bytes = unit.unit.serialize();
                    let id = module(&unit.unit)?.self_id();
                    let sender = *id.address();

                    let res = session.publish_module(module_bytes, sender, &mut gas_status);
                    if let Err(err) = res {
                        explain_publish_error(err, state, unit)?;
                        has_error = true;
                        break;
                    }
                }
            }
            Some(ordering) => {
                let module_map: BTreeMap<_, _> = package
                    .modules()
                    .into_iter()
                    .map(|unit| (unit.unit.name().to_string(), unit))
                    .collect();

                let mut sender_opt = None;
                let mut module_bytes_vec = vec![];
                for name in ordering {
                    match module_map.get(name) {
                        None => bail!("Invalid module name in publish ordering: {}", name),
                        Some(unit) => {
                            let module_bytes = unit.unit.serialize();
                            module_bytes_vec.push(module_bytes);
                            if sender_opt.is_none() {
                                sender_opt = Some(*module(&unit.unit)?.self_id().address());
                            }
                        }
                    }
                }

                match sender_opt {
                    None => bail!("No modules to publish"),
                    Some(sender) => {
                        let res = session.publish_module_bundle(
                            module_bytes_vec,
                            sender,
                            &mut gas_status,
                        );
                        if let Err(err) = res {
                            // TODO (mengxu): explain publish errors in multi-module publishing
                            println!("Invalid multi-module publishing: {}", err);
                            has_error = true;
                        }
                    }
                }
            }
        }

        if !has_error {
            let (changeset, events) = session.finish().map_err(|e| e.into_vm_status())?;
            assert!(events.is_empty());
            if verbose {
                explain_publish_changeset(&changeset, state);
            }
            let modules: Vec<_> = changeset
                .into_modules()
                .map(|(module_id, blob_opt)| (module_id, blob_opt.expect("must be non-deletion")))
                .collect();
            state.save_modules(&modules)?;
        }
    } else {
        // NOTE: the VM enforces the most strict way of module republishing and does not allow
        // backward incompatible changes, as as result, if this flag is set, we skip the VM process
        // and force the CLI to override the on-disk state directly
        let mut serialized_modules = vec![];
        for unit in package.modules() {
            let id = module(&unit.unit)?.self_id();
            let module_bytes = unit.unit.serialize();
            serialized_modules.push((id, module_bytes));
        }
        state.save_modules(&serialized_modules)?;
    }

    Ok(())
}
