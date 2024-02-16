// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod test {
    use aptos_language_e2e_tests::{
        account::Account,
        executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType},
    };
    use aptos_transaction_generator_lib::{
        publishing::{
            module_simple::{AutomaticArgs, LoopType, MultiSigConfig},
            publish_util::{Package, PackageHandler},
        },
        EntryPoints,
    };
    use aptos_types::{transaction::TransactionPayload, PeerId};
    use rand::{rngs::StdRng, SeedableRng};
    use serde_json::json;

    pub fn execute_txn(
        executor: &mut FakeExecutor,
        account: &Account,
        sequence_number: u64,
        payload: TransactionPayload,
    ) {
        // build and sign transaction
        let sign_tx = account
            .transaction()
            .sequence_number(sequence_number)
            .max_gas_amount(2_000_000)
            .gas_unit_price(200)
            .payload(payload)
            .sign();

        let txn_output = executor.execute_transaction(sign_tx);
        executor.apply_write_set(txn_output.write_set());
        assert!(txn_output.status().status().unwrap().is_success());
    }

    fn execute_and_time_entry_point(
        entry_point: &EntryPoints,
        package: &Package,
        publisher_address: &PeerId,
        executor: &mut FakeExecutor,
        iterations: u64,
    ) -> u128 {
        let mut rng = StdRng::seed_from_u64(14);
        let entry_fun = entry_point
            .create_payload(
                package.get_module_id(entry_point.module_name()),
                Some(&mut rng),
                Some(publisher_address),
            )
            .into_entry_function();

        executor.exec_func_record_running_time_with_dynamic_args(
            entry_fun.module(),
            entry_fun.function().as_str(),
            entry_fun.ty_args().to_vec(),
            entry_fun.args().to_vec(),
            iterations,
            match entry_point.automatic_args() {
                AutomaticArgs::None => ExecFuncTimerDynamicArgs::NoArgs,
                AutomaticArgs::Signer => ExecFuncTimerDynamicArgs::DistinctSigners,
                AutomaticArgs::SignerAndMultiSig => match entry_point.multi_sig_additional_num() {
                    MultiSigConfig::Publisher => {
                        ExecFuncTimerDynamicArgs::DistinctSignersAndFixed(vec![*publisher_address])
                    },
                    _ => todo!(),
                },
            },
            GasMeterType::RegularMeter,
        )
    }

    const ALLOWED_REGRESSION: f32 = 0.1;
    const ALLOWED_IMPROVEMENT: f32 = 0.1;

    #[test]
    fn performance_regression_test() {
        let executor = FakeExecutor::from_head_genesis();
        let mut executor = executor.set_not_parallel();

        let entry_points = vec![
            // too fast for the timer
            // (11, EntryPoints::Nop),
            // (42, EntryPoints::BytesMakeOrChange {
            //     data_length: Some(32),
            // }),
            // (30, EntryPoints::IncGlobal),
            (305204, EntryPoints::Loop {
                loop_count: Some(100000),
                loop_type: LoopType::NoOp,
            }),
            (173688, EntryPoints::Loop {
                loop_count: Some(10000),
                loop_type: LoopType::Arithmetic,
            }),
            // This is a cheap bcs (serializing vec<u8>), so not representative of what BCS native call should cost.
            // (, EntryPoints::Loop { loop_count: Some(1000), loop_type: LoopType::BCS { len: 1024 }}),
            (1258, EntryPoints::CreateObjects {
                num_objects: 10,
                object_payload_size: 0,
            }),
            (63352, EntryPoints::CreateObjects {
                num_objects: 10,
                object_payload_size: 10 * 1024,
            }),
            (12279, EntryPoints::CreateObjects {
                num_objects: 100,
                object_payload_size: 0,
            }),
            (77889, EntryPoints::CreateObjects {
                num_objects: 100,
                object_payload_size: 10 * 1024,
            }),
            (531, EntryPoints::InitializeVectorPicture { length: 40 }),
            (116, EntryPoints::VectorPicture { length: 40 }),
            (118, EntryPoints::VectorPictureRead { length: 40 }),
            (233054, EntryPoints::InitializeVectorPicture {
                length: 30 * 1024,
            }),
            (34154, EntryPoints::VectorPicture { length: 30 * 1024 }),
            (33906, EntryPoints::VectorPictureRead { length: 30 * 1024 }),
            (290151, EntryPoints::SmartTablePicture {
                length: 30 * 1024,
                num_points_per_txn: 200,
            }),
            (501325, EntryPoints::SmartTablePicture {
                length: 1024 * 1024,
                num_points_per_txn: 300,
            }),
            (108, EntryPoints::ResourceGroupsSenderWriteTag {
                string_length: 1024,
            }),
            (233, EntryPoints::ResourceGroupsSenderMultiChange {
                string_length: 1024,
            }),
            (2049, EntryPoints::TokenV1MintAndTransferFT),
            (3098, EntryPoints::TokenV1MintAndTransferNFTSequential),
            (2804, EntryPoints::TokenV2AmbassadorMint),
        ];

        let mut results = Vec::new();
        let mut json_lines = Vec::new();

        for (index, (expected_time, entry_point)) in entry_points.into_iter().enumerate() {
            // if let MultiSigConfig::None = entry_point.multi_sig_additional_num() {
            let publisher = executor.new_account_at(PeerId::random());

            let mut package_handler = PackageHandler::new(entry_point.package_name());
            let mut rng = StdRng::seed_from_u64(14);
            let package = package_handler.pick_package(&mut rng, *publisher.address());
            execute_txn(
                &mut executor,
                &publisher,
                0,
                package.publish_transaction_payload(),
            );
            if let Some(init_entry_point) = entry_point.initialize_entry_point() {
                execute_txn(
                    &mut executor,
                    &publisher,
                    1,
                    init_entry_point.create_payload(
                        package.get_module_id(init_entry_point.module_name()),
                        Some(&mut rng),
                        Some(publisher.address()),
                    ),
                );
            }

            let elapsed_micros = execute_and_time_entry_point(
                &entry_point,
                &package,
                publisher.address(),
                &mut executor,
                if expected_time > 10000 {
                    6
                } else if expected_time > 1000 {
                    10
                } else {
                    100
                },
            );
            println!(
                "{}us\texpected {}us\t{:?}: ",
                elapsed_micros, expected_time, entry_point
            );

            json_lines.push(json!({
                "grep": "grep_json_aptos_move_vm_perf",
                "transaction_type": format!("{:?}", entry_point),
                "wall_time_us": elapsed_micros,
                "expected_wall_time_us": expected_time,
                "test_index": index,
            }));

            if elapsed_micros as f32 > expected_time as f32 * (1.0 + ALLOWED_REGRESSION) + 2.0 {
                results.push(format!(
                    "Performance regression detected: {}us\texpected {}us\t{:?}: ",
                    elapsed_micros, expected_time, entry_point
                ));
            } else if elapsed_micros as f32 + 2.0
                < expected_time as f32 * (1.0 - ALLOWED_IMPROVEMENT)
            {
                results.push(format!(
                    "Performance improvement detected: {}us\texpected {}us\t{:?}: ",
                    elapsed_micros, expected_time, entry_point
                ));
            }
        }

        for line in json_lines {
            println!("{}", serde_json::to_string(&line).unwrap());
        }

        for result in &results {
            println!("{}", result);
        }
        assert!(results.is_empty());
    }

    // anyhow = { workspace = true }
    // aptos = { workspace = true }
    // aptos-abstract-gas-usage = { workspace = true }
    // aptos-cached-packages = { workspace = true }
    // aptos-framework = { workspace = true }
    // aptos-gas-algebra = { workspace = true }
    // aptos-gas-meter = { workspace = true }
    // aptos-gas-schedule = { workspace = true }
    // aptos-move-stdlib = { workspace = true }
    // aptos-native-interface = { workspace = true }
    // aptos-vm-types = { workspace = true }
    // bcs = { workspace = true }
    // clap = { workspace = true }
    // float-cmp = { workspace = true }
    // move-binary-format = { workspace = true }
    // move-bytecode-source-map = { workspace = true }
    // move-core-types = { workspace = true }
    // move-ir-compiler = { workspace = true }
    // move-vm-runtime = { workspace = true }
    // move-vm-test-utils = { workspace = true }
    // walkdir = { workspace = true }

    // use aptos_language_e2e_tests::executor::FakeExecutor;
    // use aptos_transaction_generator_lib::EntryPoints;
    // use aptos_types::PeerId;
}
