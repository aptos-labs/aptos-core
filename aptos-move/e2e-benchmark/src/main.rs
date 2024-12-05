// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType, Measurement},
};
use aptos_transaction_generator_lib::{
    publishing::{
        module_simple::{AutomaticArgs, LoopType, MultiSigConfig},
        publish_util::{Package, PackageHandler},
    },
    EntryPoints,
};
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use std::{collections::HashMap, process::exit};

// bump after a bigger test or perf change, so you can easily distinguish runs
// that are on top of this commit
const CODE_PERF_VERSION: &str = "v1";

pub fn execute_txn(
    executor: &mut FakeExecutor,
    account: &Account,
    sequence_number: u64,
    payload: TransactionPayload,
) {
    let sign_tx = account
        .transaction()
        .sequence_number(sequence_number)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(payload)
        .sign();

    let txn_output = executor.execute_transaction(sign_tx);
    executor.apply_write_set(txn_output.write_set());
    assert!(
        txn_output.status().status().unwrap().is_success(),
        "txn failed with {:?}",
        txn_output.status()
    );
}

fn execute_and_time_entry_point(
    entry_point: &EntryPoints,
    package: &Package,
    publisher_address: &AccountAddress,
    executor: &mut FakeExecutor,
    iterations: u64,
) -> Measurement {
    let mut rng = StdRng::seed_from_u64(14);
    let entry_fun = entry_point
        .create_payload(
            package,
            entry_point.module_name(),
            Some(&mut rng),
            Some(publisher_address),
        )
        .into_entry_function();

    executor.exec_func_record_running_time(
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
        GasMeterType::RegularGasMeter,
    )
}

const ALLOWED_REGRESSION: f64 = 0.15;
const ALLOWED_IMPROVEMENT: f64 = 0.15;
const ABSOLUTE_BUFFER_US: f64 = 2.0;

const CALIBRATION_VALUES: &str = "
Loop { loop_count: Some(100000), loop_type: NoOp }	60	0.955	1.074	41893.7
Loop { loop_count: Some(10000), loop_type: Arithmetic }	60	0.965	1.078	25915.0
CreateObjects { num_objects: 10, object_payload_size: 0 }	60	0.924	1.082	158.1
CreateObjects { num_objects: 10, object_payload_size: 10240 }	60	0.951	1.118	9356.2
CreateObjects { num_objects: 100, object_payload_size: 0 }	60	0.926	1.082	1574.2
CreateObjects { num_objects: 100, object_payload_size: 10240 }	60	0.952	1.092	11541.9
InitializeVectorPicture { length: 128 }	10	0.965	1.038	163.3
VectorPicture { length: 128 }	10	0.938	1.060	48.8
VectorPictureRead { length: 128 }	10	0.977	1.077	46.4
InitializeVectorPicture { length: 30720 }	60	0.948	1.123	27893.4
VectorPicture { length: 30720 }	60	0.931	1.125	6923.1
VectorPictureRead { length: 30720 }	60	0.934	1.102	6923.1
SmartTablePicture { length: 30720, num_points_per_txn: 200 }	60	0.952	1.109	43594.7
SmartTablePicture { length: 1048576, num_points_per_txn: 300 }	60	0.957	1.120	73865.4
ResourceGroupsSenderWriteTag { string_length: 1024 }	60	0.934	1.134	15.0
ResourceGroupsSenderMultiChange { string_length: 1024 }	60	0.929	1.122	32.3
TokenV1MintAndTransferFT	60	0.958	1.093	385.2
TokenV1MintAndTransferNFTSequential	60	0.973	1.139	588.1
TokenV2AmbassadorMint { numbered: true }	60	0.960	1.141	512.5
LiquidityPoolSwap { is_stable: true }	60	0.961	1.103	590.3
LiquidityPoolSwap { is_stable: false }	60	0.954	1.134	552.2
CoinInitAndMint	10	0.975	1.043	199.6
FungibleAssetMint	10	0.954	1.038	236.3
IncGlobalMilestoneAggV2 { milestone_every: 1 }	10	0.960	1.047	32.9
IncGlobalMilestoneAggV2 { milestone_every: 2 }	10	0.971	1.066	18.1
EmitEvents { count: 1000 }	10	0.969	1.052	8615.5
";

struct CalibrationInfo {
    // count: usize,
    expected_time_micros: f64,
}

fn get_parsed_calibration_values() -> HashMap<String, CalibrationInfo> {
    CALIBRATION_VALUES
        .trim()
        .split('\n')
        .map(|line| {
            let parts = line.split('\t').collect::<Vec<_>>();
            (parts[0].to_string(), CalibrationInfo {
                // count: parts[1].parse().unwrap(),
                expected_time_micros: parts[parts.len() - 1].parse().unwrap(),
            })
        })
        .collect()
}

