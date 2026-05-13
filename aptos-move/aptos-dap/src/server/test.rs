// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::server::DapServer;
use aptos_framework::extended_checks;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_move_cli::{
    aptos_debug_natives::aptos_debug_natives, source_locator::AptosSourceLocator,
};
use aptos_types::on_chain_config::aptos_test_feature_flags_genesis;
use aptos_vm_environment::prod_configs::set_debugging_enabled;
use move_vm_runtime::debug::dap::{
    create_dap_debug_context, install_dap_debug_context_on_thread, DapEvent,
};
use std::{io, path::PathBuf, sync::Arc, thread};

impl<R: io::Read, W: io::Write> DapServer<R, W> {
    pub(super) fn start_test_execution(
        &mut self,
        package_path: PathBuf,
        test_filter: String,
        skip_fetch_latest_git_deps: bool,
    ) -> anyhow::Result<()> {
        let source_locator = build_aptos_source_locator(&package_path, skip_fetch_latest_git_deps);

        let known_files: Vec<String> = source_locator
            .as_ref()
            .map(|loc| {
                loc.known_source_files()
                    .into_iter()
                    .map(|s| s.to_owned())
                    .collect()
            })
            .unwrap_or_default();
        self.warn_on_unreachable_breakpoints(&known_files)?;

        set_debugging_enabled(true);
        let (cmd_tx, evt_rx, evt_tx, debug_ctx) = create_dap_debug_context();

        let handle = thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(
            move || -> anyhow::Result<()> {
                install_dap_debug_context_on_thread(debug_ctx);
                eprintln!("aptos-dap: compiling and running test '{test_filter}'...");

                if let Some(locator) = source_locator {
                    move_vm_runtime::source_locator::set_source_locator(locator);
                }

                let natives =
                    aptos_debug_natives(NativeGasParameters::zeros(), MiscGasParameters::zeros());
                let genesis = aptos_test_feature_flags_genesis();

                let result = move_unit_test::package_test::run_move_unit_tests(
                    &package_path,
                    move_package::BuildConfig {
                        test_mode: true,
                        dev_mode: true,
                        skip_fetch_latest_git_deps,
                        compiler_config: move_package::CompilerConfig {
                            known_attributes: extended_checks::get_all_attribute_names().clone(),
                            compiler_version: Some(
                                move_model::metadata::CompilerVersion::latest_stable(),
                            ),
                            language_version: Some(
                                move_model::metadata::LanguageVersion::latest_stable(),
                            ),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    move_unit_test::UnitTestingConfig {
                        filter: Some(test_filter),
                        num_threads: 1,
                        ..move_unit_test::UnitTestingConfig::default()
                    },
                    natives,
                    genesis,
                    None,
                    None,
                    false,
                    &mut io::stderr(),
                    true,
                );

                move_vm_runtime::source_locator::clear_source_locator();

                let message = match result {
                    Ok(move_unit_test::package_test::UnitTestResult::Success) => {
                        eprintln!("aptos-dap: test passed");
                        None
                    },
                    Ok(move_unit_test::package_test::UnitTestResult::Failure) => {
                        eprintln!("aptos-dap: test failed");
                        Some("Test failed".to_string())
                    },
                    Err(e) => {
                        let msg = format!("{e:#}");
                        eprintln!("aptos-dap: test error: {msg}");
                        Some(msg)
                    },
                };

                let _ = evt_tx.send(DapEvent::Terminated { message });
                Ok(())
            },
        );

        self.cmd_tx = Some(cmd_tx);
        self.event_rx = Some(evt_rx);
        self.vm_thread = Some(handle?);
        Ok(())
    }
}

fn build_aptos_source_locator(
    package_path: &std::path::Path,
    skip_fetch_latest_git_deps: bool,
) -> Option<Arc<AptosSourceLocator>> {
    use legacy_move_compiler::compiled_unit::CompiledUnit;

    let build_config = move_package::BuildConfig {
        test_mode: true,
        dev_mode: true,
        skip_fetch_latest_git_deps,
        compiler_config: move_package::CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            compiler_version: Some(move_model::metadata::CompilerVersion::latest_stable()),
            language_version: Some(move_model::metadata::LanguageVersion::latest_stable()),
            ..Default::default()
        },
        ..Default::default()
    };
    let compiled = match build_config.compile_package(package_path, &mut io::stderr()) {
        Ok(pkg) => pkg,
        Err(e) => {
            eprintln!("aptos-dap: could not compile package for source maps: {e:#}");
            return None;
        },
    };

    let mut locator = AptosSourceLocator::new();
    for unit in compiled.all_compiled_units_with_source() {
        if let CompiledUnit::Module(ref named) = unit.unit {
            let sm_bytes = unit.unit.serialize_source_map();
            let source_text = std::fs::read_to_string(&unit.source_path).unwrap_or_default();
            // Canonicalize so source locations from dependency packages use
            // absolute paths, matching the canonicalized breakpoint paths.
            let filename = unit
                .source_path
                .canonicalize()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| unit.source_path.to_string_lossy().into_owned());
            if let Err(e) =
                locator.add_local_module(&named.module, &sm_bytes, &source_text, &filename)
            {
                eprintln!(
                    "aptos-dap: could not load source map for {}: {e}",
                    named.module.self_id()
                );
            }
        }
    }

    Some(Arc::new(locator))
}