fn main() {
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let calibration_values = get_parsed_calibration_values();

    let entry_points = vec![
        // too fast for the timer
        // (, EntryPoints::Nop),
        // (, EntryPoints::BytesMakeOrChange {
        //     data_length: Some(32),
        // }),
        // (, EntryPoints::IncGlobal),
        EntryPoints::Loop {
            loop_count: Some(100000),
            loop_type: LoopType::NoOp,
        },
        EntryPoints::Loop {
            loop_count: Some(10000),
            loop_type: LoopType::Arithmetic,
        },
        // This is a cheap bcs (serializing vec<u8>), so not representative of what BCS native call should cost.
        // (, EntryPoints::Loop { loop_count: Some(1000), loop_type: LoopType::BcsToBytes { len: 1024 }}),
        EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 0,
        },
        EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 10 * 1024,
        },
        EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 0,
        },
        EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 10 * 1024,
        },
        EntryPoints::InitializeVectorPicture { length: 128 },
        EntryPoints::VectorPicture { length: 128 },
        EntryPoints::VectorPictureRead { length: 128 },
        EntryPoints::InitializeVectorPicture { length: 30 * 1024 },
        EntryPoints::VectorPicture { length: 30 * 1024 },
        EntryPoints::VectorPictureRead { length: 30 * 1024 },
        EntryPoints::SmartTablePicture {
            length: 30 * 1024,
            num_points_per_txn: 200,
        },
        EntryPoints::SmartTablePicture {
            length: 1024 * 1024,
            num_points_per_txn: 300,
        },
        EntryPoints::ResourceGroupsSenderWriteTag {
            string_length: 1024,
        },
        EntryPoints::ResourceGroupsSenderMultiChange {
            string_length: 1024,
        },
        EntryPoints::TokenV1MintAndTransferFT,
        EntryPoints::TokenV1MintAndTransferNFTSequential,
        EntryPoints::TokenV2AmbassadorMint { numbered: true },
        EntryPoints::LiquidityPoolSwap { is_stable: true },
        EntryPoints::LiquidityPoolSwap { is_stable: false },
        EntryPoints::CoinInitAndMint,
        EntryPoints::FungibleAssetMint,
        EntryPoints::IncGlobalMilestoneAggV2 { milestone_every: 1 },
        EntryPoints::IncGlobalMilestoneAggV2 { milestone_every: 2 },
        EntryPoints::EmitEvents { count: 1000 },
        EntryPoints::APTPermissionedTransfer,
        EntryPoints::APTTransfer,
    ];

    let mut failures = Vec::new();
    let mut json_lines = Vec::new();

    println!(
        "{:>13} {:>13} {:>13}{:>13} {:>13} {:>13}  entry point",
        "walltime(us)", "expected(us)", "dif(- is impr)", "gas/s", "exe gas", "io gas",
    );

    for (index, entry_point) in entry_points.into_iter().enumerate() {
        let entry_point_name = format!("{:?}", entry_point);
        let expected_time_micros = calibration_values
            .get(&entry_point_name)
            .expect(&entry_point_name)
            .expected_time_micros;
        let publisher = executor.new_account_at(AccountAddress::random());

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
                    &package,
                    init_entry_point.module_name(),
                    Some(&mut rng),
                    Some(publisher.address()),
                ),
            );
        }

        let measurement = execute_and_time_entry_point(
            &entry_point,
            &package,
            publisher.address(),
            &mut executor,
            if expected_time_micros > 10000.0 {
                6
            } else if expected_time_micros > 1000.0 {
                10
            } else {
                100
            },
        );
        let elapsed_micros = measurement.elapsed_micros_f64();
        let diff = (elapsed_micros - expected_time_micros) / expected_time_micros * 100.0;
        let execution_gas_units = measurement.execution_gas_units();
        let io_gas_units = measurement.io_gas_units();
        let gps = (execution_gas_units + io_gas_units) / measurement.elapsed_secs_f64();
        println!(
            "{:13.1} {:13.1} {:12.1}% {:13.0} {:13.2} {:13.2}  {:?}",
            elapsed_micros,
            expected_time_micros,
            diff,
            gps,
            execution_gas_units,
            io_gas_units,
            entry_point
        );

        json_lines.push(json!({
            "grep": "grep_json_aptos_move_vm_perf",
            "transaction_type": entry_point_name,
            "wall_time_us": elapsed_micros,
            "gas_units_per_second": gps,
            "execution_gas_units": execution_gas_units,
            "io_gas_units": io_gas_units,
            "expected_wall_time_us": expected_time_micros,
            "code_perf_version": CODE_PERF_VERSION,
            "test_index": index,
        }));

        if elapsed_micros > expected_time_micros * (1.0 + ALLOWED_REGRESSION) + ABSOLUTE_BUFFER_US {
            failures.push(format!(
                "Performance regression detected: {:.1}us, expected: {:.1}us, diff: {}%, for {:?}",
                elapsed_micros, expected_time_micros, diff, entry_point
            ));
        } else if elapsed_micros + ABSOLUTE_BUFFER_US
            < expected_time_micros * (1.0 - ALLOWED_IMPROVEMENT)
        {
            failures.push(format!(
                "Performance improvement detected: {:.1}us, expected {:.1}us, diff: {}%, for {:?}. You need to adjust expected time!",
                elapsed_micros, expected_time_micros, diff, entry_point
            ));
        }
    }

    for line in json_lines {
        println!("{}", serde_json::to_string(&line).unwrap());
    }

    for failure in &failures {
        println!("{}", failure);
    }
    if !failures.is_empty() {
        println!("Failing, there were perf improvements or regressions.");
        exit(1);
    }

    // Assert there were no error log lines in the run.
    assert_eq!(
        0,
        aptos_logger::ERROR_LOG_COUNT.get(),
        "Error logs were found in the run."
    );
}
